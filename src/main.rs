use bevy::{
    prelude::*,
    sprite::MaterialMesh2dBundle,
};
use std::time::Duration;
use random_color::{Luminosity, RandomColor};
use random_color::Color as Color2;

// Defines the amount of time that should elapse between each physics step.
const TIME_STEP: f32 = 1.0 / 60.0;
const SUB_STEPS: u8 = 10;
const STEP_DT: f32 = TIME_STEP / SUB_STEPS as f32;

// We set the z-value of the ball to 1 so it renders on top in the case of overlapping sprites.
const BALL_STARTING_POSITION: Vec3 = Vec3::new(0.0, 240.0, 1.0);
const BALL_RADIUS: f32 = 20.;
const BALL_SIZE: Vec3 = Vec3::new(BALL_RADIUS*2., BALL_RADIUS*2., 0.0);
const BALL_SPEED: f32 = 20000.0 * SUB_STEPS as f32;
const INITIAL_BALL_DIRECTION: Vec2 = Vec2::new(1.0, 0.0);
const MAX_BALLS: i32 = 150;
const SPAWN_INTERVAL: u64 = 500; //Milliseconds

const BACKGROUND_COLOR: Color = Color::rgb(0.05, 0.05, 0.05);
//const BALL_COLOR: Color = Color::rgb(1.0, 1.0, 1.0);
const CONSTRAINT_COLOR: Color = Color::rgb(0.2, 0.2, 0.2);

const CONSTRAINT_POS: Vec2 = Vec2::new(0.0,0.0);
const CONSTRAINT_RADIUS: f32 = 400.;

const GRAVITY: Vec2 = Vec2::new(0.0,-1000.0);
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
        .add_system(object_spawner)
        .add_system(verlet.after(object_spawner))
        
        .add_system(bevy::window::close_on_esc)
        .run();
}

#[derive(Component)]
struct VerletObject;

#[derive(Resource)]
struct SpawnTimer {
    timer: Timer,
    ball_count: i32,
}

struct VerletData {
    position: Vec2,
    old_position: Vec2,
    acceleration: Vec2,
}

#[derive(Component)]
struct VerletObjects {
    objects: Vec<VerletData>
}

// Add the game's entities to our world
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,) {
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
    //Verlet Object Container
    commands.spawn(VerletObjects{objects: Vec::new()});
}

fn object_spawner(
    time: Res<Time>,
    mut spawn_timer: ResMut<SpawnTimer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut query: Query<&mut VerletObjects>,
) {
    spawn_timer.timer.tick(time.delta());
    if spawn_timer.timer.finished() && spawn_timer.ball_count < MAX_BALLS {
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
            VerletObject
        ));
        query.single_mut().objects.push(
            VerletData{
                position: BALL_STARTING_POSITION.truncate(),
                old_position: BALL_STARTING_POSITION.truncate(),
                acceleration: INITIAL_BALL_DIRECTION.normalize() * BALL_SPEED,
            });
        spawn_timer.ball_count += 1;
        spawn_timer.timer.reset();
    }
}

fn verlet(mut query: Query<&mut VerletObjects>, mut query2: Query<&mut Transform, With<VerletObject>>) {
    let verlet_objects: &mut Vec<VerletData> = &mut query.single_mut().objects;
    
    for _i in 0..SUB_STEPS {
        for object in verlet_objects.iter_mut() {
            object.acceleration += GRAVITY;
            //Constraint
            let v: Vec2 = CONSTRAINT_POS - object.position;
            let dist: f32 = (v.x * v.x + v.y * v.y).sqrt();
            if dist > CONSTRAINT_RADIUS - BALL_RADIUS {
                let n: Vec2 = v / dist;
                object.position = CONSTRAINT_POS - n * (CONSTRAINT_RADIUS - BALL_RADIUS);
            }
        }
        //Handle Collisions
        for i in 0..verlet_objects.len() {
            for j in i+1..verlet_objects.len() {
                let (left, right) = verlet_objects.split_at_mut(j);
                let object = &mut left[i];
                let object2 = &mut right[0];

                let v: Vec2 = object.position - object2.position;
                let dist2: f32 = v.x * v.x + v.y * v.y;
                let min_dist: f32 = BALL_RADIUS * 2.0;
                //Check overlapping
                if dist2 < min_dist * min_dist {
                    let dist: f32 = dist2.sqrt();
                    let n: Vec2 = v / dist;
                    //No variable size for now
                    let delta: f32 = 0.5 * RES_COEF * (dist - min_dist);
                    object.position -= n * delta;
                    object2.position += n * delta;
                }
            }
        }
        //Change positions
        for object in verlet_objects.iter_mut() {
            let displacement = object.position - object.old_position;
            object.old_position = object.position;
            //Verlet integration:
            object.position = object.position + displacement + object.acceleration * (STEP_DT * STEP_DT);
            //reset acceleration
            object.acceleration = Vec2::ZERO;
        }
    }
    for (mut a,b) in &mut query2.iter_mut().zip(query.single_mut().objects.iter_mut()) {
        a.translation = b.position.extend(1.0);
    }
}