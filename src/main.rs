//! # Trustworthy Dolphin
//! > Hey there! I'm an totally trustworthy cetacean!
//! [Click here](https://youtu.be/dQw4w9WgXcQ)
//! for FREE NOSE BONKS!
//!
//! A playful app that adds aquatic spice to desktops.
//! Contains seawater, dolphins, and lots of bubbles.

#![feature(decl_macro)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(all(not(debug_assertions), feature = "bevy_dyn"))]
compile_error!("Bevy should not be dynamically linked for release builds!");

use std::f64::consts::PI;
use std::time::Duration;

use benimator::{AnimationPlugin, SpriteSheetAnimation};
use bevy::asset::AssetPlugin;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::math::DVec2;
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use bevy::render::render_resource::AddressMode;
use bevy::tasks::IoTaskPool;
use bevy::window::WindowId;
use bevy::winit::WinitWindows;
use bevy::DefaultPlugins;
use bevy_ase::asset::{Animation as AseAnimation, AseFileMap};
use bevy_ase::loader::{AseLoaderDefaultPlugin, Loader as AseLoader};
use rand::Rng;

use crate::assets::{include_assets, EmbeddedAssetsPlugin};
use crate::util::Also;

mod assets;
mod util;
mod window;

pub const FAITH_TEXTURE_PATH: &str = "faith.ase";
pub const WAVE_TEXTURE_PATH: &str = "wave.ase";
pub const BUBBLE_TEXTURE_PATH: &str = "bubble.ase";
pub const FISH_TEXTURE_PATH: &str = "fish.ase";

pub const ASSETS: [&str; 4] = [
    FAITH_TEXTURE_PATH,
    WAVE_TEXTURE_PATH,
    BUBBLE_TEXTURE_PATH,
    FISH_TEXTURE_PATH,
];

const STANDARD_GRAVITY: f64 = 9.80665;
const SPEED_MULTIPLER: f64 = 6.0;
const MAX_STEP_TIME: f64 = 1.0 / 60.0;

const MAX_BUBBLES: u32 = 16;
const MAX_FISHES: u32 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LoadingState {
    Loading,
    FillingWater,
    Play,
}

#[derive(Component)]
struct Water {
    pub start_time: Duration,
    pub water_level: f64,
}

#[derive(Component)]
struct Faith {
    pub position: DVec2,
    pub velocity: DVec2,
    pub rotation: f64,
}

#[derive(Component)]
struct Bubbles {
    pub start_time: Duration,
    pub delta_between_bubbles: f64,
    pub height: usize,
    pub step: usize,
}

#[derive(Component)]
struct Fish {
    pub start_time: Duration,
    pub flip: bool,
    pub speed: f64,
}

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "Trustworthy Dolphin!".to_string(),
            resizable: false,
            decorations: false,
            transparent: true,
            ..WindowDescriptor::default()
        })
        .insert_resource(ClearColor(Color::NONE))
        .also(|app| {
            if cfg!(feature = "embed_assets") {
                app.insert_resource(include_assets!["faith.ase", "wave.ase", "bubble.ase", "fish.ase"]);
            }
        })
        .add_plugins_with(DefaultPlugins, |group| {
            if cfg!(feature = "embed_assets") {
                group.add_before::<AssetPlugin, _>(EmbeddedAssetsPlugin);
            }
            group
        })
        .also(|app| {
            if cfg!(debug_assertions) {
                app.add_plugin(LogDiagnosticsPlugin::default())
                    .add_plugin(FrameTimeDiagnosticsPlugin::default());
            }
        })
        .add_plugin(AnimationPlugin::default())
        .add_plugin(AseLoaderDefaultPlugin)
        .add_plugin(window::WindowHandlingPlugin)
        .add_state(LoadingState::Loading)
        .add_system_set(SystemSet::on_enter(LoadingState::Loading).with_system(load_assets))
        .add_system_set(SystemSet::on_update(LoadingState::Loading).with_system(check_loading))
        .add_system_set(
            SystemSet::on_enter(LoadingState::FillingWater)
                .with_system(setup_camera)
                .with_system(setup_water),
        )
        .add_system_set(SystemSet::on_update(LoadingState::FillingWater).with_system(fill_water))
        .add_system_set(SystemSet::on_enter(LoadingState::Play).with_system(spawn_faith))
        .add_system_set(
            SystemSet::on_update(LoadingState::Play)
                .with_system(wave_water.chain(update_faith))
                .with_system(update_bubbles)
                .with_system(update_fishes),
        )
        .run();
}

