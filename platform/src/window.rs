use std::{
    ffi::CString,
    time::{Duration, Instant},
};
use renderer::Renderer;
use windows_sys::{
    core::*,
    Win32::{
        Foundation::*, Graphics::Gdi::ValidateRect, System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    },
};

#[derive(Default)]
pub struct Window {
    pub width: u32,
    pub height: u32,
    pub internal: WindowInternal,
}

pub struct Callbacks {
    pub resize: fn(u32, u32),
    pub close: fn(),
    pub exit: fn(),
    pub repaint: fn(),
}

#[derive(Debug, Default)]
pub struct WindowInternal {
    pub initialized: bool,
    pub hinstance: HINSTANCE,
    pub hwnd: HWND,
}

pub fn update_window(window: &mut Window, callbacks: &mut Callbacks) {
    if !window.internal.initialized {
        create_window(
            "Best Game",
            window.width,
            window.height,
            &mut window.internal,
            callbacks,
        );
    }
    get_events_with_timeout(&mut window.internal, 10);
}

fn create_window(
    name: &str,
    width: u32,
    height: u32,
    internal: &mut WindowInternal,
    callbacks: &mut Callbacks,
) {
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

        let instance_name = CString::new(name).unwrap();
        internal.hwnd = CreateWindowExW(
            0,
            class_name,
            instance_name.as_ptr() as *const _,
            WS_VISIBLE | WS_OVERLAPPEDWINDOW,
            0,
            0,
            width as _,
            height as _,
            0,
            0,
            internal.hinstance,
            std::ptr::from_mut(callbacks) as *const std::ffi::c_void,
        );

        internal.initialized = true;
    }
}

fn get_events_with_timeout(internal: &mut WindowInternal, timeout_ms: u64) {
    unsafe {
        let mut message: MSG = std::mem::zeroed();
        let start = Instant::now();
        while PeekMessageW(&mut message, internal.hwnd, 0, 0, PM_REMOVE) > 0 {
            println!("Get msg: elapsed {:?}", start.elapsed());
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
        let callbacks_ptr = if message == WM_CREATE {
            let create_ptr = lparam as *const CREATESTRUCTW;
            let callbacks_ptr = (*create_ptr).lpCreateParams as *mut Callbacks;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, callbacks_ptr as _);
            callbacks_ptr
        } else {
            GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Callbacks
        };

        if let Some(callbacks) = callbacks_ptr.as_mut() {
            match message {
                WM_SIZE => {
                    (callbacks.resize)(100, 100)
                }
                WM_PAINT => {
                    (callbacks.repaint)();
                    ValidateRect(hwnd, std::ptr::null());
                    result = 0;
                }
                WM_CLOSE => {
                    (callbacks.close)();
                    result = 0;
                }
                WM_DESTROY => {
                    (callbacks.exit)();
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
