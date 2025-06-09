use bevy::{
    diagnostic::FrameCount, 
    math::bounding::{Aabb2d, BoundingCircle, BoundingVolume, IntersectsVolume}, 
    prelude::*, 
    window::{PresentMode, WindowTheme}
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
const VICTORY_TEXT_FONT_SIZE: f32 = 150.0;
const HINT_FONT_SIZE: f32 = 50.0;

const TARGET_SCORE: usize = 9;

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
        .init_state::<GameState>()
        .insert_resource(Winner::default())
        .insert_resource(Score(0, 0))
        .insert_resource(ClearColor(Color::BLACK))
        .add_event::<CollisionEvent>()
        .add_event::<ScoreEvent>()
        .init_state::<GameState>()
        .enable_state_scoped_entities::<GameState>()
        .add_systems(Startup, setup)
        .add_systems(OnEnter(GameState::Playing), game_reset)
        .add_systems(
            FixedUpdate,
            (
                apply_velocity,
                move_paddle,
                check_for_collisions,
                play_collision_sound,
                ball_reset,
            ).chain().run_if(in_state(GameState::Playing))
        )
        .add_systems(
            Update,
            (
                make_window_visible, 
                update_scoreboard,
            )
        )
        .add_systems(OnEnter(GameState::GameOver), display_winner)
        .add_systems(
            Update,
            game_over_keyboard.run_if(in_state(GameState::GameOver)
))
        .run();
}

//等待渲染，延迟3帧窗口可见
fn make_window_visible(mut window: Single<&mut Window>, frames: Res<FrameCount>){
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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
enum GameState {
    #[default]
    Playing,
    GameOver, // 存储胜利方
}

#[derive(Resource, Default)]
struct Winner(Option<PaddleType>);

#[derive(Component)]
struct VictoryText;

#[derive(Component)]
struct TextBackground;

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Event, Default)]
struct CollisionEvent;

#[derive(Event, Default)]
enum ScoreEvent {
    #[default]
    Player1Scored,
    Player2Scored,
}

#[derive(Resource, Deref)]
struct CollisionSound(Handle<AudioSource>);

#[derive(Resource, Deref)]
struct ScoreSound(Handle<AudioSource>);

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

    let score_sound = asset_server.load("sounds/score.ogg");
    commands.insert_resource(ScoreSound(score_sound));

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
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(BALL_SIZE, BALL_SIZE))),
        MeshMaterial2d(materials.add(Color::WHITE)),
        Transform::from_translation(BALL_STARTING_POSITION)
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
    mut scoreboards: Query<Entity, (With<ScoreboardUi>, With<Text>, Without<VictoryText>)>,
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
    let mut accelerate_factors = (1.0, 1.0);

    if keyboard_input.pressed(KeyCode::KeyW) {
        directions.0 += 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        directions.0 -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::ShiftLeft) {
        accelerate_factors.0 += 1.0;
    }

    if keyboard_input.pressed(KeyCode::ArrowUp) {
        directions.1 += 1.0;
    }
    if keyboard_input.pressed(KeyCode::ArrowDown) {
        directions.1 -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::NumpadEnter) {
        accelerate_factors.1 += 1.0;
    }
    for (mut paddle_transform, paddle_type) in query.iter_mut(){
        let (direction, accelerate_fact) = match paddle_type {
            PaddleType::Left => (directions.0, accelerate_factors.0),
            PaddleType::Right => (directions.1, accelerate_factors.1)
        };
        let new_paddle_position = paddle_transform.translation.y + direction * PADDLE_SPEED * accelerate_fact * time.delta_secs();
        paddle_transform.translation.y = new_paddle_position.clamp(bottom_bound, top_bound);
    }
}

