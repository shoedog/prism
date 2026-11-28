//! Behaviour tests for the refusal and unknown arms of the real walk proofs and of the shared
//! call-site readings (`walk_proofs/`): each guard has a negative that is refused or unknown and
//! a control that proves, so removing the guard turns the negative red. Everything runs in
//! process, on the proof functions and the syntax readings directly.

use petgraph::graph::NodeIndex;
use tree_sitter::Node;

use super::object_literal_arg_tests::parse_files;
use super::trace_descent_tests::occurrence;
use super::trace_proofs::{with_walk_proofs, Rule, WalkProofs};
use super::walk_proofs::syntax::{
    actual_sites, args, argument_slot, formal_matches, is_call, owned_by, owner, returned_operand,
    same_line_peers, spelling, unwrap_wrappers, writes, ActualFailure,
};
use super::walk_proofs::{prove, prove_rule, ProofCache, ProofCtx, Verdict};
use super::{CodePropertyGraph, CpgNode, LossReason, VarAccess};
use crate::ast::ParsedFile;
use crate::call_graph::FunctionId;
use crate::data_flow::VarAccessKind;
use crate::languages::Language;
use crate::reasoning::order::AstOrderView;

const JS_FLOW: &str = "function g(p) { sink(p); }\nfunction f(t) { g(t); }\n";

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

    /// `(verdict family, verdict kind)` of one rule's proof for the hop.
    fn prove(&self, rule: Rule, from: NodeIndex, to: NodeIndex) -> Verdict {
        let view = AstOrderView::new(&self.cpg, &self.files);
        prove_rule(rule, &self.cpg, &view, from, to)
    }
}

fn kind(verdict: &Verdict) -> (&'static str, &'static str) {
    let (name, _, family) = verdict.describe();
    (name, family)
}

fn clone_with(
    cpg: &mut CodePropertyGraph,
    template: NodeIndex,
    edit: impl FnOnce(&mut CpgNode),
) -> NodeIndex {
    let mut node = cpg.node(template).clone();
    edit(&mut node);
    cpg.graph.add_node(node)
}

fn rebound(cpg: &mut CodePropertyGraph, template: NodeIndex, bounds: (usize, usize)) -> NodeIndex {
    clone_with(cpg, template, |node| {
        let CpgNode::Variable {
            start_byte,
            end_byte,
            ..
        } = node
        else {
            unreachable!()
        };
        (*start_byte, *end_byte) = bounds;
    })
}

fn find<'a>(parsed: &'a ParsedFile, wanted: &dyn Fn(Node<'a>) -> bool) -> Option<Node<'a>> {
    let mut stack = vec![parsed.tree.root_node()];
    while let Some(node) = stack.pop() {
        if wanted(node) {
            return Some(node);
        }
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();
        stack.extend(children.into_iter().rev());
    }
    None
}

fn call_text<'a>(parsed: &'a ParsedFile, text: &str) -> Node<'a> {
    find(parsed, &|node| {
        is_call(node) && parsed.node_text(&node) == text
    })
    .unwrap_or_else(|| panic!("no call `{text}`"))
}

fn owner_of_call<'a>(parsed: &'a ParsedFile, text: &str) -> Node<'a> {
    owner(Some(call_text(parsed, text))).expect("call has an owner")
}

fn owner_id(file: &str, name: &str, line: usize) -> (String, String, usize) {
    (file.into(), name.into(), line)
}

// ---------------------------------------------------------------------------------------------
// walk_proofs/mod.rs
// ---------------------------------------------------------------------------------------------

#[test]
fn unknown_verdict_describes_itself_without_reason() {
    assert_eq!(
        Verdict::Unknown("some_family").describe(),
        ("unknown", "-".to_owned(), "some_family")
    );
    assert_ne!(
        Verdict::Refused(LossReason::NoMatchingSite, "fam")
            .describe()
            .0,
        "unknown"
    );
}

#[test]
#[should_panic(expected = "walk proof rule `iii` is not implemented")]
fn an_unimplemented_rule_is_asked_for_by_name_and_panics() {
    let case = Case::new(Language::JavaScript, "js", JS_FLOW);
    let from = case.at(JS_FLOW, "g(t)", 2, VarAccess::Use);
    let to = case.at(JS_FLOW, "g(p)", 2, VarAccess::Def);
    // Control: an implemented rule answers.
    assert_eq!(kind(&case.prove(Rule::I, from, to)).0, "proved");
    case.prove(Rule::Iii, from, to);
}

