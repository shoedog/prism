use std::io::{self, Read};
fn invalid() -> Option<(Vec<u8>, usize)> {
    let mut bytes = Vec::new();
    io::stdin().read_to_end(&mut bytes).ok()?;
    let failure = String::from_utf8(bytes).err()?;
    let offset = failure.utf8_error().valid_up_to();
    let original = failure.into_bytes();
    Some((original, offset))
}
