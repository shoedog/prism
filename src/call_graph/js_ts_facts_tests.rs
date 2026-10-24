//! Complete differential observations; no timing thresholds or operation-count proxy.
use super::super::CallGraph;
use super::*;
use crate::cpg::{dfg_observation, CodePropertyGraph};
use crate::languages::Language;
use crate::navigation::queries;
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::{Barrier, Condvar, Mutex};

thread_local! {
    static SEQUENTIAL: Cell<bool> = const { Cell::new(false) };
    static ORACLE_CALLS: Cell<usize> = const { Cell::new(0) };
}

pub(super) fn sequential_enabled() -> bool {
    SEQUENTIAL.with(Cell::get)
}
pub(super) fn record_oracle() {
    ORACLE_CALLS.with(|n| n.set(n.get() + 1));
}
fn with_sequential<T>(op: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            SEQUENTIAL.with(|v| v.set(self.0));
        }
    }
    let _restore = Restore(SEQUENTIAL.with(|v| v.replace(true)));
    let before = ORACLE_CALLS.with(Cell::get);
    let result = op();
    assert!(
        ORACLE_CALLS.with(Cell::get) > before,
        "sequential oracle bypassed"
    );
    result
}
fn parsed(path: &str, source: &str) -> ParsedFile {
    ParsedFile::parse(path, source, Language::from_path(path).unwrap()).unwrap()
}
// Clone would also clone a primed OnceLock; reparse to exercise cold exports.
fn reparse(files: &BTreeMap<String, ParsedFile>) -> BTreeMap<String, ParsedFile> {
    files
        .iter()
        .map(|(key, file)| {
            (
                key.clone(),
                ParsedFile::parse(&file.path, &file.source, file.language).unwrap(),
            )
        })
        .collect()
}
fn generated() -> BTreeMap<String, ParsedFile> {
    let mut files = BTreeMap::new();
    for i in 0..64 {
        let ext = ["js", "ts", "tsx"][i % 3];
        let path = format!("unit{i:03}.{ext}");
        let source = format!("export function target{i}(x) {{ return x; }}\nexport function hook{i}(p) {{ const local = p; consume(() => target{i}(local), function*() {{ yield p; }}); return {{run: target{i}}}; }}\nfunction use{i}(v) {{ const {{run}} = hook{i}(v); return run(v); }}\n");
        files.insert(path.clone(), parsed(&path, &source));
    }
    for (path, source) in [
        ("empty.js", ""),
        ("broken.ts", "const = ;"),
        ("broken-export.ts", "export function okay() {}\nconst = ;"),
        ("anonymous.tsx", "import {target0} from './unit000'; export const view = () => <Button onClick={() => target0(1)} />;"),
        ("duplicates.js", "function left() {} function right() {} function same(a) { const first = a; return {run: left}; } function same(b) { const second = b; return {run: right}; } function same() {}"),
        ("plain.py", "def f(x):\n    return x\n"),
        ("plain.go", "package p\nfunc F(x int) int { return x }\n"),
    ] {
        files.insert(path.into(), parsed(path, source));
    }
    files
}
fn maps(graph: &CallGraph) -> JsTsResolutionFacts {
    (
        graph.js_ts_exports.clone(),
        graph.js_ts_function_locals.clone(),
        graph.js_ts_return_facts.clone(),
    )
}
fn observations(graph: &CodePropertyGraph) -> Vec<Vec<u8>> {
    let mut stats = queries::call_stats(&graph.call_graph);
    stats["return_flow"] = serde_json::to_value(&graph.return_flow_stats).unwrap();
    stats["dfg_labels"] = serde_json::to_value(&graph.dfg_label_stats).unwrap();
    vec![
        serde_json::to_vec(&stats).unwrap(),
        serde_json::to_vec(&queries::call_site_dump(&graph.call_graph)).unwrap(),
        bincode::serialize(&graph.dfg).unwrap(),
        serde_json::to_vec(&queries::dfg_edge_dump(graph)).unwrap(),
        dfg_observation::fieldwise_payload(graph),
        dfg_observation::lookup_indexes(graph),
    ]
}
fn compare_graphs(files: &BTreeMap<String, ParsedFile>) {
    crate::build_pool::install(|| {
        let serial_files = reparse(files);
        let parallel_files = reparse(files);
        let expected = with_sequential(|| CodePropertyGraph::build(&serial_files));
        let actual = CodePropertyGraph::build(&parallel_files);
        assert_eq!(maps(&expected.call_graph), maps(&actual.call_graph));
        for (i, (old, new)) in observations(&expected)
            .iter()
            .zip(observations(&actual))
            .enumerate()
        {
            assert!(old == &new, "serialized observation {i} differs");
        }
    });
}

#[test]
fn generated_complete_maps_and_serialization() {
    let files = generated();
    let expected = sequential(files.iter());
    assert!(!expected.0.is_empty() && !expected.1.is_empty() && !expected.2.is_empty());
    assert!(expected.0.contains_key("empty.js"));
    assert!(!expected.0.contains_key("broken.ts"));
    assert!(expected.0.contains_key("broken-export.ts"));
    assert!(!expected.0.contains_key("plain.py"));
    assert!(expected
        .1
        .keys()
        .any(|id| crate::anon_owner::is_anonymous_owner_name(&id.name)));
    let duplicate = expected
        .1
        .iter()
        .find(|(id, _)| id.file == "duplicates.js" && id.name == "same")
        .unwrap();
    assert!(duplicate.1.contains("second") && !duplicate.1.contains("first"));
    let duplicate_return = expected
        .2
        .iter()
        .find(|(id, _)| id.file == "duplicates.js" && id.name == "same")
        .unwrap();
    assert!(
        matches!(duplicate_return.1.keys.get("run"), Some(crate::ast::JsFollow::Callable(t)) if t.local == "right")
    );
    for threads in [1, 4, 16] {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .stack_size(256 * 1024 * 1024)
            .build()
            .unwrap()
            .install(|| {
                let cold = reparse(&files);
                assert_eq!(expected, extract(cold.iter()), "cold threads={threads}");
                assert_eq!(expected, extract(cold.iter()), "warm threads={threads}");
                let reversed: Vec<_> = files.iter().rev().collect();
                assert_eq!(sequential(reversed.clone()), extract(reversed));
                assert_eq!(sequential(std::iter::empty()), extract(std::iter::empty()));
            });
    }
    compare_graphs(&files);
}

