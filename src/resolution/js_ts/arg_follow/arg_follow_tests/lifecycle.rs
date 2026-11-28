use super::*;

#[test]
fn diagnostic_records_join_facts_and_refusals() {
    let build = || {
        direct_case(
            "function holder(cb) { cb('mixed'); }",
            "holder(stale); holder(external());",
            Language::JavaScript,
        )
    };
    let (captured, lines) = diagnostic::capture::run(build);
    let ordinary = build();
    assert_eq!(captured.index, ordinary.index);
    assert_eq!(captured.graph.calls, ordinary.graph.calls);
    captured.targets("cb('mixed')", &["stale"]);
    let start = captured.source.find("cb('mixed')").unwrap();
    let records: Vec<serde_json::Value> = lines
        .iter()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let mixed: Vec<_> = records
        .iter()
        .filter(|record| record["site"]["span"][0] == start)
        .collect();
    assert_eq!(mixed.len(), 2);
    let accepted = mixed
        .iter()
        .find(|record| record["disposition"] == "admitted")
        .unwrap();
    assert_eq!(accepted["target"]["id"]["name"], "stale");
    assert_eq!(
        accepted["caller"]["span"][0],
        captured.source.find("holder(stale)").unwrap()
    );
    let refused = mixed
        .iter()
        .find(|record| record["disposition"] == "refused")
        .unwrap();
    assert_eq!(refused["reason"], "opaque_terminal");
    assert_eq!(
        refused["caller"]["span"][0],
        captured.source.find("holder(external())").unwrap()
    );
    assert_eq!(captured.index.refusal_count("opaque_terminal"), 1);
}

#[test]
fn diagnostic_capture_does_not_leak_between_builds() {
    use super::diagnostic::capture;
    let build = || {
        direct_case(
            "function holder(cb) { cb('kept'); }",
            "holder(stale);",
            Language::JavaScript,
        )
    };
    let (expired, lines) = capture::run(|| {
        let token = capture::current();
        std::thread::scope(|scope| {
            let unconnected = scope.spawn(build);
            build();
            unconnected.join().unwrap();
        });
        token
    });
    assert_eq!(lines.len(), 2);
    let (_, lines) = capture::run(|| {
        capture::inherit(expired, || {
            assert!(crate::build_pool::install(|| capture::current().is_none()));
            build();
        });
        build();
    });
    assert_eq!(lines.len(), 2);
}

#[test]
fn skeleton_and_non_js_graph_have_empty_argument_facts() {
    let js = "function invoke(cb) { cb('param'); }\nfunction handler(item) {}\ninvoke(handler);";
    let files = BTreeMap::from([(
        "m.js".into(),
        ParsedFile::parse("m.js", js, Language::JavaScript).unwrap(),
    )]);
    let direct = CallGraph::build_direct_subset(&files, &BTreeSet::from(["m.js".to_string()]));
    assert_eq!(direct.js_anonymous.arg_follow, ArgIndex::default());
    assert_eq!(
        CallGraph::build_skeleton(&files).js_anonymous.arg_follow,
        ArgIndex::default()
    );
    assert_ne!(
        CallGraph::build(&files).js_anonymous.arg_follow,
        ArgIndex::default()
    );
    let go = "package control\nfunc invoke() { handler() }\nfunc handler() {}";
    let files = BTreeMap::from([(
        "m.go".into(),
        ParsedFile::parse("m.go", go, Language::Go).unwrap(),
    )]);
    let graph = CallGraph::build(&files);
    assert_eq!(graph.js_anonymous.arg_follow, ArgIndex::default());
    assert!(graph
        .calls
        .values()
        .flatten()
        .any(|site| site.callee_name == "handler"));
}

