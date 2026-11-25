fn getenv_later() -> &'static str { "fixed" }
fn main() {
    let mode = getenv_later();
    println!("{}", mode);
}
