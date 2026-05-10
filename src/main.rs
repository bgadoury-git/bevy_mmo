mod plugin;

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin}, prelude::*, window::PresentMode
};
use avian2d::{math::*, prelude::*};
use plugin::*;
use rand::Rng;

const PLAYER_START: Vec3 = Vec3::ZERO;
const BASE_PLAYER_HEALTH: i32 = 32; // surface area in units² (π·r²)
const PLAYER_JUMP_IMPULSE: f32 = 120.0;
const PLAYER_GRAVITY_SCALE: f32 = 4.0;
const BOX_COUNT: usize = 30000;
const BASE_BOX_HEALTH: i32 = 16; // surface area in units² (side²)
const BOX_RESTITUTION: f32 = 0.7;
const FLOOR_RESTITUTION: f32 = 0.7;
const COLOR_PALETTE_STEPS: usize = 20;
const BOX_DAMAGE_RATIO: f32 = 0.1; // fraction of own max HP dealt per collision
const PLAYER_DAMAGE_RATIO: f32 = 0.2; 
const PLAYER_MOVEMENT_ACCELERATION: f32 = 1250.0;
const PLAYER_MOVEMENT_DAMPING: f32 = 5.0;
const PLAYER_SLOPE_ANGLE_DEGREES: f32 = 30.0;
const PLAYER_COLLIDER_DENSITY: f32 = 2.0;
const BOX_IMPULSE_MIN_INTERVAL: f32 = 1.0;
const BOX_IMPULSE_MAX_INTERVAL: f32 = 5.0;
const BOX_IMPULSE_SPEED: f32 = 100.0;
const ARENA_WIDTH: f32 = 10000.0;
const ARENA_HEIGHT: f32 = 1000.0;
const ARENA_WALL_THICKNESS: f32 = 10.0;

#[derive(Component)]
struct PlayerStatsText;

#[derive(Component)]
struct FpsText;

#[derive(Component)]
struct BoxCountText;

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct WorldBox;

#[derive(Component)]
struct PlayerName(String);

#[derive(Component)]
struct Health(i32);

#[derive(Component)]
struct Damage(f32); // ratio of own max HP dealt per collision

#[derive(Component)]
struct TookDamage(i32);

#[derive(Component)]
struct BoxSize(f32);

#[derive(Component)]
struct LastDamagedBy(Entity);

#[derive(Component)]
struct GrowBy(i32);

#[derive(Component)]
struct MaxHealth(i32);

#[derive(Component)]
struct PlayerRadius(f32);

/// Pre-built palette of `COLOR_PALETTE_STEPS + 1` materials ranging from
/// black (index 0, 0 HP) to yellow (last index, full HP). Boxes swap to the
/// nearest handle so Bevy can batch all boxes in the same bucket into a
/// single draw call — equivalent to UE5 material instances.
#[derive(Resource)]
struct BoxColorPalette(Vec<Handle<ColorMaterial>>);

#[derive(Resource, Default)]
struct KillGrowthQueue(Vec<(Entity, i32)>);

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin::default(),
            PhysicsPlugins::default().with_length_unit(20.0),
            CharacterControllerPlugin))
        .init_resource::<BoxVelocityTimer>()
        .init_resource::<KillGrowthQueue>()
        .add_systems(Startup, (setup_camera, setup_player_stats_ui, setup_fps_ui, setup_box_count_ui))
        .add_systems(Startup, setup_box_color_palette)
        .add_systems(Startup, (add_player, spawn_boxes, spawn_floor).after(setup_box_color_palette))
        .add_systems(Update, (follow_player_camera, update_fps_ui, update_box_count_ui, (apply_random_impulse_to_boxes, collision_damage, apply_damage_to_entity, assign_kill_growth, apply_box_growth, apply_player_growth, update_entity_color_based_on_health).chain()))
        .add_systems(PostUpdate, update_player_stats_ui)
        .run();
}



fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, MainCamera));
}

