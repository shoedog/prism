//! Baseline characterization, NOT a new proof constructor or refusal implementation.
use prism::{
    ast::ParsedFile, call_graph::CallGraph, languages::Language, resolution::ResolutionConfidence,
};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn executable_owner_design_characterizes_full_and_subset_without_authority() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../docs/eval/receiver-closure/executable-owner-fixtures.json"
    ))
    .unwrap();
    let cases = corpus["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 27);
    let mut names = BTreeSet::new();
    let mut failures = Vec::new();
    for case in cases {
        let id = case["id"].as_str().unwrap();
        assert!(names.insert(id));
        let mut sources = corpus["files"].as_object().unwrap().clone();
        if let Some(overrides) = case["files"].as_object() {
            sources.extend(overrides.clone());
        }
        for name in case["remove"].as_array().into_iter().flatten() {
            sources.remove(name.as_str().unwrap());
        }
        if let Some(replace) = case["replace_app"].as_array() {
            let app = sources["src/app.ts"].as_str().unwrap();
            let before = replace[0].as_str().unwrap();
            assert_eq!(app.matches(before).count(), 1);
            sources.insert(
                "src/app.ts".into(),
                app.replace(before, replace[1].as_str().unwrap()).into(),
            );
        }
        for language in [Language::TypeScript, Language::Tsx] {
            let app = if language == Language::Tsx {
                "src/app.tsx"
            } else {
                "src/app.ts"
            };
            let files: BTreeMap<_, _> = sources
                .iter()
                .map(|(p, s)| {
                    let p = if p == "src/app.ts" { app } else { p };
                    let lang = if p.ends_with(".tsx") {
                        Language::Tsx
                    } else {
                        Language::TypeScript
                    };
                    (
                        p.to_owned(),
                        ParsedFile::parse(p, s.as_str().unwrap(), lang).unwrap(),
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
                let calls: Vec<_> = graph
                    .calls
                    .values()
                    .flatten()
                    .filter(|s| {
                        s.caller.file == app
                            && s.caller.name == case["caller"].as_str().unwrap_or("run")
                            && s.callee_name == "m"
                    })
                    .collect();
                let owner =
                    case["baseline_exact"]
                        .as_str()
                        .map(|p| if p == "src/app.ts" { app } else { p });
                let edges: Vec<_> = calls
                    .iter()
                    .flat_map(|s| graph.resolve_call_site(s))
                    .filter(|e| e.confidence == ResolutionConfidence::Exact)
                    .collect();
                if calls.len() != 1
                    || edges.len() != usize::from(owner.is_some())
                    || owner
                        .is_some_and(|owner| edges.first().is_none_or(|e| e.target.file != owner))
                {
                    failures.push(format!(
                        "{id}/{language:?}/{mode}: calls={}, edges={edges:?}",
                        calls.len()
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "complete fixture failure population: {failures:#?}"
    );
}
