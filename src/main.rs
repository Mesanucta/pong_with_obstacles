use bevy::{
    prelude::*,
    math::bounding::{Aabb2d, BoundingCircle, BoundingVolume, IntersectsVolume},
    window::{PresentMode, WindowTheme},
    diagnostic::{FrameCount},
};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use rand::Rng;

const PADDLE_SIZE: Vec2 = Vec2::new(20.0, 120.0);
const PADDLE_SPEED: f32 = 500.0;

const DASHEDLINE_SIZE: f32 = 20.;

const BALL_STARTING_POSITION: Vec3 = Vec3::new(-610.0, 0.0, 1.0);
const BALL_SIZE: f32 = 20.;
const BALL_SPEED: f32 = 400.0;
const INITIAL_BALL_DIRECTION: Vec2 = Vec2::new(0.5, -0.5);

const WALL_THICKNESS: f32 = 1.0;
const VERTICAL_WALL_THICKNESS: f32 = 20.0;
const LEFT_WALL: f32 = -640.;
const RIGHT_WALL: f32 = 640.;
const BOTTOM_WALL: f32 = -470.;
const TOP_WALL: f32 = 470.;

const GAP_BETWEEN_PADDLE_AND_SIDES: f32 = 10.0;
const GAP_BETWEEN_DASHEDLINESEGMENTS: f32 = 40.0;

const SCOREBOARD_FONT_SIZE: f32 = 150.0;

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
        .insert_resource(Score(0, 0))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, setup)
        .add_systems(
            FixedUpdate,
            (
                apply_velocity,
                move_paddle,
            ).chain()
        )
        .add_systems(Update, (make_visible, update_scoreboard))
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

#[derive(Resource)]
struct Score(usize, usize);

#[derive(Component)]
struct ScoreboardUi;

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Resource, Deref)]
struct CollisionSound(Handle<AudioSource>);

#[derive(Component, Default)]
struct Collider;

#[derive(Component)]
#[require(Sprite, Transform, Collider)]
struct Wall;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallType {
    Left,
    Right,
    Bottom,
    Top,
}

enum WallLocation {
    Left,
    Right,
    Bottom,
    Top,
}

impl WallLocation {
    // 墙体中心位置
    fn position(&self) -> Vec2 {
        match self {
            WallLocation::Left => Vec2::new(LEFT_WALL, 0.),
            WallLocation::Right => Vec2::new(RIGHT_WALL, 0.),
            WallLocation::Bottom => Vec2::new(0., BOTTOM_WALL),
            WallLocation::Top => Vec2::new(0., TOP_WALL),
        }
    }

    // 墙面尺寸
    fn size(&self) -> Vec2 {
        let arena_height = TOP_WALL - BOTTOM_WALL;
        let arena_width = RIGHT_WALL - LEFT_WALL;

        assert!(arena_height > 0.0);
        assert!(arena_width > 0.0);

        match self {
            WallLocation::Left | WallLocation::Right => {
                Vec2::new(WALL_THICKNESS, arena_height + VERTICAL_WALL_THICKNESS)
            }
            WallLocation::Bottom | WallLocation::Top => {
                Vec2::new(arena_width + WALL_THICKNESS, VERTICAL_WALL_THICKNESS)
            }
        }
    }
}

