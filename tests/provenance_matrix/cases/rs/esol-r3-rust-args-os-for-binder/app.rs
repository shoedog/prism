pub fn show() {
    for submitted in std::env::args_os().skip(1) {
        println!("{:?}", submitted);
    }
}
