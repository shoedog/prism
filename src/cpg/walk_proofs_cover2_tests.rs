//! Second coverage round for the real walk proofs (PR 995): the guards the first round left
//! unread. Each has a negative that is refused or unknown and a control that proves, so removing
//! the guard turns the negative red. Everything runs in process.

use std::cell::Cell;

use petgraph::graph::NodeIndex;
use tree_sitter::Node;

use super::object_literal_arg_tests::parse_files;
use super::trace_descent_tests::occurrence;
use super::trace_proofs::{with_walk_proofs, Rule, WalkProofs};
use super::walk_proofs::prove_rule;
use super::walk_proofs::syntax::{argument_slot, is_call};
use super::walk_proofs::Verdict;
use super::{CodePropertyGraph, OrderingDecision, ReturnFlowMode, SameLineOrderView, VarAccess};
use crate::ast::ParsedFile;
use crate::call_graph::FunctionId;
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

    fn parsed(&self) -> &ParsedFile {
        self.files.values().next().unwrap()
    }

    fn at(&self, source: &str, needle: &str, shift: usize, access: VarAccess) -> NodeIndex {
        occurrence(&self.cpg, source.find(needle).unwrap() + shift, access)
    }

    fn prove(&self, rule: Rule, from: NodeIndex, to: NodeIndex) -> Verdict {
        let view = AstOrderView::new(&self.cpg, &self.files);
        prove_rule(rule, &self.cpg, &view, from, to)
    }
}

fn kind(verdict: &Verdict) -> (&'static str, &'static str) {
    let (name, _, family) = verdict.describe();
    (name, family)
}

fn call_text<'a>(parsed: &'a ParsedFile, text: &str) -> Node<'a> {
    let mut stack = vec![parsed.tree.root_node()];
    while let Some(node) = stack.pop() {
        if is_call(node) && parsed.node_text(&node) == text {
            return node;
        }
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();
        stack.extend(children.into_iter().rev());
    }
    panic!("no call `{text}`")
}

// ---------------------------------------------------------------------------------------------
// walk_proofs/syntax.rs
// ---------------------------------------------------------------------------------------------

#[test]
fn argument_slot_gives_no_slot_to_bytes_outside_every_argument() {
    // `a` is a semantic value use of the caller, but it is the receiver of the call: no argument
    // covers it, so it has no slot.
    let source = "function f(a, t) { a.m(t); }\n";
    let case = Case::new(Language::JavaScript, "js", source);
    let call = call_text(case.parsed(), "a.m(t)");
    let caller = FunctionId {
        file: "a.js".into(),
        name: "f".into(),
        start_line: 1,
        end_line: 1,
    };
    let slot = |needle: &str, shift: usize| {
        let node = case.at(source, needle, shift, VarAccess::Use);
        let actual = case.cpg.to_var_location(node).expect("a variable node");
        argument_slot(case.parsed(), call, &caller, &actual, true)
    };
    assert_eq!(slot("a.m", 0), None, "receiver");
    assert_eq!(slot("t);", 0), Some(0), "control: the argument");
}

// ---------------------------------------------------------------------------------------------
// walk_proofs/emission.rs
// ---------------------------------------------------------------------------------------------

/// A view whose second and later answers for a file differ from its first: the occurrence proof
/// reads the file once and the emission proof asks again.
struct Shifting<'a> {
    inner: AstOrderView<'a>,
    later: Option<&'a ParsedFile>,
    asked: Cell<usize>,
}

impl SameLineOrderView for Shifting<'_> {
    fn ledger_file(&self, file: &str) -> Option<&ParsedFile> {
        let asked = self.asked.get();
        self.asked.set(asked + 1);
        if asked == 0 {
            self.inner.ledger_file(file)
        } else {
            self.later
        }
    }

    fn admit_same_line_recovered_def_use(
        &self,
        def: NodeIndex,
        use_: NodeIndex,
    ) -> OrderingDecision {
        self.inner.admit_same_line_recovered_def_use(def, use_)
    }
}

const FLOW: &str = "function g(p) { sink(p); }\nfunction f(t) { g(t); }\n";

fn shifted_verdict(later: Option<&ParsedFile>) -> (Verdict, usize) {
    let case = Case::new(Language::JavaScript, "js", FLOW);
    let from = case.at(FLOW, "g(t)", 2, VarAccess::Use);
    let to = case.at(FLOW, "g(p)", 2, VarAccess::Def);
    let view = Shifting {
        inner: AstOrderView::new(&case.cpg, &case.files),
        later,
        asked: Cell::new(0),
    };
    let verdict = prove_rule(Rule::Ii, &case.cpg, &view, from, to);
    (verdict, view.asked.get())
}

