use crate::utils;
use std::time::{Duration, Instant};
use windows_sys::Win32::{
    Foundation::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Input::KeyboardAndMouse::{
            GetKeyboardLayout, MapVirtualKeyExW, MAPVK_VK_TO_VSC_EX, VIRTUAL_KEY,
        },
        WindowsAndMessaging::*,
    },
};

use keycodes::scancode_to_key;
pub use keycodes::KeyCode;
mod keycodes;

pub struct Window {
    pub title: String,
    pub outer_size: PhysicalSize,
    pub inner_size: PhysicalSize,
    pub events: Vec<WindowEvent>,
    pub exists: bool,
    pub(crate) internal: Box<WindowInternal>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalSize {
    pub x: u32,
    pub y: u32,
}

impl PhysicalSize {
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
    pub(crate) fn from_lparam(lparam: isize) -> Self {
        let x = get_loword(lparam as _) as u32;
        let y = get_hiword(lparam as _) as u32;
        Self { x, y }
    }
}

impl Window {
    pub fn new(title: String, width: u32, height: u32) -> Self {
        Self {
            title,
            outer_size: PhysicalSize::new(width, height),
            inner_size: PhysicalSize::new(width, height),
            events: vec![],
            exists: true,
            internal: Default::default(),
        }
    }
    pub fn hwnd(&self) -> HWND {
        self.internal.hwnd
    }
    pub fn hinstance(&self) -> HINSTANCE {
        self.internal.hinstance
    }
}

#[derive(Debug, Default)]
pub(crate) struct WindowInternal {
    pub initialized: bool,
    pub destroyed: bool,
    pub outer_size: PhysicalSize,
    pub inner_size: PhysicalSize,

    pub hinstance: HINSTANCE,
    pub hwnd: HWND,
    pub events: Vec<WindowEvent>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum WindowEvent {
    Mouse { event: MouseEvent },
    Key { pressed: bool, key: KeyCode },
    Resize,
    Close,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MouseEvent {
    Move { x: i32, y: i32 },
    Button { pressed: bool, button: MouseButton },
    Wheel,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

// PUBLIC FUNCTIONS =============================================================================
pub fn update_window(window: &mut Window) {
    if !window.internal.initialized {
        create_window(&window.title, window.outer_size, window.internal.as_mut());
        window.exists = true;
    }
    get_events_with_timeout(window.internal.as_mut(), 10);
    window.events = window.internal.events.drain(..).collect();
    window.inner_size = window.internal.inner_size;
    window.outer_size = window.internal.outer_size;

    if window.internal.destroyed {
        window.exists = false;
    }
}

// PRIVATE FUNCTIONS ===========================================================================
fn create_window(name: &str, size: PhysicalSize, internal: &mut WindowInternal) {
    unsafe {
        internal.hinstance = GetModuleHandleW(std::ptr::null());

        let title = utils::encode_wide(name);
        let class_name = utils::encode_wide("BestGameWindow");

        register_window_class(internal.hinstance, &class_name);

        internal.hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_VISIBLE | WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            size.x as _,
            size.y as _,
            0,
            0,
            internal.hinstance,
            std::ptr::from_mut(internal) as *const std::ffi::c_void,
        );
        internal.initialized = true;
    }
    utils::trace(format!("Window {:?} created", name));
}

fn get_events_with_timeout(internal: &mut WindowInternal, timeout_ms: u64) {
    unsafe {
        let mut message: MSG = std::mem::zeroed();
        let start = Instant::now();
        while PeekMessageW(&mut message, internal.hwnd, 0, 0, PM_REMOVE) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
            if start.elapsed() > Duration::from_millis(timeout_ms) {
                return;
            }
        }
    }
}

extern "system" fn wndproc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        let mut result = -1;
        let internal_ptr = if message == WM_CREATE {
            let create_ptr = lparam as *const CREATESTRUCTW;
            let internal_ptr = (*create_ptr).lpCreateParams as *mut WindowInternal;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, internal_ptr as _);
            internal_ptr
        } else {
            GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowInternal
        };