fn setup_player_stats_ui(mut commands: Commands) {
    commands.spawn((
        PlayerStatsText,
        Text::new("Player stats loading..."),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: px(12.0),
            left: px(12.0),
            ..default()
        },
    ));
}

fn setup_fps_ui(mut commands: Commands) {
    commands.spawn((
        FpsText,
        Text::new("FPS: --"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: px(12.0),
            right: px(12.0),
            ..default()
        },
    ));
}

fn setup_box_count_ui(mut commands: Commands) {
    commands.spawn((
        BoxCountText,
        Text::new("Boxes: --"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: px(42.0),
            right: px(12.0),
            ..default()
        },
    ));
}

fn add_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let player_material = materials.add(Color::srgb(0.0, 1.0, 0.0));
    let player_radius = (BASE_PLAYER_HEALTH as f32 / std::f32::consts::PI).sqrt();
    let player_mesh = meshes.add(Circle::new(player_radius));

    commands.spawn((
        Player,
        PlayerName("Elaina Proctor".to_string()),
        Health(BASE_PLAYER_HEALTH),
        MaxHealth(BASE_PLAYER_HEALTH),
        CollisionEventsEnabled,
        PlayerRadius(player_radius),
        Damage(BASE_PLAYER_HEALTH as f32 * PLAYER_DAMAGE_RATIO),
        Mesh2d(player_mesh),
        MeshMaterial2d(player_material),
        Transform::from_translation(PLAYER_START),
        CharacterControllerBundle::new(Collider::circle(player_radius)).with_movement(PLAYER_MOVEMENT_ACCELERATION, PLAYER_MOVEMENT_DAMPING, PLAYER_JUMP_IMPULSE, (PLAYER_SLOPE_ANGLE_DEGREES as Scalar).to_radians()),
        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
        Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
        ColliderDensity(PLAYER_COLLIDER_DENSITY),
        GravityScale(PLAYER_GRAVITY_SCALE),
    ));
}

fn spawn_boxes(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, palette: Res<BoxColorPalette>) {
    let box_size = (BASE_BOX_HEALTH as f32).sqrt();
    let box_mesh = meshes.add(Rectangle::new(box_size, box_size));
    let full_health_mat = palette.0[COLOR_PALETTE_STEPS].clone();
    let mut rng = rand::thread_rng();

    let spawn_half_w = ARENA_WIDTH / 2.0 - ARENA_WALL_THICKNESS - box_size / 2.0;
    let spawn_half_h = ARENA_HEIGHT / 2.0 - ARENA_WALL_THICKNESS - box_size / 2.0;

    let slot_width = (2.0 * spawn_half_w) / BOX_COUNT as f32;

    for i in 0..BOX_COUNT {
        let x = -spawn_half_w + (i as f32 + 0.5) * slot_width;
        let offset = Vec2::new(x, rng.gen_range(-spawn_half_h..spawn_half_h));

        commands.spawn((
            WorldBox,
            Mesh2d(box_mesh.clone()),
            MeshMaterial2d(full_health_mat.clone()),
            RigidBody::Dynamic,
            CollisionEventsEnabled,
            Collider::rectangle(box_size, box_size),
            Restitution::new(BOX_RESTITUTION).with_combine_rule(CoefficientCombine::Max),
            Transform::from_xyz(PLAYER_START.x + offset.x, PLAYER_START.y + offset.y, 0.0),
            LinearVelocity::default(),
            Health(BASE_BOX_HEALTH),
            MaxHealth(BASE_BOX_HEALTH),
            Damage(BASE_BOX_HEALTH as f32 * BOX_DAMAGE_RATIO),
            BoxSize(box_size),
        ));
    }
}

