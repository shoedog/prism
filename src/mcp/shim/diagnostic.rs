//! The line a shim prints when it fails closed (spec §4.1 step 7): why, the key, the runtime
//! directory, the socket, the pid file's contents, the log, and the exact `--standalone`
//! invocation that runs without the shared daemon. One line, whatever the paths hold: control
//! characters are escaped (lane README, notes for S5a).
use crate::mcp::shared::identity::{refresh_policy_name, IdentityKey};
use crate::mcp::shared::paths::DaemonPaths;
use crate::mcp::{CacheMode, RefreshPolicy, ServerConfig, StartupMode};
use std::io::Read;
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
            pid_file(paths.pid_file()),
            paths.log_file().display()
        ));
    }
    line.push_str(&format!("; to run without the shared daemon: {standalone}"));
    one_line(&line)
}

/// What the pid file says (spec §4.1 step 6: a wedged daemon is named by "the pid and start
/// time from `<key>.pid`"). The daemon rewrites it in place, so it can be empty or partial
/// while its owner writes it (S4's interface); that is reported as such.
fn pid_file(path: &Path) -> String {
    let mut text = String::new();
    let read = std::fs::File::open(path)
        .and_then(|file| file.take(PID_FILE_BYTES).read_to_string(&mut text));
    match read {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            format!("{} absent", path.display())
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
/// repository, cache, refresh policy, first-call wait and warm start, and `--standalone`.
pub(super) fn standalone_invocation(exe: &Path, cfg: &ServerConfig) -> String {
    let mut words = vec![
        exe.display().to_string(),
        "--repo".into(),
        cfg.repo_root.display().to_string(),
    ];
    if let CacheMode::Dir(base) = &cfg.cache {
        words.extend(["--cache-dir".into(), base.display().to_string()]);
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
            cfg.first_call_wait.as_secs().to_string(),
        ]);
    }
    if cfg.startup == StartupMode::Background {
        words.push("--warm-at-startup".into());
    }
    words.push("--standalone".into());
    words
        .iter()
        .map(|word| shell_word(word))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `word` as a POSIX shell reads it back: bare when it is plain, else single-quoted.
fn shell_word(word: &str) -> String {
    let plain = !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_./=:,+@%-".contains(c));
    if plain {
        word.to_string()
    } else {
        format!("'{}'", word.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
#[path = "diagnostic_tests.rs"]
mod tests;
