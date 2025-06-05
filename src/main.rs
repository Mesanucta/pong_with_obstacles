use bevy::{
    prelude::*,
    window::{PresentMode, WindowTheme},
    diagnostic::{FrameCount},
};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};

const PADDLE_SIZE: Vec2 = Vec2::new(20.0, 120.0);

const DASHEDLINE_SIZE: f32 = 20.;

const LEFT_WALL: f32 = -640.;
const RIGHT_WALL: f32 = 640.;
const BOTTOM_WALL: f32 = -470.;
const TOP_WALL: f32 = 470.;

const GAP_BETWEEN_PADDLE_AND_SIDES: f32 = 10.0;
const GAP_BETWEEN_DASHEDLINESEGMENTS: f32 = 40.0;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Pong With Obstacles".into(),
                    name: Some("bevy.app".into()),
                    resolution: (1280., 960.).into(),
                    present_mode: PresentMode::AutoVsync,
                    window_theme: Some(WindowTheme::Dark),
                    resizable: false,
                    enabled_buttons: bevy::window::EnabledButtons {
                        maximize: false,
                        ..Default::default()
                    },
                    visible: false,
                    ..default()
                }),
                ..default()
            }),
        ))
        .add_plugins(EguiPlugin { enable_multipass_for_primary_context: true })
        .add_plugins(WorldInspectorPlugin::new())
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, setup)
        .add_systems(Update, make_visible)
        .run();
}

fn make_visible(mut window: Single<&mut Window>, frames: Res<FrameCount>){
    if frames.0 == 3{
        window.visible = true;
    }
}

#[derive(Component, PartialEq, Eq)]
enum PaddleType {
    Left,
    Right,
}

#[derive(Component)]
struct Paddle;

#[derive(Component)]
struct Ball;

#[derive(Component)]
struct DashedLineSegment;

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Resource, Deref)]
struct CollisionSound(Handle<AudioSource>);

#[derive(Component, Default)]
struct Collider;

#[derive(Component)]
#[require(Sprite, Transform, Collider)]
struct Wall;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Camera
    commands.spawn(Camera2d);

    // Sound
    let ball_collision_sound = asset_server.load("sounds/pong_collision.ogg");
    commands.insert_resource(CollisionSound(ball_collision_sound));

    // Paddle 1
    commands.spawn((
        Sprite::from_color(Color::WHITE, Vec2::ONE),
        Transform {
            translation: Vec3::new(LEFT_WALL + GAP_BETWEEN_PADDLE_AND_SIDES, 0.0, 0.0),
            scale: PADDLE_SIZE.extend(1.0),
            ..default()
        },
        Paddle,
        PaddleType::Left,
        Collider,
    ));

    // Paddle 2
    commands.spawn((
        Sprite::from_color(Color::WHITE, Vec2::ONE),
        Transform {
            translation: Vec3::new(RIGHT_WALL - GAP_BETWEEN_PADDLE_AND_SIDES, 0.0, 0.0),
            scale: PADDLE_SIZE.extend(1.0),
            ..default()
        },
        Paddle,
        PaddleType::Right,
        Collider,
    ));

    // DashedLineSegment
    let center_line_start = Vec3::new(0.0, TOP_WALL, 0.0);
    let center_line_end = Vec3::new(0.0, BOTTOM_WALL, 0.0);
    let total_length = center_line_start.distance(center_line_end);
    let mut offset = 10.0;
    while offset < total_length{
        let position = center_line_start + Vec3::new(0.0, -offset, 0.0);
        commands.spawn((
            Mesh2d(meshes.add(Rectangle::new(DASHEDLINE_SIZE, DASHEDLINE_SIZE))),
            MeshMaterial2d(materials.add(Color::WHITE)),
            Transform::from_translation(position)
                .with_scale(Vec3::ONE),
            DashedLineSegment,
        ));
        offset += GAP_BETWEEN_DASHEDLINESEGMENTS;
    }

}