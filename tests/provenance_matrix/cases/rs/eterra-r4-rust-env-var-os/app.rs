fn main() {
    let mode = std::env::var_os("MODE");
    println!("{:?}", mode);
}
