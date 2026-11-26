use std::io;
fn main() {
    let mut line = String::new();
    let result = io::stdin().read_line(&mut line);
    println!("{:?} {:?}", line, result);
}
