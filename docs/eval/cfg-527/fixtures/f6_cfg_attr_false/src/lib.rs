#[cfg_attr(feature = "x", cfg(any()))]
pub fn g() -> u8 { 1 }
pub fn caller() -> u8 { g() }
