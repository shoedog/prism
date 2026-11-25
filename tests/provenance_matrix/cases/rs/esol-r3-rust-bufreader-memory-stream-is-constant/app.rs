use std::io::{BufRead, BufReader, Cursor};

pub fn read() -> Vec<u8> {
    let memory = Cursor::new(b"preview\n");
    let mut reader = BufReader::new(memory);
    let mut buffer = Vec::new();
    reader.read_until(b'\n', &mut buffer).unwrap();
    buffer
}
