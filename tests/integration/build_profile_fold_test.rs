use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn command(repo: &Path, profile: Option<&Path>) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_prism"));
    cmd.env_remove("PRISM_PROFILE")
        .env("RAYON_NUM_THREADS", "1")
        .args(["nav", "--no-cache", "dfg-stats", "--edges", "--repo"])
        .arg(repo);
    if let Some(path) = profile {
        cmd.env("PRISM_PROFILE", path);
    }
    cmd
}
fn rows(path: &Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.js"),
        "function tiny() {\n let a = 1;\n return a;\n}\n",
    )
    .unwrap();
    dir
}
#[cfg(unix)]
fn run_error(repo: &Path, profile: Option<&Path>, closed: bool, late: bool) -> Output {
    use std::os::fd::{FromRawFd, OwnedFd};
    use std::os::unix::process::CommandExt;
    let mut cmd = command(repo, profile);
    if closed {
        let mut fds = [0; 2];
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
        unsafe {
            libc::close(fds[0]);
        }
        cmd.stderr(Stdio::from(unsafe { OwnedFd::from_raw_fd(fds[1]) }));
    }
    // RLIMIT_FSIZE also truncates a coverage-instrumented child's own .profraw,
    // which would break the coverage merge; send it to a throwaway directory.
    let _discard = late.then(|| tempfile::tempdir().unwrap());
    if let Some(d) = &_discard {
        cmd.env("LLVM_PROFILE_FILE", d.path().join("discard-%p.profraw"));
    }
    if late {
        unsafe {
            cmd.pre_exec(|| {
                libc::signal(libc::SIGXFSZ, libc::SIG_IGN);
                let limit = libc::rlimit {
                    rlim_cur: 1,
                    rlim_max: 1,
                };
                if libc::setrlimit(libc::RLIMIT_FSIZE, &limit) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
    cmd.output().unwrap()
}
#[test]
#[cfg(unix)]
fn warning_errors_preserve_status_with_closed_stderr() {
    let dir = fixture();
    for closed in [false, true] {
        for late in [false, true] {
            let path = if late {
                dir.path().join("late.jsonl")
            } else {
                dir.path().join("missing/profile")
            };
            let off = run_error(dir.path(), None, closed, late);
            let on = run_error(dir.path(), Some(&path), closed, late);
            assert!(off.status.success());
            assert_eq!(
                on.status.code(),
                off.status.code(),
                "closed={closed} late={late}"
            );
            assert_eq!(on.stdout, off.stdout);
            if !closed {
                assert_eq!(
                    String::from_utf8_lossy(&on.stderr)
                        .matches("PRISM_PROFILE:")
                        .count(),
                    1
                );
            }
        }
    }
}
#[test]
fn schema_and_skipped_scope_are_truthful() {
    let dir = fixture();
    let path = dir.path().join("profile.jsonl");
    assert!(command(dir.path(), Some(&path))
        .output()
        .unwrap()
        .status
        .success());
    let rs = rows(&path);
    let scope = rs.iter().find(|r| r["name"] == "scope_graph").unwrap();
    assert_eq!(scope["status"], "measured"); // CLI supplies complete scope inputs.
    let total = rs.iter().find(|r| r["record"] == "command").unwrap();
    assert_eq!(total["time_kind"], "serial_wall");
    assert!(total["wall_ns"].as_u64().unwrap() > 0);
    let header = &rs[0];
    for key in ["prism_version", "thread_count", "argv", "os"] {
        assert!(!header[key].is_null(), "{key}");
    }
    for r in rs
        .iter()
        .filter(|r| r["record"] == "phase" && r["status"] == "measured")
    {
        assert!(r["rss_rise_bytes"].is_u64());
        if r["file"].is_string() {
            assert_eq!(r["time_kind"], "parallel_work");
        } else {
            assert_eq!(r["time_kind"], "serial_wall");
        }
    }
    let rd = rs.iter().find(|r| r["name"] == "dfg.file_rd").unwrap();
    assert!(rd["work_ns"].is_u64());
    assert!(rd.get("wall_ns").is_none());
    for name in [
        "call_graph.go_dispatch_embedding",
        "call_graph.python_properties",
        "call_graph.framework_entries",
        "call_graph.apply_js_paths",
        "call_graph.apply_js_export_resolution",
        "call_graph.js_ts_facts",
        "call_graph.parameter_slots",
        "call_graph.struct_init",
    ] {
        assert!(rs.iter().any(|r| r["name"] == name), "{name}");
    }
}
#[test]
fn ordinary_explicit_exit_publishes_a_complete_profile() {
    let dir = fixture();
    let path = dir.path().join("profile.jsonl");
    std::fs::write(&path, "old valid profile\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_prism"))
        .env("PRISM_PROFILE", &path)
        .args([
            "nav",
            "--no-cache",
            "callers",
            "--symbol",
            "doesNotExist",
            "--repo",
        ])
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let rs = rows(&path);
    assert_eq!(rs.last().unwrap()["record"], "summary");
    assert_eq!(rs.iter().filter(|r| r["record"] == "command").count(), 1);
}
#[test]
#[cfg(unix)]
fn overlapping_processes_publish_whole_last_completed_run() {
    use std::time::{Duration, Instant};
    struct Reap(std::process::Child);
    impl Drop for Reap {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let dir = fixture();
    let path = dir.path().join("shared.jsonl");
    std::fs::write(&path, "sentinel\n").unwrap();
    let mut first = Reap(
        command(dir.path(), Some(&path))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    // A sibling staging file witnesses session admission. Stop before finalization.
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut stopped = false;
    while Instant::now() < deadline {
        let admitted = std::fs::read_dir(dir.path()).unwrap().flatten().any(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(".shared.jsonl.")
        });
        if admitted {
            unsafe {
                assert_eq!(libc::kill(first.0.id() as i32, libc::SIGSTOP), 0);
                let mut status = 0;
                assert_eq!(
                    libc::waitpid(first.0.id() as i32, &mut status, libc::WUNTRACED),
                    first.0.id() as i32
                );
                assert!(libc::WIFSTOPPED(status));
            }
            stopped = true;
            break;
        }
        if first.0.try_wait().unwrap().is_some() {
            break;
        }
        std::thread::yield_now();
    }
    if !stopped {
        panic!("no staging-file admission witness");
    }
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "sentinel\n");
    let other = tempfile::tempdir().unwrap();
    std::fs::write(
        other.path().join("b.js"),
        (0..40)
            .map(|n| format!("function other{n}() {{ return {n}; }}\n"))
            .collect::<String>(),
    )
    .unwrap();
    let second = command(other.path(), Some(&path)).output().unwrap();
    unsafe {
        libc::kill(first.0.id() as i32, libc::SIGCONT);
    }
    assert!(first.0.wait().unwrap().success());
    assert!(second.status.success());
    let rs = rows(&path);
    assert_eq!(rs.iter().filter(|r| r["record"] == "summary").count(), 1);
    assert_eq!(rs.iter().filter(|r| r["record"] == "owner").count(), 1);
    assert!(rs.iter().any(|r| r["owner"] == "tiny"));
    assert!(!std::fs::read_dir(dir.path()).unwrap().flatten().any(|e| e
        .file_name()
        .to_string_lossy()
        .starts_with(".shared.jsonl.")));
}

#[test]
fn inner_gate_counts_include_refusals_aliases_and_zero_calls() {
    let dir = tempfile::tempdir().unwrap();
    // Positive: declaration + return = two shadow and fence calls, one scope call.
    // Shadowed: outer query sees four x's but refuses two before fencing;
    // inner query sees two x's, both admitted: shadow=6, fence=4, scope=2.
    // Alias: box query runs twice (3 occurrences each), a once (2), plus
    // one box.x candidate; five scope calls (3 definitions + 2 twins).
    std::fs::write(dir.path().join("a.js"), "function positive() {\n let x = 1;\n return x;\n}\nfunction shadowed() {\n let x = 1;\n { let x = 2;\n consume(x);\n }\n return x;\n}\nfunction alias() {\n let box = {};\n let a = box;\n a.x = 1;\n return box.x;\n}\nfunction zero() { return 0; }\n").unwrap();
    for threads in ["1", "4"] {
        let path = dir.path().join(format!("profile-{threads}.jsonl"));
        let out = command(dir.path(), Some(&path))
            .env("RAYON_NUM_THREADS", threads)
            .output()
            .unwrap();
        assert!(out.status.success());
        let rs = rows(&path);
        for (name, fence, shadow, scope) in [
            ("positive", 2, 2, 1),
            ("shadowed", 4, 6, 2),
            ("alias", 9, 8, 5),
            ("zero", 0, 0, 0),
        ] {
            let owner = rs.iter().find(|r| r["owner"] == name).unwrap();
            assert_eq!(
                owner["counters"]["binding_fence_calls"], fence,
                "{name} threads={threads}"
            );
            assert_eq!(
                owner["counters"]["shadow_check_calls"], shadow,
                "{name} threads={threads}"
            );
            assert_eq!(
                owner["counters"]["def_scope_calls"], scope,
                "{name} threads={threads}"
            );
            assert!(owner["rss_rise_bytes"].is_u64());
        }
    }
}

#[test]
fn tiny_owner_rd_excludes_cfg_in_either_order() {
    // Pin the real producer's nesting/order and clock anchors deterministically.
    cfg_accounting_boundaries_hold(
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/data_flow/build/file.rs"
        )),
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/profile/owner.rs")),
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profile.jsonl");
    // This extra file pins sorted record layout; it does not prove execution order.
    std::fs::write(dir.path().join("0.js"), "function warm() { return 0; }\n").unwrap();
    let tiny = "function tiny() { return 0; }\n";
    let big = format!(
        "function big() {{\n {}\n}}\n",
        (0..160)
            .map(|n| format!("if (test{n}()) {{ left{n}(); }} else {{ right{n}(); }}\n"))
            .collect::<String>()
    );
    for first in [true, false] {
        std::fs::write(
            dir.path().join("a.js"),
            if first {
                format!("{tiny}{big}")
            } else {
                format!("{big}{tiny}")
            },
        )
        .unwrap();
        let out = command(dir.path(), Some(&path)).output().unwrap();
        assert!(out.status.success());
        let rs = rows(&path);
        let owners: Vec<_> = rs.iter().filter(|r| r["record"] == "owner").collect();
        assert_eq!(owners.len(), 3);
        assert_eq!(
            owners[0]["owner"], "warm",
            "0.js sorts first in owner records"
        );
        assert!(owners[0]["file"].as_str().unwrap().ends_with("0.js"));
        let expected = if first {
            ["tiny", "big"]
        } else {
            ["big", "tiny"]
        };
        for (owner, name) in owners[1..].iter().zip(expected) {
            assert_eq!(owner["owner"], name);
            assert!(owner["file"].as_str().unwrap().ends_with("a.js"));
        }
        let phases: Vec<_> = rs
            .iter()
            .filter(|r| r["name"] == "dfg.file_cfg" && r["file"] == owners[1]["file"])
            .collect();
        assert_eq!(phases.len(), 1, "initialize once across both owners");
        let cfg = phases[0];
        assert_eq!(cfg["time_kind"], "parallel_work");
        let init = owners[1]["file_cfg_init_ns"].as_u64().unwrap();
        assert!(init > 0, "first owner must trigger initialization");
        assert_eq!(owners[2]["file_cfg_init_ns"], 0);
        // Same interval, sampled before the phase's drop.
        assert!(init <= cfg["wall_ns"].as_u64().unwrap());
        let rd = rs
            .iter()
            .find(|r| r["name"] == "dfg.file_rd" && r["file"] == owners[1]["file"])
            .unwrap();
        let sum: u64 = owners[1..]
            .iter()
            .map(|r| r["reaching_definitions_ns"].as_u64().unwrap())
            .sum();
        assert_eq!(rd["work_ns"], sum);
        assert_eq!(rd["aggregation"], "sum_owner_intervals");
    }
}

// Structural guard, deliberately separate from the live profile observations:
// it checks actual production calls, including their enclosing statement, not
// a copied model of the initializer. Main's pre-index producer is supported too.
// Anchor text and Rust grammar are intentional: producer refactors may require
// updating these assertions even when the accounting behavior stays correct.
fn cfg_accounting_boundaries_hold(source: &str, owner: &str) {
    fn calls<'a>(
        node: tree_sitter::Node<'a>,
        source: &str,
        name: &str,
    ) -> Vec<tree_sitter::Node<'a>> {
        let mut found = Vec::new();
        if node.kind() == "call_expression"
            && node
                .child_by_field_name("function")
                .is_some_and(|f| &source[f.byte_range()] == name)
        {
            found.push(node);
        }
        for child in node.named_children(&mut node.walk()) {
            found.extend(calls(child, source, name));
        }
        found
    }
    fn function_body<'a>(
        node: tree_sitter::Node<'a>,
        source: &str,
        name: &str,
    ) -> Option<tree_sitter::Node<'a>> {
        if node.kind() == "function_item"
            && node
                .child_by_field_name("name")
                .is_some_and(|n| &source[n.byte_range()] == name)
        {
            return node.child_by_field_name("body");
        }
        for child in node.named_children(&mut node.walk()) {
            if let Some(body) = function_body(child, source, name) {
                return Some(body);
            }
        }
        None
    }
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "file producer must parse as Rust"
    );
    let one_call = |name| {
        let found = calls(tree.root_node(), source, name);
        assert_eq!(found.len(), 1, "unique production call: {name}");
        found[0]
    };
    let cfg = one_call("crate::profile::file_cfg");
    let rd = one_call("crate::profile::Owner::reaching");
    let build = one_call("crate::cfg::build_cfg_edges_with_arms");
    let mut statement = cfg;
    while statement.kind() != "let_declaration" {
        statement = statement.parent().unwrap();
    }
    let inside_cfg = |node: tree_sitter::Node<'_>| {
        cfg.start_byte() < node.start_byte() && node.end_byte() < cfg.end_byte()
    };
    let index = calls(tree.root_node(), source, "FileCfgEdgeIndex::new");
    assert!(inside_cfg(build), "CFG build must be inside file_cfg");
    assert!(
        index.iter().all(|&call| inside_cfg(call)),
        "edge index build must be inside file_cfg"
    );
    assert!(
        statement.end_byte() < rd.start_byte(),
        "CFG initialization must finish before reaching"
    );
    let compact = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    let owner_tree = parser.parse(owner, None).unwrap();
    assert!(
        !owner_tree.root_node().has_error(),
        "owner producer must parse as Rust"
    );
    let body = |name| {
        let node = function_body(owner_tree.root_node(), owner, name)
            .unwrap_or_else(|| panic!("missing owner function: {name}"));
        compact(&owner[node.byte_range()])
    };
    let reaching = body("reaching");
    let flow =
        "owner.row.flow_edges_ns=ns(owner.start).saturating_sub(owner.row.file_cfg_init_ns);";
    let flow_start = reaching
        .find(flow)
        .expect("reaching flow time must subtract CFG init");
    let rd_start = reaching
        .find("owner.rd=Some(Instant::now());")
        .expect("reaching must start RD with a fresh Instant");
    assert!(
        flow_start + flow.len() <= rd_start,
        "RD clock must start after flow time is recorded"
    );
    let file_cfg = body("file_cfg");
    let phase_start = file_cfg
        .find("Phase::file(\"dfg.file_cfg\"")
        .expect("file_cfg must open its CFG phase");
    let run_start = file_cfg
        .find("run()")
        .expect("file_cfg must run the initializer");
    assert!(phase_start < run_start, "CFG phase must open before run()");
    let compact_owner = compact(owner);
    assert!(
        compact_owner.contains(
            "self.row.total_ns=ns(self.start).saturating_sub(self.row.file_cfg_init_ns);"
        ),
        "owner total must subtract CFG init"
    );
    assert!(
        compact_owner.contains("self.row.reaching_definitions_ns=self.rd.map(ns).unwrap_or(0);"),
        "owner RD must use its own clock"
    );
}

