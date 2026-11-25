use super::{collect_rules, InlineOccurrence, NameSinks, Occurrence, Witness};
use prism::access_path::AccessPath;
use prism::ast::tree_helpers::{call_arg_node, collect_calls};
use prism::ast::ParsedFile;
use prism::cpg::CodePropertyGraph;
use prism::data_flow::{FlowEdge, FlowPath, VarAccessKind, VarLocation};
use prism::frameworks::{SanitizerCategory, SinkPattern};
use prism::languages::Language;
use prism::rule_registry::{CONFIGURED_SINK, FORMAT_WRAPPER, JS_EVAL, TAINT_WRITE_DATA};
use prism::slice::{attach_rules, SliceFinding};
use std::collections::BTreeSet;

fn path_to(location: VarLocation) -> FlowPath {
    let mut from = location.clone();
    from.kind = VarAccessKind::Def;
    from.line = 1;
    FlowPath {
        edges: vec![FlowEdge { from, to: location }],
        cleansed_for: BTreeSet::new(),
    }
}

fn use_at(parsed: &ParsedFile, before: &str, name: &str, line: usize) -> VarLocation {
    let start = parsed.source.find(before).unwrap() + before.len();
    assert_eq!(&parsed.source[start..start + name.len()], name);
    VarLocation {
        file: parsed.path.clone(),
        function: "f".into(),
        function_start_line: 1,
        line,
        path: AccessPath::simple(name),
        start_byte: start,
        end_byte: start + name.len(),
        kind: VarAccessKind::Use,
    }
}

#[test]
fn combines_paths_and_inline_on_final_admitted_key() {
    let parsed = ParsedFile::parse(
        "a.js",
        "function f(x, y) { db.query('select ?', inner(x), y); db.query('select ?', {bind: x}); inline(source()); }",
        Language::JavaScript,
    )
    .unwrap();
    let x = path_to(use_at(&parsed, "inner(", "x", 1));
    let mut y_cleansed = path_to(use_at(&parsed, "inner(x), ", "y", 1));
    y_cleansed.cleansed_for.insert(SanitizerCategory::Sqli);
    let y_live = path_to(y_cleansed.edges[0].to.clone());
    let bound = path_to(use_at(&parsed, "bind: ", "x", 1));
    let mut calls = Vec::new();
    collect_calls(&parsed, parsed.tree.root_node(), &mut calls);
    let inline_call = calls
        .into_iter()
        .find(|call| parsed.node_text(call) == "inline(source())")
        .unwrap();
    let source = call_arg_node(&inline_call, 0).unwrap();
    let inline = InlineOccurrence {
        occurrence: Occurrence {
            call: inline_call,
            callee: parsed.call_function_name(&inline_call).unwrap(),
            operand: source,
            slot: 0,
        },
        source,
    };
    let paths = [&x, &x, &y_cleansed, &y_live, &bound];
    static QUERY: SinkPattern = SinkPattern {
        call_path: "query",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[1, 2],
        semantic_check: None,
    };
    let cpg = CodePropertyGraph::empty();
    let mut visited = Vec::new();
    let mut project = |occ: Occurrence<'_>, witness: Witness<'_, '_>, rows: &mut Vec<_>| {
        let name = parsed.node_text(&occ.callee);
        match witness {
            Witness::Path { path, reached } => {
                assert!(!reached.byte_range().is_empty());
                visited.push((name, occ.slot, parsed.node_text(&occ.operand)));
                if name == "inner" {
                    rows.push((&JS_EVAL, true));
                }
                if occ.matches_path_pattern(&parsed, &cpg, 1, path, &QUERY) {
                    assert_ne!(parsed.node_text(&occ.operand), "{bind: x}");
                    assert!(!std::ptr::eq(path, &y_cleansed));
                    rows.push((&TAINT_WRITE_DATA, false));
                }
            }
            Witness::Inline(source) => {
                assert_eq!(parsed.node_text(&source), "source()");
                rows.push((&JS_EVAL, false));
            }
        }
    };
    assert!(collect_rules(&parsed, 1, &BTreeSet::new(), paths, [inline], &mut project).is_empty());
    let admitted = BTreeSet::from([(parsed.path.clone(), 1)]);
    let rejected_inline = InlineOccurrence {
        source: inline.occurrence.callee,
        ..inline
    };
    let rows = collect_rules(
        &parsed,
        1,
        &admitted,
        paths,
        [inline, rejected_inline],
        &mut project,
    );
    assert_eq!(rows.len(), 6);
    assert_eq!(
        visited,
        [
            ("inner", 0, "x"),
            ("query", 1, "inner(x)"),
            ("inner", 0, "x"),
            ("query", 1, "inner(x)"),
            ("query", 2, "y"),
            ("query", 2, "y"),
            ("query", 1, "{bind: x}"),
        ]
    );
    let mut finding = SliceFinding {
        severity: "warning".into(),
        ..SliceFinding::default()
    };
    attach_rules(&mut finding, rows);
    assert_eq!(
        finding.rules,
        [JS_EVAL.rule_ref(), TAINT_WRITE_DATA.rule_ref()]
    );
}