#[test]
fn a_hop_with_no_parsed_file_in_the_view_is_unknown_no_view() {
    let case = Case::new(Language::JavaScript, "js", JS_FLOW);
    let from = case.at(JS_FLOW, "g(t)", 2, VarAccess::Use);
    let to = case.at(JS_FLOW, "g(p)", 2, VarAccess::Def);
    let mut cache = ProofCache::default();
    let nothing = Default::default();
    let empty = AstOrderView::new(&case.cpg, &nothing);
    let full = AstOrderView::new(&case.cpg, &case.files);
    let mut hop = |view: Option<&AstOrderView<'_>>| {
        let mut ctx = ProofCtx {
            view: view.map(|view| view as &dyn super::SameLineOrderView),
            label: None,
            cache: &mut cache,
        };
        prove(
            Rule::I,
            &case.cpg,
            &mut ctx,
            super::Relation::CallDescent,
            from,
            to,
        )
    };
    assert_eq!(kind(&hop(None)), ("unknown", "no_view"));
    assert_eq!(kind(&hop(Some(&empty))), ("unknown", "no_view"));
    // Control: the view that holds the file proves the same hop (twice: the second answer is
    // the cached one and agrees).
    let proved = hop(Some(&full));
    assert_eq!(kind(&proved).0, "proved");
    assert_eq!(hop(Some(&full)), proved);
}

#[test]
fn a_hop_between_non_variable_nodes_is_unknown_not_variable() {
    let case = Case::new(Language::JavaScript, "js", JS_FLOW);
    let from = case.at(JS_FLOW, "g(t)", 2, VarAccess::Use);
    let to = case.at(JS_FLOW, "g(p)", 2, VarAccess::Def);
    let statement = case
        .cpg
        .graph
        .node_indices()
        .find(|&node| !matches!(case.cpg.node(node), CpgNode::Variable { .. }))
        .expect("a function or statement node");
    assert_eq!(case.cpg.node(statement).file(), "a.js");
    let view = AstOrderView::new(&case.cpg, &case.files);
    let mut cache = ProofCache::default();
    for (a, b) in [(statement, to), (from, statement)] {
        let mut ctx = ProofCtx {
            view: Some(&view),
            label: None,
            cache: &mut cache,
        };
        let verdict = prove(
            Rule::I,
            &case.cpg,
            &mut ctx,
            super::Relation::CallDescent,
            a,
            b,
        );
        assert_eq!(kind(&verdict), ("unknown", "not_variable"));
    }
    let mut ctx = ProofCtx {
        view: Some(&view),
        label: None,
        cache: &mut cache,
    };
    let control = prove(
        Rule::I,
        &case.cpg,
        &mut ctx,
        super::Relation::CallDescent,
        from,
        to,
    );
    assert_eq!(kind(&control).0, "proved");
}

// ---------------------------------------------------------------------------------------------
// walk_proofs/occurrence.rs
// ---------------------------------------------------------------------------------------------

#[test]
fn only_a_use_to_def_hop_has_an_occurrence() {
    let case = Case::new(Language::JavaScript, "js", JS_FLOW);
    let use_t = case.at(JS_FLOW, "g(t)", 2, VarAccess::Use);
    let def_p = case.at(JS_FLOW, "g(p)", 2, VarAccess::Def);
    let def_t = case.at(JS_FLOW, "f(t)", 2, VarAccess::Def);
    for rule in [Rule::I, Rule::Ii] {
        assert_eq!(
            kind(&case.prove(rule, def_t, def_p)),
            ("unknown", "not_use_to_def")
        );
        assert_eq!(
            kind(&case.prove(rule, use_t, use_t)),
            ("unknown", "not_use_to_def")
        );
        assert_eq!(kind(&case.prove(rule, use_t, def_p)).0, "proved", "control");
    }
}

#[test]
fn an_endpoint_file_missing_from_the_view_is_unknown_no_source() {
    let case = Case::new(Language::JavaScript, "js", JS_FLOW);
    let from = case.at(JS_FLOW, "g(t)", 2, VarAccess::Use);
    let to = case.at(JS_FLOW, "g(p)", 2, VarAccess::Def);
    let nothing = Default::default();
    let empty = AstOrderView::new(&case.cpg, &nothing);
    for rule in [Rule::I, Rule::Ii] {
        let verdict = prove_rule(rule, &case.cpg, &empty, from, to);
        assert_eq!(kind(&verdict), ("unknown", "no_source"), "{rule:?}");
        assert_eq!(kind(&case.prove(rule, from, to)).0, "proved", "control");
    }
}

#[test]
fn a_site_the_call_graph_does_not_list_is_unknown_unless_a_wider_call_is() {
    // Nothing lists a site of `g`: undecided, not refused.
    let mut case = Case::new(Language::JavaScript, "js", JS_FLOW);
    let from = case.at(JS_FLOW, "g(t)", 2, VarAccess::Use);
    let to = case.at(JS_FLOW, "g(p)", 2, VarAccess::Def);
    assert_eq!(kind(&case.prove(Rule::I, from, to)).0, "proved", "control");
    case.cpg.call_graph.callers.get_mut("g").unwrap().clear();
    for rule in [Rule::I, Rule::Ii] {
        assert_eq!(
            kind(&case.prove(rule, from, to)),
            ("unknown", "target_or_site_unbound"),
            "{rule:?}"
        );
    }

    // The only listed site is the call that wraps the inner one: the bytes supply the inner
    // call's actual only.
    let nested = "function g(p) { return p; }\nfunction f(t) { g(g(t)); }\n";
    let mut case = Case::new(Language::JavaScript, "js", nested);
    let from = case.at(nested, "g(t)", 2, VarAccess::Use);
    let to = case.at(nested, "g(p)", 2, VarAccess::Def);
    assert_eq!(kind(&case.prove(Rule::I, from, to)).0, "proved", "control");
    let outer = nested.find("g(g(t))").unwrap();
    case.cpg
        .call_graph
        .callers
        .get_mut("g")
        .unwrap()
        .retain(|site| site.start_byte == outer);
    let verdict = case.prove(Rule::I, from, to);
    assert_eq!(kind(&verdict), ("refused", "outer_site_only"));
    assert!(matches!(
        verdict,
        Verdict::Refused(LossReason::NoMatchingSite, _)
    ));
}

