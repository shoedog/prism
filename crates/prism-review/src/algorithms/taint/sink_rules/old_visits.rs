//! Test-only observations of legacy detector visits without a located witness.

use prism::ast::ParsedFile;
use prism::data_flow::{FlowPath, VarLocation};
use prism::frameworks::SinkPattern;
use std::cell::RefCell;
use std::collections::BTreeMap;

// Finding category is taint_sink. The count is visits; occurrence multiplicity is unknown.
type Key = (String, usize, &'static str, String, &'static str);

struct Capture {
    groups: BTreeMap<Key, usize>,
    fixture_path_visits: Option<Vec<prism::data_flow::FlowPath>>,
}

thread_local! {
    static ACTIVE: RefCell<Option<Capture>> = const { RefCell::new(None) };
}

pub(in crate::algorithms::taint) fn replace_fixture_paths(paths: &mut Vec<FlowPath>) {
    ACTIVE.with_borrow_mut(|active| {
        if let Some(fixture) = active
            .as_mut()
            .and_then(|capture| capture.fixture_path_visits.take())
        {
            *paths = fixture;
        }
    });
}

pub(in crate::algorithms::taint) fn unlocated(parsed: &ParsedFile, target: &VarLocation) -> bool {
    super::uses::exact_use(parsed, target).is_none()
}

pub(in crate::algorithms::taint) fn record(
    parsed: &ParsedFile,
    line: usize,
    detector: &'static str,
    entry: &str,
    branch: &'static str,
) {
    ACTIVE.with_borrow_mut(|active| {
        if let Some(capture) = active {
            let key = (
                parsed.path.clone(),
                line,
                detector,
                entry.to_owned(),
                branch,
            );
            *capture.groups.entry(key).or_default() += 1;
        }
    });
}

pub(in crate::algorithms::taint) fn structured(
    parsed: &ParsedFile,
    line: usize,
    pat: &SinkPattern,
    branch: &'static str,
) {
    record(
        parsed,
        line,
        "structured",
        &format!("{:?}:{}", pat.category, pat.call_path),
        branch,
    );
}

fn capture(
    fixture_path_visits: Option<Vec<prism::data_flow::FlowPath>>,
    run: impl FnOnce(),
) -> BTreeMap<Key, usize> {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            ACTIVE.with_borrow_mut(|active| *active = None);
        }
    }
    ACTIVE.with_borrow_mut(|active| {
        assert!(active.is_none());
        *active = Some(Capture {
            groups: BTreeMap::new(),
            fixture_path_visits,
        });
    });
    let _reset = Reset;
    run();
    ACTIVE.with_borrow_mut(|active| active.take().unwrap().groups)
}

#[test]
fn legacy_run_counts_two_raw_spanless_visits_and_distinguishes_zero() {
    use crate::algorithms::taint::{run, TaintConfig};
    use prism::access_path::AccessPath;
    use prism::ast::ParsedFile;
    use prism::cpg::CpgContext;
    use prism::data_flow::{FlowEdge, FlowPath, VarAccessKind, VarLocation};
    use prism::diff::DiffInput;
    use prism::languages::Language;
    use std::collections::BTreeSet;

    let file = "a.js";
    let parsed = ParsedFile::parse(
        file,
        "function f(x) {\n  audit(x);\n}\n",
        Language::JavaScript,
    )
    .unwrap();
    let from = VarLocation {
        file: file.into(),
        function: "f".into(),
        function_start_line: 1,
        line: 1,
        path: AccessPath::simple("x"),
        start_byte: 11,
        end_byte: 12,
        kind: VarAccessKind::Def,
    };
    let mut to = from.clone();
    to.line = 2;
    to.kind = VarAccessKind::Use;
    let x = parsed.source.find("audit(x").unwrap() + "audit(".len();
    to.start_byte = x;
    to.end_byte = x + 1;
    let located = FlowPath {
        edges: vec![FlowEdge {
            from: from.clone(),
            to,
        }],
        cleansed_for: BTreeSet::new(),
    };
    let mut spanless = located.clone();
    spanless.edges[0].to.end_byte = x;
    let mut unlocated = located.clone();
    unlocated.edges[0].to.start_byte += 1;
    unlocated.edges[0].to.end_byte += 1;
    let files = BTreeMap::from([(file.into(), parsed)]);
    let ctx = CpgContext::build(&files, None);
    let diff = DiffInput::parse_unified_diff("");
    let config = TaintConfig {
        sources: vec![(file.into(), 1)],
        extra_sinks: vec!["=audit".into()],
        taint_from_diff: false,
        ..TaintConfig::default()
    };
    let check = || {
        let result = run::slice(&ctx, &diff, &config).unwrap();
        assert!(result
            .findings
            .iter()
            .any(|f| f.line == 2 && f.category.as_deref() == Some("taint_sink")));
    };
    let visits = capture(Some(vec![spanless.clone(), spanless]), check);
    assert_eq!(
        visits,
        BTreeMap::from([((file.into(), 2, "configured", "=audit".into(), "path"), 2)])
    );
    assert!(capture(Some(vec![located]), check).is_empty());
    let visits = capture(Some(vec![unlocated]), check);
    assert_eq!(
        visits,
        BTreeMap::from([((file.into(), 2, "configured", "=audit".into(), "path"), 1)])
    );
}