fn check_for_collisions(
    mut score: ResMut<Score>,
    mut winner: ResMut<Winner>,
    mut next_state: ResMut<NextState<GameState>>,
    ball_query: Single<(&mut Velocity, &Transform), With<Ball>>,
    collider_query: Query<(&Transform, Option<&WallType>, Option<&Paddle>), With<Collider>>,
    mut collision_events: EventWriter<CollisionEvent>,
    mut score_events: EventWriter<ScoreEvent>,
) {
    let (mut ball_velocity, ball_transform) = ball_query.into_inner();

    for (collider_transform, maybe_wall_type, maybe_paddle) in &collider_query {
        let collision = ball_collision(
            BoundingCircle::new(ball_transform.translation.truncate(), BALL_SIZE / 2.),
            Aabb2d::new(
                collider_transform.translation.truncate(),
                collider_transform.scale.truncate() / 2.,
            ),
        );

        if let Some(collision) = collision {
            if let Some(wall_type) = maybe_wall_type {
                match wall_type {
                    WallType::Right => {
                        score.0 += 1;
                        score_events.write(ScoreEvent::Player1Scored);
                        if score.0 >= TARGET_SCORE {
                            winner.0 = Some(PaddleType::Left);
                            next_state.set(GameState::GameOver);
                        }
                        continue;
                    }
                    WallType::Left => {
                        score.1 += 1;
                        score_events.write(ScoreEvent::Player2Scored);
                        if score.1 >= TARGET_SCORE {
                            winner.0 = Some(PaddleType::Right);
                            next_state.set(GameState::GameOver);
                        }
                        continue;
                    }
                    WallType::Top | WallType::Bottom => {collision_events.write_default();}
                }
            } else{
                collision_events.write_default();
            }

            // 每次成功接球后，球速加到1.2倍
            if maybe_paddle.is_some(){
                ball_velocity.x *= 1.2;
                ball_velocity.y *= 1.2;
            }
            
            let mut reflect_x = false;
            let mut reflect_y = false;

            match collision {
                Collision::Left => reflect_x = ball_velocity.x > 0.0,
                Collision::Right => reflect_x = ball_velocity.x < 0.0,
                Collision::Top => reflect_y = ball_velocity.y < 0.0,
                Collision::Bottom => reflect_y = ball_velocity.y > 0.0,
            }

            if reflect_x {
                ball_velocity.x = -ball_velocity.x;
            }
            if reflect_y {
                ball_velocity.y = -ball_velocity.y;
            }
        }
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

fn play_collision_sound(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    score_events: EventReader<ScoreEvent>,
    collision_sound: Res<CollisionSound>,
    score_sound: Res<ScoreSound>,
) {
    if !collision_events.is_empty() {
        collision_events.clear();
        commands.spawn((AudioPlayer(collision_sound.clone()), PlaybackSettings::DESPAWN));
    }
    if !score_events.is_empty() {
        commands.spawn((AudioPlayer(score_sound.clone()), PlaybackSettings::DESPAWN));
    }
}

fn ball_reset(
    ball_query: Single<(&mut Velocity, &mut Transform), With<Ball>>,
    mut score_events: EventReader<ScoreEvent>,
) {
    if !score_events.is_empty() {
        score_events.clear();
        let (mut ball_velocity, mut ball_transform) = ball_query.into_inner();
        
        let sign  = if rand::rng().random_bool(0.5) { 1.0 } else { -1.0 };
        let temp_num = sign * rand::rng().random_range(0.1..=0.5);
        ball_velocity.y = ball_velocity.x * temp_num; // 随机发球角度

        **ball_velocity = ball_velocity.normalize() * BALL_SPEED; // 恢复球速
        
        if ball_transform.translation.x > 0.0 {
            ball_transform.translation.x = LEFT_WALL + 40.0;
        } else {
            ball_transform.translation.x = RIGHT_WALL - 40.0;
        }

        ball_transform.translation.y = 0.0;
    }
}

fn display_winner(
    mut commands: Commands, 
    winner: Res<Winner>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let victory_font = asset_server.load("fonts/Bit3.ttf");

    let message = match winner.0 {
            Some(PaddleType::Left) => "PLAYER 1 WIN!",
            Some(PaddleType::Right) => "PLAYER 2 WIN!",
            _ => "GAME OVER!",
        };

    // 文本背景框
    commands.spawn((
        StateScoped(GameState::GameOver),
        Mesh2d(meshes.add(Rectangle::new(1000.0, 250.0))),
        MeshMaterial2d(materials.add(Color::BLACK)),
        Transform::from_translation(Vec3::new(0.0, -25.0, 0.0))
            .with_scale(Vec3::ONE),
        TextBackground,
    ));

    // 胜利文本
    commands.spawn((
        StateScoped(GameState::GameOver),
        VictoryText,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        children![
            (
                Text::new(message.to_string()),
                TextFont {
                    font: victory_font.clone(),
                    font_size: VICTORY_TEXT_FONT_SIZE,
                    ..default()
                },
                TextColor(Color::WHITE),
            ),
            (
                Text::new("PRESS K TO RESTART"),
                TextFont {
                    font: victory_font.clone(),
                    font_size: HINT_FONT_SIZE,
                    ..default()
                },
                TextColor(Color::WHITE),
            ),
        ],
    ));
}

fn game_over_keyboard(
    mut next_state: ResMut<NextState<GameState>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyK) {
        next_state.set(GameState::Playing);
    }
}

fn game_reset(
    mut score: ResMut<Score>,
    ball_query: Single<(&mut Velocity, &mut Transform), (With<Ball>, Without<Paddle>)>,
    mut paddle_query: Query<&mut Transform, (With<Paddle>, Without<Ball>)>,
) {
    // 重置分数   
    score.0 = 0;
    score.1 = 0;

    // 重置挡板位置
    for mut paddle_transform in paddle_query.iter_mut(){
        paddle_transform.translation.y = 0.0;
    }

    // 重置小球位置、速度、发球角度
    let (mut ball_velocity, mut ball_transform) = ball_query.into_inner();
    **ball_velocity = INITIAL_BALL_DIRECTION.normalize() * BALL_SPEED;
    ball_transform.translation = BALL_STARTING_POSITION;
}