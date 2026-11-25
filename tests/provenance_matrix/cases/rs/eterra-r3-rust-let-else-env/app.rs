fn main() {
    let Ok(mode) = std::env::var("MODE") else { return; };
    println!("{}", mode);
}
