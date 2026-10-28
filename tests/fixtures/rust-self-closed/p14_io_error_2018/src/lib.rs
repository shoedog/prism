pub trait Error { fn code(&self) -> Option<i32>; }
#[allow(bare_trait_objects)]
impl Error {
    pub fn raw_os_error(&self) -> bool { true }
}
mod io_impl {
    use std::io::Error;
    impl super::Error for Error {
        fn code(&self) -> Option<i32> { Self::raw_os_error(self) }
    }
}
