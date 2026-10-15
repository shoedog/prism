//! The daemon's log: its standard error, which the shim that starts it points at `<key16>.log`
//! (spec §4.1 step 6, §4.3). The daemon stops adding its own lines past 8 MiB and says so once
//! (§4.3). A panic message, which the panic hook writes, is not counted: it is followed by the
//! end of the process.
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};

/// The daemon writes no line of its own past this size.
const CAP_BYTES: u64 = 8 << 20;
/// Longest line the daemon writes; text that comes from elsewhere (an error's message) is cut.
const MAX_LINE_BYTES: usize = 1000;

/// Bytes this process has written to the log.
static WRITTEN: AtomicU64 = AtomicU64::new(0);

/// Write one line, `prism-mcp daemon: <text>`, with control characters replaced and at most
/// 1000 bytes of `text`, unless the log has reached its cap. The line that would cross the cap
/// is replaced by one note that the cap was reached. A failed write is ignored: there is nowhere
/// else to report it.
pub(super) fn line(text: &str) {
    let text = printable_prefix(text, MAX_LINE_BYTES);
    let line = format!("prism-mcp daemon: {text}\n");
    let len = u64::try_from(line.len()).unwrap_or(u64::MAX);
    let before = WRITTEN.fetch_add(len, Ordering::SeqCst);
    let mut stderr = std::io::stderr().lock();
    if before.saturating_add(len) <= CAP_BYTES {
        let _ = stderr.write_all(line.as_bytes());
    } else if before <= CAP_BYTES {
        let _ = writeln!(
            stderr,
            "prism-mcp daemon: the log reached its cap of {} MiB; later lines are dropped",
            CAP_BYTES >> 20
        );
    }
}

/// At most `max` bytes of `text`, cut on a character boundary, with every control character
/// replaced: a line must not be able to start another or drive a terminal.
pub(super) fn printable_prefix(text: &str, max: usize) -> String {
    let mut out = String::with_capacity(text.len().min(max));
    for c in text.chars() {
        let c = if c.is_control() { '\u{fffd}' } else { c };
        if out.len() + c.len_utf8() > max {
            break;
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Text from elsewhere cannot start a new line in the log or grow it past a line's bound.
    #[test]
    fn a_line_has_no_control_characters_and_a_bounded_length() {
        assert_eq!(
            printable_prefix("a\nb\x1b[2Jc", 100),
            "a\u{fffd}b\u{fffd}[2Jc"
        );
        assert_eq!(printable_prefix("ééé", 5), "éé");
        assert_eq!(printable_prefix(&"x".repeat(5000), 1000).len(), 1000);
    }
}
