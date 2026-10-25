//! Execution-boundary regressions for the large-stack build pool (issue #513).
//!
//! Each route runs four cases in a child process of this test binary, under a
//! parent watchdog that kills the child at a bounded deadline, so a stack abort
//! or a foreign-pool deadlock fails one case and leaves no stuck process:
//! - `ordinary`: a 2 MiB ordinary thread, shallow input (behaves as before);
//! - `same`: a build-pool worker (inline, same thread);
//! - `foreign`: a one-worker foreign pool, `rayon::join(A, B)` where A holds a
//!   mutex across the call and B takes it (issue #513 item 1);
//! - `deep`: a 2 MiB ordinary thread, 8192-deep input (issue #513 item 2).
use super::{build_pool, install};
use crate::{ast::ParsedFile, call_graph::CallGraph, languages::Language};
use std::{
    collections::BTreeMap,
    process::{Command, Stdio},
    sync::{mpsc, Arc, Barrier, Mutex},
    time::{Duration, Instant},
};

type Prebuilt = Option<crate::cpg::CodePropertyGraph>;

fn construct(route: &str, files: &BTreeMap<String, ParsedFile>, prebuilt: Prebuilt) {
    if route == "dfg" || route == "dfg_subset" {
        let graph = if route == "dfg" {
            crate::data_flow::DataFlowGraph::build(files)
        } else {
            crate::data_flow::DataFlowGraph::build_subset(files, &files.keys().cloned().collect())
        };
        assert!(graph.defs().next().is_some());
        return;
    }
    if route == "cpg_without" {
        let ctx = crate::cpg::CpgContext::without_cpg(files, None);
        assert!(ctx.live_types.contains("Thing"));
        return;
    }
    if route == "cpg_scoped" {
        // One changed file with no callers or callees: the scope stays under half
        // of the files, so the scoped (second-pass) path runs.
        let diff = crate::diff::DiffInput {
            files: vec![crate::diff::DiffInfo {
                file_path: "b.rs".into(),
                modify_type: crate::diff::ModifyType::Modified,
                diff_lines: [1].into(),
            }],
        };
        let ctx = crate::cpg::CpgContext::build_scoped(files, &diff, None);
        assert!(ctx.scope.is_some(), "scoped path taken");
        assert!(ctx.live_types.contains("Thing"));
        return;
    }
    if route.starts_with("cpg") {
        use crate::cpg::CpgContext;
        let ctx = match (route, prebuilt) {
            ("cpg_fresh", Some(cpg)) => CpgContext::build_with_fresh_cpg(files, cpg, None),
            ("cpg_cached", Some(cpg)) => CpgContext::build_with_cached_cpg(files, cpg, None),
            ("cpg_registry", _) => CpgContext::build_with_registry(
                files,
                None,
                crate::type_provider::TypeRegistry::empty(),
            ),
            ("cpg", None) => CpgContext::build(files, None),
            _ => panic!("unknown route {route}"),
        };
        assert!(ctx.cpg.call_graph.functions.contains_key("caller"));
        return;
    }
    use crate::{call_graph::ScopeGraphBuildInputs, resolution::ReceiverRecoveryConfig};
    let config = ReceiverRecoveryConfig::default();
    let inputs = ScopeGraphBuildInputs::from_files_convention(files);
    let only = files.keys().cloned().collect();
    let graph = match route {
        "build" => CallGraph::build(files),
        "skeleton" => CallGraph::build_skeleton(files),
        "config" => CallGraph::build_with_receiver_config(files, &config),
        "scope" => CallGraph::build_with_scope_graph_inputs(files, Some(&inputs)),
        "config_scope" => CallGraph::build_with_receiver_config_and_scope_graph_inputs(
            files,
            &config,
            Some(&inputs),
        ),
        "subset" => CallGraph::build_direct_subset(files, &only),
        "subset_config" => {
            CallGraph::build_direct_subset_with_receiver_config(files, &only, &config)
        }
        _ => panic!("unknown route {route}"),
    };
    assert!(graph.functions.contains_key("caller"));
    assert!(graph
        .calls
        .values()
        .flatten()
        .any(|s| s.callee_name == "leaf"));
}

fn ts_source(deep: bool) -> String {
    let depth = if deep { 8192 } else { 2 };
    format!(
        "class Thing {{}} const t = new Thing(); function leaf() {{ return 1; }} function caller() {{ leaf(); const v = {}0{}; return v; }}",
        "[".repeat(depth),
        "]".repeat(depth)
    )
}

