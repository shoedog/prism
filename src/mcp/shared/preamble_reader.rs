//! Reads one preamble line from a connection, bounded while the bytes arrive (spec §4.2, §7: a
//! preamble line is at most 4 KiB) and without taking a single byte past its newline, because
//! the same connection carries MCP frames next.
use super::preamble::PREAMBLE_MAX_BYTES;
use std::fmt;
use std::io::{self, Read};

/// Why no line was returned.
#[derive(Debug)]
pub enum PreambleReadError {
    /// More than [`PREAMBLE_MAX_BYTES`] bytes arrived without a newline. The reader has taken
    /// one byte past the limit and will not return a line again.
    TooLong,
    /// The stream ended before the newline; `received` bytes of the line had arrived. For a
    /// shim waiting for the reply this is the transient case of spec §4.1 step 5.
    Eof { received: usize },
    /// A read failed. For `WouldBlock` and `TimedOut` (a non-blocking socket, a read timeout)
    /// the bytes read so far are kept: calling [`PreambleLineReader::read_line`] again
    /// continues the same line. Whether to call again is the caller's deadline to decide.
    Io(io::Error),
}

impl fmt::Display for PreambleReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong => write!(f, "preamble line is longer than {PREAMBLE_MAX_BYTES} bytes"),
            Self::Eof { received } => write!(
                f,
                "connection closed {received} bytes into the preamble line"
            ),
            Self::Io(error) => write!(f, "reading the preamble line failed: {error}"),
        }
    }
}

impl std::error::Error for PreambleReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

/// Accumulates one line across as many reads as it takes. Holds at most
/// [`PREAMBLE_MAX_BYTES`] bytes.
#[derive(Debug, Default)]
pub struct PreambleLineReader {
    line: Vec<u8>,
    overflowed: bool,
}

impl PreambleLineReader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read from `source` up to and including the next newline and return the line without
    /// it. Reads one byte at a time: a preamble is a few hundred bytes read once per
    /// connection, and whatever follows the newline must stay in `source` for the transport
    /// that takes over. `source` is typically a `UnixStream`, not a `BufReader`.
    pub fn read_line(&mut self, source: &mut impl Read) -> Result<Vec<u8>, PreambleReadError> {
        if self.overflowed {
            return Err(PreambleReadError::TooLong);
        }
        let mut byte = [0u8; 1];
        loop {
            match source.read(&mut byte) {
                Ok(0) => {
                    return Err(PreambleReadError::Eof {
                        received: self.line.len(),
                    })
                }
                Ok(_) if byte[0] == b'\n' => return Ok(std::mem::take(&mut self.line)),
                Ok(_) => {
                    if self.line.len() == PREAMBLE_MAX_BYTES {
                        self.overflowed = true;
                        return Err(PreambleReadError::TooLong);
                    }
                    self.line.push(byte[0]);
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(PreambleReadError::Io(error)),
            }
        }
    }
}

#[cfg(test)]
#[path = "preamble_reader_tests.rs"]
mod tests;
