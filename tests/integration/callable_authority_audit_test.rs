//! External callable observations remain non-authorizing until a separately approved consumer.
use prism::{
    ast::ParsedFile, call_graph::CallGraph, languages::Language, resolution::ResolutionConfidence,
};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn callable_authority_audit_retains_guards_and_independent_controls() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../docs/eval/receiver-closure/callable-authority-fixtures.json"
    ))
    .unwrap();
    let cases = corpus["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 20);
    let mut names = BTreeSet::new();
    let mut failures = Vec::new();
    for case in cases {
        let id = case["id"].as_str().unwrap();
        assert!(names.insert(id));
        for language in [Language::TypeScript, Language::Tsx] {
            let files: BTreeMap<_, _> = case["files"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(p, s)| {
                    let language = if p.ends_with(".js") {
                        Language::JavaScript
                    } else {
                        language
                    };
                    (
                        p.clone(),
                        ParsedFile::parse(p, s.as_str().unwrap(), language).unwrap(),
                    )
                })
                .collect();
            for (mode, graph) in [
                ("full", CallGraph::build(&files)),
                (
                    "subset",
                    CallGraph::build_direct_subset(&files, &files.keys().cloned().collect()),
                ),
            ] {
                if std::panic::catch_unwind(|| {
                    let call_file = case["call_file"].as_str().unwrap_or("app.ts");
                    let calls: Vec<_> = graph
                        .calls
                        .values()
                        .flatten()
                        .filter(|s| s.caller.file == call_file && s.callee_name == "m")
                        .collect();
                    assert_eq!(calls.len(), 1, "{id}: call must exist");
                    let edges: Vec<_> = graph
                        .resolve_call_site(calls[0])
                        .into_iter()
                        .filter(|e| e.confidence == ResolutionConfidence::Exact)
                        .collect();
                    let owner = case["prism_exact_owner"].as_str();
                    assert_eq!(edges.len(), usize::from(owner.is_some()), "{id}: {edges:?}");
                    if let Some(owner) = owner {
                        assert_eq!(edges[0].target.file, owner);
                    }
                })
                .is_err()
                {
                    failures.push((id, language, mode));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "complete audit population: {failures:?}"
    );
}
