fn write() {
    let words: Vec<String> = std::env::args().skip(1).collect();
    if let [first @ _, tail @ ..] = words.as_slice() {
        let count = tail.len();
        println!("{:?}", (first, count));
    }
}