#[test]
fn rule_ii_without_the_source_at_emission_time_is_unknown_no_source() {
    let (verdict, asked) = shifted_verdict(None);
    assert_eq!(kind(&verdict), ("unknown", "no_source"));
    assert_eq!(
        asked, 2,
        "the occurrence proof read it, then the emission proof"
    );
    // Control: a view that keeps answering proves.
    let case = Case::new(Language::JavaScript, "js", FLOW);
    let from = case.at(FLOW, "g(t)", 2, VarAccess::Use);
    let to = case.at(FLOW, "g(p)", 2, VarAccess::Def);
    assert_eq!(kind(&case.prove(Rule::Ii, from, to)).0, "proved");
}

#[test]
fn rule_ii_with_a_source_that_holds_no_such_call_is_unknown_emission_unbound() {
    let other = ParsedFile::parse("a.js", "function z() {}\n", Language::JavaScript).unwrap();
    let (verdict, _) = shifted_verdict(Some(&other));
    assert_eq!(kind(&verdict), ("unknown", "emission_unbound"));
    // Control: the same view answering with the real source proves.
    let same = ParsedFile::parse("a.js", FLOW, Language::JavaScript).unwrap();
    assert_eq!(kind(&shifted_verdict(Some(&same)).0).0, "proved");
}

// ---------------------------------------------------------------------------------------------
// walk_proofs/occurrence.rs
// ---------------------------------------------------------------------------------------------

#[test]
fn a_wider_listed_site_with_no_inner_site_in_the_index_is_unknown() {
    let nested = "function g(p) { return p; }\nfunction f(t) { g(g(t)); }\n";
    let mut case = Case::new(Language::JavaScript, "js", nested);
    let from = case.at(nested, "g(t)", 2, VarAccess::Use);
    let to = case.at(nested, "g(p)", 2, VarAccess::Def);
    let outer = nested.find("g(g(t))").unwrap();
    let inner = nested.find("g(t)").unwrap();
    case.cpg
        .call_graph
        .callers
        .get_mut("g")
        .unwrap()
        .retain(|site| site.start_byte == outer);
    // Control: the index still holds the inner call, spelled with the target's own name, so the
    // bytes are provably the inner call's actual only.
    assert_eq!(
        kind(&case.prove(Rule::I, from, to)),
        ("refused", "outer_site_only")
    );
    for sites in case.cpg.call_graph.calls.values_mut() {
        sites.retain(|site| site.start_byte != inner);
    }
    for rule in [Rule::I, Rule::Ii] {
        assert_eq!(
            kind(&case.prove(rule, from, to)),
            ("unknown", "inner_site_unbound"),
            "{rule:?}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// walk_proofs/mod.rs, trace_proofs.rs
// ---------------------------------------------------------------------------------------------

#[test]
fn rule_ii_refuses_an_outer_only_site_as_no_compatible_emission() {
    let a = "import { run } from './b.js';\nfunction g(b) { return b; }\nfunction f(t) { run(g(t)); }\n";
    let b = "export function run(p) { sink(p); }\n";
    let files = parse_files(Language::JavaScript, &[("a", a), ("b", b)], "js");
    let cpg = CodePropertyGraph::build(&files);
    let from = occurrence(&cpg, a.find("g(t)").unwrap() + 2, VarAccess::Use);
    let to = occurrence(&cpg, b.find("(p)").unwrap() + 1, VarAccess::Def);
    let view = AstOrderView::new(&cpg, &files);
    let verdict = prove_rule(Rule::Ii, &cpg, &view, from, to);
    assert_eq!(kind(&verdict), ("refused", "no_compatible_emission"));
}

#[test]
fn compare_mode_is_observational_on_a_return_input_hop() {
    // A return operand that is not a bare read flows through a ReturnInput hop, which is no
    // crossing: compare mode prints nothing for it and never changes the walk.
    let source = include_str!("../../tests/taint_parity/w05/a.js");
    let files = parse_files(Language::JavaScript, &[("a", source)], "js");
    let cpg = CodePropertyGraph::build(&files);
    let root = occurrence(&cpg, source.find("f(y)").unwrap() + 2, VarAccess::Use);
    let view = AstOrderView::new(&cpg, &files);
    let walk = |compare: bool| {
        let proofs = WalkProofs::from_parts_real(None, None, Some("i"), compare)
            .unwrap()
            .unwrap();
        with_walk_proofs(Some(proofs), || {
            cpg.taint_trace_nodes_with_mode(&[root], Some(&view), ReturnFlowMode::On)
        })
    };
    let quiet = walk(false);
    let compared = walk(true);
    assert!(quiet
        .parents_by_root
        .values()
        .any(|(_, relation)| *relation == super::Relation::ReturnInput));
    assert_eq!(quiet.frontier_by_root, compared.frontier_by_root);
    assert_eq!(quiet.parents_by_root, compared.parents_by_root);
    assert_eq!(quiet.boundary, compared.boundary);
    assert_eq!(quiet.cuts, compared.cuts);
}
