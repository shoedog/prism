//! Shell-escape sanitizers (CWE-78 / OsCommand category).
//!
//! Empty in Phase 1 per spec §3.9 — the shell cleanser would be a no-op because
//! no Phase 1 sink consumes `*exec.Cmd`. The const exists for symmetry with
//! `PATH_RECOGNIZERS` and forward-extension; future phases populate it as new
//! sinks consuming cleansed `*exec.Cmd` arrive.

use super::SanitizerRecognizer;

pub const SHELL_RECOGNIZERS: &[SanitizerRecognizer] = &[];
