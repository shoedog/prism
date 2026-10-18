fn owner(input: i32) -> i32 {
    let value = input;
    let result = value + input;
    {
        let value = 7;
        consume(value);
    }
    consume(value);
    result
}

fn sibling(input: i32) -> i32 {
    let value = input;
    consume(value);
    value
}

fn consume(value: i32) -> i32 {
    value
}
