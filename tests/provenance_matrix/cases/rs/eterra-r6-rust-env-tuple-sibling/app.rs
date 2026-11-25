fn main() {
    let (mode, count) = (std::env::var("MODE"), 3);
    println!("{:?} {}", mode, count);
}
