use std::{
    ffi::CString,
    time::{Duration, Instant},
};
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
    pub exit: bool,
    pub(crate) internal: Box<WindowInternal>,
}

#[derive(Debug, Default)]
pub struct WindowInternal {
    pub initialized: bool,
    pub hinstance: HINSTANCE,
    pub hwnd: HWND,
    pub events: Vec<WindowEvent>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum WindowEvent {
    Resize,
    Key,
    Mouse,
    Exit,
}

pub fn update_window(window: &mut Window) {
    window.internal.events.clear();

    if !window.internal.initialized {
        create_window(
            "Best Game",
            window.width,
            window.height,
            window.internal.as_mut(),
        );
    }
    get_events_with_timeout(window.internal.as_mut(), 10);

    if !window.internal.events.is_empty() {
        println!("Events:\n{:#?}", window.internal.events);
    }

    if window.internal.events.contains(&WindowEvent::Exit) {
        window.exit = true;
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
            println!("Get msg: elapsed {:?}", start.elapsed());
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
                WM_MOUSEMOVE => {
                    internal.events.push(WindowEvent::Mouse);
                    result = 0;
                }
                WM_PAINT => {
                    ValidateRect(hwnd, std::ptr::null());
                    result = 0;
                }
                WM_DESTROY => {
                    internal.events.push(WindowEvent::Exit);
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