#[test]
fn an_actual_wider_than_the_reached_bytes_is_a_nested_value_operand() {
    let source = "function g(p) { sink(p); }\nfunction f(t) { g(t + 1); }\n";
    let case = Case::new(Language::JavaScript, "js", source);
    let from = case.at(source, "t + 1", 0, VarAccess::Use);
    let to = case.at(source, "g(p)", 2, VarAccess::Def);
    let verdict = case.prove(Rule::I, from, to);
    assert_eq!(kind(&verdict), ("proved", "nested_value_operand"));
    // Control: the bare actual is an ordinary one.
    let bare = Case::new(Language::JavaScript, "js", JS_FLOW);
    let from = bare.at(JS_FLOW, "g(t)", 2, VarAccess::Use);
    let to = bare.at(JS_FLOW, "g(p)", 2, VarAccess::Def);
    assert_eq!(
        kind(&bare.prove(Rule::I, from, to)),
        ("proved", "ordinary_actual")
    );
}

#[test]
fn erased_ts_wrappers_and_parentheses_are_peeled_to_an_ordinary_actual() {
    for wrapped in ["(t)", "t as any", "t!", "t satisfies any", "<any>t"] {
        let source =
            format!("function g(p: any) {{ sink(p); }}\nfunction f(t: any) {{ g({wrapped}); }}\n");
        let case = Case::new(Language::TypeScript, "ts", &source);
        let call = format!("g({wrapped})");
        let bytes = source.find(&call).unwrap() + 2 + wrapped.find('t').unwrap();
        let from = occurrence(&case.cpg, bytes, VarAccess::Use);
        let to = case.at(&source, "g(p", 2, VarAccess::Def);
        assert_eq!(
            kind(&case.prove(Rule::I, from, to)),
            ("proved", "ordinary_actual"),
            "{wrapped}"
        );
    }
    // The peeler itself, on the argument node.
    let source = "function f(t: any) { g(<any>t); g(t as any); g(t!); g((t)); }\n";
    let case = Case::new(Language::TypeScript, "ts", source);
    for call in ["g(<any>t)", "g(t as any)", "g(t!)", "g((t))"] {
        let call = call_text(case.parsed(), call);
        let arg = args(call)[0];
        let inner = unwrap_wrappers(Some(arg)).unwrap();
        assert_eq!(case.parsed().node_text(&inner), "t", "{}", arg.kind());
        assert_ne!(
            arg.kind(),
            "identifier",
            "control: the argument is a wrapper"
        );
    }
    // A node that is no wrapper is returned as it is.
    let call = call_text(case.parsed(), "g(t!)");
    assert_eq!(unwrap_wrappers(Some(call)), Some(call));
    assert_eq!(unwrap_wrappers(None), None);
}

// ---------------------------------------------------------------------------------------------
// walk_proofs/syntax.rs: actual_sites and the owner an occurrence belongs to
// ---------------------------------------------------------------------------------------------

#[test]
fn bytes_outside_every_function_have_no_execution_owner() {
    let source = "function g(p) { sink(p); }\nfunction f(t) { g(t); }\ng(1);\n";
    let mut case = Case::new(Language::JavaScript, "js", source);
    let from = case.at(source, "g(t)", 2, VarAccess::Use);
    let to = case.at(source, "g(p)", 2, VarAccess::Def);
    let top = source.rfind("1)").unwrap();
    let moved = rebound(&mut case.cpg, from, (top, top + 1));
    for rule in [Rule::I, Rule::Ii] {
        assert_eq!(
            kind(&case.prove(rule, moved, to)),
            ("unknown", "execution_owner_unbound"),
            "{rule:?}"
        );
        assert_eq!(kind(&case.prove(rule, from, to)).0, "proved", "control");
    }
    let id = owner_id("a.js", "f", 2);
    assert_eq!(
        actual_sites(case.parsed(), (top, top + 1), &id).err(),
        Some(ActualFailure::Unbound("execution_owner_unbound"))
    );
}

