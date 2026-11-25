unsafe fn read_mode() -> *mut std::ffi::c_char {
    let mode = unsafe { libc::getenv(c"MODE".as_ptr()) };
    mode
}
