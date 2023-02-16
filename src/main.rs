use bevy::{
    prelude::*,
    sprite::MaterialMesh2dBundle,
    //time::FixedTimestep,
};
use std::time::Duration;
use random_color::{Luminosity, RandomColor};
use random_color::Color as Color2;

// Defines the amount of time that should elapse between each physics step.
const TIME_STEP: f32 = 1.0 / 60.0;
const SUB_STEPS: u8 = 10;
const STEP_DT: f32 = TIME_STEP / SUB_STEPS as f32;

// We set the z-value of the ball to 1 so it renders on top in the case of overlapping sprites.
const BALL_STARTING_POSITION: Vec3 = Vec3::new(0.0, 200.0, 1.0);
const BALL_RADIUS: f32 = 10.;
const BALL_SIZE: Vec3 = Vec3::new(BALL_RADIUS*2., BALL_RADIUS*2., 0.0);
const BALL_SPEED: f32 = 1000000.0;
const INITIAL_BALL_DIRECTION: Vec2 = Vec2::new(1.0, 0.0);
const MAX_BALLS: i32 = 1;
const SPAWN_INTERVAL: u64 = 50; //Milliseconds

const BACKGROUND_COLOR: Color = Color::rgb(0.05, 0.05, 0.05);
//const BALL_COLOR: Color = Color::rgb(1.0, 1.0, 1.0);
const CONSTRAINT_COLOR: Color = Color::rgb(0.1, 0.1, 0.1);

const CONSTRAINT_POS: Vec2 = Vec2::new(0.,0.);
const CONSTRAINT_RADIUS: f32 = 250.;

const GRAVITY: Vec2 = Vec2::new(0.,-1000.);
const RES_COEF: f32 = 0.75;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .insert_resource(SpawnTimer {
            timer: Timer::new(Duration::from_millis(SPAWN_INTERVAL), TimerMode::Repeating),
            ball_count: 0,
        })
        .add_startup_system(setup)
        //.add_event::<CollisionEvent>()
        .add_system(ball_spawner)
        .add_system(update)
        .add_system(bevy::window::close_on_esc)
        .run();
}

#[derive(Component, Deref, DerefMut)]
struct OldPos(Vec2);
#[derive(Component, Deref, DerefMut)]
struct Accel(Vec2);

#[derive(Resource)]
struct SpawnTimer {
    timer: Timer,
    ball_count: i32,
}

// Add the game's entities to our world
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    _asset_server: Res<AssetServer>,
) {
    // Camera
    commands.spawn(Camera2dBundle::default());

    // Constraint
    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::default().into()).into(),
            material: materials.add(ColorMaterial::from(CONSTRAINT_COLOR)),
            transform: Transform::from_translation(CONSTRAINT_POS.extend(0.))
                .with_scale(Vec3::new(CONSTRAINT_RADIUS*2.,CONSTRAINT_RADIUS*2.,0.)),
            ..default()
        },
    ));
}

fn ball_spawner(
    time: Res<Time>,
    mut spawn_timer: ResMut<SpawnTimer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    spawn_timer.timer.tick(time.delta());
    if spawn_timer.timer.finished() && spawn_timer.ball_count < MAX_BALLS{
        let c = RandomColor::new()
            .hue(Color2::Blue)
            .luminosity(Luminosity::Bright)
            .to_rgb_array();
        commands.spawn((
            MaterialMesh2dBundle {
                mesh: meshes.add(shape::Circle::default().into()).into(),
                material: materials.add(ColorMaterial::from(Color::rgb_u8(c[0], c[1], c[2]))),
                transform: Transform::from_translation(BALL_STARTING_POSITION).with_scale(BALL_SIZE),
                ..default()
            },
            OldPos(BALL_STARTING_POSITION.truncate()),
            Accel(INITIAL_BALL_DIRECTION.normalize() * BALL_SPEED),
        ));
        spawn_timer.ball_count += 1;
        //spawn_timer.timer.reset(); //Should reset automatically
    }
}

fn update(mut query: Query<(&mut Transform, &mut OldPos, &mut Accel)>) {
    for _i in 0..SUB_STEPS {
        apply_gravity(&mut query);
        check_collisions(&mut query);
        apply_constraint(&mut query);
        update_objects(&mut query);
    }
}

fn apply_gravity(query: &mut Query<(&mut Transform, &mut OldPos, &mut Accel)>) {
    let iter = query.iter_mut();
    for (_, _, mut acceleration) in iter {
        acceleration.0 += GRAVITY;
    }
}

fn apply_constraint(query: &mut Query<(&mut Transform, &mut OldPos, &mut Accel)>) {
    let iter = query.iter_mut();
    for (mut transform, _, _) in iter {
        //Constraint
        let pos_2d = transform.translation.truncate();
        let v: Vec2 = CONSTRAINT_POS - pos_2d;
        let dist: f32 = (v.x * v.x + v.y * v.y).sqrt();
        if dist > CONSTRAINT_RADIUS - BALL_RADIUS {
            let n: Vec2 = v / dist;
            transform.translation = (CONSTRAINT_POS - n * (CONSTRAINT_RADIUS - BALL_RADIUS)).extend(1.);
        }
    }
}

fn check_collisions(query: &mut Query<(&mut Transform, &mut OldPos, &mut Accel)>) {
    let mut iter = query.iter_combinations_mut();
    while let Some([
        (mut transform, _, _),
        (mut transform2, _, _)])
        = iter.fetch_next()  {
        let v: Vec2 = transform.translation.truncate() - transform2.translation.truncate();
        let dist2: f32 = v.x * v.x + v.y * v.y;
        let min_dist: f32 = BALL_RADIUS * 2.0;
        //Check overlapping
        if dist2 < min_dist * min_dist {
            let dist: f32 = dist2.sqrt();
            let n: Vec2 = v / dist;
            //No variable size for now
            let delta: f32 = 0.5 * RES_COEF * (dist - min_dist);
            transform.translation -= (n * delta).extend(1.0);
            transform2.translation += (n * delta).extend(1.0);
        }
    }
}

fn update_objects(query: &mut Query<(&mut Transform, &mut OldPos, &mut Accel)>) {
    let iter = query.iter_mut();
    for (mut transform, mut old_position, mut acceleration) in iter {
        let mut pos_2d = transform.translation.truncate();
        let displacement = pos_2d - old_position.0;
        old_position.0 = pos_2d;
        //Verlet integration:
        pos_2d = pos_2d + displacement + acceleration.0 * (STEP_DT * STEP_DT);
        transform.translation = pos_2d.extend(1.);
        //reset acceleration
        acceleration.0 *= 0.;
    }
}