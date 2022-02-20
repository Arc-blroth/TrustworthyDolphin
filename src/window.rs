use bevy::prelude::{ResMut, error};
use bevy::winit::WinitWindows;
use bevy::window::WindowId;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::window::Window;

pub fn setup(winit_windows: ResMut<WinitWindows>) {
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
        setup_win32(&primary);
    }
}

/// Removes the window from the taskbar and passes through clicks.
#[cfg(target_os = "windows")]
unsafe fn setup_win32(window: &Window) {
    use raw_window_handle::{HasRawWindowHandle, RawWindowHandle, Win32Handle};
    use winapi::shared::basetsd::LONG_PTR;
    use winapi::shared::windef::HWND;
    use winapi::um::winuser::*;

    if let RawWindowHandle::Win32(Win32Handle { hwnd, .. }) = window.raw_window_handle() {
        let hwnd = hwnd as HWND;
        ShowWindow(hwnd, SW_HIDE);

        let mut ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        ex_style |= WS_EX_TOOLWINDOW as LONG_PTR;
        ex_style |= (WS_EX_LAYERED | WS_EX_TRANSPARENT) as LONG_PTR;
        ex_style &= !WS_EX_APPWINDOW as LONG_PTR;
        ex_style &= !WS_EX_ACCEPTFILES as LONG_PTR;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style);
        SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);

        ShowWindow(hwnd, SW_SHOW);
        SetFocus(std::ptr::null_mut());
    } else {
        error!("Couldn't get raw window handle, things will probably look weird!");
    }
}
