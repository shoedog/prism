//! Raw path fixtures for the S2-WC detector-site observer.

use prism::data_flow::FlowPath;
use std::cell::RefCell;

thread_local! {
    static FIXTURE_PATHS: RefCell<Option<Vec<FlowPath>>> = const { RefCell::new(None) };
}

pub(in crate::algorithms::taint) fn replace_fixture_paths(paths: &mut Vec<FlowPath>) {
    if let Some(fixture) = FIXTURE_PATHS.with_borrow_mut(Option::take) {
        *paths = fixture;
    }
}

fn with_fixture_paths(paths: Vec<FlowPath>, run: impl FnOnce()) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            FIXTURE_PATHS.with_borrow_mut(|fixture| *fixture = None);
        }
    }
    FIXTURE_PATHS.with_borrow_mut(|fixture| {
        assert!(fixture.is_none());
        *fixture = Some(paths);
    });
    let _reset = Reset;
    run();
}

#[test]
fn raw_visit_fixtures_preserve_admission_without_rules() {
    use crate::algorithms::taint::{run, TaintConfig};
    use prism::access_path::AccessPath;
    use prism::ast::ParsedFile;
    use prism::cpg::CpgContext;
    use prism::data_flow::{FlowEdge, FlowPath, VarAccessKind, VarLocation};
    use prism::diff::DiffInput;
    use prism::languages::Language;
    use std::collections::{BTreeMap, BTreeSet};

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
    for paths in [
        vec![spanless.clone(), spanless],
        vec![located],
        vec![unlocated],
    ] {
        with_fixture_paths(paths, || {
            let result = run::slice(&ctx, &diff, &config).unwrap();
            let finding = result
                .findings
                .iter()
                .find(|f| f.line == 2 && f.category.as_deref() == Some("taint_sink"))
                .expect("raw visits preserve admission");
            assert!(finding.rules.is_empty());
        });
    }
}
