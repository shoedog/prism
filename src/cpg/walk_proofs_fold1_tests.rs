//! Fold-1 regressions for the real walk proofs (I-walk PR 1, round 1 reviews): an inner site the
//! call-graph index files under another spelling, a block-local alias out of scope, the
//! recursive self-binding fence, and the production proof-cache key.

use petgraph::graph::NodeIndex;

use super::object_literal_arg_tests::parse_files;
use super::trace_descent_tests::occurrence;
use super::trace_proofs::{with_walk_proofs, Rule, WalkProofs};
use super::walk_proofs::syntax::{call_at, spelling, write_visible_at};
use super::walk_proofs::{prove, prove_rule, ProofCache, ProofCtx, Verdict};
use super::{
    BoundaryKind, CodePropertyGraph, CpgNode, FlowConfidence, LossReason, Relation, Trace,
    VarAccess,
};
use crate::ast::ParsedFile;
use crate::languages::Language;
use crate::reasoning::order::AstOrderView;

struct Case {
    files: std::collections::BTreeMap<String, ParsedFile>,
    cpg: CodePropertyGraph,
}

impl Case {
    fn new(sources: &[(&str, &str)]) -> Self {
        let files = parse_files(Language::JavaScript, sources, "js");
        let cpg = CodePropertyGraph::build(&files);
        Self { files, cpg }
    }

    fn prove(&self, rule: Rule, from: NodeIndex, to: NodeIndex) -> Verdict {
        let view = AstOrderView::new(&self.cpg, &self.files);
        prove_rule(rule, &self.cpg, &view, from, to)
    }