fn spawn_floor(mut commands: Commands) {
    let half_w = ARENA_WIDTH / 2.0;
    let half_h = ARENA_HEIGHT / 2.0;
    let wall_color = Color::srgb(0.7, 0.7, 0.8);

    // Bottom
    commands.spawn((
        Sprite { color: wall_color, custom_size: Some(Vec2::new(ARENA_WIDTH, ARENA_WALL_THICKNESS)), ..default() },
        Transform::from_xyz(0.0, -half_h, 0.0),
        RigidBody::Static,
        Collider::rectangle(ARENA_WIDTH, ARENA_WALL_THICKNESS),
        Restitution::new(FLOOR_RESTITUTION).with_combine_rule(CoefficientCombine::Max),
    ));

    // Top
    commands.spawn((
        Sprite { color: wall_color, custom_size: Some(Vec2::new(ARENA_WIDTH, ARENA_WALL_THICKNESS)), ..default() },
        Transform::from_xyz(0.0, half_h, 0.0),
        RigidBody::Static,
        Collider::rectangle(ARENA_WIDTH, ARENA_WALL_THICKNESS),
        Restitution::new(FLOOR_RESTITUTION).with_combine_rule(CoefficientCombine::Max),
    ));

    // Right
    commands.spawn((
        Sprite { color: wall_color, custom_size: Some(Vec2::new(ARENA_WALL_THICKNESS, ARENA_HEIGHT)), ..default() },
        Transform::from_xyz(half_w, 0.0, 0.0),
        RigidBody::Static,
        Collider::rectangle(ARENA_WALL_THICKNESS, ARENA_HEIGHT),
        Restitution::new(FLOOR_RESTITUTION).with_combine_rule(CoefficientCombine::Max),
    ));

    // Left
    commands.spawn((
        Sprite { color: wall_color, custom_size: Some(Vec2::new(ARENA_WALL_THICKNESS, ARENA_HEIGHT)), ..default() },
        Transform::from_xyz(-half_w, 0.0, 0.0),
        RigidBody::Static,
        Collider::rectangle(ARENA_WALL_THICKNESS, ARENA_HEIGHT),
        Restitution::new(FLOOR_RESTITUTION).with_combine_rule(CoefficientCombine::Max),
    ));
}

#[derive(Resource)]
struct BoxVelocityTimer(Timer);

impl Default for BoxVelocityTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(rand::thread_rng().gen_range(BOX_IMPULSE_MIN_INTERVAL..BOX_IMPULSE_MAX_INTERVAL), TimerMode::Once))
    }
}

fn apply_random_impulse_to_boxes(
    mut timer: ResMut<BoxVelocityTimer>,
    time: Res<Time>,
    mut box_query: Query<&mut LinearVelocity, With<WorldBox>>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let mut rng = rand::thread_rng();
    for mut velocity in &mut box_query {
        velocity.x += rng.gen_range(-BOX_IMPULSE_SPEED..BOX_IMPULSE_SPEED);
        velocity.y += rng.gen_range(-BOX_IMPULSE_SPEED..BOX_IMPULSE_SPEED);
    }

    timer.0 = Timer::from_seconds(rng.gen_range(BOX_IMPULSE_MIN_INTERVAL..BOX_IMPULSE_MAX_INTERVAL), TimerMode::Once);
}

fn update_player_stats_ui(
    player_query: Query<(&PlayerName, &Health, &MaxHealth, &Transform), With<Player>>,
    mut text_query: Query<&mut Text, With<PlayerStatsText>>,
) {
    let Ok((name, health, max_health, transform)) = player_query.single() else {
        return;
    };

    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    text.0 = format!(
        "Name: {}\nHealth: {}/{}\nPosition: ({:.1}, {:.1})",
        name.0,
        health.0,
        max_health.0,
        transform.translation.x,
        transform.translation.y
    );
}

fn update_fps_ui(
    diagnostics: Res<DiagnosticsStore>,
    mut text_query: Query<&mut Text, With<FpsText>>,
) {
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|metric| metric.smoothed());

    if let Some(fps) = fps {
        text.0 = format!("FPS: {:.0}", fps);
    } else {
        text.0 = "FPS: --".to_string();
    }
}