#[test]
fn a_read_that_is_no_call_actual_is_refused_and_a_jsx_read_is_unknown() {
    let source = "function g(p) { sink(p); }\nfunction f(t) { const x = t; g(x); }\n";
    let case = Case::new(Language::JavaScript, "js", source);
    let mut case = case;
    // The graph holds no use for the initialiser read; borrow a use's node for those bytes.
    let template = case.at(source, "g(x)", 2, VarAccess::Use);
    let read = source.find("= t").unwrap() + 2;
    let from = rebound(&mut case.cpg, template, (read, read + 1));
    let to = case.at(source, "g(p)", 2, VarAccess::Def);
    let refused = case.prove(Rule::I, from, to);
    assert_eq!(kind(&refused), ("refused", "no_ordinary_actual"));
    assert!(matches!(
        refused,
        Verdict::Refused(LossReason::NoMatchingSite, _)
    ));
    assert_eq!(
        case.prove(Rule::Ii, from, to).describe().1,
        LossReason::NoMatchingSite.to_string()
    );
    // Control: the same variable as a call actual proves.
    let actual = case.at(source, "g(x)", 2, VarAccess::Use);
    assert_eq!(kind(&case.prove(Rule::I, actual, to)).0, "proved");

    let jsx = "function g(p) { sink(p); }\nfunction f(t) { return <A b={t} />; }\n";
    let case = Case::new(Language::JavaScript, "jsx", jsx);
    let from = case.at(jsx, "{t}", 1, VarAccess::Use);
    let to = case.at(jsx, "g(p)", 2, VarAccess::Def);
    for rule in [Rule::I, Rule::Ii] {
        assert_eq!(
            kind(&case.prove(rule, from, to)),
            ("unknown", "jsx_or_binding_read"),
            "{rule:?}"
        );
    }
}

#[test]
fn owner_identity_follows_the_line_the_graph_names_and_the_decorated_start() {
    let source = "def g(p):\n    sink(p)\n@deco\ndef f(t):\n    g(t)\n";
    let case = Case::new(Language::Python, "py", source);
    let from = case.at(source, "g(t)", 2, VarAccess::Use);
    let to = case.at(source, "g(p)", 2, VarAccess::Def);
    // A decorated owner matches by its decorator line or by its `def` line.
    let function = owner_of_call(case.parsed(), "g(t)");
    assert!(owned_by(
        case.parsed(),
        Some(function),
        &owner_id("a.py", "f", 3)
    ));
    assert!(owned_by(
        case.parsed(),
        Some(function),
        &owner_id("a.py", "f", 4)
    ));
    assert!(!owned_by(
        case.parsed(),
        Some(function),
        &owner_id("a.py", "f", 5)
    ));
    assert!(!owned_by(
        case.parsed(),
        Some(function),
        &owner_id("a.py", "other", 4)
    ));
    assert!(!owned_by(case.parsed(), None, &owner_id("a.py", "f", 4)));
    // An anonymous owner is matched by its start line alone.
    assert!(owned_by(
        case.parsed(),
        Some(function),
        &owner_id("a.py", "<anon@4>", 4)
    ));
    assert!(!owned_by(
        case.parsed(),
        Some(function),
        &owner_id("a.py", "<anon@9>", 9)
    ));

    // Through the proof: the producer's own owner id proves; one naming another line or name
    // is an occurrence of another callable.
    assert_eq!(kind(&case.prove(Rule::I, from, to)).0, "proved", "control");
    for edit in [
        (|node: &mut CpgNode| {
            if let CpgNode::Variable {
                function_start_line,
                ..
            } = node
            {
                *function_start_line = 99;
            }
        }) as fn(&mut CpgNode),
        |node| {
            if let CpgNode::Variable { function, .. } = node {
                *function = "other".into();
            }
        },
    ] {
        let mut case = Case::new(Language::Python, "py", source);
        let from = case.at(source, "g(t)", 2, VarAccess::Use);
        let moved = clone_with(&mut case.cpg, from, edit);
        let verdict = case.prove(Rule::I, moved, to);
        assert_eq!(kind(&verdict), ("refused", "intervening_callable"));
        assert!(matches!(
            verdict,
            Verdict::Refused(LossReason::NoMatchingSite, _)
        ));
    }
}

#[test]
fn a_call_without_an_argument_list_has_no_arguments() {
    let case = Case::new(
        Language::JavaScript,
        "js",
        "function f() { new Foo; new Foo(a); }\n",
    );
    let bare = call_text(case.parsed(), "new Foo");
    assert!(args(bare).is_empty());
    let listed = call_text(case.parsed(), "new Foo(a)");
    assert_eq!(args(listed).len(), 1, "control");
}

// ---------------------------------------------------------------------------------------------
// walk_proofs/syntax.rs: the ledger's readings
// ---------------------------------------------------------------------------------------------

fn location(case: &Case, node: NodeIndex) -> crate::data_flow::VarLocation {
    case.cpg.to_var_location(node).expect("a variable node")
}

fn function_id(file: &str, name: &str, start_line: usize) -> FunctionId {
    FunctionId {
        file: file.into(),
        name: name.into(),
        start_line,
        end_line: start_line,
    }
}

