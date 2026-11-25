fn main() {
    let mut mode = std::env::var("MODE").unwrap_or_default();
    mode = String::from("fixed");
    println!("{}", mode);
}
