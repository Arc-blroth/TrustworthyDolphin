//! # Trustworthy Dolphin
//! > Hey there! I'm an totally trustworthy cetacean!
//! [Click here](https://youtu.be/dQw4w9WgXcQ)
//! for FREE NOSE BONKS!
//!
//! A playful app that adds aquatic spice to desktops.
//! Contains seawater, dolphins, and lots of bubbles.

#[cfg(all(not(debug_assertions), feature = "bevy_dyn"))]
compile_error!("Bevy should not be dynamically linked for release builds!");

use benimator::{AnimationPlugin, SpriteSheetAnimation};
use bevy::DefaultPlugins;
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use bevy::render::render_resource::AddressMode;
use bevy::window::WindowId;
use bevy::winit::WinitWindows;
use bevy_ase::asset::{Animation as AseAnimation, AseFileMap};
use bevy_ase::loader::{AseLoaderDefaultPlugin, Loader as AseLoader};
use winit::dpi::{PhysicalPosition, PhysicalSize};

pub const FAITH_TEXTURE_PATH: &str = "faith.ase";
pub const WAVE_TEXTURE_PATH: &str = "wave.ase";

pub const ASSETS: [&str; 2] = [
    FAITH_TEXTURE_PATH,
    WAVE_TEXTURE_PATH
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LoadingState {
    Loading,
    Loaded,
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
        .add_plugins(DefaultPlugins)
        .add_plugin(AnimationPlugin::default())
        .add_plugin(AseLoaderDefaultPlugin)
        .add_startup_system(window_setup.exclusive_system())
        .add_state(LoadingState::Loading)
        .add_system_set(SystemSet::on_enter(LoadingState::Loading).with_system(load_assets.system()))
        .add_system_set(SystemSet::on_update(LoadingState::Loading).with_system(check_loading.system()))
        .add_system_set(SystemSet::on_enter(LoadingState::Loaded).with_system(setup.system()))
        .run();
}

fn window_setup(winit_windows: ResMut<WinitWindows>) {
    let primary = winit_windows.get_window(WindowId::primary()).expect("Primary window doesn't exist?");
    let monitor = primary.current_monitor().expect("Current window has no monitor?");
    primary.set_always_on_top(true);

    // on Windows, making the window take up the full screen
    // seems to automatically put it into fullscreen mode,
    // which we don't want
    primary.set_outer_position({
        let pos = monitor.position();
        PhysicalPosition::new(pos.x, pos.y + 1)
    });
    primary.set_inner_size({
        let size = monitor.size();
        PhysicalSize::new(size.width, size.height - 1)
    });

    // remove the window from the taskbar and pass through clicks
    #[cfg(target_os = "windows")]
    unsafe {
        use raw_window_handle::{HasRawWindowHandle, RawWindowHandle, Win32Handle};
        use winapi::shared::basetsd::LONG_PTR;
        use winapi::shared::windef::HWND;
        use winapi::um::winuser::*;

        if let RawWindowHandle::Win32(Win32Handle { hwnd, .. }) = primary.raw_window_handle() {
            let hwnd = hwnd as HWND;
            ShowWindow(hwnd, SW_HIDE);

            let mut ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            ex_style |= WS_EX_TOOLWINDOW as LONG_PTR;
            ex_style |= (WS_EX_COMPOSITED | WS_EX_LAYERED | WS_EX_TRANSPARENT) as LONG_PTR;
            ex_style &= !WS_EX_APPWINDOW as LONG_PTR;
            ex_style &= !WS_EX_ACCEPTFILES as LONG_PTR;
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style);

            ShowWindow(hwnd, SW_SHOW);
            SetFocus(std::ptr::null_mut());
        } else {
            error!("Couldn't get raw window handle, things will probably look weird!");
        }
    }
}

fn load_assets(asset_server: Res<AssetServer>, mut ase_loader: ResMut<AseLoader>) {
    for asset in ASSETS {
        ase_loader.add(asset_server.load(asset));
    }
}

fn check_loading(mut state: ResMut<State<LoadingState>>, ase_loader: Res<AseLoader>) {
    if ase_loader.is_loaded() {
        state.set(LoadingState::Loaded).unwrap()
    }
}

fn setup(
    mut commands: Commands,
    windows: Res<WinitWindows>,
    ase_assets: Res<AseFileMap>,
    mut images: ResMut<Assets<Image>>,
    ase_animations: Res<Assets<AseAnimation>>,
    mut animations: ResMut<Assets<SpriteSheetAnimation>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    let primary_window = windows.get_window(WindowId::primary()).unwrap();
    let window_size = primary_window.inner_size().to_logical::<f32>(primary_window.scale_factor());
    let window_size = Vec2::new(window_size.width, window_size.height);

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

    commands.spawn_bundle(ColorMesh2dBundle {
        mesh: meshes.add(wave_mesh).into(),
        material: materials.add(ColorMaterial::from(wave_texture_handle.clone())),
        transform: Transform {
            translation: Vec3::new(0.0, window_size.y * -0.5 + 24.0, 1.0),
            ..Transform::default()
        },
        ..ColorMesh2dBundle::default()
    });

    let faith_ase = ase_assets.get(FAITH_TEXTURE_PATH.as_ref()).unwrap();
    let swim_animation = ase_animations.get(faith_ase.animations("swim").unwrap().first().unwrap()).unwrap();
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
        .insert(benimator::Play);
}
