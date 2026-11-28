//! Known limits of the real walk proofs (I-walk PR 1, notice 33.1: findings left open at the
//! declared review cap). Each `known_limit_` test pins TODAY's behaviour so the limit is visible;
//! a later fix turns it red on purpose and the pin is then inverted. Each sits beside a control
//! that shows the nearby shape is still handled.
//!
//! * KL-A (strict arm only, introduced by fold 2): `{ let h; h = g; } h(0, g(t))` with `h` taking
//!   two parameters. The inner edge `t -> g.p` uses slot 0 and the outer call cannot compete for
//!   that slot, but `alias_scope_unknown` is returned before the slot check, so `ii+unknown`
//!   cuts the hop (`no_matching_site`). Plain `ii` admits it, and the classifier proves it.
//! * KL-B (pre-existing, real rule on only): the renamed argument-to-formal Def variant of the
//!   recursive fence. The Def check searches `callers["f"]` and the paired-formal fallback needs
//!   a Use destination, so `observe(p)` is reached at depth zero instead of exiting `self_param`.

use petgraph::graph::NodeIndex;

use super::object_literal_arg_tests::parse_files;
use super::trace_descent_tests::occurrence;
use super::trace_proofs::{with_walk_proofs, Rule, WalkProofs};
use super::walk_proofs::{prove_rule, Verdict};
use super::{BoundaryKind, CodePropertyGraph, CpgNode, LossReason, Trace, VarAccess};
use crate::ast::ParsedFile;
use crate::languages::Language;
use crate::reasoning::order::AstOrderView;

struct Case {
    files: std::collections::BTreeMap<String, ParsedFile>,
    cpg: CodePropertyGraph,
}

impl Case {
    fn js(source: &str) -> Self {
        let files = parse_files(Language::JavaScript, &[("a", source)], "js");
        let cpg = CodePropertyGraph::build(&files);
        Self { files, cpg }
    }

    fn prove(&self, rule: Rule, from: NodeIndex, to: NodeIndex) -> Verdict {
        let view = AstOrderView::new(&self.cpg, &self.files);
        prove_rule(rule, &self.cpg, &view, from, to)
    }

    /// A walk from `root` under the real rules `real` (none is main), with the order view.
    fn walk(&self, real: Option<&str>, root: NodeIndex) -> Trace {
        let proofs = real.map(|real| {
            WalkProofs::from_parts_real(None, None, Some(real), false)
                .unwrap()
                .unwrap()
        });
        let view = AstOrderView::new(&self.cpg, &self.files);
        with_walk_proofs(proofs, || {
            self.cpg
                .taint_trace_nodes(&[root], Some(&view as &dyn super::SameLineOrderView))
        })
    }
}

fn kind(verdict: &Verdict) -> (&'static str, &'static str) {
    let (name, _, family) = verdict.describe();
    (name, family)
}

/// The occurrence of `text` (a path) on `line` with the wanted access.
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

// ---------------------------------------------------------------------------------------------
// KL-A: the unknown alias scope is returned before the slot check
// ---------------------------------------------------------------------------------------------

/// `h` takes two parameters, so in `h(0, g(t))` slot 0 of `h` is the literal.
const TWO: &str = "function g(p) { sink(p); }\nfunction h(a, b) { sink(b); }\n";
/// The single-parameter control, as the fold-2 tests use it.
const ONE: &str = "function g(p) { sink(p); }\nfunction h(p) { sink(p); }\n";

struct Hop {
    case: Case,
    from: NodeIndex,
    to: NodeIndex,
}

fn hop(prefix: &str, body: &str) -> Hop {
    let source = format!("{prefix}function f(t) {{\n{body}\n}}\n");
    let case = Case::js(&source);
    let from = occurrence(&case.cpg, source.find("g(t)").unwrap() + 2, VarAccess::Use);
    let to = occurrence(&case.cpg, source.find("g(p)").unwrap() + 2, VarAccess::Def);
    Hop { case, from, to }
}

fn reaches(hop: &Hop, real: &str) -> bool {
    let trace = hop.case.walk(Some(real), hop.from);
    trace.frontier_by_root[&hop.from].contains(&hop.to)
}

const BLOCK_ALIAS: &str = "  { let h; h = g; }";

