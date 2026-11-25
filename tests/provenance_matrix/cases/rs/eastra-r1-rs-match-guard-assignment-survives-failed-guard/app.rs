pub fn label() -> String {
    let mut selected = std::env::var("EDITION").unwrap_or_default();
    match () {
        () if { selected = String::from("sample"); false } => unreachable!(),
        () => selected,
    }
}
