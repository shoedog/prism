//! Semantic differential oracle and deterministic ordering witnesses.
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
fn with_sequential<T: Send>(op: impl FnOnce() -> T + Send) -> T {
    crate::build_pool::install(|| {
        struct Restore(bool);
        impl Drop for Restore {
            fn drop(&mut self) {
                SEQUENTIAL.with(|v| v.set(self.0));
            }
        }
        let _restore = Restore(SEQUENTIAL.with(|v| v.replace(true)));
        let before = ORACLE_CALLS.with(Cell::get);
        let result = op();
        assert!(ORACLE_CALLS.with(Cell::get) > before, "oracle bypassed");
        result
    })
}
fn parsed(path: &str, source: &str) -> ParsedFile {
    ParsedFile::parse(path, source, Language::from_path(path).unwrap()).unwrap()
}
// Reparse rather than cloning potentially primed OnceLocks.
fn reparse(files: &BTreeMap<String, ParsedFile>) -> BTreeMap<String, ParsedFile> {
    files
        .iter()
        .map(|(path, p)| (path.clone(), parsed(path, &p.source)))
        .collect()
}
fn pool(threads: usize) -> rayon::ThreadPool {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .stack_size(256 * 1024 * 1024)
        .build()
        .unwrap()
}
fn generated() -> BTreeMap<String, ParsedFile> {
    let mut files = BTreeMap::new();
    for i in 0..64 {
        let path = format!("unit{i:03}.{}", ["js", "ts", "tsx"][i % 3]);
        let source = format!("export function target{i}(x) {{ return x; }}\nexport const view{i} = (p) => {{ const alias = target{i}; consume(() => alias(p), function*() {{ yield p; }}); return {{run: target{i}}}; }};\nfunction use{i}(v) {{ const {{run}} = view{i}(v); return run(v); }}\n");
        files.insert(path.clone(), parsed(&path, &source));
    }
    for (path, source) in [
        ("empty.js", ""),
        ("broken.ts", "const = ; function f() { return () => missing(); }"),
        ("unsupported.js", "with (obj) { (() => target())(); } eval('x');"),
        ("escaped.ts", r"const \u0061 = () => target();"),
        ("jsx.tsx", "import {target0 as Button} from './unit000'; import * as Q from './unit003'; export const view = () => <><Button onClick={() => Q.target3(1)} /><Button></Button></>;"),
        ("methods.js", "const obj = { run() { return target(); }, arrow: () => target() }; class C { field = () => target(); method() { return target(); } }"),
        // Same name and same line ranges produce duplicate FunctionIds. The last
        // return dependency differs: it names an import, not a local callable.
        ("duplicates.ts", "import {remote} from './unit000'; function local() {} function same() { return local; } function same() { return remote; }"),
        ("plain.py", "def f(x):\n    return x\n"),
        ("plain.go", "package p\nfunc F(x int) int { return x }\n"),
    ] {
        files.insert(path.into(), parsed(path, source));
    }
    files
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
    let expected = with_sequential(|| CodePropertyGraph::build(&reparse(files)));
    let actual = CodePropertyGraph::build(&reparse(files));
    assert_eq!(
        expected.call_graph.js_anonymous,
        actual.call_graph.js_anonymous
    );
    for (i, (old, new)) in observations(&expected)
        .iter()
        .zip(observations(&actual))
        .enumerate()
    {
        assert!(old == &new, "serialized observation {i} differs");
    }
}

#[test]
fn generated_complete_index_and_serialization() {
    let files = generated();
    let expected = sequential::build(&files);
    assert_eq!(expected.0.len(), files.len() - 2);
    assert!(expected.0["empty.js"].program_supported);
    for file in ["broken.ts", "unsupported.js", "escaped.ts"] {
        assert!(!expected.0[file].program_supported, "{file}");
    }
    let methods = &expected.0["methods.js"];
    assert!(methods.property_methods.len() >= 3);
    assert!(!methods.owners.is_empty() && !methods.returns.is_empty() && !methods.sites.is_empty());
    assert!(expected.0["unit000.js"]
        .owners
        .keys()
        .any(|id| crate::anon_owner::is_anonymous_owner_name(&id.name)));
    let duplicate = expected.0["duplicates.ts"]
        .returns
        .iter()
        .find(|(id, _)| id.name == "same")
        .unwrap();
    assert!(duplicate.1 .1.iter().any(|e| e.binding.local == "remote"));
    let jsx = &expected.0["jsx.tsx"];
    assert!(jsx.sites.len() >= 3);
    assert!(jsx.sites.values().any(|e| !e.imports.is_empty()));
    for threads in [1, 4, 16] {
        pool(threads).install(|| {
            let cold = reparse(&files);
            assert_eq!(expected, ProofIndex::build(&cold), "cold threads={threads}");
            assert_eq!(expected, ProofIndex::build(&cold), "warm threads={threads}");
            let reversed: Vec<_> = files.iter().rev().collect();
            assert_eq!(
                sequential::build(reversed.clone()),
                collect(reversed, build_file)
            );
            assert_eq!(ProofIndex::default(), ProofIndex::build(&BTreeMap::new()));
            assert_eq!(
                with_sequential(|| CallGraph::build_skeleton(&cold)).js_anonymous,
                CallGraph::build_skeleton(&cold).js_anonymous
            );
            compare_graphs(&files);
        });
    }
}