#[test]
fn known_limit_a_a_two_parameter_outer_call_cannot_compete_yet_the_alias_scope_cuts_the_hop() {
    let h = hop(TWO, &format!("{BLOCK_ALIAS}\n  h(0, g(t));"));
    // Today: Unknown before the slot check. The classifier proves this hop; a fix that checks
    // slot compatibility first makes this `proved` and the strict arm admit it.
    assert_eq!(
        kind(&h.case.prove(Rule::Ii, h.from, h.to)),
        ("unknown", "alias_scope_unknown")
    );
    assert!(reaches(&h, "i"), "plain i admits");
    assert!(reaches(&h, "ii"), "plain ii admits");
    assert!(
        !reaches(&h, "ii+unknown"),
        "KL-A: the strict arm cuts a hop the outer call cannot compete for"
    );
}

#[test]
fn known_limit_a_control_the_single_parameter_slot_case_is_unknown_and_cut_only_when_strict() {
    let h = hop(ONE, &format!("{BLOCK_ALIAS}\n  h(g(t));"));
    assert_eq!(
        kind(&h.case.prove(Rule::Ii, h.from, h.to)),
        ("unknown", "alias_scope_unknown")
    );
    assert!(reaches(&h, "ii"));
    assert!(!reaches(&h, "ii+unknown"));
}

#[test]
fn known_limit_a_control_an_in_scope_competitor_still_refuses() {
    for body in ["  h = g;\n  h(g(t));", "  { h = g; }\n  h(g(t));"] {
        let h = hop(ONE, body);
        assert_eq!(
            kind(&h.case.prove(Rule::Ii, h.from, h.to)),
            ("refused", "nested_same_spelling"),
            "{body}"
        );
        assert!(!reaches(&h, "ii"), "{body}");
    }
}

// ---------------------------------------------------------------------------------------------
// KL-B: a renamed recursive call into a formal Def is not fenced
// ---------------------------------------------------------------------------------------------

const DEF_VARIANT: &str = "function f(p, t, flag) {\n  const again = f;\n  if (flag) again(t, 0, false);\n  observe(p);\n}\n";
/// The Use-destination control: the renamed recursive call passes an object literal.
const USE_VARIANT: &str = "function f(p, flag) {\n  const x = acquire();\n  const again = f;\n  if (flag) again({a:x}, false);\n  observe(p.a);\n}\n";
/// A call to another owner is never a recursive binding.
const OTHER_OWNER: &str = "function g(q, t, flag) { observe(q); }\nfunction f(p, t, flag) {\n  if (flag) g(t, 0, false);\n  observe(p);\n}\n";

#[test]
fn known_limit_b_a_renamed_recursive_call_into_a_formal_def_reaches_the_sink_unfenced() {
    let case = Case::js(DEF_VARIANT);
    let seed = var(&case.cpg, "t", 1, VarAccess::Def);
    let sink = var(&case.cpg, "p", 4, VarAccess::Use);
    for real in ["i", "ii", "i,ii+unknown"] {
        let trace = case.walk(Some(real), seed);
        // KL-B: observe(p) is reached at depth zero. A fix fences it as `self_param`, which
        // turns both assertions red.
        assert!(
            trace.frontier_by_root[&seed].contains(&sink),
            "{real}: {:?}",
            trace.boundary
        );
        assert!(!self_param(&trace, seed, sink), "{real}");
    }
}

#[test]
fn known_limit_b_control_the_use_destination_renamed_recursive_call_is_fenced() {
    let case = Case::js(USE_VARIANT);
    let x = var(&case.cpg, "x", 4, VarAccess::Use);
    let sink = var(&case.cpg, "p.a", 5, VarAccess::Use);
    for real in ["i", "ii", "i,ii+unknown"] {
        let trace = case.walk(Some(real), x);
        assert!(!trace.frontier_by_root[&x].contains(&sink), "{real}");
        assert!(self_param(&trace, x, sink), "{real}: {:?}", trace.boundary);
    }
}

#[test]
fn known_limit_b_control_a_call_to_another_owner_is_not_fenced() {
    let case = Case::js(OTHER_OWNER);
    let seed = var(&case.cpg, "t", 2, VarAccess::Def);
    for real in [None, Some("i"), Some("ii+unknown")] {
        let trace = case.walk(real, seed);
        assert!(
            trace
                .boundary
                .iter()
                .all(|hop| hop.reason != LossReason::SelfParam),
            "{real:?}: {:?}",
            trace.boundary
        );
    }
}
