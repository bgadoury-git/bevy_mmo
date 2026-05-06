use bevy::prelude::*;
use rand::Rng;

const PLAYER_START: Vec3 = Vec3::ZERO;
const PLAYER_SIZE: f32 = 24.0;
const MOVE_SPEED: f32 = 6.0;
const BOX_COUNT: usize = 12;
const BOX_SIZE: f32 = 40.0;
const BOX_SPAWN_RADIUS: f32 = 500.0;

#[derive(Component)]
struct PlayerStatsText;

#[derive(Component)]
struct MainCamera;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_camera, setup_player_stats_ui))
        .add_systems(Startup, (add_player, spawn_boxes))
        .add_systems(Update, (move_player, follow_player_camera, update_player_stats_ui))
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

#[derive(Component)]
struct Position(f32, f32);

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
        Position(PLAYER_START.x, PLAYER_START.y),
        Mesh2d(player_mesh),
        MeshMaterial2d(player_material),
        Transform::from_translation(PLAYER_START),
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
            Transform::from_xyz(PLAYER_START.x + offset.x, PLAYER_START.y + offset.y, -0.1),
        ));
    }
}

fn move_player(
    mut players: Query<(&mut Transform, &mut Position), With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    for (mut transform, mut position) in players.iter_mut() {
        let mut direction = Vec3::ZERO;

        if keys.pressed(KeyCode::KeyW) {
            direction.y += 1.0;
        }

        if keys.pressed(KeyCode::KeyA) {
            direction.x -= 1.0;
        }

        if keys.pressed(KeyCode::KeyS) {
            direction.y -= 1.0;
        }

        if keys.pressed(KeyCode::KeyD) {
            direction.x += 1.0;
        }

        if 0.0 < direction.length() {
            transform.translation += MOVE_SPEED * direction.normalize();
            position.0 = transform.translation.x;
            position.1 = transform.translation.y;
        }
    }
}

fn update_player_stats_ui(
    player_query: Query<(&Name, &Health, &Position), With<Player>>,
    mut text_query: Query<&mut Text, With<PlayerStatsText>>,
) {
    let Ok((name, health, position)) = player_query.single() else {
        return;
    };

    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    text.0 = format!(
        "Name: {}\nHealth: {}\nPosition: ({:.1}, {:.1})",
        name.0, health.0, position.0, position.1
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