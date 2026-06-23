use crate::common::*;
use prism::call_graph::CallGraph;
use prism::resolution::{ResolutionConfidence, ResolutionKind, ResolutionOutcome};

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

#[test]
fn cpp_template_wrapper_inner_capture_is_unchanged() {
    let source =
        "template <typename T>\nT id(T value) { return value; }\n\nint run() {\n    return id(1);\n}\n";
    let parsed = ParsedFile::parse("tmpl.cpp", source, Language::Cpp).unwrap();
    let id_functions: Vec<_> = parsed
        .all_functions()
        .into_iter()
        .filter(|node| {
            parsed
                .language
                .function_name(node)
                .map(|name| parsed.node_text(&name) == "id")
                .unwrap_or(false)
        })
        .collect();

    assert_eq!(
        id_functions.len(),
        2,
        "C++ template_declaration/function_definition double capture is out of scope"
    );
    assert!(id_functions
        .iter()
        .any(|node| node.kind() == "template_declaration"));
    assert!(id_functions
        .iter()
        .any(|node| node.kind() == "function_definition"));

    let cg = CallGraph::build(&files(&[("tmpl.cpp", source)]));
    let out = resolve_call(&cg, "tmpl.cpp", "run", "id");
    assert_eq!(out.resolved.len(), 2);
    assert!(out
        .resolved
        .iter()
        .all(|callee| callee.confidence == ResolutionConfidence::Exact));
    assert!(out
        .resolved
        .iter()
        .all(|callee| callee.kind == ResolutionKind::LocalDef));
}
