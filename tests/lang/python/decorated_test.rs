use crate::common::*;
use petgraph::visit::EdgeRef;
use prism::call_graph::CallGraph;
use prism::cfg;
use prism::cpg::CpgEdge;
use prism::resolution::{ResolutionConfidence, ResolutionKind, ResolutionOutcome};
use prism::slice::SliceResult;
use std::fs;

fn files(pairs: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    pairs
        .iter()
        .map(|(path, source)| {
            let lang = Language::from_path(path).expect("known extension");
            (
                (*path).to_string(),
                ParsedFile::parse(path, source, lang).expect("parse"),
            )
        })
        .collect()
}

fn resolve_call<'a>(
    cg: &'a CallGraph,
    caller_file: &str,
    caller_name: &str,
    callee: &str,
) -> ResolutionOutcome<'a> {
    let caller = cg
        .functions
        .get(caller_name)
        .and_then(|ids| ids.iter().find(|id| id.file == caller_file))
        .expect("caller function");
    let site = cg
        .calls
        .get(caller)
        .and_then(|sites| sites.iter().find(|site| site.callee_name == callee))
        .expect("call site");
    cg.resolve_call_site_full(site)
}

fn control_flow_pairs(cpg: &CodePropertyGraph) -> Vec<(usize, usize)> {
    cpg.graph
        .edge_references()
        .filter(|edge| matches!(edge.weight(), CpgEdge::ControlFlow))
        .map(|edge| (edge.source().index(), edge.target().index()))
        .collect()
}

fn run_contract_delta(
    old_source: &str,
    new_source: &str,
    diff_lines: BTreeSet<usize>,
) -> SliceResult {
    let path = "svc.py";
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join(path), old_source).unwrap();

    let mut parsed_files = BTreeMap::new();
    parsed_files.insert(
        path.to_string(),
        ParsedFile::parse(path, new_source, Language::Python).unwrap(),
    );
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines,
        }],
    };

    algorithms::contract_slice::slice_delta(&parsed_files, &diff, tmp.path()).unwrap()
}

fn run_taint(source: &str, diff_lines: BTreeSet<usize>) -> SliceResult {
    let path = "svc.py";
    let mut parsed_files = BTreeMap::new();
    parsed_files.insert(
        path.to_string(),
        ParsedFile::parse(path, source, Language::Python).unwrap(),
    );
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines,
        }],
    };

    algorithms::run_slicing_compat(
        &parsed_files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap()
}

#[test]
fn decorated_free_function_localdef_resolves_single_exact() {
    let cg = CallGraph::build(&files(&[(
        "svc.py",
        "@deco\ndef f():\n    return 1\n\ndef g():\n    return f()\n",
    )]));

    assert_eq!(
        cg.functions.get("f").map(Vec::len),
        Some(1),
        "decorated free function should have one canonical FunctionId"
    );

    let out = resolve_call(&cg, "svc.py", "g", "f");
    assert_eq!(out.resolved.len(), 1, "expected one resolved callee");
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out.resolved[0].kind, ResolutionKind::LocalDef);
}

#[test]
fn decorated_method_self_and_class_calls_resolve_exact() {
    let cg = CallGraph::build(&files(&[(
        "svc.py",
        "class Cls:\n    @deco\n    def m(self):\n        return 1\n    def buy(self):\n        return self.m()\n\ndef call_cls():\n    return Cls.m(None)\n",
    )]));

    assert_eq!(
        cg.functions.get("m").map(Vec::len),
        Some(1),
        "decorated method should have one canonical FunctionId"
    );

    let self_out = resolve_call(&cg, "svc.py", "buy", "m");
    assert_eq!(self_out.resolved.len(), 1);
    assert_eq!(self_out.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(self_out.resolved[0].kind, ResolutionKind::SelfReceiver);

    let class_out = resolve_call(&cg, "svc.py", "call_cls", "m");
    assert_eq!(class_out.resolved.len(), 1);
    assert_eq!(
        class_out.resolved[0].confidence,
        ResolutionConfidence::Exact
    );
    assert_eq!(class_out.resolved[0].kind, ResolutionKind::QualifierOwner);
}

#[test]
fn decorated_function_cfg_count_matches_undecorated_and_has_no_parallel_cpg_edges() {
    let undecorated = "def f():\n    x = 1\n    y = x\n    return y\n";
    let decorated = "@deco\ndef f():\n    x = 1\n    y = x\n    return y\n";
    let plain = ParsedFile::parse("plain.py", undecorated, Language::Python).unwrap();
    let wrapped = ParsedFile::parse("wrapped.py", decorated, Language::Python).unwrap();

    assert_eq!(
        cfg::build_cfg_edges(&wrapped).len(),
        cfg::build_cfg_edges(&plain).len()
    );

    let cpg = build_cpg("wrapped.py", decorated, Language::Python);
    let raw_pairs = control_flow_pairs(&cpg);
    let unique_pairs: BTreeSet<_> = raw_pairs.iter().copied().collect();
    assert_eq!(raw_pairs.len(), unique_pairs.len());
}

#[test]
fn decorated_function_contract_delta_sees_wrapper_body() {
    let old_source =
        "@deco\ndef f(x):\n    if x is None:\n        raise ValueError('x')\n    return 1\n";
    let new_source =
        "@deco\ndef f(x):\n    if x is None:\n        raise ValueError('x')\n    return None\n";
    let result = run_contract_delta(old_source, new_source, BTreeSet::from([5]));

    assert!(
        result
            .findings
            .iter()
            .any(|finding| finding.category.as_deref() == Some("contract_postcondition_weakened")),
        "decorated wrapper should expose postconditions to contract delta, got {:?}",
        result.findings
    );
}

#[test]
fn decorated_flask_taint_propagates_past_synthetic_assignment_seed() {
    let source = r#"from flask import Flask, request
from django.utils.html import format_html
from django.utils.safestring import mark_safe

app = Flask(__name__)

@app.route("/profile")
def profile():
    fmt = request.args.get("fmt")
    safe_html = format_html(fmt, "value")
    return mark_safe(safe_html)
"#;
    let result = run_taint(source, BTreeSet::from([1]));

    assert!(
        result
            .findings
            .iter()
            .any(|finding| finding.category.as_deref() == Some("taint_sink") && finding.line == 11),
        "decorated wrapper seed should reach mark_safe, got {:?}",
        result.findings
    );
}

#[test]
fn enclosing_function_inside_decorated_function_remains_inner_definition() {
    let parsed = ParsedFile::parse(
        "svc.py",
        "@deco\ndef f():\n    value = 1\n    return value\n",
        Language::Python,
    )
    .unwrap();
    let func = parsed.enclosing_function(3).expect("enclosing function");

    assert_eq!(func.kind(), "function_definition");
    let name = parsed.language.function_name(&func).unwrap();
    assert_eq!(parsed.node_text(&name), "f");
}