fn load_assets(asset_server: Res<AssetServer>, mut ase_loader: ResMut<AseLoader>) {
    for asset in ASSETS {
        ase_loader.add(asset_server.load(asset));
    }
}

fn check_loading(mut state: ResMut<State<LoadingState>>, ase_loader: Res<AseLoader>) {
    if ase_loader.is_loaded() {
        state.set(LoadingState::FillingWater).unwrap()
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn get_primary_window_size(windows: &Res<WinitWindows>) -> Vec2 {
    let primary_window = windows.get_window(WindowId::primary()).unwrap();
    let logical_size = primary_window
        .inner_size()
        .to_logical::<f32>(primary_window.scale_factor());
    Vec2::new(logical_size.width, logical_size.height)
}

fn setup_water(
    mut commands: Commands,
    windows: Res<WinitWindows>,
    ase_assets: Res<AseFileMap>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    time: Res<Time>,
) {
    let window_size = get_primary_window_size(&windows);

    let wave_ase = ase_assets.get(WAVE_TEXTURE_PATH.as_ref()).unwrap();
    let wave_texture_handle = wave_ase.texture(0).unwrap();
    let wave_texture = images.get_mut(wave_texture_handle).unwrap();
    wave_texture.sampler_descriptor.address_mode_u = AddressMode::Repeat;
    wave_texture.sampler_descriptor.address_mode_v = AddressMode::ClampToEdge;

    let wave_mesh = {
        let mut mesh = Mesh::from(shape::Quad::new(window_size));
        if let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
            //  0  1  2  3
            // SW NW NE SE
            const UV_SCALE: f32 = 1.0 / 128.0;
            uvs[0][1] = window_size.y * UV_SCALE;
            uvs[2][0] = window_size.x * UV_SCALE;
            uvs[3] = [window_size.x * UV_SCALE, window_size.y * UV_SCALE];
        } else {
            panic!("Mesh doesn't have UV coordinates?");
        }
        mesh
    };

    commands
        .spawn_bundle(ColorMesh2dBundle {
            mesh: meshes.add(wave_mesh).into(),
            material: materials.add(ColorMaterial::from(wave_texture_handle.clone())),
            transform: Transform {
                translation: Vec3::new(0.0, -window_size.y, 1.0),
                ..Transform::default()
            },
            ..ColorMesh2dBundle::default()
        })
        .insert(Water {
            start_time: time.time_since_startup(),
            water_level: 0.0,
        });
}

fn fill_water(
    mut query: Query<(&mut Water, &mut Transform)>,
    mut state: ResMut<State<LoadingState>>,
    windows: Res<WinitWindows>,
    time: Res<Time>,
) {
    let (mut water, transform): (Mut<Water>, Mut<Transform>) = query.single_mut();
    let anim_time = (time.time_since_startup() - water.start_time).as_secs_f64();

    if anim_time >= 1.0 {
        water.water_level = 1.0;
        water.start_time = time.time_since_startup();
        state.set(LoadingState::Play).unwrap();
    } else {
        water.water_level = -16.0 * (anim_time - 1.0).powf(4.0) + 1.0;
    }

    update_water_transform(water, transform, windows);
}

fn wave_water(mut query: Query<(&mut Water, &mut Transform)>, windows: Res<WinitWindows>, time: Res<Time>) {
    let (mut water, transform): (Mut<Water>, Mut<Transform>) = query.single_mut();
    let anim_time = time.time_since_startup() - water.start_time;
    let wave_time = Duration::new(anim_time.as_secs() % 10, anim_time.subsec_nanos()).as_secs_f64();
    let wave_y = f64::sin(0.4 * PI * wave_time) + f64::sin(0.6 * PI * wave_time);
    water.water_level = 1.0 + 0.01 * wave_y;

    update_water_transform(water, transform, windows);
}

fn update_water_transform(water: Mut<Water>, mut transform: Mut<Transform>, windows: Res<WinitWindows>) {
    let window_size = get_primary_window_size(&windows);
    transform.translation.y = ((-1.0 + water.water_level * 0.5) * window_size.y as f64) as f32;
}

fn spawn_faith(
    mut commands: Commands,
    ase_assets: Res<AseFileMap>,
    ase_animations: Res<Assets<AseAnimation>>,
    mut animations: ResMut<Assets<SpriteSheetAnimation>>,
    windows: Res<WinitWindows>,
) {
    let window_size = get_primary_window_size(&windows);

    let faith_ase = ase_assets.get(FAITH_TEXTURE_PATH.as_ref()).unwrap();
    let swim_animation = ase_animations
        .get(faith_ase.animations("swim").unwrap().first().unwrap())
        .unwrap();
    let animation_handle = animations.add(swim_animation.into());

    commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: swim_animation.atlas(),
            transform: Transform {
                scale: Vec2::splat(4.0).extend(0.0),
                ..Transform::default()
            },
            ..SpriteSheetBundle::default()
        })
        .insert(animation_handle)
        .insert(benimator::Play)
        .insert(Faith {
            position: DVec2::new(0.0, window_size.y as f64 * 0.5),
            velocity: DVec2::default(),
            rotation: 0.0,
        });
}

