//! Fold-2 regressions for the real walk proofs (I-walk PR 1, round 2 reviews): the CFG-impossible
//! earlier read (b04b) carries no sink boundary, a plain assignment to a block-declared name is
//! no whole-function alias, and a renamed recursive call is fenced like a plain one.

use petgraph::graph::NodeIndex;

use super::object_literal_arg_tests::parse_files;
use super::trace_descent_tests::occurrence;
use super::trace_proofs::{with_walk_proofs, Rule, WalkProofs};
use super::walk_proofs::prove_rule;
use super::walk_proofs::Verdict;
use super::{BoundaryKind, CodePropertyGraph, CpgNode, LossReason, Trace, VarAccess};
use crate::ast::ParsedFile;
use crate::languages::Language;
use crate::reasoning::order::AstOrderView;

struct Case {
    files: std::collections::BTreeMap<String, ParsedFile>,
    cpg: CodePropertyGraph,
}

impl Case {
    fn new(language: Language, ext: &str, source: &str) -> Self {
        let files = parse_files(language, &[("a", source)], ext);
        let cpg = CodePropertyGraph::build(&files);
        Self { files, cpg }
    }

    fn prove(&self, rule: Rule, from: NodeIndex, to: NodeIndex) -> Verdict {
        let view = AstOrderView::new(&self.cpg, &self.files);
        prove_rule(rule, &self.cpg, &view, from, to)
    }

    /// A walk from `root` under the real rules `real` (none is main), with or without a view.
    fn walk(&self, real: Option<&str>, root: NodeIndex, with_view: bool) -> Trace {
        let proofs = real.map(|real| {
            WalkProofs::from_parts_real(None, None, Some(real), false)
                .unwrap()
                .unwrap()
        });
        let view = AstOrderView::new(&self.cpg, &self.files);
        let view = with_view.then_some(&view as &dyn super::SameLineOrderView);
        with_walk_proofs(proofs, || self.cpg.taint_trace_nodes(&[root], view))
    }
}

fn kind(verdict: &Verdict) -> (&'static str, &'static str) {
    let (name, _, family) = verdict.describe();
    (name, family)
}

/// The occurrence of `base` (no fields) or `base.field` on `line`.
fn var(cpg: &CodePropertyGraph, text: &str, line: usize, wanted: VarAccess) -> NodeIndex {
    let mut found = cpg.graph.node_indices().filter(|&node| {
        matches!(cpg.node(node), CpgNode::Variable { path, line: at, access, .. }
            if *at == line && *access == wanted && path.to_string() == text)
    });
    let node = found
        .next()
        .unwrap_or_else(|| panic!("no {wanted:?} `{text}` on line {line}"));
    assert!(found.next().is_none(), "duplicate `{text}` on line {line}");
    node
}

fn self_param(trace: &Trace, from: NodeIndex, to: NodeIndex) -> bool {
    trace.boundary.iter().any(|hop| {
        hop.from == from
            && hop.to == to
            && hop.reason == LossReason::SelfParam
            && hop.kind == BoundaryKind::SelfFunctionParam
    })
}

const REAL_ARMS: [&str; 3] = ["i", "ii", "i,ii+unknown"];

// ---------------------------------------------------------------------------------------------
// W1: b04b, the sink precedes the source; the CFG cut stays what main records
// ---------------------------------------------------------------------------------------------

const B04: &str = include_str!("../../tests/taint_parity/b04/a.js");
const B04B: &str = include_str!("../../tests/taint_parity/b04b/a.js");

