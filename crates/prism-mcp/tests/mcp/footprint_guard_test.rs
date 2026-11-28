//! Measure real idle disposal, including the allocator-retention negative control.
#![cfg(target_os = "macos")]

use crate::transcript::spawn_line_reader;
use serde_json::{json, Value};
use std::io::{self, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const MIB: u64 = 1024 * 1024;
// The guard indexes a PINNED corpus (issue #966), never the live checkout, so its
// three numbers mean the same thing on every PR whatever the PR adds to the
// repository. The corpus is this repository at PINNED_SHA, exported by
// `git archive` without `eval/secbench`. Changing PINNED_SHA changes every number:
// re-pin the baseline below in the same PR and say why.
//
// Adjudication rule: the guard prints `footprint_guard <arm>: loaded=.. unloaded=..`
// for prime, guarded and control. A PR that moves the guarded (product claim)
// number beyond run-to-run spread, or flips the control, says so in its body and
// either re-pins the baseline (better behaviour, with the reason) or fixes the
// regression. The three thresholds alone discriminate; the baseline documents.
//
// Baseline (macOS arm64 host, three runs of main at the pinning change, MiB,
// LOADED / UNLOADED): prime 3074-3130 / 29-31; guarded 2835-2919 / 17-18;
// control 2909-2941 / 1986-2020. Six product trees (main and four PR heads)
// measured 1997-2020 for the control on this corpus.
const PINNED_SHA: &str = "e37d644dcdc323a99cc67028655b045dde3175cc";
const MIN_LOADED: u64 = 512 * MIB;
const GUARDED_CAP: u64 = 128 * MIB; // I0 unloaded-footprint cap.
const MIN_CONTROL: u64 = 256 * MIB;
const BUILD_TIMEOUT: Duration = Duration::from_secs(600);
const DISPOSAL_TIMEOUT: Duration = Duration::from_secs(600);

struct ChildGuard {
    child: Child,
    readers: Vec<JoinHandle<()>>,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        for reader in self.readers.drain(..) {
            let _ = reader.join();
        }
    }
}

/// Read the current footprint, not RSS or the lifetime peak.
fn phys_footprint(pid: u32) -> io::Result<u64> {
    let pid = libc::pid_t::try_from(pid).map_err(io::Error::other)?;
    // SAFETY: zero is valid for this integer-only C structure. The writable
    // buffer has the layout and size selected by RUSAGE_INFO_V2.
    let mut usage: libc::rusage_info_v2 = unsafe { std::mem::zeroed() };
    let status = unsafe {
        libc::proc_pid_rusage(
            pid,
            libc::RUSAGE_INFO_V2,
            (&mut usage as *mut libc::rusage_info_v2).cast(),
        )
    };
    if status != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(usage.ri_phys_footprint)
}

type Lines = Receiver<io::Result<Vec<u8>>>;

fn line_before(lines: &Lines, deadline: Instant, context: &str) -> Vec<u8> {
    lines
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .unwrap_or_else(|error| panic!("{context}: child output unavailable: {error}"))
        .unwrap_or_else(|error| panic!("{context}: child pipe read failed: {error}"))
}

fn request(child: &mut Child, stdout: &Lines, message: Value, deadline: Instant) -> Value {
    writeln!(child.stdin.as_mut().unwrap(), "{message}").expect("write MCP request");
    let line = line_before(
        stdout,
        deadline,
        "waiting for MCP response within build deadline",
    );
    let response: Value = serde_json::from_slice(&line).expect("JSON MCP response");
    assert_eq!(response["id"], message["id"], "{response}");
    assert!(response.get("error").is_none(), "{response}");
    assert!(response["result"].is_object(), "{response}");
    response
}