fn update_faith(
    mut faith_query: Query<(&mut Faith, &mut Transform)>,
    water_query: Query<&Water>,
    time: Res<Time>,
    windows: Res<WinitWindows>,
) {
    let (mut faith, mut faith_transform): (Mut<Faith>, Mut<Transform>) = faith_query.single_mut();

    let window_size = get_primary_window_size(&windows);
    let water_level = (-0.5 + water_query.single().water_level * 0.5) * window_size.y as f64;

    // subdivide simulation time to guard against lag spikes
    let full_delta = time.delta_seconds_f64() * SPEED_MULTIPLER;
    let steps = (full_delta / MAX_STEP_TIME).ceil() as u64;
    for i in 0..steps {
        let delta = if i == steps - 1 {
            full_delta % MAX_STEP_TIME
        } else {
            MAX_STEP_TIME
        };

        // Update second order displacement
        if faith.position.y > water_level {
            faith.velocity.y -= STANDARD_GRAVITY * delta;
        } else {
            faith.velocity.y += ((water_level - faith.position.y).sqrt()) * delta;
        }

        // Update first order displacement
        let displacement = faith.velocity * delta;
        faith.position += displacement;

        // Update rotation
        let dist_from_water = 2.0 * (faith.position.y - water_level).abs() / window_size.y as f64;
        let dir = faith.velocity.y * ((1.0 - dist_from_water).powf(2.0) - 0.01 * dist_from_water);
        faith.rotation = if faith.position.y > water_level {
            f64::atan(dir)
        } else {
            -PI - f64::atan(dir)
        };

        // Update transform
        faith_transform.translation = faith.position.as_vec2().extend(0.0);
        faith_transform.rotation = Quat::from_rotation_z(faith.rotation as f32);
    }
}

