fn load() -> Option<String> {
    let mode = 'lookup: loop {
        break 'lookup std::env::var("MODE").ok();
    };
    mode
}
