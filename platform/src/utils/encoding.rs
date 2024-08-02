use std::{ffi::OsStr, fmt::Display, iter::once, os::windows::ffi::OsStrExt};

pub fn encode_wide(string: impl AsRef<OsStr>) -> Vec<u16> {
    string.as_ref().encode_wide().chain(once(0)).collect()
}
