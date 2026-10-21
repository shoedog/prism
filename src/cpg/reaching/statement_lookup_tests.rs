//! Operation-only statement lookup census and regression witnesses.
use super::*;
use crate::data_flow::DataFlowGraph;
use std::cell::{Cell, RefCell};
use std::path::Path;

/// Full RD census (unlike work_tests' CFG-only scaling census).
/// PRISM_STATEMENT_FIXTURES points to fixtures-c1; OUT selects the JSON receipt.
#[test]
#[ignore = "explicit external operation census"]
fn operation_census() {
    let root = std::env::var("PRISM_STATEMENT_FIXTURES").unwrap();
    let mut observations = BTreeMap::new();
    let mut directories: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(Result::unwrap)
        .collect();
    directories.sort_by_key(|entry| entry.path());
    for directory in directories {
        let mut files: Vec<_> = std::fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        files.sort();
        assert_eq!(files.len(), 1);
        let file = &files[0];
        let path = file.to_str().unwrap();
        let parsed = ParsedFile::parse(
            path,
            &std::fs::read_to_string(file).unwrap(),
            Language::from_path(path).unwrap(),
        )
        .unwrap();
        let (_, records) = work_tests::build(parsed);
        eprintln!(
            "counted {}: {} owners",
            directory.path().display(),
            records.len()
        );
        observations.insert(directory.file_name().to_string_lossy().to_string(), records);
    }
    std::fs::write(
        Path::new(&std::env::var("PRISM_STATEMENT_COUNTS_OUT").unwrap()),
        serde_json::to_vec_pretty(&observations).unwrap(),
    )
    .unwrap();
}

#[test]
fn repeated_edge_lines_scan_each_span_once() {
    for repeats in [2, 16, 64] {
        let source = format!(
            "function f(p) {{\nlet x=p;\n{}\nreturn x;\n}}",
            "consume(x);".repeat(repeats)
        );
        let parsed = ParsedFile::parse("cost.ts", &source, Language::TypeScript).unwrap();
        let (_, records) = work_tests::build(parsed);
        assert_eq!(records.len(), 1);
        let counts = &records[0];
        let distinct = counts.statement_query_lines.len();
        assert!(counts.statement_span_entries > 0, "{counts:?}");
        assert!(counts.statement_lookup_calls > distinct * 2, "{counts:?}");
        assert!(
            counts.statement_span_visits <= distinct * counts.statement_span_entries,
            "{counts:?}"
        );
    }
}

#[derive(Default, Debug, PartialEq, Eq)]
struct Observation {
    outcomes: Vec<RdOutcome>,
    sets: Vec<(Vec<BitSet>, Vec<BitSet>)>,
}

thread_local! {
    static LEGACY: Cell<bool> = const { Cell::new(false) };
    static OBSERVED: RefCell<Option<Observation>> = const { RefCell::new(None) };
}

pub(super) fn legacy_enabled() -> bool {
    LEGACY.with(Cell::get)
}

pub(super) fn record_outcome(outcome: &RdOutcome) {
    OBSERVED.with(|slot| {
        if let Some(observed) = slot.borrow_mut().as_mut() {
            observed.outcomes.push(outcome.clone());
        }
    });
}

pub(super) fn record_sets(bound: &[BitSet], flat: &[BitSet]) {
    OBSERVED.with(|slot| {
        if let Some(observed) = slot.borrow_mut().as_mut() {
            observed.sets.push((bound.to_vec(), flat.to_vec()));
        }
    });
}

struct Mode;
impl Mode {
    fn begin(legacy: bool) -> Self {
        LEGACY.with(|mode| assert!(!mode.replace(legacy)));
        OBSERVED.with(|slot| assert!(slot.replace(Some(Observation::default())).is_none()));
        Self
    }
    fn take() -> Observation {
        OBSERVED.with(|slot| slot.take().unwrap())
    }
}
impl Drop for Mode {
    fn drop(&mut self) {
        LEGACY.with(|mode| mode.set(false));
        OBSERVED.with(|slot| slot.take());
    }
}

