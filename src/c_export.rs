use std::{ffi::CStr, os::raw::c_char};

use crate::combine::combine_mp4;

#[export_name = "mp4_combine"]
pub extern "C" fn c_combine(
    init: *const c_char,
    part: *const c_char,
    output: *const c_char,
) -> bool {
    let init = unsafe { CStr::from_ptr(init) }
        .to_string_lossy()
        .to_string();
    let part = unsafe { CStr::from_ptr(part) }
        .to_string_lossy()
        .to_string();
    let output = unsafe { CStr::from_ptr(output) }
        .to_string_lossy()
        .to_string();

    match combine_mp4(init, part, output) {
        Ok(_) => true,
        Err(err) => {
            eprintln!("combine error: {}", err);
            false
        }
    }
}