fn wait_for_event(stderr: &Lines, event: &str, deadline: Instant, seen: &mut Vec<String>) {
    let token = format!("event={event}");
    loop {
        let line = line_before(stderr, deadline, &format!("waiting for {token}"));
        let text = String::from_utf8_lossy(&line).into_owned();
        let found = text.split_whitespace().any(|field| field == token);
        seen.push(text);
        if found {
            return;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {token}");
    }
}

struct Measurement {
    arm: &'static str,
    loaded: u64,
    unloaded: u64,
}

fn measure(
    arm: &'static str,
    opt_out: bool,
    cache: &std::path::Path,
    corpus: &std::path::Path,
) -> Measurement {
    let mut command = Command::new(env!("CARGO_BIN_EXE_prism-mcp"));
    command
        .arg("--repo")
        .arg(corpus)
        .arg("--cache-dir")
        .arg(cache)
        .args(["--index-idle-secs", "3"])
        .env_remove("PRISM_MCP_INDEX_IDLE_SECS")
        .env_remove("MallocSpaceEfficient")
        .env_remove("PRISM_MCP_MALLOC_SPACE_EFFICIENT")
        .env_remove("PRISM_MCP_MEM_REPORT")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if opt_out {
        command.env("PRISM_MCP_MALLOC_SPACE_EFFICIENT", "0");
    }
    let mut process = ChildGuard {
        child: command.spawn().expect("spawn footprint guard child"),
        readers: Vec::new(),
    };
    let pid = process.child.id();
    let (stdout, reader) = spawn_line_reader(process.child.stdout.take().unwrap());
    process.readers.push(reader);
    let (stderr, reader) = spawn_line_reader(process.child.stderr.take().unwrap());
    process.readers.push(reader);
    // Fail early if this environment refuses child-process inspection.
    phys_footprint(pid).expect("proc_pid_rusage child inspection preflight");
    let started = Instant::now();
    let deadline = started + BUILD_TIMEOUT;
    request(
        &mut process.child,
        &stdout,
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "protocolVersion":"2025-11-25","capabilities":{},
            "clientInfo":{"name":"footprint-guard","version":"0"}
        }}),
        deadline,
    );
    writeln!(
        process.child.stdin.as_mut().unwrap(),
        "{}",
        json!({"jsonrpc":"2.0","method":"notifications/initialized"})
    )
    .unwrap();
    let mut id = 2;
    loop {
        assert!(
            Instant::now() < deadline,
            "{arm}: nav_repo_map still index warming after 600 s; footprint guard requires a served index"
        );
        let response = request(
            &mut process.child,
            &stdout,
            json!({"jsonrpc":"2.0","id":id,"method":"tools/call",
                "params":{"name":"nav_repo_map","arguments":{}}}),
            deadline,
        );
        if response["result"]["_meta"]["prism/index_state"] != "warming" {
            assert_eq!(response["result"]["isError"], false, "{arm}: {response}");
            break;
        }
        id += 1;
        std::thread::sleep(Duration::from_millis(250));
    }
    let loaded = phys_footprint(pid).expect("proc_pid_rusage LOADED child footprint");
    let build_wait = started.elapsed();
    let deadline = Instant::now() + DISPOSAL_TIMEOUT;
    let mut seen = Vec::new();
    wait_for_event(&stderr, "unload", deadline, &mut seen);
    wait_for_event(&stderr, "disposal_end", deadline, &mut seen);
    // Diagnostic: the server's own events, so a number is read together with how
    // the index came to be (a cache load takes seconds, a build about a minute).
    let events: Vec<&str> = seen
        .iter()
        .flat_map(|l| l.split_whitespace())
        .filter_map(|f| f.strip_prefix("event="))
        .collect();
    eprintln!("footprint_guard {arm}: server events: {}", events.join(","));
    std::thread::sleep(Duration::from_secs(10));
    let unloaded = phys_footprint(pid).expect("proc_pid_rusage UNLOADED child footprint");
    let summary = format!(
        "footprint_guard {arm}: loaded={:.1} MiB unloaded={:.1} MiB build_wait={:.2}s",
        loaded as f64 / MIB as f64,
        unloaded as f64 / MIB as f64,
        build_wait.as_secs_f64()
    );
    eprintln!("{summary}");
    if let Some(path) = std::env::var_os("GITHUB_STEP_SUMMARY") {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("open GITHUB_STEP_SUMMARY");
        writeln!(file, "{summary}").expect("append footprint summary");
    }
    Measurement {
        arm,
        loaded,
        unloaded,
    }
}

/// Export the pinned corpus into a fresh directory. The commit comes from the
/// checkout's object store when present, else it is fetched (`--depth 1`) from
/// `origin`; no workflow edit is needed because CI checks the repository out.
fn pinned_corpus() -> tempfile::TempDir {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(args)
            .output()
            .expect("run git")
    };
    let spec = format!("{PINNED_SHA}^{{commit}}");
    if !git(&["cat-file", "-e", &spec]).status.success() {
        let fetched = git(&["fetch", "--depth", "1", "origin", PINNED_SHA]);
        assert!(
            fetched.status.success(),
            "cannot fetch pinned corpus {PINNED_SHA}: {}",
            String::from_utf8_lossy(&fetched.stderr)
        );
    }
    let dir = tempfile::tempdir().expect("pinned corpus directory");
    let archive = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args([
            "archive",
            "--format=tar",
            PINNED_SHA,
            "--",
            ".",
            ":(exclude)eval/secbench",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn git archive");
    let mut archive = archive;
    let untar = Command::new("tar")
        .arg("-x")
        .arg("-C")
        .arg(dir.path())
        .stdin(archive.stdout.take().unwrap())
        .status()
        .expect("run tar");
    assert!(
        untar.success(),
        "tar extraction of the pinned corpus failed"
    );
    assert!(archive.wait().expect("wait git archive").success());
    assert!(
        dir.path().join("Cargo.toml").is_file(),
        "pinned corpus is empty"
    );
    dir
}

#[test]
#[ignore = "macOS footprint guard; run by the macos-footprint CI job"]
fn footprint_guard() {
    // A reload after idle loads the index from the cache, so both measured arms
    // hit a cache the prime run builds; a cache hit also retains more memory
    // without the allocator setting than a cold build does (532 vs ~220 MiB).
    let cache = tempfile::tempdir().expect("fresh isolated cache directory");
    let corpus = pinned_corpus();
    // measure returns only after its child is killed and reaped: never overlap arms.
    let prime = measure("prime", false, cache.path(), corpus.path());
    let guarded = measure("guarded", false, cache.path(), corpus.path());
    let control = measure("control", true, cache.path(), corpus.path());
    for measured in [&prime, &guarded, &control] {
        assert!(
            measured.loaded >= MIN_LOADED,
            "{}: LOADED {} MiB < 512 MiB; index was not really built, guard is vacuous",
            measured.arm,
            measured.loaded / MIB
        );
    }
    assert!(
        guarded.unloaded <= GUARDED_CAP,
        "guarded: UNLOADED {} MiB > 128 MiB I0 cap; idle unload retained memory",
        guarded.unloaded / MIB
    );
    assert!(
        control.unloaded >= MIN_CONTROL,
        "guard cannot discriminate on this host: control UNLOADED {} MiB < 256 MiB",
        control.unloaded / MIB
    );
}