#[test]
fn argument_slot_refuses_what_is_not_a_use_of_the_caller() {
    let case = Case::new(Language::JavaScript, "js", JS_FLOW);
    let use_t = location(&case, case.at(JS_FLOW, "g(t)", 2, VarAccess::Use));
    let call = call_text(case.parsed(), "g(t)");
    let caller = function_id("a.js", "f", 2);
    let slot = |actual: &crate::data_flow::VarLocation, caller: &FunctionId| {
        argument_slot(case.parsed(), call, caller, actual, true)
    };
    assert_eq!(slot(&use_t, &caller), Some(0), "control");

    let mut other = use_t.clone();
    other.function = "elsewhere".into();
    assert_eq!(slot(&other, &caller), None, "another function");
    let mut other = use_t.clone();
    other.function_start_line = 9;
    assert_eq!(slot(&other, &caller), None, "another start line");
    let mut other = use_t.clone();
    other.file = "b.js".into();
    assert_eq!(slot(&other, &caller), None, "another file");
    let mut other = use_t.clone();
    other.kind = VarAccessKind::Def;
    assert_eq!(slot(&other, &caller), None, "a def");
    let mut other = use_t.clone();
    other.end_byte = other.start_byte;
    assert_eq!(slot(&other, &caller), None, "zero width");
    // The callee name is no value use.
    let mut other = use_t.clone();
    let callee = JS_FLOW.find("g(t)").unwrap();
    (other.start_byte, other.end_byte) = (callee, callee + 1);
    assert_eq!(slot(&other, &caller), None, "callee token");
}

#[test]
fn argument_slot_positional_mode_refuses_keywords_and_splats() {
    let source = "def g(p):\n    sink(p)\ndef f(t, a):\n    g(p=t)\n    g(*a)\n";
    let case = Case::new(Language::Python, "py", source);
    let caller = function_id("a.py", "f", 3);
    for (text, needle, shift) in [("g(p=t)", "p=t", 2), ("g(*a)", "*a", 1)] {
        let actual = location(&case, case.at(source, needle, shift, VarAccess::Use));
        let call = call_text(case.parsed(), text);
        assert_eq!(
            argument_slot(case.parsed(), call, &caller, &actual, true),
            None,
            "{text}: positional"
        );
        assert_eq!(
            argument_slot(case.parsed(), call, &caller, &actual, false),
            Some(0),
            "{text}: control, not positional"
        );
    }
    let source = "function f(t, a) { g(...a); }\n";
    let case = Case::new(Language::JavaScript, "js", source);
    let actual = location(&case, case.at(source, "...a", 3, VarAccess::Use));
    let call = call_text(case.parsed(), "g(...a)");
    let caller = function_id("a.js", "f", 1);
    assert_eq!(
        argument_slot(case.parsed(), call, &caller, &actual, true),
        None
    );
    assert_eq!(
        argument_slot(case.parsed(), call, &caller, &actual, false),
        Some(0),
        "control"
    );
}

#[test]
fn positional_mode_gives_up_on_arguments_after_a_splat() {
    // After a splat the slot of a later argument is unknown, so a positional reading refuses
    // it; a reading that does not need the slot (not positional) still has one.
    let cases: [(Language, &str, &str, &str, &str); 2] = [
        (
            Language::Python,
            "py",
            "def f(t, a):\n    g(*a, t)\n",
            "g(*a, t)",
            "caller",
        ),
        (
            Language::JavaScript,
            "js",
            "function f(t, a) { g(...a, t); }\n",
            "g(...a, t)",
            "caller",
        ),
    ];
    for (language, ext, source, call_text_, _) in cases {
        let case = Case::new(language, ext, source);
        let call = call_text(case.parsed(), call_text_);
        let at = source.find(", t").unwrap() + 2;
        let template = case
            .cpg
            .graph
            .node_indices()
            .find(|&node| {
                matches!(
                    case.cpg.node(node),
                    CpgNode::Variable {
                        access: VarAccess::Use,
                        ..
                    }
                )
            })
            .expect("a use");
        let mut actual = location(&case, template);
        (actual.start_byte, actual.end_byte) = (at, at + 1);
        let caller = function_id(&format!("a.{ext}"), "f", 1 + usize::from(ext == "py"));
        actual.file = caller.file.clone();
        actual.function = "f".into();
        actual.function_start_line = caller.start_line;
        actual.kind = VarAccessKind::Use;
        assert_eq!(
            argument_slot(case.parsed(), call, &caller, &actual, true),
            None,
            "{language:?}: positional"
        );
        assert_eq!(
            argument_slot(case.parsed(), call, &caller, &actual, false),
            Some(1),
            "{language:?}: control, not positional"
        );
    }
}

/// A def location of the formal `p` at `start`, whatever the graph holds for that owner (the
/// graph has no formal def for a method).
fn formal_location(case: &Case, start: usize) -> crate::data_flow::VarLocation {
    let template = case
        .cpg
        .graph
        .node_indices()
        .find(|&node| {
            matches!(case.cpg.node(node), CpgNode::Variable { path, access: VarAccess::Def, .. }
                if path.base == "p")
        })
        .expect("a def of p");
    let mut location = location(case, template);
    (location.start_byte, location.end_byte) = (start, start + 1);
    location
}