#[test]
fn b04b_the_cfg_impossible_earlier_read_is_not_reached_and_carries_no_sink_boundary() {
    let case = Case::new(Language::JavaScript, "js", B04B);
    let x = var(&case.cpg, "x", 4, VarAccess::Use);
    let sink = var(&case.cpg, "p.a", 2, VarAccess::Use);
    assert!(
        case.cpg.graph.find_edge(x, sink).is_some(),
        "the edge exists"
    );
    for with_view in [true, false] {
        let mut arms: Vec<Option<&str>> = vec![None];
        arms.extend(REAL_ARMS.map(Some));
        for real in arms {
            let trace = case.walk(real, x, with_view);
            let name = format!("{real:?} view={with_view}");
            assert!(!trace.frontier_by_root[&x].contains(&sink), "{name}");
            assert!(
                trace.boundary.iter().all(|hop| hop.to != sink),
                "{name}: a boundary projected onto the earlier read: {:?}",
                trace.boundary
            );
            assert!(
                trace
                    .cuts
                    .iter()
                    .any(|cut| cut.to == sink && cut.reason == LossReason::CfgPruned),
                "{name}: main's CfgPruned cut stays: {:?}",
                trace.cuts
            );
        }
    }
}

#[test]
fn b04_the_later_read_keeps_its_self_param_boundary_with_and_without_a_view() {
    let case = Case::new(Language::JavaScript, "js", B04);
    let x = var(&case.cpg, "x", 3, VarAccess::Use);
    let sink = var(&case.cpg, "p.a", 4, VarAccess::Use);
    for with_view in [true, false] {
        // OFF is main: the sink is reached at depth zero with no boundary.
        let off = case.walk(None, x, with_view);
        assert!(off.frontier_by_root[&x].contains(&sink));
        assert!(off.boundary.is_empty());
        for real in REAL_ARMS {
            let on = case.walk(Some(real), x, with_view);
            assert!(!on.frontier_by_root[&x].contains(&sink), "{real}");
            assert!(self_param(&on, x, sink), "{real}: {:?}", on.boundary);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// W2: a plain assignment to a name declared in a block that does not cover the call
// ---------------------------------------------------------------------------------------------

const JS_G: &str = "function g(p) { sink(p); }\nfunction h(p) { sink(p); }\n";

fn js_emission(source: &str) -> Verdict {
    let case = Case::new(Language::JavaScript, "js", source);
    let from = occurrence(&case.cpg, source.find("g(t)").unwrap() + 2, VarAccess::Use);
    let to = occurrence(&case.cpg, source.find("g(p)").unwrap() + 2, VarAccess::Def);
    case.prove(Rule::Ii, from, to)
}

fn go_emission(source: &str) -> Verdict {
    let case = Case::new(Language::Go, "go", source);
    let from = occurrence(&case.cpg, source.find("g(t)").unwrap() + 2, VarAccess::Use);
    let to = occurrence(&case.cpg, source.find("g(p").unwrap() + 2, VarAccess::Def);
    case.prove(Rule::Ii, from, to)
}

fn go_source(block: &str) -> String {
    format!(
        "package p\nfunc g(p int) int {{ sink(p); return p }}\nfunc h(p int) int {{ sink(p); return p }}\nfunc f(t int) {{\n\t{block}\n\th(g(t))\n}}\n"
    )
}

#[test]
fn a_plain_assignment_to_a_block_declared_name_is_not_a_competing_spelling() {
    // Before the fix: refused `nested_same_spelling` (the assignment was followed as function-wide).
    let source = format!("{JS_G}function f(t) {{\n  {{ let h; h = g; }}\n  h(g(t));\n}}\n");
    assert_eq!(
        kind(&js_emission(&source)),
        ("unknown", "alias_scope_unknown")
    );
    let source = go_source("{ var h func(int) int; h = g; _ = h }");
    assert_eq!(
        kind(&go_emission(&source)),
        ("unknown", "alias_scope_unknown")
    );
}

#[test]
fn the_unknown_alias_assignment_is_cut_only_under_plus_unknown() {
    let source = format!("{JS_G}function f(t) {{\n  {{ let h; h = g; }}\n  h(g(t));\n}}\n");
    let case = Case::new(Language::JavaScript, "js", &source);
    let from = occurrence(&case.cpg, source.find("g(t)").unwrap() + 2, VarAccess::Use);
    let to = occurrence(&case.cpg, source.find("g(p)").unwrap() + 2, VarAccess::Def);
    let plain = case.walk(Some("ii"), from, true);
    assert!(plain.frontier_by_root[&from].contains(&to));
    let strict = case.walk(Some("ii+unknown"), from, true);
    assert!(!strict.frontier_by_root[&from].contains(&to));
}

#[test]
fn a_plain_assignment_visible_at_the_call_still_competes() {
    // Controls: the only write is the in-scope assignment (no declaration was dropped), JS and Go.
    for block in ["h = g;\n  h(g(t));", "{ h = g; }\n  h(g(t));"] {
        let source = format!("{JS_G}function f(t) {{\n  {block}\n}}\n");
        assert_eq!(
            kind(&js_emission(&source)),
            ("refused", "nested_same_spelling"),
            "{block}"
        );
    }
    let source = go_source("h = g");
    assert_eq!(
        kind(&go_emission(&source)),
        ("refused", "nested_same_spelling")
    );
}

#[test]
fn an_unknown_alias_does_not_hide_a_proven_competitor() {
    // The outer-most `h` is an unknown alias, the middle `g(...)` competes for certain.
    let source = format!("{JS_G}function f(t) {{\n  {{ let h; h = g; }}\n  h(g(g(t)));\n}}\n");
    let case = Case::new(Language::JavaScript, "js", &source);
    let at = source.find("g(t)").unwrap();
    let from = occurrence(&case.cpg, at + 2, VarAccess::Use);
    let to = occurrence(&case.cpg, source.find("g(p)").unwrap() + 2, VarAccess::Def);
    assert_eq!(
        kind(&case.prove(Rule::Ii, from, to)),
        ("refused", "nested_same_spelling")
    );
}

// ---------------------------------------------------------------------------------------------
// sol r2 #3: a renamed recursive call is a recursive binding
// ---------------------------------------------------------------------------------------------

const RENAMED: &str = "function f(p, flag) {\n  const x = acquire(); // @SRC\n  const again = f;\n  if (flag) again({a:x}, false);\n  observe(p.a); // @SINK\n}\n";
const OTHER: &str = "function g(q) { observe(q.a); }\nfunction f(p, flag) {\n  const x = acquire();\n  if (flag) g({a:x});\n  observe(p.a);\n}\n";

#[test]
fn a_renamed_recursive_call_is_a_self_param_exit_with_a_real_rule() {
    let case = Case::new(Language::JavaScript, "js", RENAMED);
    let x = var(&case.cpg, "x", 4, VarAccess::Use);
    let sink = var(&case.cpg, "p.a", 5, VarAccess::Use);
    assert!(
        case.cpg.graph.find_edge(x, sink).is_some(),
        "the edge exists"
    );
    for with_view in [true, false] {
        // OFF is main: the hop is local flow, the sink is reached at depth zero.
        let off = case.walk(None, x, with_view);
        assert!(off.frontier_by_root[&x].contains(&sink));
        assert!(off.boundary.is_empty());
        for real in REAL_ARMS {
            let on = case.walk(Some(real), x, with_view);
            assert!(!on.frontier_by_root[&x].contains(&sink), "{real}");
            assert!(self_param(&on, x, sink), "{real}: {:?}", on.boundary);
        }
    }
}

#[test]
fn a_call_to_another_owner_is_not_a_recursive_binding() {
    // Control: the actual feeds `g`'s read, a different owner; the ordinary sink in `f` is
    // untouched and nothing is fenced as `self_param`.
    let case = Case::new(Language::JavaScript, "js", OTHER);
    let x = var(&case.cpg, "x", 3, VarAccess::Def);
    let sink = var(&case.cpg, "p.a", 5, VarAccess::Use);
    let used = var(&case.cpg, "x", 4, VarAccess::Use);
    for real in [None, Some("i"), Some("ii+unknown")] {
        let trace = case.walk(real, x, true);
        assert!(trace.frontier_by_root[&x].contains(&used), "{real:?}");
        assert!(
            trace
                .boundary
                .iter()
                .all(|hop| hop.reason != LossReason::SelfParam),
            "{real:?}: {:?}",
            trace.boundary
        );
        assert!(!trace.frontier_by_root[&x].contains(&sink), "{real:?}");
    }
}