fn update_box_count_ui(
    mut box_count: Local<usize>,
    added: Query<(), Added<WorldBox>>,
    mut removed: RemovedComponents<WorldBox>,
    mut text_query: Query<&mut Text, With<BoxCountText>>,
) {
    let added_count = added.iter().count();
    let removed_count = removed.read().count();

    if added_count == 0 && removed_count == 0 {
        return;
    }

    *box_count += added_count;
    *box_count = box_count.saturating_sub(removed_count);

    let Ok(mut text) = text_query.single_mut() else {
        return;
    };
    text.0 = format!("Boxes: {}", *box_count);
}

fn collision_damage(
    mut collision_events: MessageReader<CollisionStart>,
    damage_query: Query<&Damage>,
    health_query: Query<Entity, With<Health>>,
    took_damage_query: Query<&TookDamage>,
    mut commands: Commands,
    mut damage_acc: Local<std::collections::HashMap<Entity, std::collections::HashMap<Entity, i32>>>,
) {
    damage_acc.clear();

    for event in collision_events.read() {
        for (damager, target) in [(event.collider1, event.collider2), (event.collider2, event.collider1)] {
            if let (Ok(damage), Ok(_)) = (damage_query.get(damager), health_query.get(target)) {
                *damage_acc.entry(target).or_default().entry(damager).or_insert(0) += damage.0 as i32;
            }
        }
    }

    for (target, damager_map) in damage_acc.iter() {
        let total_damage: i32 = damager_map.values().sum();
        // Kill credit goes to the entity that dealt the most cumulative damage this collision event.
        let top_damager = damager_map.iter().max_by_key(|(_, v)| *v).map(|(&e, _)| e);
        if let Some(top_damager) = top_damager {
            let existing = took_damage_query.get(*target).map(|td| td.0).unwrap_or(0);
            commands.entity(*target)
                .insert(TookDamage(existing + total_damage))
                .insert(LastDamagedBy(top_damager));
        }
    }
}

/// Applies damage to all entities that took a hit this frame, despawns those
/// that reach 0 HP, and queues growth credit for their killers.
fn apply_damage_to_entity(
    mut commands: Commands,
    mut query: Query<(Entity, &TookDamage, &mut Health, Option<&MaxHealth>, Option<&LastDamagedBy>)>,
    mut kill_queue: ResMut<KillGrowthQueue>,
) {
    kill_queue.0.clear();
    let mut dying: std::collections::HashSet<Entity> = std::collections::HashSet::new();

    for (entity, took_damage, mut health, max_health, last_damaged_by) in &mut query {
        health.0 -= took_damage.0;
        commands.entity(entity).remove::<TookDamage>().remove::<LastDamagedBy>();
        if health.0 <= 0 {
            if let (Some(MaxHealth(pool)), Some(LastDamagedBy(killer))) = (max_health, last_damaged_by) {
                kill_queue.0.push((*killer, *pool));
            }
            dying.insert(entity);
            commands.entity(entity).despawn();
        }
    }

    // Drop credit for killers that also died this frame.
    kill_queue.0.retain(|(killer, _)| !dying.contains(killer));
}

/// Distributes growth (absorbed HP pool) from `kill_queue` to the winning killers.
fn assign_kill_growth(
    mut commands: Commands,
    kill_queue: Res<KillGrowthQueue>,
    grow_query: Query<&GrowBy>,
) {
    for &(killer, pool) in &kill_queue.0 {
        if let Ok(mut ec) = commands.get_entity(killer) {
            let existing = grow_query.get(killer).map(|g| g.0).unwrap_or(0);
            ec.insert(GrowBy(existing + pool));
        }
    }
}