fn compare(path: &str, source: &str) -> (usize, bool) {
    rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .stack_size(64 * 1024 * 1024)
        .build()
        .unwrap()
        .install(|| {
            let parsed =
                ParsedFile::parse(path, source, Language::from_path(path).unwrap()).unwrap();
            let files = BTreeMap::from([(path.to_string(), parsed)]);
            let mode = Mode::begin(true);
            let old = DataFlowGraph::build(&files);
            let old_observed = Mode::take();
            drop(mode);
            let mode = Mode::begin(false);
            let new = DataFlowGraph::build(&files);
            let new_observed = Mode::take();
            drop(mode);
            assert_eq!(
                old_observed, new_observed,
                "complete RD outcomes/in-sets: {path}"
            );
            assert_eq!(
                crate::cpg::dfg_observation::dfg_storage(&old),
                crate::cpg::dfg_observation::dfg_storage(&new),
                "full DFG edge/label serialization and doubt reasons: {path}"
            );
            let incomplete = new_observed.outcomes.iter().any(|outcome| match outcome {
                RdOutcome::Available(result) => result
                    .labels
                    .values()
                    .chain(result.exact_labels.values())
                    .any(|label| *label == FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)),
                _ => false,
            });
            (new_observed.outcomes.len(), incomplete)
        })
}

#[test]
fn differential_all_fixtures() {
    let mut files = Vec::new();
    for root in ["tests/fixtures", "eval/fixtures"] {
        work_tests::fixture_files(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join(root),
            &mut files,
        );
    }
    files.sort();
    assert!(files.len() >= 100);
    let mut owners = 0;
    let mut incomplete = false;
    for file in &files {
        let (count, flag) = compare(
            file.to_str().unwrap(),
            &std::fs::read_to_string(file).unwrap(),
        );
        owners += count;
        incomplete |= flag;
    }
    assert!(owners > 100);
    assert!(incomplete, "cfg_incomplete labels must be compared");
    eprintln!(
        "statement memo differential: {} files, {owners} RD invocations",
        files.len()
    );
}

#[test]
fn differential_generated() {
    let mut incomplete = false;
    let mut owners = 0;
    for n in [1, 4, 16] {
        for ext in ["js", "ts", "tsx", "java", "py", "go", "rs"] {
            let path = format!("generated.{ext}");
            let body = "try {\nx=p;\nconsume(x);\n} catch(e) {\nconsume(x);\n} finally {\nconsume(x);\n}\n".repeat(n);
            let source = match ext {
                "java" => format!("class C {{ int f(int p) {{\nint x=p;\n{}return x;\n}} }}", body.replace("catch(e)", "catch(Exception e)")),
                "py" => format!("def f(p):\n    x = p\n{}    return x\n", "    try:\n        x = p\n        consume(x)\n    except Exception:\n        consume(x)\n    finally:\n        consume(x)\n".repeat(n)),
                "go" => format!("package p\nfunc f(p int) int {{\nx:=p\n{}x=2\nreturn x\n}}", "defer consume(x)\n".repeat(n)),
                "rs" => format!("fn f(p: i32) -> i32 {{\nlet mut x=p;\n{}x\n}}", "if p>0 {\nx=2;\n}\nconsume(x);\n".repeat(n)),
                _ => format!("function f(p) {{\nlet x=p; x=2; consume(x);\n{body}let y = (\nx +\np\n);\nreturn y;\n}}\n"),
            };
            let (count, flag) = compare(&path, &source);
            owners += count;
            incomplete |= flag;
        }
    }
    for source in [
        "function f(p) {\nlet x=p;\n{ let x=p; consume(x); }\nlet y = (\nx + p\n);\nreturn y;\n}",
        "function f(p) {\nlet x=p; x=1;\nfunction inner() {\nreturn x;\n}\nreturn inner();\n}\nconsume(p => { let x=p; return x; });",
        "function f(p) {\nlet x=p;\nfor (var x of p) {\nconsume(x);\n}\nreturn x;\n}",
    ] {
        let (count, flag) = compare("extra.ts", source);
        owners += count;
        incomplete |= flag;
    }
    assert!(owners >= 21);
    assert!(incomplete);
}

fn span(start_line: Line, end_line: Line, start_byte: usize, end_byte: usize) -> StatementLineSpan {
    StatementLineSpan {
        start_line,
        end_line,
        start_byte,
        end_byte,
    }
}