impl Wall {
    fn new(location: WallLocation) -> (Wall, WallType, Sprite, Transform) {
        // 上下墙白色，左右墙不可见
        let color = match location{
            WallLocation::Left | WallLocation::Right => {
                Color::NONE
            }
            WallLocation::Bottom | WallLocation::Top => {
                Color::WHITE
            }
        };
        let walltype = match location{
            WallLocation::Left => {
                WallType::Left
            }
            WallLocation::Right => {
                WallType::Right
            }
            WallLocation::Top => {
                WallType::Top
            }
            WallLocation::Bottom => {
                WallType::Bottom
            }
        };
        (
            Wall,
            walltype,
            Sprite::from_color(color, Vec2::ONE),
            Transform {
                translation: location.position().extend(0.0),
                scale: location.size().extend(1.0),
                ..default()
            },
        )
    }
}

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

    // Walls
    commands.spawn(Wall::new(WallLocation::Left));
    commands.spawn(Wall::new(WallLocation::Right));
    commands.spawn(Wall::new(WallLocation::Bottom));
    commands.spawn(Wall::new(WallLocation::Top));

    // Ball
    let starting_position = rand::rng().random_range(-450.0..=450.0);
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(BALL_SIZE, BALL_SIZE))),
        MeshMaterial2d(materials.add(Color::WHITE)),
        Transform::from_translation(BALL_STARTING_POSITION + Vec3::new(0.0, starting_position, 0.0))
            .with_scale(Vec3::ONE),
        Ball,
        Velocity(INITIAL_BALL_DIRECTION.normalize() * BALL_SPEED),
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

    // Scoreboard
    let scoreboard_font = asset_server.load("fonts/Bit3.ttf");
    commands.spawn((
        Text::new(""),
        ScoreboardUi,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            left: Val::Px(520.0),
            ..default()
        },
        children![(
            TextSpan::default(),
            TextFont {
                font: scoreboard_font.clone(),
                font_size: SCOREBOARD_FONT_SIZE,
                ..default()
            },
            TextColor(Color::WHITE),
        )],
    ));
    commands.spawn((
        Text::new(""),
        ScoreboardUi,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            right: Val::Px(510.0),
            ..default()
        },
        children![(
            TextSpan::default(),
            TextFont {
                font: scoreboard_font.clone(),
                font_size: SCOREBOARD_FONT_SIZE,
                ..default()
            },
            TextColor(Color::WHITE),
        )],
    ));
}

fn update_scoreboard(
    score: Res<Score>,
    mut scoreboards: Query<Entity, (With<ScoreboardUi>, With<Text>)>,
    mut writer: TextUiWriter,
) {
    let entities = scoreboards.iter_mut().collect::<Vec<_>>();
    if entities.len() == 2 {
        *writer.text(entities[0], 1) = score.0.to_string();
        *writer.text(entities[1], 1) = score.1.to_string();
    }
}

fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.x * time.delta_secs();
        transform.translation.y += velocity.y * time.delta_secs();
    }
}

fn move_paddle(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &PaddleType), With<Paddle>>,
    time: Res<Time>,
) {
    let top_bound = TOP_WALL - WALL_THICKNESS / 2.0 - PADDLE_SIZE.y / 2.0;
    let bottom_bound = BOTTOM_WALL + WALL_THICKNESS / 2.0 + PADDLE_SIZE.y / 2.0;
    let mut directions = (0.0, 0.0);

    if keyboard_input.pressed(KeyCode::KeyW) {
        directions.0 += 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        directions.0 -= 1.0;
    }

    if keyboard_input.pressed(KeyCode::ArrowUp) {
        directions.1 += 1.0;
    }
    if keyboard_input.pressed(KeyCode::ArrowDown) {
        directions.1 -= 1.0;
    }

    for (mut paddle_transform, paddle_type) in query.iter_mut(){
        let direction = match paddle_type {
            PaddleType::Left => directions.0,
            PaddleType::Right => directions.1
        };
        let new_paddle_position = paddle_transform.translation.y + direction * PADDLE_SPEED * time.delta_secs();
        paddle_transform.translation.y = new_paddle_position.clamp(bottom_bound, top_bound);
    }

}


#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum Collision {
    Left,
    Right,
    Top,
    Bottom,
}

fn ball_collision(ball: BoundingCircle, bounding_box: Aabb2d) -> Option<Collision> {
    if !ball.intersects(&bounding_box) {
        return None;
    }

    let closest = bounding_box.closest_point(ball.center());
    let offset = ball.center() - closest;
    let side = if offset.x.abs() > offset.y.abs() {
        if offset.x < 0. {
            Collision::Left
        } else {
            Collision::Right
        }
    } else if offset.y > 0. {
        Collision::Top
    } else {
        Collision::Bottom
    };

    Some(side)
}