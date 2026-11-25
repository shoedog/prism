pub fn label() -> String {
    let known: Result<String, ()> = Ok(String::from("sample"));
    let choice = known.unwrap_or(std::env::var("EDITION").unwrap_or_default());
    choice
}
