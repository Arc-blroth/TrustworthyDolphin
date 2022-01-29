//! # Trustworthy Dolphin
//! > Hey there! I'm an totally trustworthy cetacean!
//! [Click here](https://youtu.be/dQw4w9WgXcQ)
//! for FREE NOSE BONKS!
//!
//! A playful app that adds aquatic spice to desktops.
//! Contains seawater, dolphins, and lots of bubbles.

use bevy::DefaultPlugins;
use bevy::prelude::*;
use bevy::window::WindowId;
use bevy::winit::WinitWindows;
use winit::dpi::{PhysicalPosition, PhysicalSize};

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
        .add_startup_system(window_setup.exclusive_system())
        .add_startup_system(setup)
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

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 0.0),
                scale: Vec3::new(64.0, 64.0, 0.0),
                ..Transform::default()
            },
            sprite: Sprite {
                color: Color::rgb(0.0, 0.5, 1.0),
                ..Sprite::default()
            },
            ..SpriteBundle::default()
        });
}