fn apply_box_growth(
    mut commands: Commands,
    mut query: Query<(Entity, &GrowBy, &mut BoxSize, &mut Transform, &mut Health, &mut MaxHealth, &mut Damage)>,
    mut meshes: ResMut<Assets<Mesh>>,
    palette: Res<BoxColorPalette>,
) {
    for (entity, grow_by, mut box_size, mut transform, mut health, mut max_health, mut damage) in &mut query {
        let new_max = max_health.0 + grow_by.0;
        let new_side = (new_max as f32).sqrt();
        box_size.0 = new_side;
        max_health.0 = new_max;
        health.0 = (health.0 + grow_by.0).min(new_max);
        damage.0 = max_health.0 as f32 * BOX_DAMAGE_RATIO;
        transform.scale = Vec3::ONE;
        commands.entity(entity)
            .insert(Mesh2d(meshes.add(Rectangle::new(new_side, new_side))))
            .insert(MeshMaterial2d(palette.0[COLOR_PALETTE_STEPS].clone()))
            .insert(Collider::rectangle(new_side, new_side))
            .remove::<GrowBy>();
    }
}

fn apply_player_growth(
    mut commands: Commands,
    mut query: Query<(Entity, &GrowBy, &mut PlayerRadius, &mut Health, &mut MaxHealth, &mut Damage), With<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (entity, grow_by, mut radius, mut health, mut max_health, mut damage) in &mut query {
        let new_max = max_health.0 + grow_by.0;
        let new_radius = (new_max as f32 / std::f32::consts::PI).sqrt();
        radius.0 = new_radius;
        max_health.0 = new_max;
        health.0 = (health.0 + grow_by.0).min(new_max);
        damage.0 = max_health.0 as f32 * PLAYER_DAMAGE_RATIO;
        let mut caster_shape = Collider::circle(new_radius as Scalar);
        caster_shape.set_scale(Vector::ONE * 0.99, 10);
        commands.entity(entity)
            .insert(Mesh2d(meshes.add(Circle::new(new_radius))))
            .insert(Collider::circle(new_radius as Scalar))
            .insert(ShapeCaster::new(caster_shape, Vector::ZERO, 0.0, Dir2::NEG_Y).with_max_distance(10.0))
            .remove::<GrowBy>();
    }
}

fn update_entity_color_based_on_health(
    mut params: ParamSet<(
        Query<(&Health, &MaxHealth, &mut MeshMaterial2d<ColorMaterial>), (With<WorldBox>, Changed<Health>)>,
        Query<(&Health, &MaxHealth, &MeshMaterial2d<ColorMaterial>), With<Player>>,
    )>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    palette: Res<BoxColorPalette>,
) {
    for (health, max_health, mut mat_handle) in &mut params.p0() {
        let ratio = (health.0 as f32 / max_health.0 as f32).clamp(0.0, 1.0);
        let idx = ((ratio * COLOR_PALETTE_STEPS as f32).round() as usize).min(COLOR_PALETTE_STEPS);
        mat_handle.0 = palette.0[idx].clone();
    }

    for (health, max_health, material_handle) in &params.p1() {
        if let Some(material) = materials.get_mut(material_handle) {
            let health_ratio = (health.0 as f32 / max_health.0 as f32).clamp(0.0, 1.0);
            // full HP → green, 0 HP → black
            material.color = Color::srgb(0.0, health_ratio, 0.0);
        }
    }
}

fn setup_box_color_palette(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    let palette = (0..=COLOR_PALETTE_STEPS)
        .map(|i| {
            let t = i as f32 / COLOR_PALETTE_STEPS as f32;
            // t=0 → black (dead), t=1 → yellow (full HP)
            materials.add(Color::srgb(t, t, 0.0))
        })
        .collect();
    commands.insert_resource(BoxColorPalette(palette));
}

fn follow_player_camera(
    player_query: Query<&Transform, (With<Player>, Without<MainCamera>)>,
    mut camera_query: Query<&mut Transform, (With<MainCamera>, Without<Player>)>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    camera_transform.translation.x = player_transform.translation.x;
    //camera_transform.translation.y = player_transform.translation.y;
}