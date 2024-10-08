use std::ffi::CStr;

pub const FRAMES_IN_FLIGHT: usize = 2;
pub const VALIDATION_NAME: &CStr =
    unsafe { CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0") };
