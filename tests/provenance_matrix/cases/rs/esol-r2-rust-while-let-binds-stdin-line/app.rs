fn main() {
    let mut lines = std::io::stdin().lines();
    while let Some(Ok(line)) = lines.next() {
        println!("{}", line);
    }
}