fn files(deep: bool) -> BTreeMap<String, ParsedFile> {
    [
        ("a.ts", ts_source(deep), Language::TypeScript),
        (
            "b.rs",
            "struct T; impl T { fn hit(&self) {} } fn rust_call(x: T) { x.hit(); }".into(),
            Language::Rust,
        ),
        (
            "c.go",
            "package p\ntype T struct{}\nfunc (x T) Hit() {}\nfunc goCall(x T) { x.Hit() }".into(),
            Language::Go,
        ),
    ]
    .into_iter()
    .map(|(path, source, lang)| {
        let parsed = ParsedFile::parse(path, &source, lang).unwrap();
        assert!(!parsed.tree.root_node().has_error(), "parse {path}");
        (path.into(), parsed)
    })
    .collect()
}

// Occupy every build worker until the caller has had time to enter its wait.
// Without this the build pool could finish before the foreign worker starts
// waiting, and the pre-fix cross-pool wait would not need to run B.
fn hold_build_workers() -> std::thread::JoinHandle<()> {
    let workers = build_pool().current_num_threads();
    let gate = Arc::new(Barrier::new(workers + 1));
    let (tx, rx) = mpsc::channel();
    for _ in 0..workers {
        let gate = gate.clone();
        let tx = tx.clone();
        build_pool().spawn(move || {
            tx.send(()).unwrap();
            gate.wait();
        });
    }
    for _ in 0..workers {
        rx.recv_timeout(Duration::from_secs(5)).unwrap();
    }
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        gate.wait();
    })
}

/// A worker of a foreign one-worker pool. With `hazard`, `op` runs in A of
/// `rayon::join(A, B)` holding a mutex that B needs: if the caller waits by
/// running its own pool's queued jobs, B runs on A's worker and blocks forever.
fn foreign(hazard: bool, op: impl FnOnce() + Send) {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let lock = Mutex::new(());
    let release = hazard.then(hold_build_workers);
    pool.install(|| {
        assert!(rayon::current_thread_index().is_some());
        assert!(build_pool().current_thread_index().is_none());
        if !hazard {
            return op();
        }
        rayon::join(
            || {
                let _guard = lock.lock().unwrap();
                eprintln!("A entering constructor with mutex");
                op();
                eprintln!("A returned from constructor");
            },
            || {
                eprintln!("B taking mutex");
                let _guard = lock.lock().unwrap();
            },
        );
    });
    if let Some(release) = release {
        release.join().unwrap();
    }
}

fn panic_value<T: 'static>(f: impl FnOnce() + std::panic::UnwindSafe) -> T {
    *std::panic::catch_unwind(f)
        .unwrap_err()
        .downcast::<T>()
        .unwrap()
}

// The contract of `install` on each route: counter generation reaches `op`,
// a panic in `op` reaches the caller, nested calls run inline, borrows work.
fn install_contract() {
    use crate::type_providers::go::test_counters::{current_generation, inherit_generation};
    let on_pool = |tag: u32| {
        assert!(build_pool().current_thread_index().is_some());
        assert_eq!(current_generation(), 513);
        let before = std::thread::current().id();
        install(|| assert_eq!(std::thread::current().id(), before));
        assert_eq!(
            panic_value::<u32>(|| install(|| std::panic::panic_any(tag))),
            tag
        );
    };
    let mut borrowed = 0;
    inherit_generation(513, || install(|| on_pool(1)));
    install(|| borrowed += 1);
    assert_eq!(borrowed, 1);
    assert_eq!(current_generation(), 0);
    foreign(false, || {
        inherit_generation(513, || install(|| on_pool(2)));
        assert_eq!(current_generation(), 0);
        assert_eq!(
            panic_value::<u32>(|| install(|| std::panic::panic_any(3_u32))),
            3
        );
        assert_eq!(current_generation(), 0);
    });
}