#[test]
fn a_receiver_bearing_signature_matches_no_formal() {
    let cases: [(Language, &str, &str, &str, &str); 3] = [
        (
            Language::Go,
            "go",
            "package p\nfunc g(p int) { sink(p) }\ntype R int\nfunc (r R) m(p int) {}\nfunc f(t int) { g(t) }\n",
            "func (r R) m(p int)",
            "func g(p int)",
        ),
        (
            Language::Python,
            "py",
            "def g(p):\n    sink(p)\nclass C:\n    def m(self, p):\n        pass\ndef f(t):\n    g(t)\n",
            "def m(self, p)",
            "def g(p)",
        ),
        (
            Language::Python,
            "py",
            "def g(p):\n    sink(p)\nclass C:\n    @staticmethod\n    def m(self, p):\n        pass\ndef f(t):\n    g(t)\n",
            "def m(self, p)",
            "def g(p)",
        ),
    ];
    for (language, ext, source, method, plain) in cases {
        let case = Case::new(language, ext, source);
        let parsed = case.parsed();
        let function_at = |header: &str| {
            let at = source.find(header).unwrap();
            find(parsed, &|node| {
                node.start_byte() == at
                    && matches!(
                        node.kind(),
                        "function_definition" | "function_declaration" | "method_declaration"
                    )
            })
            .expect("function node")
        };
        let formal_of = |owner: Node<'_>| {
            let formals = parsed.function_parameter_slot_occurrences(&owner).unwrap();
            let (slot, (_, start, _)) = formals
                .iter()
                .enumerate()
                .find(|(_, (name, _, _))| name == "p")
                .map(|(slot, formal)| (slot, formal.clone()))
                .expect("a formal p");
            (slot, start)
        };
        let owner = function_at(method);
        let (slot, start) = formal_of(owner);
        let receiver = formal_location(&case, start);
        assert!(
            !formal_matches(parsed, owner, slot, &receiver),
            "{language:?}: method with a receiver"
        );
        // Control: a plain function's formal matches its own slot.
        let owner = function_at(plain);
        let (slot, start) = formal_of(owner);
        let receiver = formal_location(&case, start);
        assert!(
            formal_matches(parsed, owner, slot, &receiver),
            "{language:?}: plain function"
        );
        // And a field path is no formal.
        let mut field = receiver.clone();
        field.path.fields.push("x".into());
        assert!(!formal_matches(parsed, owner, slot, &field));
    }
}

#[test]
fn only_a_use_or_the_return_node_can_be_the_returned_operand() {
    let source = "function f(t) { return t; }\n";
    let mut case = Case::new(Language::JavaScript, "js", source);
    let use_t = case.at(source, "return t", 7, VarAccess::Use);
    let def_twin = clone_with(&mut case.cpg, use_t, |node| {
        if let CpgNode::Variable { access, .. } = node {
            *access = VarAccess::Def;
        }
    });
    let parsed = case.parsed();
    let function = owner(Some(
        find(parsed, &|node| node.kind() == "return_statement").unwrap(),
    ))
    .unwrap();
    let operand = |node: NodeIndex, slot: usize, arity: usize| {
        returned_operand(parsed, function, case.cpg.node(node), slot, arity)
    };
    assert!(operand(use_t, 0, 1), "control");
    assert!(!operand(use_t, 0, 2), "arity");
    assert!(!operand(use_t, 1, 1), "slot");
    assert!(
        !operand(def_twin, 0, 1),
        "a def on the returned bytes is no returned operand"
    );
}

// ---------------------------------------------------------------------------------------------
// walk_proofs/syntax.rs: peers, writes and spelling
// ---------------------------------------------------------------------------------------------

#[test]
fn a_nested_function_call_is_no_peer_of_the_enclosing_call() {
    let peers = |source: &str, call: &str| {
        let case = Case::new(Language::JavaScript, "js", source);
        let parsed = case.parsed();
        let call = call_text(parsed, call);
        same_line_peers(parsed, call, owner(Some(call))).len()
    };
    assert_eq!(
        peers("function f(a, b) { g(a); g(b); }\n", "g(a)"),
        1,
        "control"
    );
    assert_eq!(
        peers("function f(a, b) { g(a); (() => g(b))(); }\n", "g(a)"),
        0,
        "the arrow's call belongs to the arrow"
    );
}

fn written(language: Language, ext: &str, source: &str, name: &str) -> usize {
    let case = Case::new(language, ext, source);
    let parsed = case.parsed();
    let function = find(parsed, &|node| node.kind().contains("function")).unwrap();
    writes(parsed, function, name, source.len()).len()
}

