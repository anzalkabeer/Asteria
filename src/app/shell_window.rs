use std::sync::Arc;

use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use crate::renderer::window::chrome::ChromeShell;
use crate::shell::TabManager;

/// Apply Windows DWM Native Dark Mode Title Bar & Custom Caption Color (#0b1326)
#[cfg(target_os = "windows")]
fn apply_native_window_theme(window: &Window) {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    if let Ok(handle) = window.window_handle() {
        if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
            let hwnd = win32_handle.hwnd.get();
            unsafe {
                unsafe extern "system" {
                    fn LoadLibraryA(name: *const u8) -> *mut std::ffi::c_void;
                    fn GetProcAddress(
                        module: *mut std::ffi::c_void,
                        name: *const u8,
                    ) -> *mut std::ffi::c_void;
                }

                let module = LoadLibraryA(b"dwmapi.dll\0".as_ptr());
                if !module.is_null() {
                    let proc = GetProcAddress(module, b"DwmSetWindowAttribute\0".as_ptr());
                    if !proc.is_null() {
                        let dwm_set_attr: unsafe extern "system" fn(
                            isize,
                            u32,
                            *const std::ffi::c_void,
                            u32,
                        ) -> i32 = std::mem::transmute(proc);

                        // DWMWA_USE_IMMERSIVE_DARK_MODE = 20
                        let dark_mode: i32 = 1;
                        let _ = dwm_set_attr(
                            hwnd as isize,
                            20,
                            &dark_mode as *const _ as *const _,
                            std::mem::size_of::<i32>() as u32,
                        );

                        // DWMWA_CAPTION_COLOR = 35 (#0b1326 -> BGR COLORREF 0x0026130B)
                        let caption_color: u32 = 0x0026130B;
                        let _ = dwm_set_attr(
                            hwnd as isize,
                            35,
                            &caption_color as *const _ as *const _,
                            std::mem::size_of::<u32>() as u32,
                        );
                    }
                }
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn apply_native_window_theme(_window: &Window) {}

/// External Application Shell Host
///
/// Manages the native OS application window lifecycle with customized native exterior frame,
/// DWM dark mode title bar integration, and Laboratory HUD Chrome UI components.
pub struct ShellWindow {
    pub window: Arc<Window>,
    pub chrome: ChromeShell,
    pub tab_manager: TabManager,
}

impl ShellWindow {
    pub fn new<T: 'static>(
        event_loop: &EventLoop<T>,
        title: &str,
        width: u32,
        height: u32,
        tab_manager: TabManager,
    ) -> Self {
        let window = Arc::new(
            WindowBuilder::new()
                .with_title(title)
                .with_decorations(true)
                .with_inner_size(PhysicalSize::new(width, height))
                .with_min_inner_size(PhysicalSize::new(480, 360))
                .build(event_loop)
                .expect("Failed to create native OS application window"),
        );

        apply_native_window_theme(&window);

        Self {
            window,
            chrome: ChromeShell::new(),
            tab_manager,
        }
    }
}