#[test]
fn duplicate_paths_merge_in_input_order_despite_reverse_completion() {
    let path = "repeat.ts".to_string();
    let first = parsed(
        &path,
        "export function left() {} function hook(a) { return {run: left}; }",
    );
    let last = parsed(
        &path,
        "export function right() {} function hook(b) { return {run: right}; }",
    );
    let opaque = parsed(&path, "const = ;");
    let inputs = vec![(&path, &first), (&path, &last)];
    let expected = sequential(inputs.clone());
    assert_ne!(expected, sequential(inputs.iter().copied().rev()));
    let barrier = Barrier::new(2);
    let finished = (Mutex::new(Vec::new()), Condvar::new());
    let actual = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            collect_on_current_pool(inputs, |path, parsed| {
                let facts = extract_file(path, parsed);
                barrier.wait();
                let (lock, notify) = &finished;
                let mut order = lock.lock().unwrap();
                if parsed.source.contains("left") {
                    let (next, timeout) = notify
                        .wait_timeout_while(order, std::time::Duration::from_secs(10), |v| {
                            v.is_empty()
                        })
                        .unwrap();
                    order = next;
                    assert!(!timeout.timed_out(), "later file never completed");
                    order.push(0);
                } else {
                    order.push(1);
                    notify.notify_one();
                }
                facts
            })
        });
    assert_eq!(*finished.0.lock().unwrap(), [1, 0]);
    assert_eq!(actual, expected);
    let inputs = vec![(&path, &first), (&path, &last), (&path, &opaque)];
    // A skipped export and None returns must not erase the preceding inserts.
    assert_eq!(extract(inputs.clone()), sequential(inputs));
}

fn fixture_paths(root: &Path, paths: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(Result::unwrap)
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            fixture_paths(&entry.path(), paths);
        } else if kind.is_file() && Language::from_path(&entry.path().to_string_lossy()).is_some() {
            paths.push(entry.path());
        }
    }
}

#[test]
fn repository_fixtures_complete_maps_and_serialization() {
    crate::build_pool::install(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let mut paths = Vec::new();
        fixture_paths(&root, &mut paths);
        assert!(
            paths.len() >= 100,
            "fixture population unexpectedly truncated"
        );
        let mut all = BTreeMap::new();
        for path in &paths {
            let relative = path.strip_prefix(&root).unwrap().to_str().unwrap();
            let source = std::fs::read_to_string(path).unwrap();
            let files = BTreeMap::from([(relative.to_string(), parsed(relative, &source))]);
            let cold = reparse(&files);
            assert_eq!(sequential(files.iter()), extract(cold.iter()));
            compare_graphs(&files);
            all.extend(files);
        }
        compare_graphs(&all);
        eprintln!(
            "JS/TS facts identity: {} repository fixtures individually and together",
            paths.len()
        );
    });
}

#[test]
fn incremental_partial_update_matches_full_and_sequential() {
    crate::build_pool::install(|| {
        let mut files = BTreeMap::from([
            ("lib.ts".into(), parsed("lib.ts", "export function left(x) { return x; } export function hook(a) { return {run: left}; }")),
            ("use.ts".into(), parsed("use.ts", "import {hook} from './lib'; export function use(v) { const {run} = hook(v); return run(v); }")),
        ]);
        let old = CodePropertyGraph::build(&files);
        files.insert("lib.ts".into(), parsed("lib.ts", "export function right(y) { return y + 1; } export function hook(b) { const local = b; return {run: right}; }"));
        let changed = BTreeSet::from(["lib.ts".to_string()]);
        let direct = CallGraph::build_direct_subset(&files, &changed);
        assert_eq!(
            maps(&direct),
            sequential(files.iter().filter(|(path, _)| changed.contains(*path)))
        );
        let partial = CodePropertyGraph::build_incremental(
            old.call_graph.clone(),
            old.dfg.clone(),
            &changed,
            &files,
            None,
        );
        let full = CodePropertyGraph::build(&files);
        let serial = with_sequential(|| CodePropertyGraph::build(&files));
        assert_eq!(maps(&partial.call_graph), maps(&full.call_graph));
        assert_eq!(maps(&partial.call_graph), maps(&serial.call_graph));
        // Incremental assembly can have a different storage insertion order;
        // compare its complete public dumps, and full-vs-oracle storage bytes.
        assert_eq!(
            queries::call_site_dump(&partial.call_graph),
            queries::call_site_dump(&full.call_graph)
        );
        assert_eq!(
            serde_json::to_vec(&queries::dfg_edge_dump(&partial)).unwrap(),
            serde_json::to_vec(&queries::dfg_edge_dump(&full)).unwrap()
        );
        assert!(observations(&full) == observations(&serial));
    });
}
