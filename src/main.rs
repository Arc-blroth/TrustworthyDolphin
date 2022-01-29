//! # Trustworthy Dolphin
//! > Hey there! I'm an totally trustworthy cetacean!
//! [Click here](https://youtu.be/dQw4w9WgXcQ)
//! for FREE NOSE BONKS!
//!
//! A playful app that adds aquatic spice to desktops.
//! Contains seawater, dolphins, and lots of bubbles.

use bevy::DefaultPlugins;
use bevy::prelude::{App, error, IntoExclusiveSystem, ResMut, WindowDescriptor, Windows};
use bevy::window::WindowId;
use bevy::winit::WinitWindows;

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "Trustworthy Dolphin!".to_string(),
            resizable: false,
            decorations: false,
            transparent: true,
            ..WindowDescriptor::default()
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(window_setup.exclusive_system())
        .run();
}

fn window_setup(mut winit_windows: ResMut<WinitWindows>) {
    let primary = winit_windows.get_window(WindowId::primary()).expect("Primary window doesn't exist?");
    primary.set_always_on_top(true);

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
            ex_style |= WS_EX_LAYERED as LONG_PTR;
            ex_style |= WS_EX_TRANSPARENT as LONG_PTR;
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