use std::ffi::CString;
use windows_sys::{
    core::*, Win32::Foundation::*, Win32::Graphics::Gdi::ValidateRect,
    Win32::System::LibraryLoader::GetModuleHandleA, Win32::UI::WindowsAndMessaging::*,
};

pub struct Window {
    hwnd: HWND,
    pub exit: bool,
}

pub fn create(name: &str, width: u32, height: u32) -> Window {
    println!("Create {:?}", name);
    unsafe {
        let instance = GetModuleHandleA(std::ptr::null());
        let class_name = s!("window");
        let window_class = WNDCLASSA {
            style: CS_VREDRAW | CS_HREDRAW,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: 0,
            hCursor: 0,
            hbrBackground: 0,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name,
        };
        let registered = RegisterClassA(&window_class);
        debug_assert_ne!(registered, 0);

        let instance_name = CString::new(name).unwrap();
        let hwnd = CreateWindowExA(
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
            instance,
            std::ptr::null(),
        );
        Window { hwnd, exit: false }
    }
}

pub fn update(window: &mut Window) {
    unsafe {
        let mut message = std::mem::zeroed();
        let result = GetMessageA(&mut message, window.hwnd, 0, 0);
        if result == 0 {
            window.exit = true;
        } else {
            DispatchMessageA(&message);
        }
    }
}

extern "system" fn wndproc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match message {
            WM_PAINT => {
                ValidateRect(window, std::ptr::null());
                0
            }
            WM_CLOSE => {
                println!("WM_CLOSE");
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcA(window, message, wparam, lparam),
        }
    }
}
