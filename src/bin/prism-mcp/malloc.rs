//! libmalloc reads its policy at process start; re-exec before dispatch or worker creation.
use std::ffi::OsStr;

#[derive(Debug, PartialEq, Eq)]
enum MallocPlan {
    Reexec,
    Keep,
    InvalidOptOut,
}

fn malloc_plan(space_efficient: Option<&OsStr>, opt_out: Option<&OsStr>) -> MallocPlan {
    if space_efficient.is_some() {
        return MallocPlan::Keep;
    }
    match opt_out {
        None => MallocPlan::Reexec,
        Some(value) if value.is_empty() || value == "1" => MallocPlan::Reexec,
        Some(value) if value == "0" => MallocPlan::Keep,
        Some(_) => MallocPlan::InvalidOptOut,
    }
}

#[cfg(target_os = "macos")]
pub(super) fn restart_if_needed() {
    use std::io::Write;

    let space_efficient = std::env::var_os("MallocSpaceEfficient");
    let opt_out = std::env::var_os("PRISM_MCP_MALLOC_SPACE_EFFICIENT");
    let plan = malloc_plan(space_efficient.as_deref(), opt_out.as_deref());
    if let Some(warning) = apply_plan(plan, restart) {
        let _ = writeln!(std::io::stderr(), "{warning}");
    }
}

/// Returning a warning leaves startup free to continue after either kind of restart error.
fn apply_plan(plan: MallocPlan, restart: impl FnOnce() -> std::io::Error) -> Option<String> {
    match plan {
        MallocPlan::Keep => None,
        MallocPlan::InvalidOptOut => Some(
            "prism-mcp: PRISM_MCP_MALLOC_SPACE_EFFICIENT accepts unset, empty, 1, or 0; continuing with default allocator settings".into()
        ),
        MallocPlan::Reexec => {
            let error = restart();
            Some(format!(
                "prism-mcp: could not restart with MallocSpaceEfficient=1: {error}; continuing with default allocator settings"
            ))
        }
    }
}

#[cfg(target_os = "macos")]
fn restart() -> std::io::Error {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    let executable = match std::env::current_exe() {
        Ok(executable) => executable,
        Err(error) => return error,
    };
    let mut command = Command::new(executable);
    if let Some(arg0) = std::env::args_os().next() {
        command.arg0(arg0);
    }
    command
        .args(std::env::args_os().skip(1))
        .env("MallocSpaceEfficient", "1")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .exec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keep_does_not_restart_or_warn() {
        assert_eq!(
            apply_plan(MallocPlan::Keep, || panic!("must not restart")),
            None
        );
    }

    #[test]
    fn invalid_opt_out_warns_without_restarting() {
        assert_eq!(
            apply_plan(MallocPlan::InvalidOptOut, || panic!("must not restart")),
            Some("prism-mcp: PRISM_MCP_MALLOC_SPACE_EFFICIENT accepts unset, empty, 1, or 0; continuing with default allocator settings".into())
        );
    }

    #[test]
    fn restart_errors_return_one_warning_and_allow_continuation() {
        for message in ["current_exe failed", "exec failed"] {
            let mut attempts = 0;
            let warning = apply_plan(MallocPlan::Reexec, || {
                attempts += 1;
                std::io::Error::other(message)
            });
            assert_eq!(attempts, 1);
            assert_eq!(
                warning,
                Some(format!("prism-mcp: could not restart with MallocSpaceEfficient=1: {message}; continuing with default allocator settings"))
            );
            assert_eq!(warning.unwrap().lines().count(), 1);
        }
    }

    #[test]
    fn caller_setting_wins_even_when_empty_or_opt_out_invalid() {
        for space in ["", "0", "1", "other"] {
            for opt_out in [None, Some(""), Some("0"), Some("1"), Some("bad")] {
                assert_eq!(
                    malloc_plan(Some(OsStr::new(space)), opt_out.map(OsStr::new)),
                    MallocPlan::Keep,
                    "space={space:?} opt_out={opt_out:?}"
                );
            }
        }
    }

    #[test]
    fn absent_opt_out_enables_restart() {
        assert_eq!(malloc_plan(None, None), MallocPlan::Reexec);
    }

    #[test]
    fn empty_or_one_opt_out_enables_restart() {
        for value in ["", "1"] {
            assert_eq!(
                malloc_plan(None, Some(OsStr::new(value))),
                MallocPlan::Reexec
            );
        }
    }

    #[test]
    fn zero_opt_out_keeps_default_allocator() {
        assert_eq!(malloc_plan(None, Some(OsStr::new("0"))), MallocPlan::Keep);
    }

    #[test]
    fn other_opt_out_values_are_invalid() {
        for value in ["bad", "01", "true", " 1", "0\n", "é"] {
            assert_eq!(
                malloc_plan(None, Some(OsStr::new(value))),
                MallocPlan::InvalidOptOut
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn non_unicode_is_present_for_caller_setting_and_invalid_for_opt_out() {
        use std::os::unix::ffi::OsStrExt;
        let bytes = OsStr::from_bytes(b"\xff");
        assert_eq!(malloc_plan(Some(bytes), Some(bytes)), MallocPlan::Keep);
        assert_eq!(malloc_plan(None, Some(bytes)), MallocPlan::InvalidOptOut);
    }
}