fn child(route: &str, case: &str) {
    if route == "install" && case == "contract" {
        install_contract();
    } else if route == "install" {
        foreign(true, || install(|| ()));
    } else {
        // Parsing is deliberately outside the small constructor thread. A parse
        // failure cannot be mistaken for a constructor's stack failure.
        let deep = case == "deep";
        let loader_dir = (route == "loader").then(|| {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(dir.path().join("a.ts"), ts_source(deep)).unwrap();
            dir
        });
        let files = if loader_dir.is_some() {
            BTreeMap::new()
        } else {
            files(deep)
        };
        // Prebuilt before the case, so only the constructor under test runs in it.
        let prebuilt = matches!(route, "cpg_fresh" | "cpg_cached")
            .then(|| install(|| crate::cpg::CodePropertyGraph::build_enriched(&files, None)));
        eprintln!("parsed fixture; entering {route}/{case}");
        let check = move || match &loader_dir {
            Some(dir) => {
                // The loader parses on the build pool; the caller thread only waits.
                let repo = crate::repo_loader::load_repo(dir.path()).unwrap();
                assert!(repo.files.contains_key("a.ts"));
            }
            None => construct(route, &files, prebuilt),
        };
        match case {
            "foreign" => foreign(true, check),
            "same" => install(|| {
                let before = std::thread::current().id();
                check();
                assert_eq!(std::thread::current().id(), before);
            }),
            "ordinary" | "deep" => std::thread::scope(|s| {
                std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn_scoped(s, || {
                        assert!(rayon::current_thread_index().is_none());
                        check();
                    })
                    .unwrap()
                    .join()
                    .unwrap();
            }),
            _ => panic!("unknown case {case}"),
        }
    }
    eprintln!("completed {route}/{case}");
}

/// Kills the child on every exit path, including a panic in the parent.
struct KillOnDrop(std::process::Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn watchdog(route: &'static str, test: &str) {
    if let Ok(case) = std::env::var("PRISM_BOUNDARY_CASE") {
        std::thread::Builder::new()
            .stack_size(256 * 1024 * 1024)
            .spawn(move || child(route, &case))
            .unwrap()
            .join()
            .unwrap();
        return;
    }
    let cases: &[&str] = if route == "install" {
        &["contract", "foreign"]
    } else {
        &["ordinary", "same", "foreign", "deep"]
    };
    // Run every case before reporting failures, retaining all evidence.
    let mut failures = Vec::new();
    for case in cases {
        let stderr = tempfile::tempfile().unwrap();
        let mut process = KillOnDrop(
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", test, "--nocapture"])
                .env("PRISM_BOUNDARY_CASE", case)
                .env("RAYON_NUM_THREADS", "2")
                .env_remove("RAYON_RS_NUM_CPUS")
                .env_remove("RUST_MIN_STACK")
                .stdout(Stdio::null())
                .stderr(Stdio::from(stderr.try_clone().unwrap()))
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + WATCHDOG;
        let (status, timed_out) = loop {
            if let Some(status) = process.0.try_wait().unwrap() {
                break (status, false);
            }
            if Instant::now() >= deadline {
                process.0.kill().unwrap();
                break (process.0.wait().unwrap(), true);
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        use std::io::{Read, Seek};
        let mut err = stderr;
        err.rewind().unwrap();
        let mut output = String::new();
        err.read_to_string(&mut output).unwrap();
        if timed_out || !status.success() || !output.contains(&format!("completed {route}/{case}"))
        {
            failures.push(format!(
                "{route}/{case}: {status}; watchdog={timed_out}\n{output}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// Generous: a debug build parsing and walking a deep input under a loaded test
// run takes seconds; a deadlocked child never finishes whatever the bound.
const WATCHDOG: Duration = Duration::from_secs(45);

macro_rules! route_test {
    ($name:ident, $route:literal) => {
        #[test]
        fn $name() {
            watchdog(
                $route,
                concat!("build_pool::boundary_tests::", stringify!($name)),
            );
        }
    };
}

route_test!(install_contexts, "install");
route_test!(build_contexts, "build");
route_test!(skeleton_contexts, "skeleton");
route_test!(config_contexts, "config");
route_test!(scope_contexts, "scope");
route_test!(config_scope_contexts, "config_scope");
route_test!(subset_contexts, "subset");
route_test!(subset_config_contexts, "subset_config");
route_test!(dfg_contexts, "dfg");
route_test!(dfg_subset_contexts, "dfg_subset");
route_test!(cpg_contexts, "cpg");
route_test!(cpg_fresh_contexts, "cpg_fresh");
route_test!(cpg_cached_contexts, "cpg_cached");
route_test!(cpg_registry_contexts, "cpg_registry");
route_test!(cpg_without_contexts, "cpg_without");
route_test!(cpg_scoped_contexts, "cpg_scoped");
route_test!(loader_contexts, "loader");