#[test]
fn coalesced_reads_do_not_cross_a_binding_or_write() {
    let parsed = ParsedFile::parse(
        "a.js",
        "function f(x, y) { one(x); two(x, x); { let x = 'clean'; shadow(x); } three(y); y = 'clean'; rebound(y); }",
        Language::JavaScript,
    )
    .unwrap();
    let x = path_to(use_at(&parsed, "one(", "x", 1));
    let y = path_to(use_at(&parsed, "three(", "y", 1));
    let mut slots = Vec::new();
    let rows = collect_rules(
        &parsed,
        1,
        &BTreeSet::from([(parsed.path.clone(), 1)]),
        [&x, &y],
        [],
        |occ, _, _| slots.push((parsed.node_text(&occ.callee), occ.slot)),
    );
    assert!(rows.is_empty());
    assert_eq!(slots, [("one", 0), ("two", 0), ("two", 1), ("three", 0)]);
}

#[test]
fn only_located_arguments_can_contribute_name_detector_rows() {
    let parsed = ParsedFile::parse(
        "a.py",
        "def f(x):\n    x.report('ok'); report_extra(x); report('%s', x)\n",
        Language::Python,
    )
    .unwrap();
    let path = path_to(use_at(&parsed, "    ", "x", 2));
    let mut other_file = use_at(&parsed, "report_extra(", "x", 2);
    other_file.file = "other.py".into();
    let mut spanless = use_at(&parsed, "'%s', ", "x", 2);
    spanless.start_byte = 0;
    spanless.end_byte = 0;
    let other_file = path_to(other_file);
    let spanless = path_to(spanless);
    let configured = ["report".into()];
    let wrappers = ["=report".into()];
    let names = NameSinks::new(&configured, &wrappers);
    let admitted = BTreeSet::from([(parsed.path.clone(), 2)]);
    let mut project = |occ: Occurrence<'_>, _, rows: &mut Vec<_>| {
        names.select(parsed.node_text(&occ.callee), rows)
    };
    assert!(collect_rules(
        &parsed,
        2,
        &admitted,
        [&path, &other_file, &spanless],
        [],
        &mut project,
    )
    .is_empty());

    let path = path_to(use_at(&parsed, "report_extra(", "x", 2));
    let rows = collect_rules(&parsed, 2, &admitted, [&path], [], &mut project);
    assert_eq!(
        rows.iter().map(|(row, _)| row.id()).collect::<Vec<_>>(),
        [
            CONFIGURED_SINK.id(),
            CONFIGURED_SINK.id(),
            FORMAT_WRAPPER.id()
        ]
    );
    let exact_pattern = ["=report".into()];
    let exact = NameSinks::new(&exact_pattern, &[]);
    let rows = collect_rules(&parsed, 2, &admitted, [&path], [], |occ, _, rows| {
        exact.select(parsed.node_text(&occ.callee), rows)
    });
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0.id(), CONFIGURED_SINK.id());
    assert!(!rows[0].1);
}
