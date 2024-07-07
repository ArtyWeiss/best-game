use std::time::{Duration, Instant};
use windows_sys::{
    core::*,
    Win32::{Foundation::*, System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*},
};

use crate::keycodes::{to_keycode, KeyCode};

pub struct Window {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub events: Vec<WindowEvent>,
    pub exists: bool,
    pub(crate) internal: Box<WindowInternal>,
}

impl Window {
    pub fn new(title: String, width: u32, height: u32) -> Self {
        Self {
            title,
            width,
            height,
            events: vec![],
            exists: true,
            internal: Default::default(),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct WindowInternal {
    pub initialized: bool,
    pub destroyed: bool,

    pub hinstance: HINSTANCE,
    pub hwnd: HWND,
    pub events: Vec<WindowEvent>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum WindowEvent {
    Mouse { event: MouseEvent },
    Key { pressed: bool, key: KeyCode },
    Resize { width: u32, height: u32 },
    Close,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MouseEvent {
    Move { x: i16, y: i16 },
    Button { pressed: bool, button: MouseButton },
    Wheel,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

pub fn update_window(window: &mut Window) {
    if !window.internal.initialized {
        create_window(
            &window.title,
            window.width,
            window.height,
            window.internal.as_mut(),
        );
        window.exists = true;
    }
    get_events_with_timeout(window.internal.as_mut(), 10);
    window.events = window.internal.events.drain(..).collect();

    if window.internal.destroyed {
        window.exists = false;
    }
}

fn create_window(name: &str, width: u32, height: u32, internal: &mut WindowInternal) {
    println!("Create {:?}", name);
    unsafe {
        internal.hinstance = GetModuleHandleW(std::ptr::null());

        let class_name = w!("BestGameWindow");
        let window_class = WNDCLASSW {
            style: CS_VREDRAW | CS_HREDRAW,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: internal.hinstance,
            hIcon: 0,
            hCursor: 0,
            hbrBackground: 0,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name,
        };
        let registered = RegisterClassW(&window_class);
        debug_assert_ne!(registered, 0);

        let mut instance_name: Vec<u16> = name.encode_utf16().collect();
        instance_name.push(0);

        internal.hwnd = CreateWindowExW(
            0,
            class_name,
            instance_name.as_ptr(),
            WS_VISIBLE | WS_OVERLAPPEDWINDOW,
            0,
            0,
            width as _,
            height as _,
            0,
            0,
            internal.hinstance,
            std::ptr::from_mut(internal) as *const std::ffi::c_void,
        );

        internal.initialized = true;
    }
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

        if let Some(internal) = internal_ptr.as_mut() {
            match message {
                WM_SIZE => {
                    internal.events.push(WindowEvent::Resize {
                        width: get_loword(lparam as _) as u32,
                        height: get_hiword(lparam as _) as u32,
                    });
                }
                WM_KEYDOWN => {
                    internal.events.push(WindowEvent::Key {
                        pressed: true,
                        key: to_keycode(wparam as u32),
                    });
                }
                WM_KEYUP => {
                    internal.events.push(WindowEvent::Key {
                        pressed: false,
                        key: to_keycode(wparam as u32),
                    });
                }
                WM_LBUTTONDOWN => {
                    internal.events.push(WindowEvent::Mouse {
                        event: MouseEvent::Button {
                            pressed: true,
                            button: MouseButton::Left,
                        },
                    });
                }
                WM_LBUTTONUP => {
                    internal.events.push(WindowEvent::Mouse {
                        event: MouseEvent::Button {
                            pressed: false,
                            button: MouseButton::Left,
                        },
                    });
                }
                WM_RBUTTONDOWN => {
                    internal.events.push(WindowEvent::Mouse {
                        event: MouseEvent::Button {
                            pressed: true,
                            button: MouseButton::Right,
                        },
                    });
                }
                WM_RBUTTONUP => {
                    internal.events.push(WindowEvent::Mouse {
                        event: MouseEvent::Button {
                            pressed: false,
                            button: MouseButton::Right,
                        },
                    });
                }
                WM_MOUSEMOVE => {
                    let x = get_x_lparam(lparam as _);
                    let y = get_y_lparam(lparam as _);
                    internal.events.push(WindowEvent::Mouse {
                        event: MouseEvent::Move { x, y },
                    });
                    result = 0;
                }
                WM_MOUSEWHEEL => {
                    internal.events.push(WindowEvent::Mouse {
                        event: MouseEvent::Wheel,
                    });
                    result = 0;
                }
                // todo: разобраться, что писать в QuitMessage
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

fn get_loword(lparam: i32) -> u16 {
    lparam as u16
}

fn get_hiword(lparam: i32) -> u16 {
    (lparam >> 16) as u16
}

fn get_x_lparam(lparam: i32) -> i16 {
    lparam as i16
}

fn get_y_lparam(lparam: i32) -> i16 {
    (lparam >> 16) as i16
}
