//! Opt-in build profiling (`PRISM_PROFILE=/path/profile.jsonl`). No AST walks when disabled.
mod owner;
mod summary;
pub(crate) use owner::{file_cfg, Owner};
pub(crate) use search::search_refs;
mod search;
use crate::process_usage::{cpu_ns, ru_maxrss_bytes};
use serde::Serialize;
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static PROFILE: OnceLock<Mutex<Option<Profile>>> = OnceLock::new();
static ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub(crate) mod gates;
mod output;

struct Profile {
    output: output::Output,
    started: Instant,
    cpu: Option<u64>,
    phases: Vec<PhaseRecord>,
    owners: Vec<owner::OwnerRecord>,
}
fn collector() -> &'static Mutex<Option<Profile>> {
    PROFILE.get_or_init(|| Mutex::new(None))
}
pub(crate) fn enabled() -> bool {
    ACTIVE.load(std::sync::atomic::Ordering::Relaxed)
}
/// Process-wide command lifetime. Join build workers before dropping the guard.
/// Calls outside a session never collect, even with PRISM_PROFILE set.
pub struct Session(bool);
impl Session {
    pub fn start() -> Self {
        let Some(path) = std::env::var_os("PRISM_PROFILE") else {
            return Self(false);
        };
        let started = Instant::now();
        let Ok(mut slot) = collector().lock() else {
            return Self(false);
        };
        if slot.is_some() {
            return Self(false);
        }
        let cpu = cpu_ns(false);
        match output::Output::create(std::path::Path::new(&path)) {
            Ok(output) => {
                *slot = Some(Profile {
                    output,
                    started,
                    cpu,
                    phases: Vec::new(),
                    owners: Vec::new(),
                });
                ACTIVE.store(true, std::sync::atomic::Ordering::Relaxed);
                Self(true)
            }
            Err(error) => {
                output::warning(format_args!(
                    "cannot create {}: {error}",
                    std::path::Path::new(&path).display()
                ));
                Self(false)
            }
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        if self.0 {
            finish();
        }
    }
}
/// Finalize normal nonzero CLI outcomes, which bypass Rust destructors.
pub fn exit(code: i32) -> ! {
    finish();
    std::process::exit(code)
}
pub fn clap_exit(error: clap::Error) -> ! {
    finish();
    error.exit()
}

fn finish() {
    if !enabled() {
        return;
    }
    ACTIVE.store(false, std::sync::atomic::Ordering::Relaxed);
    let Ok(mut slot) = collector().lock() else {
        return;
    };
    let Some(mut profile) = slot.take() else {
        return;
    };
    let command = serde_json::json!({"record":"command", "time_kind":"serial_wall",
        "wall_ns":ns(profile.started), "cpu_ns":delta(profile.cpu, cpu_ns(false)),
        "peak_rss_bytes":ru_maxrss_bytes()});
    for name in [
        "load_parse",
        "scope_graph",
        "call_graph",
        "dfg",
        "cpg_assembly",
        "cache.load",
        "cache.save",
    ] {
        if !profile.phases.iter().any(|p| p.name == name) {
            profile.phases.push(PhaseRecord {
                record: "phase",
                name,
                file: None,
                time_kind: "serial_wall",
                aggregation: "single_interval",
                status: "not_run",
                wall_ns: Some(0),
                work_ns: None,
                cpu_ns: None,
                peak_rss_bytes: None,
                rss_rise_bytes: None,
            });
        }
    }
    profile
        .phases
        .sort_by(|a, b| (&a.name, &a.file).cmp(&(&b.name, &b.file)));
    profile
        .owners
        .sort_by(|a, b| (&a.file, a.start_byte, &a.owner).cmp(&(&b.file, b.start_byte, &b.owner)));
    let result = (|| -> std::io::Result<()> {
        let writer = &mut profile.output.writer;
        line(
            writer,
            &serde_json::json!({"record":"header", "schema_version":1, "time_unit":"ns", "memory_unit":"bytes", "peak_rss_scope":"process_lifetime", "top_n":10, "prism_version":env!("CARGO_PKG_VERSION"), "commit":env!("GIT_SHA"), "thread_count":rayon::current_num_threads(), "argv":std::env::args_os().map(|s| s.to_string_lossy().into_owned()).collect::<Vec<_>>(), "os":std::env::consts::OS}),
        )?;
        line(writer, &command)?;
        for phase in &profile.phases {
            line(writer, phase)?;
        }
        for phase in owner::file_rd(&profile.owners, &profile.phases) {
            line(writer, &phase)?;
        }
        for owner in &profile.owners {
            line(writer, owner)?;
        }
        line(writer, &summary::summarize(&profile.owners, 10))?;
        profile.output.publish()
    })();
    if let Err(error) = result {
        output::warning(format_args!("cannot write profile: {error}"));
    }
}

fn line(writer: &mut impl Write, value: &impl Serialize) -> std::io::Result<()> {
    serde_json::to_writer(&mut *writer, value)?;
    writer.write_all(b"\n")
}

#[derive(Serialize)]
pub(crate) struct PhaseRecord {
    record: &'static str,
    name: &'static str,
    file: Option<String>,
    time_kind: &'static str,
    aggregation: &'static str,
    status: &'static str,
    #[serde(rename = "wall_ns", skip_serializing_if = "Option::is_none")]
    wall_ns: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    work_ns: Option<u64>,
    cpu_ns: Option<u64>,
    peak_rss_bytes: Option<u64>,
    rss_rise_bytes: Option<u64>,
}

pub(crate) struct Phase {
    name: &'static str,
    file: Option<String>,
    thread: bool,
    started: Instant,
    cpu: Option<u64>,
    rss: Option<u64>,
}

impl Phase {
    pub(crate) fn serial(name: &'static str) -> Option<Self> {
        Self::begin(name, None)
    }
    pub(crate) fn file(name: &'static str, file: &str) -> Option<Self> {
        Self::begin(name, Some(file))
    }
    fn begin(name: &'static str, file: Option<&str>) -> Option<Self> {
        if !enabled() {
            return None;
        }
        Some(Self {
            name,
            file: file.map(str::to_owned),
            thread: file.is_some(),
            started: Instant::now(),
            cpu: cpu_ns(file.is_some()),
            rss: ru_maxrss_bytes(),
        })
    }
}
impl Drop for Phase {
    fn drop(&mut self) {
        let phase = PhaseRecord {
            record: "phase",
            name: self.name,
            file: self.file.take(),
            time_kind: if self.thread {
                "parallel_work"
            } else {
                "serial_wall"
            },
            aggregation: "single_interval",
            status: "measured",
            wall_ns: Some(ns(self.started)),
            work_ns: None,
            cpu_ns: delta(self.cpu, cpu_ns(self.thread)),
            peak_rss_bytes: ru_maxrss_bytes(),
            rss_rise_bytes: delta(self.rss, ru_maxrss_bytes()),
        };
        if let Ok(mut slot) = collector().lock() {
            if let Some(p) = slot.as_mut() {
                p.phases.push(phase);
            }
        }
    }
}

/// Non-overlapping subphases, nested inside an inclusive parent phase.
pub(crate) struct Stages {
    _total: Option<Phase>,
    current: Option<Phase>,
}
impl Stages {
    pub(crate) fn new(total: &'static str, first: &'static str) -> Self {
        Self {
            _total: Phase::serial(total),
            current: Phase::serial(first),
        }
    }
    pub(crate) fn next(&mut self, name: &'static str) {
        self.current = None;
        self.current = Phase::serial(name);
    }
}
// Drop the last child before its parent, so the inclusive parent covers every child.
impl Drop for Stages {
    fn drop(&mut self) {
        self.current = None;
    }
}

fn ns(start: Instant) -> u64 {
    start.elapsed().as_nanos().min(u64::MAX as u128) as u64
}
fn delta(start: Option<u64>, end: Option<u64>) -> Option<u64> {
    Some(end?.saturating_sub(start?))
}

#[cfg(test)]
mod tests {
    use super::line;
    use std::io::{self, Write};

    struct FailingWrite {
        calls: usize,
        fail_at: usize,
    }

    impl Write for FailingWrite {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.calls += 1;
            if self.calls == self.fail_at {
                Err(io::Error::other("profile write regression witness"))
            } else {
                Ok(bytes.len())
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn record_write_errors_cover_json_and_newline() {
        // Integer JSON is one write; the JSONL delimiter is the next write.
        for fail_at in [1, 2] {
            let mut writer = FailingWrite { calls: 0, fail_at };
            assert!(
                line(&mut writer, &0).is_err(),
                "write {fail_at} was swallowed"
            );
            assert_eq!(writer.calls, fail_at, "writing continued after an error");
        }
    }
}
