//! Opt-in build profiling (`PRISM_PROFILE=/path/profile.jsonl`). No AST walks when disabled.
mod owner;
mod summary;
pub(crate) use owner::Owner;
pub(crate) use search::search_refs;
mod search;
use crate::process_usage::{cpu_ns, ru_maxrss_bytes};
use serde::Serialize;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static PROFILE: OnceLock<Option<Mutex<Profile>>> = OnceLock::new();

struct Profile {
    writer: Option<BufWriter<File>>,
    phases: Vec<PhaseRecord>,
    owners: Vec<owner::OwnerRecord>,
}

fn collector() -> Option<&'static Mutex<Profile>> {
    PROFILE
        .get_or_init(|| {
            let path = std::env::var_os("PRISM_PROFILE")?;
            match File::create(&path) {
                Ok(file) => Some(Mutex::new(Profile {
                    writer: Some(BufWriter::new(file)),
                    phases: Vec::new(),
                    owners: Vec::new(),
                })),
                Err(error) => {
                    eprintln!(
                        "PRISM_PROFILE: cannot create {}: {error}",
                        std::path::Path::new(&path).display()
                    );
                    None
                }
            }
        })
        .as_ref()
}

pub(crate) fn enabled() -> bool {
    collector().is_some()
}

/// CLI lifetime guard. Finishes even when a command returns an error.
pub struct Session;
impl Session {
    pub fn start() -> Self {
        let _ = collector();
        Self
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        finish();
    }
}

fn finish() {
    let Some(profile) = collector() else { return };
    let Ok(mut profile) = profile.lock() else {
        return;
    };
    let Some(mut writer) = profile.writer.take() else {
        return;
    };
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
                wall_ns: 0,
                cpu_ns: None,
                peak_rss_bytes: None,
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
        line(
            &mut writer,
            &serde_json::json!({"record":"header", "schema_version":1, "time_unit":"ns", "memory_unit":"bytes", "peak_rss_scope":"process_lifetime", "top_n":10}),
        )?;
        for phase in &profile.phases {
            line(&mut writer, phase)?;
        }
        for phase in owner::file_rd(&profile.owners, &profile.phases) {
            line(&mut writer, &phase)?;
        }
        for owner in &profile.owners {
            line(&mut writer, owner)?;
        }
        line(&mut writer, &summary::summarize(&profile.owners, 10))?;
        writer.flush()
    })();
    if let Err(error) = result {
        eprintln!("PRISM_PROFILE: cannot write profile: {error}");
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
    wall_ns: u64,
    cpu_ns: Option<u64>,
    peak_rss_bytes: Option<u64>,
}

pub(crate) struct Phase {
    name: &'static str,
    file: Option<String>,
    thread: bool,
    started: Instant,
    cpu: Option<u64>,
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
            wall_ns: ns(self.started),
            cpu_ns: delta(self.cpu, cpu_ns(self.thread)),
            peak_rss_bytes: ru_maxrss_bytes(),
        };
        if let Some(profile) = collector() {
            if let Ok(mut p) = profile.lock() {
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
