pub fn flow(input: i32) -> i32 {
    let value = input;
    let copied = value + 1;
    {
        let value = copied;
        consume(value);
    }
    consume(value)
}

fn consume(value: i32) -> i32 {
    value
}