#[test]
fn skipped_scope_and_session_only_collection() {
    const CHILD: &str = "PRISM_PROFILE_LIFETIME_CHILD";
    if std::env::var_os(CHILD).is_some() {
        use prism::{ast::ParsedFile, call_graph::CallGraph, languages::Language};
        let path = std::path::PathBuf::from(std::env::var_os("PRISM_PROFILE").unwrap());
        let mut files = std::collections::BTreeMap::new();
        files.insert(
            "a.js".to_owned(),
            ParsedFile::parse("a.js", "function f() { return 0; }", Language::JavaScript).unwrap(),
        );
        // No Session: repeated library work must never admit a collector.
        for _ in 0..3 {
            let _ = CallGraph::build(&files);
        }
        assert!(!path.exists());
        let session = prism::profile::Session::start();
        let _ = CallGraph::build_with_scope_graph_inputs(&files, None);
        drop(session);
        let rs = rows(&path);
        assert_eq!(rs.iter().filter(|r| r["name"] == "call_graph").count(), 1);
        assert!(rs
            .iter()
            .any(|r| r["name"] == "scope_graph" && r["status"] == "not_run"));
        let len = std::fs::metadata(&path).unwrap().len();
        let _ = CallGraph::build(&files);
        assert_eq!(std::fs::metadata(&path).unwrap().len(), len);
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let out = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "build_profile_fold_test::skipped_scope_and_session_only_collection",
        ])
        .env(CHILD, "1")
        .env("PRISM_PROFILE", dir.path().join("profile.jsonl"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
