enum Edition { Command(String), Environment(String) }
fn label() -> Option<String> {
    let item = if let Some(arg) = std::env::args().nth(1) {
        Edition::Command(arg)
    } else {
        Edition::Environment(std::env::var("EDITION").ok()?)
    };
    match item {
        Edition::Environment(word) | Edition::Command(word) => {
            Some(word)
        }
    }
}
