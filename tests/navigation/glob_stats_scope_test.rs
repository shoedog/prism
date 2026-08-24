//! `call_stats` glob-expansion telemetry must be scoped to the graph it
//! measures, not a process-global sink.
//!
//! Regression for a CI flake in
//! `go_concrete_cache_test::concrete_receiver_outputs_match_*` (PR #191): that
//! test compares `call_stats` JSON byte-equal across four cache paths and once
//! failed with `glob_expand.resolved_l1: 0 vs 1`. `call_stats` used to reset a
//! PROCESS-GLOBAL `glob_stats::GLOBAL` at entry and snapshot it after the
//! resolution pass, and the name-resolution engine fell back to that same
//! global whenever no scoped sink was injected — so any *other* test in the
//! same test binary doing resolution concurrently landed its increments inside
//! the reset→snapshot window.

use prism::navigation::{queries, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier};

fn write(root: &std::path::Path, path: &str, source: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("fixture parent")).unwrap();
    std::fs::write(path, source).unwrap();
}

/// A tiny Rust repo whose calls only resolve through a two-hop `use ...::*`
/// chain, so `call_stats` reports non-zero `glob_expand` counters.
fn glob_session(dir: &std::path::Path, tag: &str) -> NavigationSession {
    write(
        dir,
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nedition = \"2021\"\n[lib]\npath = \"src/lib.rs\"\n",
    );
    write(
        dir,
        "src/lib.rs",
        &format!(
            "mod prelude;\nmod inner;\npub use crate::prelude::*;\nfn run_{tag}() -> i32 {{ helper() + nested() }}\n"
        ),
    );
    write(
        dir,
        "src/prelude.rs",
        "pub use crate::inner::*;\npub fn helper() -> i32 { 1 }\n",
    );
    write(dir, "src/inner.rs", "pub fn nested() -> i32 { 2 }\n");
    let repo = Arc::new(load_repo(dir).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    NavigationSession { repo, index }
}

fn glob_expand(session: &NavigationSession) -> serde_json::Value {
    queries::call_stats(session.index.call_graph())["glob_expand"].clone()
}

#[test]
fn call_stats_glob_expand_is_unaffected_by_concurrent_resolution_on_another_graph() {
    // The contaminating write is a data race on a shared counter, so there is
    // no injectable observer that makes a single overlap deterministic: the
    // window is widened instead by keeping the interfering thread in a tight
    // resolution loop for the whole measured stretch and repeating the measured
    // call N times. Pre-fix this fails on the first few iterations.
    const ITERATIONS: usize = 50;

    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
    let session_a = glob_session(dir_a.path(), "a");
    let session_b = glob_session(dir_b.path(), "b");

    // Single-threaded reference for graph A, taken with nothing else running.
    let baseline = glob_expand(&session_a);
    assert!(
        baseline["resolved_l1"].as_u64().unwrap_or(0) > 0,
        "fixture must actually exercise glob expansion, got {baseline}"
    );

    let stop = Arc::new(AtomicBool::new(false));
    let barrier = Arc::new(Barrier::new(2));
    let interferer = {
        let stop = Arc::clone(&stop);
        let barrier = Arc::clone(&barrier);
        let index_b = Arc::clone(&session_b.index);
        std::thread::spawn(move || {
            barrier.wait();
            while !stop.load(Ordering::Relaxed) {
                // Independent graph, independent measurement: whatever this
                // does must be invisible to graph A's counters.
                let _ = queries::call_stats(index_b.call_graph());
            }
        })
    };
    barrier.wait();

    let mut observed = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        observed.push(glob_expand(&session_a));
    }
    stop.store(true, Ordering::Relaxed);
    interferer.join().unwrap();

    for (i, ge) in observed.iter().enumerate() {
        assert_eq!(
            ge, &baseline,
            "iteration {i}: call_stats glob_expand must equal the single-threaded \
             result; concurrent resolution on an unrelated graph leaked into it"
        );
    }
}
