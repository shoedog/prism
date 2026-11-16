//! Isolated child-process sessions; no engine or review-statistics integration claim.
//! P05 alone controls inactive/absent/poisoned collector states in its disposable child.
use super::super::{Phase, Session};
use super::counter;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

fn scenario(name: &str, child: impl FnOnce(&Path), check: impl FnOnce(&Path)) {
    const CHILD: &str = "PRISM_SCALAR_COUNTER_CHILD";
    const PATH: &str = "PRISM_SCALAR_COUNTER_PATH";
    if std::env::var(CHILD).ok().as_deref() == Some(name) {
        child(Path::new(&std::env::var_os(PATH).unwrap()));
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profile.jsonl");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &format!("profile::counters::tests::{name}")])
        .env(CHILD, name)
        .env(PATH, &path)
        .env("PRISM_PROFILE", &path)
        .output()
        .unwrap();
    assert!(output.status.success(), "child {name}: {output:?}");
    check(&path);
}
fn text(path: &Path) -> String {
    assert!(
        path.is_file(),
        "published profile missing: {}",
        path.display()
    );
    std::fs::read_to_string(path).unwrap()
}
fn rows(path: &Path) -> Vec<Value> {
    text(path)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
fn scalar_rows(rows: &[Value]) -> Vec<Value> {
    rows.iter()
        .filter(|row| row["record"] == "counter")
        .cloned()
        .collect()
}
fn session(path: &Path, values: &[(&'static str, u64)]) {
    std::env::set_var("PRISM_PROFILE", path);
    let session = Session::start();
    drop(Phase::serial("call_graph"));
    for &(name, value) in values {
        counter(name, value);
    }
    drop(session);
}
fn schema(rows: &[Value]) -> Vec<Vec<String>> {
    rows.iter()
        .filter(|r| r["record"] != "counter")
        .map(|row| {
            let keys = row
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>();
            let expected: &[&str] = match row["record"].as_str().unwrap() {
                "header" => &[
                    "record",
                    "schema_version",
                    "time_unit",
                    "memory_unit",
                    "peak_rss_scope",
                    "top_n",
                    "prism_version",
                    "commit",
                    "thread_count",
                    "argv",
                    "os",
                ],
                "command" => &["record", "time_kind", "wall_ns", "cpu_ns", "peak_rss_bytes"],
                "phase" => &[
                    "record",
                    "name",
                    "file",
                    "time_kind",
                    "aggregation",
                    "status",
                    "wall_ns",
                    "cpu_ns",
                    "peak_rss_bytes",
                    "rss_rise_bytes",
                    "instance",
                ],
                "summary" => &[
                    "record",
                    "time_kind",
                    "owner_count",
                    "total_owner_ns",
                    "top_owners",
                    "top_dfg_share",
                    "other_owners",
                ],
                other => panic!("unexpected existing row {other}"),
            };
            assert_eq!(
                keys,
                expected
                    .iter()
                    .map(|key| key.to_string())
                    .collect::<BTreeSet<_>>()
            );
            keys.into_iter().collect()
        })
        .collect()
}
fn stable_existing(rows: &[Value]) -> Vec<u8> {
    let mut rows: Vec<_> = rows
        .iter()
        .filter(|r| r["record"] != "counter")
        .cloned()
        .collect();
    for row in &mut rows {
        match row["record"].as_str().unwrap() {
            "header" => {
                assert!(row["argv"]
                    .as_array()
                    .is_some_and(|args| !args.is_empty() && args.iter().all(Value::is_string)));
                // argv is process-specific; preserve its schema, explicitly normalize its values.
                row["argv"] = serde_json::json!(["<volatile argv>"]);
            }
            "command" | "phase" => {
                // Only these existing clock/resource values vary across sequential sessions.
                for key in ["wall_ns", "cpu_ns", "peak_rss_bytes", "rss_rise_bytes"] {
                    if let Some(value) = row.get_mut(key) {
                        assert!(value.is_null() || value.is_u64());
                        *value = Value::String("<volatile clock/resource>".into());
                    }
                }
            }
            _ => {}
        }
    }
    serde_json::to_vec(&rows).unwrap()
}

#[test]
fn p01_counters_are_written_between_owners_and_summary() {
    scenario(
        "p01_counters_are_written_between_owners_and_summary",
        |path| {
            std::env::set_var("PRISM_PROFILE", path);
            let session = Session::start();
            drop(Phase::serial("call_graph"));
            let parsed = crate::ast::ParsedFile::parse(
                "owner.py",
                "def f(x):\n    return x\n",
                crate::languages::Language::Python,
            )
            .unwrap();
            let node = parsed.tree.root_node().named_child(0).unwrap();
            assert_eq!(
                node.kind(),
                "function_definition",
                "inadmissible precondition: real owner node"
            );
            drop(
                super::super::Owner::begin("owner.py", &parsed, node, "f")
                    .expect("inadmissible precondition: active owner"),
            );
            counter("z", 7);
            counter("a", 2);
            drop(session);
        },
        |path| {
            let text = text(path);
            let rows = rows(path);
            let kinds: Vec<_> = rows.iter().map(|r| r["record"].as_str().unwrap()).collect();
            assert_eq!(&kinds[..2], &["header", "command"]);
            assert!(
                kinds.contains(&"owner"),
                "inadmissible precondition: existing owner produced"
            );
            let counters = scalar_rows(&rows);
            assert_eq!(counters.len(), 2);
            let owner = kinds.iter().position(|k| *k == "owner").unwrap();
            let first_counter = kinds.iter().position(|k| *k == "counter").unwrap();
            let summary = kinds.iter().position(|k| *k == "summary").unwrap();
            assert!(kinds
                .iter()
                .enumerate()
                .filter(|(_, k)| **k == "phase")
                .all(|(i, _)| i < owner));
            assert!(owner < first_counter && first_counter + 2 == summary);
            assert_eq!(kinds.last(), Some(&"summary"));
            assert_eq!(
                text.lines()
                    .filter(|line| line.contains("\"record\":\"counter\""))
                    .collect::<Vec<_>>(),
                vec![
                    "{\"record\":\"counter\",\"name\":\"a\",\"value\":2}",
                    "{\"record\":\"counter\",\"name\":\"z\",\"value\":7}"
                ]
            );
        },
    );
}

#[test]
fn p02_same_name_accumulates() {
    scenario(
        "p02_same_name_accumulates",
        |path| session(path, &[("a", 2), ("a", 2)]),
        |path| {
            assert_eq!(
                scalar_rows(&rows(path)),
                vec![serde_json::json!({"record":"counter","name":"a","value":4})]
            );
        },
    );
}

#[test]
fn p03_rows_are_in_name_order() {
    scenario(
        "p03_rows_are_in_name_order",
        |path| session(path, &[("b", 20), ("a", 10), ("c", 30)]),
        |path| {
            assert_eq!(
                scalar_rows(&rows(path))
                    .iter()
                    .map(|r| (r["name"].as_str().unwrap(), r["value"].as_u64().unwrap()))
                    .collect::<Vec<_>>(),
                vec![("a", 10), ("b", 20), ("c", 30)]
            );
        },
    );
}

#[test]
fn p04_saturates_instead_of_wrapping() {
    scenario(
        "p04_saturates_instead_of_wrapping",
        |path| session(path, &[("max", u64::MAX), ("max", 1), ("zero", 0)]),
        |path| {
            assert_eq!(
                scalar_rows(&rows(path)),
                vec![
                    serde_json::json!({"record":"counter","name":"max","value":u64::MAX}),
                    serde_json::json!({"record":"counter","name":"zero","value":0})
                ]
            );
        },
    );
}

#[test]
fn p05_no_session_records_nothing() {
    scenario(
        "p05_no_session_records_nothing",
        |path| {
            std::env::remove_var("PRISM_PROFILE");
            counter("disabled", 1);
            assert!(!path.exists());
            assert!(super::super::collector().lock().unwrap().is_none());
            // CONTROLLED absent/poisoned collector branches, isolated from every other session/test.
            super::super::ACTIVE.store(true, std::sync::atomic::Ordering::Relaxed);
            counter("absent", 2);
            assert!(super::super::collector().lock().unwrap().is_none());
            let poison = std::panic::catch_unwind(|| {
                let _lock = super::super::collector().lock().unwrap();
                panic!("controlled collector poison");
            });
            assert!(poison.is_err());
            counter("poisoned", 3); // Must silently return, not panic or publish.
            super::super::ACTIVE.store(false, std::sync::atomic::Ordering::Relaxed);
            assert!(!path.exists());
        },
        |path| assert!(!path.exists()),
    );
}

#[test]
fn p06_counter_after_finish_is_dropped() {
    scenario(
        "p06_counter_after_finish_is_dropped",
        |path| {
            session(path, &[("a", 2)]);
            assert!(!super::super::enabled());
            counter("a", 9);
            counter("late", 1);
        },
        |path| {
            assert_eq!(
                scalar_rows(&rows(path)),
                vec![serde_json::json!({"record":"counter","name":"a","value":2})]
            );
        },
    );
}

#[test]
fn p07_next_session_starts_empty() {
    scenario(
        "p07_next_session_starts_empty",
        |path| {
            session(path, &[("old", 2)]);
            counter("between", 9);
            session(&path.with_extension("second.jsonl"), &[("new", 7)]);
        },
        |path| {
            assert_eq!(
                scalar_rows(&rows(path)),
                vec![serde_json::json!({"record":"counter","name":"old","value":2})]
            );
            assert_eq!(
                scalar_rows(&rows(&path.with_extension("second.jsonl"))),
                vec![serde_json::json!({"record":"counter","name":"new","value":7})]
            );
        },
    );
}

#[test]
fn p08_no_counter_means_no_row_and_same_bytes() {
    scenario(
        "p08_no_counter_means_no_row_and_same_bytes",
        |path| {
            session(path, &[]);
            session(&path.with_extension("second.jsonl"), &[]);
        },
        |path| {
            let first = rows(path);
            let second = rows(&path.with_extension("second.jsonl"));
            assert!(scalar_rows(&first).is_empty() && scalar_rows(&second).is_empty());
            assert_eq!(schema(&first), schema(&second));
            assert_eq!(stable_existing(&first), stable_existing(&second));
            let kinds: Vec<_> = first
                .iter()
                .map(|r| r["record"].as_str().unwrap())
                .collect();
            assert_eq!(&kinds[..2], &["header", "command"]);
            assert_eq!(kinds.last(), Some(&"summary"));
            assert!(kinds[2..kinds.len() - 1]
                .iter()
                .all(|kind| *kind == "phase"));
            assert!(first
                .iter()
                .any(|r| r["name"] == "call_graph" && r["status"] == "measured"));
        },
    );
}

#[test]
fn p09_counters_from_worker_threads_are_summed() {
    scenario(
        "p09_counters_from_worker_threads_are_summed",
        |path| {
            std::env::set_var("PRISM_PROFILE", path);
            let session = Session::start();
            rayon::scope(|scope| {
                for _ in 0..4 {
                    scope.spawn(|_| counter("workers", 1));
                }
            }); // Every task joins before the process-wide Session is dropped.
            drop(session);
        },
        |path| {
            assert_eq!(
                scalar_rows(&rows(path)),
                vec![serde_json::json!({"record":"counter","name":"workers","value":4})]
            );
        },
    );
}

#[test]
fn p10_existing_rows_are_untouched() {
    scenario(
        "p10_existing_rows_are_untouched",
        |path| {
            session(path, &[]);
            session(&path.with_extension("second.jsonl"), &[("present", 5)]);
        },
        |path| {
            let without = rows(path);
            let with = rows(&path.with_extension("second.jsonl"));
            assert_eq!(schema(&without), schema(&with));
            assert_eq!(stable_existing(&without), stable_existing(&with));
            assert_eq!(
                scalar_rows(&with).len(),
                1,
                "counter presence is a behavioral oracle"
            );
        },
    );
}
