fn main() {
    let mode = std::env::var("MODE");
    { let mode = "fixed"; println!("{}", mode); }
    println!("{:?}", mode);
}
