fn report(_: u8, _: std::io::Stdin) {}

fn read_value() {
    let label = 7;
    report(label, std::io::stdin());
}
