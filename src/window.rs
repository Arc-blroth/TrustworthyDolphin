use bevy::prelude::{error, App, IntoExclusiveSystem, Plugin, ResMut};
use bevy::window::WindowId;
use bevy::winit::WinitWindows;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::window::Window;

pub struct WindowHandlingPlugin;

impl Plugin for WindowHandlingPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup.exclusive_system());

        #[cfg(target_os = "windows")]
        if on_desktop() {
            app.add_system(ensure_show_on_desktop.exclusive_system());
        }
    }

    fn name(&self) -> &str {
        "WindowHandlingPlugin"
    }
}

fn on_desktop() -> bool {
    std::env::args().find(|x| x == "-d" || x == "-desktop").is_some()
}

pub fn setup(winit_windows: ResMut<WinitWindows>) {
    let primary = winit_windows
        .get_window(WindowId::primary())
        .expect("Primary window doesn't exist?");
    let monitor = primary.current_monitor().expect("Current window has no monitor?");

    // Display only on the desktop rather than over all other apps
    // if `-desktop` is passed
    let on_desktop = on_desktop();
    if !on_desktop {
        primary.set_always_on_top(true);
    }

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

    #[cfg(target_os = "windows")]
    unsafe {
        setup_win32(&primary);
    }
}

/// Removes the window from the taskbar and passes through clicks.
#[cfg(target_os = "windows")]
unsafe fn setup_win32(window: &Window) {
    use std::ffi::c_void;

    use raw_window_handle::{HasRawWindowHandle, RawWindowHandle, Win32Handle};
    use winapi::shared::basetsd::LONG_PTR;
    use winapi::shared::minwindef::{BOOL, DWORD};
    use winapi::shared::windef::HWND;
    use winapi::um::dwmapi::*;
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

        const YES: BOOL = true as BOOL;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_EXCLUDED_FROM_PEEK,
            &YES as *const _ as *const c_void,
            std::mem::size_of::<BOOL>() as DWORD,
        );

        ShowWindow(hwnd, SW_SHOW);
        SetFocus(std::ptr::null_mut());
    } else {
        error!("Couldn't get raw window handle, things will probably look weird!");
    }
}

#[cfg(target_os = "windows")]
fn ensure_show_on_desktop(winit_windows: ResMut<WinitWindows>) {
    use std::ffi::CString;

    use raw_window_handle::{HasRawWindowHandle, RawWindowHandle, Win32Handle};
    use winapi::shared::minwindef::{BOOL, LPARAM};
    use winapi::shared::windef::HWND;
    use winapi::um::winuser::*;

    let primary = winit_windows.get_window(WindowId::primary()).unwrap();
    if let RawWindowHandle::Win32(Win32Handle { hwnd: primary_hwnd, .. }) = primary.raw_window_handle() {
        let primary_hwnd = primary_hwnd as HWND;

        // Enumerate through all top-level windows and see if ours is
        // behind the window that handles the "show desktop" feature.
        // If so, then "show desktop" is currently active and we'll
        // force our window to render above the "show desktop" window.
        // This approximates Rainmeter's "show desktop" behavior but
        // is slightly more efficient since we only have one window.

        let show_desktop = {
            #[derive(Debug, PartialEq, Eq)]
            enum EnumWindowCallbackState {
                Initial,
                FoundWorkerW,
                FoundDolphin,
            }

            struct EnumWindowCallbackLParam {
                state: EnumWindowCallbackState,
                primary_hwnd: HWND,
            }

            unsafe extern "system" fn enum_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
                let lparam = &mut *(lparam as *mut EnumWindowCallbackLParam);
                (match lparam.state {
                    // The "show desktop" window can either be a WorkerW or a Progman
                    // window, but will always have a single child of class SHELLDLL_DefView
                    EnumWindowCallbackState::Initial => {
                        let def_view_class = CString::new("SHELLDLL_DefView").unwrap();
                        let def_view = FindWindowExA(
                            hwnd,
                            std::ptr::null_mut(),
                            def_view_class.as_ptr(),
                            std::ptr::null_mut(),
                        );
                        if !def_view.is_null() {
                            lparam.state = EnumWindowCallbackState::FoundWorkerW;
                        }
                        true
                    }
                    // While "show desktop" is active, all application windows
                    // will have a later z-order than the WorkerW/Progman window,
                    // so we expect EnumWindows to find our window afterwards
                    EnumWindowCallbackState::FoundWorkerW => {
                        if hwnd == lparam.primary_hwnd {
                            lparam.state = EnumWindowCallbackState::FoundDolphin;
                            false
                        } else {
                            true
                        }
                    }
                    // Should be impossible unless lparam is pointing
                    // to entirely invalid data and UB has occurred
                    _ => false,
                }) as BOOL
            }

            let mut lparam = EnumWindowCallbackLParam {
                state: EnumWindowCallbackState::Initial,
                primary_hwnd,
            };
            unsafe {
                EnumWindows(Some(enum_window_callback), &mut lparam as *mut _ as isize);
            }
            lparam.state == EnumWindowCallbackState::FoundDolphin
        };

        if show_desktop {
            println!("Show Desktop detected!");
        }
        unsafe {
            // set always-on-top if show desktop is detected
            SetWindowPos(
                primary_hwnd,
                if show_desktop { HWND_TOPMOST } else { HWND_NOTOPMOST },
                0,
                0,
                0,
                0,
                SWP_ASYNCWINDOWPOS | SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
}
