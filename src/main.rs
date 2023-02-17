use bevy::{
    prelude::*,
    sprite::MaterialMesh2dBundle,
};
use std::collections::HashSet;
use std::time::Duration;
use random_color::{Luminosity, RandomColor};
use random_color::Color as Color2;
//use dashmap::*;
use bevy::utils::HashMap;

// Defines the amount of time that should elapse between each physics step.
const TIME_STEP: f32 = 1.0 / 60.0;
const SUB_STEPS: u8 = 8;
const STEP_DT: f32 = TIME_STEP / SUB_STEPS as f32;

// We set the z-value of the ball to 1 so it renders on top in the case of overlapping sprites.
const BALL_STARTING_POSITION: Vec3 = Vec3::new(0.0, 200.0, 1.0);
const BALL_RADIUS: f32 = 2.;
const BALL_DIAMETER: f32 = BALL_RADIUS * 2.0;
const BALL_SIZE: Vec3 = Vec3::new(BALL_DIAMETER, BALL_DIAMETER, 0.0);
const BALL_SPEED: f32 = 100000.0 * SUB_STEPS as f32;
const INITIAL_BALL_DIRECTION: Vec2 = Vec2::new(0.5, 0.0);
const MAX_BALLS: i32 = 2000;
const SPAWN_INTERVAL: u64 = 10; //Milliseconds

const BACKGROUND_COLOR: Color = Color::rgb(0.05, 0.05, 0.05);
//const BALL_COLOR: Color = Color::rgb(1.0, 1.0, 1.0);
const CONSTRAINT_COLOR: Color = Color::rgb(0.1, 0.1, 0.1);

const CONSTRAINT_POS: Vec2 = Vec2::new(0.0,0.0);
const CONSTRAINT_RADIUS: f32 = 300.;

const GRAVITY: Vec2 = Vec2::new(0.0,-1000.0);
const RES_COEF: f32 = 0.5;
const MAX_SPEED: f32 = 100.0;

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
        for i in 0..10 {
        let c = RandomColor::new()
            .hue(Color2::Orange)
            .luminosity(Luminosity::Bright)
            .to_rgb_array();
        commands.spawn((
            MaterialMesh2dBundle {
                mesh: meshes.add(shape::Circle::default().into()).into(),
                material: materials.add(ColorMaterial::from(Color::rgb_u8(c[0], c[1], c[2]))),
                transform: Transform::from_translation(Vec3::new(0.0+7.*i as f32,0.,0.)).with_scale(BALL_SIZE),
                ..default()
            },
            VerletObject
        ));
        query.single_mut().objects.push(
            VerletData{
                position: Vec3::new(0.0+7.*i as f32,0.,0.).truncate(),
                old_position: Vec3::new(0.0+7.*i as f32,0.,0.).truncate(),
                acceleration: INITIAL_BALL_DIRECTION.normalize() * BALL_SPEED,
            });
        spawn_timer.ball_count += 1;
        spawn_timer.timer.reset();
        }
    }
}

fn neighbours((x, y): (i32,i32)) -> Vec<(i32,i32)> {
    vec![
    (x-1,y-1), (x,y-1), (x+1,y-1),
    (x-1,y),            (x+1,y),
    (x-1,y+1), (x,y+1), (x+1,y+1),
    ]
}

fn collide_pair(object: &mut VerletData, object2: &mut VerletData) {
    let v: Vec2 = object.position - object2.position;
    let dist2: f32 = v.x * v.x + v.y * v.y;
    let min_dist: f32 = BALL_DIAMETER;
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

fn collide_one_to_many(verlet_objects: &mut Vec<VerletData>, indicies: Vec<usize>) {
    let index_0 = indicies[0];
    for index in &indicies[1..indicies.len()] {
        if index_0 < *index {
            let (left, right) = verlet_objects.split_at_mut(index_0 + 1);
            let object = &mut left[index_0];
            let object2 = &mut right[*index - (index_0+1)];
            collide_pair(object, object2);
        } else {
            let (left, right) = verlet_objects.split_at_mut(*index + 1);
            let object = &mut left[*index];
            let object2 = &mut right[index_0 - (*index+1)];
            collide_pair(object, object2);
        }
        //if let (Some(object), Some(object2)) = (verlet_objects.get_mut(indicies[0]), verlet_objects.get_mut(*index)) {
        //let object2 = verlet_objects.get_mut(*index).unwrap();
        //}
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

        ///////////////////
        //Handle Collisions
        ///////////////////

        //This stores the grid position and the indicies of each object at that position
        let mut collision_map: HashMap<(i32,i32),Vec<usize>> = HashMap::new();
        for i in 0..verlet_objects.len() {
            let object = verlet_objects.get_mut(i).unwrap();
            let grid_x:i32 = (object.position.x / BALL_DIAMETER).floor() as i32;
            let grid_y:i32 = (object.position.y / BALL_DIAMETER).floor() as i32;
            collision_map.entry((grid_x,grid_y))
                .or_default()
                .push(i);
        }

        let mut already_checked: HashSet<(i32,i32)> = HashSet::new();
        for &grid_position in collision_map.keys() {
        //while let Some(&grid_position) = collision_map.
            already_checked.insert(grid_position);
            let neighbors = neighbours(grid_position);
            let mut colliding_objects: Vec<usize> = Vec::new();
            let mut n_in_tile = 0;
            for n in collision_map.get(&grid_position).unwrap() {
                colliding_objects.push(*n);
                n_in_tile += 1;
            }
            for neighbor in neighbors {
                if !already_checked.contains(&neighbor) {
                    if let Some(n_values) = collision_map.get(&neighbor) {
                        for n_value in n_values.iter() {
                            colliding_objects.push(*n_value);
                        }
                    }
                }
            }
            if colliding_objects.len() > 1 {
                for _ in [0..n_in_tile] {
                    collide_one_to_many(verlet_objects, colliding_objects.to_owned());
                    colliding_objects.remove(0);
                }
            }
        }

        //Change positions
        for object in verlet_objects.iter_mut() {
            let mut displacement = object.position - object.old_position;
            if displacement.length() > MAX_SPEED {
                displacement = displacement.normalize() * MAX_SPEED
            }
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