//! The line a shim prints when it fails closed (spec §4.1 step 7): why, the key, the runtime
//! directory, the socket, the pid file's contents, the log, and the exact `--standalone`
//! invocation that runs without the shared daemon. One line, whatever the paths hold: control
//! characters are escaped (lane README, notes for S5a), and the invocation is quoted for the
//! shell so that it still names the very paths.
use crate::mcp::daemon::files::{open_private, open_runtime_dir, Create, FilesError};
use crate::mcp::shared::identity::{refresh_policy_name, IdentityKey};
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::{CacheMode, RefreshPolicy, ServerConfig, StartupMode};
use std::ffi::OsString;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::time::Duration;

/// The default `--first-call-wait`, which the standalone invocation leaves out.
const DEFAULT_FIRST_CALL_WAIT: Duration = Duration::from_secs(20);
/// Most bytes of a pid file the line repeats.
const PID_FILE_BYTES: u64 = 200;

/// Where the shim looked, as far as it got.
pub(super) struct Where<'a> {
    /// `None` when the failure came before the key could be computed.
    pub(super) key: Option<&'a IdentityKey>,
    /// `None` when no runtime directory could be resolved.
    pub(super) runtime_dir: Option<&'a Path>,
    /// `None` when the socket path does not fit (or no runtime directory was resolved).
    pub(super) paths: Option<&'a DaemonPaths>,
    /// Whether the runtime directory passed validation. When it did not, nothing in it is read:
    /// what is there may be anyone's, a FIFO or a terminal among them (review round 1, Opus F1,
    /// sol 1).
    pub(super) dir_validated: bool,
}

/// `text` with every control character escaped (`\n`, `\u{1b}`), so it cannot end the line or
/// drive a terminal.
pub(super) fn one_line(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            let escaped: Vec<char> = if c.is_control() {
                c.escape_default().collect()
            } else {
                vec![c]
            };
            escaped
        })
        .collect()
}

/// The fail-closed line, without its newline.
pub(super) fn fail_closed_line(reason: &str, at: &Where<'_>, standalone: &str) -> String {
    let mut line = format!("prism-mcp: --shared failed closed, no index was built: {reason}");
    match at.key {
        Some(key) => line.push_str(&format!("; key {key}")),
        None => line.push_str("; key unresolved"),
    }
    match at.runtime_dir {
        Some(dir) => line.push_str(&format!("; runtime dir {}", dir.display())),
        None => line.push_str("; runtime dir unresolved"),
    }
    if let Some(paths) = at.paths {
        line.push_str(&format!(
            "; socket {}; pid file {}; log {}",
            paths.socket().display(),
            pid_file(paths, at.dir_validated),
            paths.log_file().display()
        ));
    }
    // The invocation is not escaped again: `standalone_invocation` quotes it for the shell
    // already, with no control character left, and escaping inside its quotes would change the
    // paths it names (review round 1, sol 2).
    let mut line = one_line(&line);
    line.push_str("; to run without the shared daemon: ");
    line.push_str(standalone);
    line
}

/// What the pid file says (spec §4.1 step 6: a wedged daemon is named by "the pid and start
/// time from `<key>.pid`"). The daemon rewrites it in place, so it can be empty or partial
/// while its owner writes it (S4's interface); that is reported as such.
///
/// It is read only in a runtime directory that passed validation, and then as the daemon's own
/// files are: through the directory's descriptor, never through a symlink, without waiting for
/// a writer (`O_NONBLOCK`), and only when it is a regular file of this user. So nothing at its
/// path can keep the line from being printed (review round 1, Opus F1, sol 1).
pub(super) fn pid_file(paths: &DaemonPaths, dir_validated: bool) -> String {
    let path = paths.pid_file();
    if !dir_validated {
        return format!(
            "{} not read (the runtime directory failed validation)",
            path.display()
        );
    }
    let mut text = String::new();
    let read = open_runtime_dir(paths.runtime_dir())
        .and_then(|dir| open_private(&dir, path, libc::O_RDONLY | libc::O_NONBLOCK, Create::No))
        .and_then(|file| {
            file.take(PID_FILE_BYTES)
                .read_to_string(&mut text)
                .map_err(|source| FilesError::Io {
                    op: "read",
                    path: path.to_path_buf(),
                    source,
                })
        });
    match read {
        Err(FilesError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
            format!("{} absent", path.display())
        }
        Err(FilesError::Unusable { path: dir, why }) => {
            format!("{} not read: {} {why}", path.display(), dir.display())
        }
        Err(error) => format!("{} unreadable ({error})", path.display()),
        Ok(_) => {
            let fields: Vec<&str> = text.split('\n').collect();
            match fields.as_slice() {
                [pid, start, build, ""]
                    if pid.parse::<u32>().is_ok() && start.parse::<u64>().is_ok() =>
                {
                    format!(
                        "{}: pid {pid}, start time {start}, build {build}",
                        path.display()
                    )
                }
                _ => format!("{}: empty or partial ({text:?})", path.display()),
            }
        }
    }
}

