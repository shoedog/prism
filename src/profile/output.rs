//! Complete-file publication: independent writers never share a file descriptor.
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);

pub(super) struct Output {
    pub(super) writer: BufWriter<File>,
    staging: PathBuf,
    destination: PathBuf,
}
impl Output {
    pub(super) fn create(path: &Path) -> io::Result<Self> {
        let name = path
            .file_name()
            .ok_or_else(|| io::Error::other("invalid profile path"))?;
        // create_new prevents collisions, including stale files after PID reuse.
        loop {
            let mut temp = std::ffi::OsString::from(".");
            temp.push(name);
            temp.push(format!(
                ".{}.{}.tmp",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let staging = path.with_file_name(temp);
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&staging)
            {
                Ok(file) => {
                    return Ok(Self {
                        writer: BufWriter::new(file),
                        staging,
                        destination: path.to_owned(),
                    })
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
    }
    pub(super) fn publish(&mut self) -> io::Result<()> {
        self.writer.flush()?;
        std::fs::rename(&self.staging, &self.destination)
    }
}
impl Drop for Output {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.staging);
    }
}
pub(super) fn warning(message: std::fmt::Arguments<'_>) {
    let _ = writeln!(io::stderr().lock(), "PRISM_PROFILE: {message}");
}