#[test]
fn writes_read_every_binding_shape_of_the_name() {
    let js = |body: &str, name: &str| {
        written(
            Language::JavaScript,
            "js",
            &format!("function f(o, x) {{ {body} }}\n"),
            name,
        )
    };
    assert_eq!(js("const h = o;", "h"), 1, "declarator");
    assert_eq!(
        js("let h; h = o;", "h"),
        2,
        "uninitialised declarator and assignment"
    );
    assert_eq!(js("const { k: h } = o;", "h"), 1, "pair pattern");
    assert_eq!(js("const { h = 1 } = o;", "h"), 1, "pattern default");
    assert_eq!(js("const [a, ...h] = o;", "h"), 1, "rest");
    assert_eq!(js("[h] = o;", "h"), 1, "array assignment");
    assert_eq!(js("h++;", "h"), 1, "update");
    assert_eq!(js("o.h = x;", "h"), 0, "a member is no binding");
    assert_eq!(js("const k = o;", "h"), 0, "another name");
    assert_eq!(js("function inner() { h = 1; }", "h"), 0, "nested function");
    assert_eq!(js("const h = o; const h2 = x;", "h2"), 1, "control");

    let py = |body: &str, name: &str| {
        written(
            Language::Python,
            "py",
            &format!("def f(o, x):\n    {body}\n"),
            name,
        )
    };
    assert_eq!(py("h = o", "h"), 1);
    assert_eq!(py("a, *h = o", "h"), 1, "splat target");
    assert_eq!(py("h += 1", "h"), 1, "augmented");
    assert_eq!(py("(h := o)", "h"), 1, "walrus");
    assert_eq!(py("o.h = x", "h"), 0);

    let go = |body: &str, name: &str| {
        written(
            Language::Go,
            "go",
            &format!("package p\nfunc f(o int) {{\n{body}\n}}\n"),
            name,
        )
    };
    assert_eq!(go("h := o", "h"), 1, "short declaration");
    assert_eq!(go("var a, h = 1, 2", "h"), 1, "second name of a spec");
    assert_eq!(go("var h = o", "h"), 1);
    assert_eq!(go("a, h := 1, 2", "h"), 1, "expression list");
    assert_eq!(go("h++", "h"), 1, "increment");
    assert_eq!(go("_ = o", "h"), 0);
}

#[test]
fn writes_stop_at_the_cutoff() {
    let source = "function f(o) { const h = o; h(1); const h = 2; }\n";
    let case = Case::new(Language::JavaScript, "js", source);
    let parsed = case.parsed();
    let function = find(parsed, &|node| node.kind() == "function_declaration").unwrap();
    let all = writes(parsed, function, "h", source.len());
    assert_eq!(all.len(), 2);
    let before = writes(parsed, function, "h", source.find("h(1)").unwrap());
    assert_eq!(before.len(), 1);
    assert!(before[0].0.start_byte() < source.find("h(1)").unwrap());
}

fn spelled(language: Language, ext: &str, source: &str, call: &str) -> (String, Option<String>) {
    let case = Case::new(language, ext, source);
    let parsed = case.parsed();
    let call = call_text(parsed, call);
    let (selector, receiver) = spelling(parsed, call, owner(Some(call)));
    (selector.unwrap().to_owned(), receiver.map(str::to_owned))
}

#[test]
fn a_bare_callee_is_spelled_as_the_one_unconditional_alias_it_is_bound_to() {
    let js = |body: &str| {
        spelled(
            Language::JavaScript,
            "js",
            &format!("function f(g, k, c, t) {{ {body} }}\n"),
            "h(t)",
        )
    };
    assert_eq!(
        js("const h = g; h(t);"),
        ("g".into(), None),
        "control: alias"
    );
    assert_eq!(js("const h = g; const m = k; h(t);"), ("g".into(), None));
    assert_eq!(
        js("if (c) { h = g; } h(t);"),
        ("h".into(), None),
        "conditional write"
    );
    assert_eq!(
        js("let h = g; h = k; h(t);"),
        ("h".into(), None),
        "two writes"
    );
    assert_eq!(
        js("const h = g.x; h(t);"),
        ("h".into(), None),
        "value is no identifier"
    );
    assert_eq!(
        js("const h = c ? g : k; h(t);"),
        ("h".into(), None),
        "conditional value"
    );
    assert_eq!(js("let h; h(t);"), ("h".into(), None), "no value");
    assert_eq!(
        js("try { h = g; } catch (e) {} h(t);"),
        ("h".into(), None),
        "try write"
    );
    assert_eq!(js("h(t);"), ("h".into(), None), "no write");

    // A member call keeps its selector and receiver, whatever aliases exist.
    let member = spelled(
        Language::JavaScript,
        "js",
        "function f(g, o, t) { const h = g; o.h(t); }\n",
        "o.h(t)",
    );
    assert_eq!(member, ("h".into(), Some("o".into())));

    let py = |body: &str| {
        spelled(
            Language::Python,
            "py",
            &format!("def f(g, k, c, t):\n    {body}\n"),
            "h(t)",
        )
    };
    assert_eq!(py("h = g; h(t)"), ("g".into(), None), "python alias");
    assert_eq!(
        py("h = g,; h(t)"),
        ("g".into(), None),
        "one-element expression list"
    );
    assert_eq!(
        py("h, m = g, k; h(t)"),
        ("h".into(), None),
        "pair of values"
    );

    let go = |body: &str| {
        spelled(
            Language::Go,
            "go",
            &format!("package p\nfunc f(g func(int), k func(int), t int) {{\n{body}\n}}\n"),
            "h(t)",
        )
    };
    assert_eq!(
        go("h := g; h(t)"),
        ("g".into(), None),
        "go alias is a one-element list"
    );
    assert_eq!(
        go("h, m := g, k; h(t); _ = m"),
        ("h".into(), None),
        "two values"
    );
    assert_eq!(
        go("for h := range []int{1} { h(t) }"),
        ("h".into(), None),
        "loop-conditional"
    );
}