/// The invocation that serves the same configuration in process: this executable, the same
/// repository, cache, refresh policy, first-call wait and warm start, and `--standalone`. It is
/// one line of shell a user can run as printed: every word is quoted for the shell from its
/// bytes, so it holds no control character whatever the paths hold (review round 1, sol 2).
pub(super) fn standalone_invocation(exe: &Path, cfg: &ServerConfig) -> String {
    let mut words: Vec<OsString> = vec![
        exe.as_os_str().to_owned(),
        "--repo".into(),
        cfg.repo_root.as_os_str().to_owned(),
    ];
    if let CacheMode::Dir(base) = &cfg.cache {
        words.extend(["--cache-dir".into(), base.as_os_str().to_owned()]);
    }
    if cfg.refresh_policy != RefreshPolicy::WarnOnly {
        words.extend([
            "--refresh-policy".into(),
            refresh_policy_name(cfg.refresh_policy).into(),
        ]);
    }
    if cfg.first_call_wait != DEFAULT_FIRST_CALL_WAIT {
        words.extend([
            "--first-call-wait".into(),
            cfg.first_call_wait.as_secs().to_string().into(),
        ]);
    }
    if cfg.startup == StartupMode::Background {
        words.push("--warm-at-startup".into());
    }
    words.push("--standalone".into());
    words
        .iter()
        .map(|word| shell_word(word.as_bytes()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `word` as a shell reads it back: bare when it is plain; single-quoted when it is UTF-8 text
/// with no control character; otherwise ANSI-C quoted (`$'...'`, which bash, zsh and ksh read),
/// with `\n`, `\t` and a three-digit octal `\NNN` for every control character and every byte that
/// is not UTF-8, so that the quoted word is one line and names the very bytes.
fn shell_word(word: &[u8]) -> String {
    match std::str::from_utf8(word) {
        Ok(text) if !text.contains(char::is_control) => {
            let plain = !text.is_empty()
                && text
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "_./=:,+@%-".contains(c));
            if plain {
                text.to_string()
            } else {
                format!("'{}'", text.replace('\'', "'\\''"))
            }
        }
        _ => ansi_c_quoted(word),
    }
}

/// `word` as `$'...'`. Each byte escape is three octal digits, which every one of those shells
/// reads as exactly three; ksh93 reads `\xHH` greedily, taking a hex digit after it as part of it
/// (review round 2, Opus F-A).
fn ansi_c_quoted(word: &[u8]) -> String {
    let mut quoted = String::from("$'");
    let hex = |quoted: &mut String, bytes: &[u8]| {
        for byte in bytes {
            quoted.push_str(&format!("\\{byte:03o}"));
        }
    };
    for chunk in word.utf8_chunks() {
        for c in chunk.valid().chars() {
            match c {
                '\n' => quoted.push_str("\\n"),
                '\t' => quoted.push_str("\\t"),
                '\\' => quoted.push_str("\\\\"),
                '\'' => quoted.push_str("\\'"),
                c if c.is_control() => hex(&mut quoted, c.encode_utf8(&mut [0; 4]).as_bytes()),
                c => quoted.push(c),
            }
        }
        hex(&mut quoted, chunk.invalid());
    }
    quoted.push('\'');
    quoted
}

#[cfg(test)]
#[path = "diagnostic_tests.rs"]
mod tests;