#[test]
fn duplicate_paths_merge_in_input_order_despite_reverse_completion() {
    let path = "repeat.ts".to_string();
    let first = parsed(&path, "export const left = () => 1;");
    let last = parsed(&path, "eval('x'); export const right = () => left();");
    let empty = parsed(&path, "");
    let non_js = ParsedFile::parse(&path, "def f():\n    return 1\n", Language::Python).unwrap();
    let inputs = vec![(&path, &first), (&path, &last)];
    let expected = sequential::build(inputs.clone());
    assert_ne!(expected, sequential::build(inputs.iter().copied().rev()));
    let barrier = Barrier::new(2);
    let finished = (Mutex::new(Vec::new()), Condvar::new());
    let actual = pool(4).install(|| {
        collect_on_current_pool(inputs, |path, parsed| {
            let facts = build_file(path, parsed);
            barrier.wait();
            let (lock, notify) = &finished;
            let mut order = lock.lock().unwrap();
            if parsed.source.contains("right") {
                order.push(1);
                notify.notify_one();
            } else {
                let (next, timeout) = notify
                    .wait_timeout_while(order, std::time::Duration::from_secs(10), |v| v.is_empty())
                    .unwrap();
                order = next;
                assert!(!timeout.timed_out(), "later file never completed");
                order.push(0);
            }
            facts
        })
    });
    assert_eq!(*finished.0.lock().unwrap(), [1, 0]);
    assert_eq!(actual, expected);
    // A skipped language erases nothing; an admitted empty file replaces all fields.
    for tail in [&non_js, &empty] {
        let inputs = vec![(&path, &first), (&path, &last), (&path, tail)];
        assert_eq!(
            collect(inputs.clone(), build_file),
            sequential::build(inputs)
        );
    }
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
fn repository_fixtures_complete_index_and_serialization() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut paths = Vec::new();
    fixture_paths(&root, &mut paths);
    assert!(
        paths.len() >= 100,
        "fixture population unexpectedly truncated"
    );
    let mut all = BTreeMap::new();
    pool(4).install(|| {
        for path in &paths {
            let relative = path.strip_prefix(&root).unwrap().to_str().unwrap();
            let source = std::fs::read_to_string(path).unwrap();
            let files = BTreeMap::from([(relative.to_string(), parsed(relative, &source))]);
            assert_eq!(
                sequential::build(&files),
                ProofIndex::build(&reparse(&files))
            );
            compare_graphs(&files);
            all.extend(files);
        }
    });
    for threads in [1, 4, 16] {
        pool(threads).install(|| {
            assert_eq!(
                sequential::build(&reparse(&all)),
                ProofIndex::build(&reparse(&all))
            );
            compare_graphs(&all);
        });
    }
    eprintln!(
        "Proof index identity: {} repository fixtures individually and together",
        paths.len()
    );
}

#[test]
fn incremental_direct_subset_and_merge_match_sequential() {
    for threads in [1, 4, 16] {
        pool(threads).install(|| {
            let mut files = generated();
            let old = CallGraph::build(&files);
            files.insert(
                "unit000.js".into(),
                parsed("unit000.js", "export const changed = () => target1(2);"),
            );
            let changed = BTreeSet::from(["unit000.js".to_string()]);
            let direct = CallGraph::build_direct_subset(&files, &changed);
            let serial_direct =
                with_sequential(|| CallGraph::build_direct_subset(&reparse(&files), &changed));
            assert_eq!(direct.js_anonymous, serial_direct.js_anonymous);
            // Main's direct constructor deliberately supplies ALL files to proofs.
            assert_eq!(direct.js_anonymous, sequential::build(&files));
            let merge = |mut previous: CallGraph, incoming: CallGraph| {
                previous.remove_files(&changed);
                previous.merge(incoming);
                previous
            };
            let actual = merge(old.clone(), direct);
            let expected = merge(old, serial_direct);
            assert_eq!(actual.js_anonymous, expected.js_anonymous);
            assert_eq!(
                bincode::serialize(&actual.js_anonymous).unwrap(),
                bincode::serialize(&sequential::build(&files)).unwrap()
            );
            let old_cpg = CodePropertyGraph::build(&files);
            files.insert(
                "unit001.ts".into(),
                parsed(
                    "unit001.ts",
                    "export const changedAgain = () => target2(3);",
                ),
            );
            let changed = BTreeSet::from(["unit001.ts".to_string()]);
            let incremental = || {
                CodePropertyGraph::build_incremental(
                    old_cpg.call_graph.clone(),
                    old_cpg.dfg.clone(),
                    &changed,
                    &files,
                    None,
                )
            };
            let expected = with_sequential(incremental);
            let actual = incremental();
            assert_eq!(
                expected.call_graph.js_anonymous,
                actual.call_graph.js_anonymous
            );
            assert!(observations(&expected) == observations(&actual));
            assert_eq!(actual.call_graph.js_anonymous, ProofIndex::build(&files));
        });
    }
}
