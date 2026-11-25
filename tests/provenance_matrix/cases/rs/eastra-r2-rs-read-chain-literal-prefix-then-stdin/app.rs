use std::io::{self, Cursor, Read};
fn bytes() -> io::Result<(u8, u8)> {
    let mut joined = Cursor::new([42u8]).chain(io::stdin());
    let mut pair = [0u8; 2];
    joined.read_exact(&mut pair)?;
    let (prefix, payload) = (pair[0], pair[1]);
    Ok((prefix, payload))
}
