use std::{fs, io, path::Path};

fn load(path: &Path) -> io::Result<Vec<u8>> {
    let bytes = fs::read(path)?;
    Ok(bytes)
}
