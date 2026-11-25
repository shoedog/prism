fn main() {
    if let Ok(mode) = std::env::var("MODE") {
        println!("{}", mode);
    }
}
