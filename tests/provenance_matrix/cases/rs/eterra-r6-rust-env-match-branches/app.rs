fn main() {
    let mode;
    if std::env::args().len() > 1 { mode = std::env::var("MODE").unwrap(); }
    else { mode = String::from("fixed"); }
    println!("{}", mode);
}