// ---------------------------------------------------------------------------------------------
// trace_proofs.rs: the real-rule and compare switches, and a refusal inside a walk
// ---------------------------------------------------------------------------------------------

fn os(text: &str) -> Option<std::ffi::OsString> {
    Some(text.into())
}

#[test]
fn an_unknown_real_rule_name_is_a_configuration_error_naming_the_variable() {
    let error = WalkProofs::from_parts_real(None, None, Some("i,bogus"), false).unwrap_err();
    assert!(
        error.starts_with("PRISM_TAINT_WALK_PROOFS_REAL: unknown rule `bogus`"),
        "{error}"
    );
    // Control: the implemented rules parse, with and without the unknown suffix.
    assert!(WalkProofs::from_parts_real(None, None, Some("i,ii"), false)
        .unwrap()
        .is_some());
    assert!(
        WalkProofs::from_parts_real(None, None, Some("i+unknown"), false)
            .unwrap()
            .is_some()
    );
}

#[test]
fn a_compare_value_other_than_one_or_zero_is_a_configuration_error() {
    for bad in ["2", "yes", "true", "-1"] {
        let error = WalkProofs::from_env(None, None, None, os(bad)).unwrap_err();
        assert_eq!(
            error,
            format!("PRISM_TAINT_WALK_PROOFS_COMPARE: expected 1 or 0, got `{bad}`")
        );
    }
    // Control: `1` turns compare on, `0`, blank and unset leave everything off.
    assert!(WalkProofs::from_env(None, None, None, os("1"))
        .unwrap()
        .is_some());
    for off in [os("0"), os(""), os("  "), None] {
        assert!(WalkProofs::from_env(None, None, None, off)
            .unwrap()
            .is_none());
    }
}

#[test]
fn a_refused_real_proof_cuts_the_crossing_in_a_walk() {
    let source = "function g(p) { return p; }\nfunction f(t) { g(g(t)); }\n";
    let walk = |real: Option<&str>| {
        let mut case = Case::new(Language::JavaScript, "js", source);
        let inner = source.find("g(t)").unwrap();
        // Model a gate that has nominated the inner site only.
        case.cpg
            .call_graph
            .callers
            .get_mut("g")
            .unwrap()
            .retain(|site| site.start_byte == inner);
        let from = case.at(source, "g(t)", 2, VarAccess::Use);
        let to = case.at(source, "g(p)", 2, VarAccess::Def);
        let proofs = real.map(|real| {
            WalkProofs::from_parts_real(None, None, Some(real), false)
                .unwrap()
                .unwrap()
        });
        let view = AstOrderView::new(&case.cpg, &case.files);
        let trace = with_walk_proofs(proofs, || case.cpg.taint_trace_nodes(&[from], Some(&view)));
        let crossed = trace.frontier_by_root[&from].contains(&to);
        let cut = trace
            .boundary
            .iter()
            .filter(|hop| hop.from == from && hop.to == to)
            .map(|hop| hop.reason)
            .collect::<Vec<_>>();
        (crossed, cut)
    };
    // Controls: with no real rule, or with the rule that proves the inner actual, it crosses.
    assert_eq!(walk(None), (true, vec![]));
    assert_eq!(walk(Some("i")), (true, vec![]));
    // Rule (ii) refuses it: a wider call of the same spelling can emit the same edge.
    assert_eq!(walk(Some("ii")), (false, vec![LossReason::AmbiguousSite]));
}

#[test]
fn a_call_outside_every_function_is_spelled_by_its_own_text() {
    let source = "const h = g;\nh(t);\nfunction f(g, t) { const h = g; h(t); }\n";
    let case = Case::new(Language::JavaScript, "js", source);
    let parsed = case.parsed();
    let top = find(parsed, &|node| {
        is_call(node) && node.start_byte() == source.find("h(t);").unwrap()
    })
    .unwrap();
    assert_eq!(owner(Some(top)), None);
    assert_eq!(spelling(parsed, top, None), (Some("h"), None));
    // Control: inside the function the same alias is followed.
    let inner = find(parsed, &|node| {
        is_call(node) && node.start_byte() == source.rfind("h(t)").unwrap()
    })
    .unwrap();
    assert_eq!(
        spelling(parsed, inner, owner(Some(inner))),
        (Some("g"), None)
    );
}