fn update_bubbles(
    mut commands: Commands,
    mut bubbles_query: Query<(Entity, &mut Bubbles, &Children)>,
    ase_assets: Res<AseFileMap>,
    images: Res<Assets<Image>>,
    time: Res<Time>,
    windows: Res<WinitWindows>,
) {
    let bubble_texture = ase_assets
        .get(BUBBLE_TEXTURE_PATH.as_ref())
        .unwrap()
        .texture(0)
        .unwrap();

    let mut num_bubbles = 0;
    for component in bubbles_query.iter_mut() {
        num_bubbles += 1;

        let (entity, mut bubbles, children): (Entity, Mut<Bubbles>, &Children) = component;
        if time.time_since_startup() < bubbles.start_time {
            // these bubbles aren't supposed to show yet
            continue;
        } else {
            let time_since_start = (time.time_since_startup() - bubbles.start_time).as_secs_f64();
            let steps_since_start = (time_since_start / bubbles.delta_between_bubbles).floor() as usize;

            if steps_since_start >= bubbles.height * 2 {
                // no more bubbles
                commands.entity(entity).despawn_recursive();
            } else if bubbles.step < bubbles.height {
                // add bubbles
                for i in bubbles.step..steps_since_start.min(bubbles.height) {
                    commands.entity(entity).with_children(|builder| {
                        builder.spawn_bundle(SpriteBundle {
                            transform: Transform::from_translation(Vec3::new(
                                (i % 2) as f32 * 6.0,
                                i as f32 * 6.0,
                                0.0,
                            )),
                            texture: images.get_handle(bubble_texture),
                            ..SpriteBundle::default()
                        });
                    });
                }
                bubbles.step = steps_since_start.min(bubbles.height);
            } else {
                // remove bubbles
                for i in bubbles.step..steps_since_start.min(bubbles.height * 2) {
                    let child = children[(i - bubbles.height) - (bubbles.height - children.len())];
                    commands.entity(entity).remove_children(&[child]);
                    commands.entity(child).despawn();
                }
                bubbles.step = steps_since_start.min(bubbles.height * 2);
            }
        }
    }

    // make new bubbles
    if num_bubbles < MAX_BUBBLES {
        let window_size = get_primary_window_size(&windows);
        let mut rng = rand::thread_rng();

        commands
            .spawn()
            .insert(Bubbles {
                start_time: time.time_since_startup() + Duration::from_secs_f64(rng.gen_range(0.0..=2.0)),
                delta_between_bubbles: rng.gen_range(0.1..=0.4),
                height: rng.gen_range(4..=16),
                step: 0,
            })
            .insert_bundle(TransformBundle::from_transform(Transform {
                translation: Vec3::new(
                    rng.gen_range::<i8, _>(-15..=15) as f32 / 16.0 * window_size.x / 2.0,
                    -window_size.y / 2.0 + 6.0,
                    -0.0,
                ),
                scale: Vec2::splat(2.0).extend(0.0),
                ..Transform::default()
            }))
            .insert(Children::default());
    }
}

fn update_fishes(
    mut commands: Commands,
    mut fish_query: Query<(Entity, &Fish, &mut Visibility, &mut Transform)>,
    ase_assets: Res<AseFileMap>,
    images: Res<Assets<Image>>,
    time: Res<Time>,
    windows: Res<WinitWindows>,
) {
    let window_size = get_primary_window_size(&windows);

    let mut num_fishes = 0;
    for component in fish_query.iter_mut() {
        num_fishes += 1;

        let (entity, fish, mut visibility, mut transform): (Entity, &Fish, Mut<Visibility>, Mut<Transform>) = component;
        if time.time_since_startup() < fish.start_time {
            // this fish doesn't exist yet
            continue;
        } else {
            visibility.is_visible = true;
            let time_since_start = (time.time_since_startup() - fish.start_time).as_secs_f64();
            let direction = if fish.flip { -1.0 } else { 1.0 };
            transform.translation.x = (time_since_start * fish.speed) as f32 + -direction * (window_size.x / 2.0);

            if !fish.flip && transform.translation.x > window_size.x / 2.0
                || fish.flip && transform.translation.x < -window_size.x / 2.0
            {
                commands.entity(entity).despawn();
            }
        }
    }

    // make new fishes
    if num_fishes < MAX_FISHES {
        let mut rng = rand::thread_rng();

        let fish_ase = ase_assets.get(FISH_TEXTURE_PATH.as_ref()).unwrap();
        let fish_texture = fish_ase.texture(rng.gen_range(0..=5)).unwrap();

        let flip = rng.gen::<bool>();
        let direction = if flip { -1.0 } else { 1.0 };

        commands
            .spawn_bundle(SpriteBundle {
                sprite: Sprite {
                    flip_x: flip,
                    ..Sprite::default()
                },
                transform: Transform {
                    translation: Vec3::new(
                        -direction as f32 * window_size.x / 2.0,
                        rng.gen_range(0.1..=0.5) * -window_size.y,
                        0.0,
                    ),
                    scale: Vec2::splat(4.0).extend(0.0),
                    ..Transform::default()
                },
                texture: images.get_handle(fish_texture),
                visibility: Visibility { is_visible: false },
                ..SpriteBundle::default()
            })
            .insert(Fish {
                start_time: time.time_since_startup() + Duration::from_secs_f64(rng.gen_range(1.0..=5.0)),
                flip,
                speed: direction * rng.gen_range(20.0..=100.0),
            });
    }
}