#[test]
fn tied_nested_and_multiline_spans_match_oracle() {
    // Deliberate abstract spans expose every key component independently;
    // the production contract only reads these four fields.
    let cases = [
        (vec![span(1, 10, 0, 100), span(4, 6, 20, 30)], 5, Some(4)),
        (vec![span(2, 8, 10, 20), span(3, 8, 20, 30)], 4, Some(3)),
        (vec![span(2, 8, 10, 20), span(3, 8, 10, 20)], 4, Some(3)),
        (vec![span(3, 8, 10, 20), span(2, 8, 10, 20)], 4, Some(3)),
        (vec![span(2, 5, 9, 18)], 2, Some(2)),
        (vec![span(2, 5, 9, 18)], 5, Some(2)),
        (vec![span(2, 5, 9, 18)], 1, None),
        (vec![span(2, 5, 9, 18)], 6, None),
        (vec![], 1, None),
        (vec![span(2, 5, 9, 18), span(2, 5, 9, 18)], 3, Some(2)),
        // Preserve saturating length semantics even for abstract reversed bytes.
        (vec![span(2, 5, 20, 10), span(3, 5, 9, 18)], 4, Some(2)),
    ];
    for (spans, line, expected) in cases {
        assert_eq!(innermost_statement(line, &spans), expected, "oracle");
        let mut lookup = statement_lookup::StatementLookup::new(&spans);
        for _ in 0..3 {
            assert_eq!(lookup.get(line), expected, "line {line}");
        }
    }
    // Mixed queries, both orders, catch cache-key and history dependence.
    let spans = [span(2, 4, 10, 20), span(6, 9, 30, 40)];
    for lines in [[0, 2, 4, 6, 9, 10], [10, 9, 6, 4, 2, 0]] {
        let mut lookup = statement_lookup::StatementLookup::new(&spans);
        for _ in 0..3 {
            for line in lines {
                assert_eq!(lookup.get(line), innermost_statement(line, &spans));
            }
        }
    }
}

#[test]
fn uncovered_lines_are_memoized_too() {
    let spans = [span(2, 5, 9, 18), span(7, 8, 20, 30)];
    let counts = work_tests::cfg_selection_counts(|| {
        let mut lookup = statement_lookup::StatementLookup::new(&spans);
        for _ in 0..16 {
            assert_eq!(lookup.get(0), None);
            assert_eq!(lookup.get(99), None);
        }
    });
    assert_eq!(counts.statement_lookup_calls, 32);
    assert_eq!(counts.statement_query_lines, BTreeSet::from([0, 99]));
    assert_eq!(counts.statement_span_visits, 2 * spans.len());
}

#[test]
fn owners_keep_their_own_spans_and_memos() {
    let first = [span(2, 7, 10, 20)];
    let second = [span(4, 8, 10, 20)];
    let mut a = statement_lookup::StatementLookup::new(&first);
    let mut b = statement_lookup::StatementLookup::new(&second);
    for _ in 0..3 {
        assert_eq!(a.get(5), Some(2));
        assert_eq!(b.get(5), Some(4));
        assert_eq!(a.get(8), None);
        assert_eq!(b.get(8), Some(4));
    }
}

#[test]
#[ignore = "explicit external full RD/DFG differential; PRISM_STATEMENT_FIXTURES"]
fn differential_external_fixtures() {
    let mut directories: Vec<_> =
        std::fs::read_dir(std::env::var("PRISM_STATEMENT_FIXTURES").unwrap())
            .unwrap()
            .map(Result::unwrap)
            .collect();
    directories.sort_by_key(|entry| entry.path());
    assert_eq!(directories.len(), 23);
    let mut owners = 0;
    for directory in directories {
        let files: Vec<_> = std::fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(files.len(), 1);
        let (count, _) = compare(
            files[0].to_str().unwrap(),
            &std::fs::read_to_string(&files[0]).unwrap(),
        );
        owners += count;
        eprintln!(
            "external differential {}: {count} owners",
            directory.path().display()
        );
    }
    assert!(owners > 1000);
    eprintln!("external differential: 23 fixtures, {owners} RD invocations per mode");
}