        let kb_layout = GetKeyboardLayout(0);

        if let Some(internal) = internal_ptr.as_mut() {
            match message {
                WM_SIZE => {
                    let size = PhysicalSize::from_lparam(lparam);
                    let mut rect: RECT = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                    let result = GetWindowRect(hwnd, &mut rect);
                    assert_eq!(result, 1);
                    let outer_size = PhysicalSize::new(
                        (rect.right - rect.left) as u32,
                        (rect.bottom - rect.top) as u32,
                    );

                    internal.inner_size = size;
                    internal.outer_size = outer_size;
                    internal.events.push(WindowEvent::Resize);
                }
                WM_KEYDOWN => {
                    let v_key = wparam as VIRTUAL_KEY;
                    let scancode = MapVirtualKeyExW(v_key as u32, MAPVK_VK_TO_VSC_EX, kb_layout);
                    internal
                        .events
                        .push(WindowEvent::Key { pressed: true, key: scancode_to_key(scancode) });
                }
                WM_KEYUP => {
                    let v_key = wparam as VIRTUAL_KEY;
                    let scancode = MapVirtualKeyExW(v_key as u32, MAPVK_VK_TO_VSC_EX, kb_layout);
                    internal
                        .events
                        .push(WindowEvent::Key { pressed: false, key: scancode_to_key(scancode) });
                }
                WM_LBUTTONDOWN => {
                    internal.events.push(WindowEvent::Mouse {
                        event: MouseEvent::Button { pressed: true, button: MouseButton::Left },
                    });
                }
                WM_LBUTTONUP => {
                    internal.events.push(WindowEvent::Mouse {
                        event: MouseEvent::Button { pressed: false, button: MouseButton::Left },
                    });
                }
                WM_RBUTTONDOWN => {
                    internal.events.push(WindowEvent::Mouse {
                        event: MouseEvent::Button { pressed: true, button: MouseButton::Right },
                    });
                }
                WM_RBUTTONUP => {
                    internal.events.push(WindowEvent::Mouse {
                        event: MouseEvent::Button { pressed: false, button: MouseButton::Right },
                    });
                }
                WM_MOUSEMOVE => {
                    let x = get_x_lparam(lparam as _);
                    let y = get_y_lparam(lparam as _);
                    internal.events.push(WindowEvent::Mouse { event: MouseEvent::Move { x, y } });
                    result = 0;
                }
                WM_MOUSEWHEEL => {
                    internal.events.push(WindowEvent::Mouse { event: MouseEvent::Wheel });
                    result = 0;
                }
                WM_CLOSE => {
                    internal.events.push(WindowEvent::Close);
                    internal.destroyed = true;
                    PostQuitMessage(0);
                    result = 0;
                }
                WM_DESTROY => {
                    internal.destroyed = true;
                    PostQuitMessage(0);
                    result = 0;
                }
                _ => {}
            }
        }

        if result >= 0 {
            result
        } else {
            DefWindowProcW(hwnd, message, wparam, lparam)
        }
    }
}

unsafe fn register_window_class(hinstance: HINSTANCE, class_name: &[u16]) {
    let class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wndproc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: 0,
        hCursor: 0,
        hbrBackground: 0,
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_name.as_ptr(),
        hIconSm: 0,
    };
    let registered = RegisterClassExW(&class);
    debug_assert_ne!(registered, 0);
}

fn get_loword(lparam: u32) -> u32 {
    lparam & 0xffff
}

fn get_hiword(lparam: u32) -> u32 {
    (lparam >> 16) & 0xffff
}

fn get_x_lparam(lparam: i32) -> i32 {
    lparam & 0xffff
}

fn get_y_lparam(lparam: i32) -> i32 {
    (lparam >> 16) & 0xffff
}
