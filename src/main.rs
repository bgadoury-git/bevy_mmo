mod plugin;

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::PresentMode,
};
use avian2d::{math::*, prelude::*};
use plugin::*;
use rand::Rng;

const PLAYER_START: Vec3 = Vec3::ZERO;
const PLAYER_SIZE: f32 = 24.0;
const PLAYER_JUMP_IMPULSE: f32 = 120.0;
const PLAYER_GRAVITY_SCALE: f32 = 4.0;
const BOX_COUNT: usize = 12;
const BOX_SIZE: f32 = 40.0;
const BOX_SPAWN_RADIUS: f32 = 500.0;
const BOX_RESTITUTION: f32 = 0.7;
const FLOOR_RESTITUTION: f32 = 0.7;

#[derive(Component)]
struct PlayerStatsText;

#[derive(Component)]
struct FpsText;

#[derive(Component)]
struct MainCamera;

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
        .add_systems(Startup, (setup_camera, setup_player_stats_ui, setup_fps_ui))
        .add_systems(Startup, (add_player, spawn_boxes, spawn_floor))
        .add_systems(Update, update_fps_ui)
        .add_systems(PostUpdate, (follow_player_camera, update_player_stats_ui))
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct WorldBox;

#[derive(Component)]
struct Name(String);

#[derive(Component)]
struct Health(i32);

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

fn add_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let player_material = materials.add(Color::srgb(0.2, 0.7, 0.9));
    let player_mesh = meshes.add(Circle::new(PLAYER_SIZE));

    commands.spawn((
        Player,
        Name("Elaina Proctor".to_string()),
        Health(100),
        Mesh2d(player_mesh),
        MeshMaterial2d(player_material),
        Transform::from_translation(PLAYER_START),
        CharacterControllerBundle::new(Collider::circle(PLAYER_SIZE)).with_movement(1250.0, 5.0, PLAYER_JUMP_IMPULSE, (30.0 as Scalar).to_radians()),
        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
        Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
        ColliderDensity(2.0),
        GravityScale(PLAYER_GRAVITY_SCALE),
    ));
}

fn spawn_boxes(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<ColorMaterial>>) {
    let box_material = materials.add(Color::srgb(0.95, 0.8, 0.3));
    let box_mesh = meshes.add(Rectangle::new(BOX_SIZE, BOX_SIZE));
    let mut rng = rand::thread_rng();

    for _ in 0..BOX_COUNT {
        let offset = Vec2::new(
            rng.gen_range(-BOX_SPAWN_RADIUS..BOX_SPAWN_RADIUS),
            rng.gen_range(-BOX_SPAWN_RADIUS..BOX_SPAWN_RADIUS),
        );

        commands.spawn((
            WorldBox,
            Mesh2d(box_mesh.clone()),
            MeshMaterial2d(box_material.clone()),
            RigidBody::Dynamic,
            Collider::rectangle(BOX_SIZE, BOX_SIZE),
            Restitution::new(BOX_RESTITUTION).with_combine_rule(CoefficientCombine::Max),
            Transform::from_xyz(PLAYER_START.x + offset.x, PLAYER_START.y + offset.y, -0.1),
        ));
    }
}

fn spawn_floor(mut commands: Commands, _meshes: ResMut<Assets<Mesh>>, _materials: ResMut<Assets<ColorMaterial>>) {
    commands.spawn((
        Sprite {
            color: Color::srgb(0.7, 0.7, 0.8),
            custom_size: Some(Vec2::new(1000.0, 10.0)),
            ..default()
        },
        Transform::from_xyz(0.0, -110.0, 0.0),
        RigidBody::Static,
        Collider::rectangle(1000.0, 10.0),
        Restitution::new(FLOOR_RESTITUTION).with_combine_rule(CoefficientCombine::Max),
    ));
}

fn update_player_stats_ui(
    player_query: Query<(&Name, &Health, &Transform), With<Player>>,
    mut text_query: Query<&mut Text, With<PlayerStatsText>>,
) {
    let Ok((name, health, transform)) = player_query.single() else {
        return;
    };

    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    text.0 = format!(
        "Name: {}\nHealth: {}\nPosition: ({:.1}, {:.1})",
        name.0,
        health.0,
        transform.translation.x,
        transform.translation.y
    );
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
    camera_transform.translation.y = player_transform.translation.y;
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