#[test]
fn index_partial_hit_equals_cold() {
    use crate::cpg::CodePropertyGraph;
    use crate::cpg_cache::{compute_file_hashes, load_cache, save_cache, CacheResult, SaveOutcome};

    let temp = tempfile::tempdir().unwrap();
    let mut sources = BTreeMap::from([
        (
            "holder.js".to_string(),
            "export function holder(cb) { cb('bound'); }".to_string(),
        ),
        (
            "caller-base.js".to_string(),
            "import { holder } from './holder';\n".to_string(),
        ),
    ]);
    let files_of = |sources: &BTreeMap<String, String>| -> BTreeMap<String, ParsedFile> {
        sources
            .iter()
            .map(|(name, source)| {
                (
                    name.clone(),
                    ParsedFile::parse(name, source, Language::JavaScript).unwrap(),
                )
            })
            .collect()
    };
    for i in 0..8 {
        sources
            .get_mut("caller-base.js")
            .unwrap()
            .push_str(&format!(
                "function target{i}(value) {{}}\nholder(target{i});\n"
            ));
    }
    let ninth = "import { holder } from './holder';\nfunction added(value) {}\nholder(added);";
    sources.insert("caller-ninth.js".into(), String::new());
    let initial = CodePropertyGraph::build(&files_of(&sources));
    let site = initial
        .call_graph
        .calls
        .values()
        .flatten()
        .find(|site| site.callee_name == "cb")
        .unwrap()
        .clone();
    let facts = |graph: &CodePropertyGraph| {
        bincode::serialize(&graph.call_graph.js_anonymous.arg_follow).unwrap()
    };
    assert!(
        matches!(initial.call_graph.js_anonymous.arg_follow.lookup(&site), Some(ArgOutcome::Targets(targets)) if targets.len() == 8)
    );
    let hashes = compute_file_hashes(&sources);
    assert_eq!(
        save_cache(&initial, &hashes, false, temp.path()).unwrap(),
        SaveOutcome::Saved
    );
    let CacheResult::Hit(restored) = load_cache(&hashes, false, temp.path()) else {
        panic!("expected full hit");
    };
    assert_eq!(facts(&initial), facts(&restored));
    let mut previous = restored;
    for (caller, expected) in [(ninth, 9), ("", 8), (ninth, 9)] {
        sources.insert("caller-ninth.js".into(), caller.into());
        let hashes = compute_file_hashes(&sources);
        let files = files_of(&sources);
        let CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
        } = load_cache(&hashes, false, temp.path())
        else {
            panic!("expected partial hit after caller-only change");
        };
        assert_eq!(
            changed_files,
            BTreeSet::from(["caller-ninth.js".to_string()])
        );
        let partial = CodePropertyGraph::build_incremental(
            cached_call_graph,
            cached_dfg,
            &changed_files,
            &files,
            None,
        );
        let cold = CodePropertyGraph::build(&files);
        assert_eq!(facts(&partial), facts(&cold));
        assert_eq!(partial.call_graph.calls, cold.call_graph.calls);
        let outcome = partial.call_graph.js_anonymous.arg_follow.lookup(&site);
        if expected == 9 {
            assert_eq!(outcome, Some(&ArgOutcome::Fanout(9)));
        } else {
            assert!(matches!(outcome, Some(ArgOutcome::Targets(targets)) if targets.len() == 8));
        }
        assert_eq!(
            save_cache(&partial, &hashes, false, temp.path()).unwrap(),
            SaveOutcome::Saved
        );
        previous = partial;
    }
    sources.remove("caller-ninth.js");
    let hashes = compute_file_hashes(&sources);
    assert!(matches!(
        load_cache(&hashes, false, temp.path()),
        CacheResult::Miss
    ));
    let files = files_of(&sources);
    let deleted = CodePropertyGraph::build_incremental(
        previous.call_graph,
        previous.dfg,
        &BTreeSet::from(["caller-ninth.js".to_string()]),
        &files,
        None,
    );
    assert_eq!(facts(&deleted), facts(&CodePropertyGraph::build(&files)));
    assert!(
        matches!(deleted.call_graph.js_anonymous.arg_follow.lookup(&site), Some(ArgOutcome::Targets(targets)) if targets.len() == 8)
    );
}
