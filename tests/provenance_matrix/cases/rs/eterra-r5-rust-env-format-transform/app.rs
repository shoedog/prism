fn main() {
    let raw = std::env::var("MODE").unwrap();
    let value = format!("mode={}", raw);
    println!("{}", value);
}