    fn walk(&self, real: Option<&str>, root: NodeIndex) -> Trace {
        let proofs = real.map(|real| {
            WalkProofs::from_parts_real(None, None, Some(real), false)
                .unwrap()
                .unwrap()
        });
        let view = AstOrderView::new(&self.cpg, &self.files);
        with_walk_proofs(proofs, || self.cpg.taint_trace_nodes(&[root], Some(&view)))
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

fn def_in(cpg: &CodePropertyGraph, file: &str, start: usize) -> NodeIndex {
    cpg.graph
        .node_indices()
        .find(|&node| {
            matches!(cpg.node(node), CpgNode::Variable { file: at, start_byte, access: VarAccess::Def, .. }
                if at == file && *start_byte == start)
        })
        .expect("def")
}

// ---------------------------------------------------------------------------------------------
// sol #1: the inner site is indexed under another spelling
// ---------------------------------------------------------------------------------------------

const A: &str = "import { run, run as go } from './b.js';\nfunction f(t) { run(go(t)); }\n";
const B: &str = "export function run(p) { sink(p); }\n";

fn alias_case() -> (Case, NodeIndex, NodeIndex) {
    let case = Case::new(&[("a", A), ("b", B)]);
    let from = occurrence(&case.cpg, A.find("go(t)").unwrap() + 3, VarAccess::Use);
    let to = def_in(&case.cpg, "b.js", B.find("(p)").unwrap() + 1);
    (case, from, to)
}

#[test]
fn an_inner_site_filed_under_an_import_alias_is_unknown_not_refused() {
    let (case, from, to) = alias_case();
    // The gate admits the hop: the outer `run` is the only site the index files under `run`.
    assert_eq!(
        case.cpg.binding_site(from, to).map(|site| site.start_byte),
        Ok(A.find("run(go").unwrap())
    );
    for rule in [Rule::I, Rule::Ii] {
        assert_eq!(kind(&case.prove(rule, from, to)).0, "unknown", "{rule:?}");
    }
}

#[test]
fn an_alias_inner_site_is_cut_only_under_plus_unknown() {
    let (case, from, to) = alias_case();
    for real in ["i", "ii"] {
        let plain = case.walk(Some(real), from);
        assert!(plain.frontier_by_root[&from].contains(&to), "{real}");
        let strict = case.walk(Some(&format!("{real}+unknown")), from);
        assert!(!strict.frontier_by_root[&from].contains(&to), "{real}");
        assert!(strict.boundary.iter().any(|hop| hop.from == from
            && hop.to == to
            && hop.reason == LossReason::NoMatchingSite));
    }
}

#[test]
fn an_outer_only_site_with_a_proven_other_inner_callee_stays_refused() {
    // Control: no alias; the inner callee resolves to a different function.
    let a = "import { run } from './b.js';\nfunction g(b) { return b; }\nfunction f(t) { run(g(t)); }\n";
    let case = Case::new(&[("a", a), ("b", B)]);
    let from = occurrence(&case.cpg, a.find("g(t)").unwrap() + 2, VarAccess::Use);
    let to = def_in(&case.cpg, "b.js", B.find("(p)").unwrap() + 1);
    let verdict = case.prove(Rule::I, from, to);
    assert_eq!(kind(&verdict), ("refused", "outer_site_only"));
    assert!(matches!(
        verdict,
        Verdict::Refused(LossReason::NoMatchingSite, _)
    ));
}

#[test]
fn an_inner_callee_that_resolves_to_nothing_is_unknown() {
    let a = "import { run } from './b.js';\nfunction f(t) { run(wrap(t)); }\n";
    let case = Case::new(&[("a", a), ("b", B)]);
    let from = occurrence(&case.cpg, a.find("wrap(t)").unwrap() + 5, VarAccess::Use);
    let to = def_in(&case.cpg, "b.js", B.find("(p)").unwrap() + 1);
    assert_eq!(kind(&case.prove(Rule::I, from, to)).0, "unknown");
}

// ---------------------------------------------------------------------------------------------
// sol #2: a block-local alias out of scope is no competitor
// ---------------------------------------------------------------------------------------------

fn emission(source: &str) -> Verdict {
    let case = Case::new(&[("a", source)]);
    let from = occurrence(&case.cpg, source.find("g(t)").unwrap() + 2, VarAccess::Use);
    let to = occurrence(&case.cpg, source.find("g(p)").unwrap() + 2, VarAccess::Def);
    case.prove(Rule::Ii, from, to)
}

const G: &str = "function g(p) { sink(p); }\nfunction h(p) { sink(p); }\n";

#[test]
fn a_block_local_alias_out_of_scope_is_not_a_competing_spelling() {
    let source = format!("{G}function f(t) {{\n  {{ const h = g; }}\n  h(g(t));\n}}\n");
    assert_eq!(kind(&emission(&source)).0, "proved");
}

#[test]
fn an_alias_in_scope_still_competes() {
    // Controls: the same alias visible at the call (same block; hoisted `var`; function level).
    for decl in [
        "{ const h = g; h(g(t)); }",
        "{ var h = g; }\n  h(g(t));",
        "const h = g;\n  h(g(t));",
    ] {
        let source = format!("{G}function f(t) {{\n  {decl}\n}}\n");
        assert_eq!(
            kind(&emission(&source)),
            ("refused", "nested_same_spelling"),
            "{decl}"
        );
    }
}

#[test]
fn a_block_local_alias_is_followed_inside_its_block_and_ignored_outside_it() {
    let source = format!("{G}function f(t) {{\n  {{ const h = g; h(t); }}\n  h(t);\n}}\n");
    let case = Case::new(&[("a", &source)]);
    let parsed = case.files.values().next().unwrap();
    let inside = call_at(
        parsed,
        (
            source.find("h(t); }").unwrap(),
            source.find("h(t); }").unwrap() + 4,
        ),
    )
    .unwrap();
    let outside_start = source.rfind("h(t)").unwrap();
    let outside = call_at(parsed, (outside_start, outside_start + 4)).unwrap();
    let function = super::walk_proofs::syntax::owner(Some(inside));
    assert_eq!(spelling(parsed, inside, function), (Some("g"), None));
    assert_eq!(spelling(parsed, outside, function), (Some("h"), None));
}

#[test]
fn a_declaration_whose_scope_cannot_be_established_has_no_visibility() {
    // A Java declarator is no JS-family declaration: its scope is not modelled.
    let source = "class C { void m() { int x = 1; } }";
    let file = ParsedFile::parse("C.java", source, Language::Java).unwrap();
    let mut stack = vec![file.tree.root_node()];
    let mut declarator = None;
    while let Some(node) = stack.pop() {
        if node.kind() == "variable_declarator" {
            declarator = Some(node);
        }
        let mut cursor = node.walk();
        stack.extend(node.children(&mut cursor));
    }
    let declarator = declarator.unwrap();
    assert_eq!(write_visible_at(declarator, declarator), None);
}

// ---------------------------------------------------------------------------------------------
// sol #3: the recursive self-binding fence, only with a real rule on
// ---------------------------------------------------------------------------------------------

const B04: &str = include_str!("../../tests/taint_parity/b04/a.js");
const B04B: &str = include_str!("../../tests/taint_parity/b04b/a.js");

fn self_param(trace: &Trace, from: NodeIndex, to: NodeIndex) -> bool {
    trace.boundary.iter().any(|hop| {
        hop.from == from
            && hop.to == to
            && hop.reason == LossReason::SelfParam
            && hop.kind == BoundaryKind::SelfFunctionParam
    })
}

#[test]
fn a_graph_only_recursive_use_binding_is_a_self_param_exit_with_a_real_rule() {
    for (source, leaf_line, sink_line) in [(B04, 3, 4), (B04B, 4, 2)] {
        let case = Case::new(&[("a", source)]);
        let x = var(&case.cpg, "x", leaf_line, VarAccess::Use);
        let sink = var(&case.cpg, "p.a", sink_line, VarAccess::Use);
        assert!(
            case.cpg.graph.find_edge(x, sink).is_some(),
            "the edge exists"
        );
        // OFF is main: the hop is local flow, so b04 reaches the sink at depth zero, while in
        // b04b the sink lies before the source and the CFG cut stops it. No boundary either way.
        let off = case.walk(None, x);
        assert_eq!(
            off.frontier_by_root[&x].contains(&sink),
            sink_line > leaf_line
        );
        assert!(off.boundary.is_empty());
        for real in ["i", "ii", "i,ii+unknown"] {
            let on = case.walk(Some(real), x);
            assert!(!on.frontier_by_root[&x].contains(&sink), "{real}");
            assert!(self_param(&on, x, sink), "{real}: {:?}", on.boundary);
        }
    }
}

#[test]
fn an_ordinary_local_def_to_use_still_passes_with_a_real_rule() {
    let source = "function f(n) {\n  const x = acquire();\n  observe(x);\n  if (n) f(n - 1);\n}\n";
    let case = Case::new(&[("a", source)]);
    let def = var(&case.cpg, "x", 2, VarAccess::Def);
    let used = var(&case.cpg, "x", 3, VarAccess::Use);
    for real in [None, Some("i"), Some("ii+unknown")] {
        let trace = case.walk(real, def);
        assert!(trace.frontier_by_root[&def].contains(&used), "{real:?}");
        assert!(trace.boundary.is_empty(), "{real:?}: {:?}", trace.boundary);
    }
}

// ---------------------------------------------------------------------------------------------
// Opus S4: the production cache key, through `prove`
// ---------------------------------------------------------------------------------------------

const FLOW: &str = "function g(p) { sink(p); }\nfunction f(t) { g(t); }\n";

#[test]
fn the_production_key_reuses_a_verdict_only_for_the_same_hop() {
    let case = Case::new(&[("a", FLOW)]);
    let from = occurrence(&case.cpg, FLOW.find("g(t)").unwrap() + 2, VarAccess::Use);
    let to = occurrence(&case.cpg, FLOW.find("g(p)").unwrap() + 2, VarAccess::Def);
    let view = AstOrderView::new(&case.cpg, &case.files);
    let mut cache = ProofCache::default();
    let mut ask = |rule, relation, label, (from, to): (NodeIndex, NodeIndex)| {
        let mut ctx = ProofCtx {
            view: Some(&view as &dyn super::SameLineOrderView),
            label,
            cache: &mut cache,
        };
        let verdict = prove(rule, &case.cpg, &mut ctx, relation, from, to);
        (kind(&verdict).0, ctx.cache.entries())
    };
    let call = Relation::CallDescent;
    let exact = Some(FlowConfidence::Exact);
    assert_eq!(ask(Rule::I, call, exact, (from, to)), ("proved", 1));
    // The same hop, rule, relation and label is a hit on the production key.
    assert_eq!(ask(Rule::I, call, exact, (from, to)), ("proved", 1));
    // Each other dimension of the key is a new entry.
    assert_eq!(ask(Rule::Ii, call, exact, (from, to)).1, 2);
    assert_eq!(ask(Rule::I, Relation::ReturnFlow, exact, (from, to)).1, 3);
    assert_eq!(ask(Rule::I, call, None, (from, to)).1, 4);
    let other = def_in(&case.cpg, "a.js", FLOW.find("(t)").unwrap() + 1);
    assert_eq!(ask(Rule::I, call, exact, (from, other)).1, 5);
    // Control: asking any of them again adds nothing.
    assert_eq!(ask(Rule::I, call, None, (from, to)).1, 5);
}
