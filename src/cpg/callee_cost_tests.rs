//! Test-only counters at the actual fact-computation boundary.
use super::*;
use crate::languages::Language;
use std::sync::Mutex;

type Counts = BTreeMap<(String, FunctionId), usize>;
static COUNTS: Mutex<Counts> = Mutex::new(BTreeMap::new());

pub(super) fn record(kind: &str, callee: &FunctionId) {
    record_n(kind, callee, 1);
}

pub(super) fn record_n(kind: &str, callee: &FunctionId, n: usize) {
    // Separate fixture identities isolate tests, including parallel lib tests.
    if callee.file.starts_with("cost-") || std::env::var_os("CALLEE_COUNT_REPLAY").is_some() {
        *COUNTS
            .lock()
            .unwrap()
            .entry((kind.into(), callee.clone()))
            .or_default() += n;
    }
}

pub(super) fn take_counts() -> Counts {
    std::mem::take(&mut *COUNTS.lock().unwrap())
}

fn files(kind: &str, m: usize) -> BTreeMap<String, ParsedFile> {
    let helper = format!("cost-{kind}-{m}.js");
    let mut files = BTreeMap::from([(helper.clone(), ParsedFile::parse(&helper,
        "export function helper(x) { return x; }\nexport function empty(x) { let y = x; }\nexport function refused(x,x) { return x; }\n", Language::JavaScript).unwrap())]);
    let other = format!("cost-other-{kind}-{m}.js");
    files.insert(
        other.clone(),
        ParsedFile::parse(
            &other,
            "export function other(x) { return x; }\n",
            Language::JavaScript,
        )
        .unwrap(),
    );
    for file in 0..4 {
        let path = format!("cost-caller-{kind}-{m}-{file}.js");
        let mut source = format!("import {{helper,empty,refused}} from './cost-{kind}-{m}';\nimport {{other}} from './cost-other-{kind}-{m}';\n");
        for i in 0..m {
            source.push_str(&format!("function caller{file}_{i}(x) {{ let a = helper(x); let b = helper(x); let c = empty(x); let d = refused(x,x); let e = other(x); return a; }}\n"));
        }
        files.insert(
            path.clone(),
            ParsedFile::parse(&path, &source, Language::JavaScript).unwrap(),
        );
    }
    files
}

fn constant(kind: &str) {
    for m in [1, 8, 32] {
        let files = files(kind, m);
        let cpg = CodePropertyGraph::build(&files);
        assert!(
            cpg.return_flow_stats.return_flow_edges > 0,
            "cost fixture must emit relevant edges"
        );
        let counts = COUNTS.lock().unwrap().clone();
        let helper = format!("cost-{kind}-{m}.js");
        let other = format!("cost-other-{kind}-{m}.js");
        for (file, callee) in [
            (&helper, "helper"),
            (&helper, "empty"),
            (&helper, "refused"),
            (&other, "other"),
        ] {
            let rows: Vec<_> = counts
                .iter()
                .filter(|((k, id), _)| k == kind && &id.file == file && id.name == callee)
                .collect();
            assert_eq!(
                rows.len(),
                1,
                "{kind}, M={m}, {callee}: missing computation witness"
            );
            assert_eq!(
                *rows[0].1, 1,
                "{kind}, M={m}, {callee}: facts must be computed once across callers"
            );
        }
        if kind == "returns" {
            for file in [&helper, &other] {
                // Concurrent first demanders can charge initialization to any
                // callee in the file; the per-file sum is the invariant.
                let lookups: usize = counts
                    .iter()
                    .filter(|((k, id), _)| k == "return-lookups" && &id.file == file)
                    .map(|(_, n)| *n)
                    .sum();
                assert_eq!(
                    lookups, 1,
                    "M={m}, {file}: function map must be built once per file"
                );
            }
        }
    }
}

#[test]
fn parameter_computations_constant_across_callers() {
    constant("parameters");
}

#[test]
fn return_computations_constant_across_callers() {
    constant("returns");
}
