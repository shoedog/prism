//! PR256 guard corpus; the direct imported object-alias case was promoted by its successor.
use prism::{
    ast::ParsedFile,
    call_graph::CallGraph,
    languages::Language,
    resolution::{ResolutionConfidence, ResolutionKind},
};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn imported_props_identity_audit_full_and_subset_guards() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../docs/eval/receiver-closure/imported-props-identity-fixtures.json"
    ))
    .unwrap();
    let cases = corpus["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 24, "do not silently lose audit obligations");
    let mut names = BTreeSet::new();
    for case in cases {
        let name = case["id"].as_str().unwrap();
        assert!(names.insert(name));
        for language in [Language::TypeScript, Language::Tsx] {
            let files: BTreeMap<_, _> = case["files"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(p, s)| {
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
                let sites: Vec<_> = graph
                    .calls
                    .iter()
                    .filter(|(f, _)| f.file == "app.ts" && f.name == "run")
                    .flat_map(|(_, sites)| sites)
                    .filter(|site| site.callee_name == "m")
                    .collect();
                assert_eq!(
                    sites.len(),
                    1,
                    "{name}/{language:?}/{mode}: call must exist"
                );
                let exact: Vec<_> = graph
                    .resolve_call_site(sites[0])
                    .into_iter()
                    .filter(|edge| edge.confidence == ResolutionConfidence::Exact)
                    .collect();
                let owner = case["exact_owner"].as_str();
                assert_eq!(
                    exact.len(),
                    usize::from(owner.is_some()),
                    "{name}/{language:?}/{mode}: {exact:?}"
                );
                if let Some(owner) = owner {
                    assert_eq!(exact[0].target.file, owner, "{name}/{mode}");
                    assert_eq!(exact[0].target.start_line, 2, "{name}/{mode}");
                    assert_eq!(exact[0].kind, ResolutionKind::TypedParam);
                }
            }
        }
    }
}
