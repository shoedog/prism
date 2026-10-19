//! Syntax identity, demand reuse and build isolation independent of graph indices.
use super::*;
use crate::languages::Language;

fn id(file: &str, name: &str, start_line: usize, end_line: usize) -> FunctionId {
    FunctionId {
        file: file.into(),
        name: name.into(),
        start_line,
        end_line,
    }
}

fn facts(cell: &FactCell) -> String {
    format!("{:?}", cell.get().unwrap())
}

#[test]
fn first_match_full_identity_and_negative_results() {
    let files = super::super::callee_cases::parsed(
        "js",
        Language::JavaScript,
        "function same(x) { return x; } function same(x) { return 99; }\nfunction empty() {}\n",
    );
    let cache = ReturnCache::new(&files);
    let first = id("case.js", "same", 1, 1);
    let cell = cache.get(&first);
    assert_eq!(cell.get().unwrap().as_ref().unwrap()[0].values[0].text, "x");
    assert_eq!(
        facts(&cell),
        format!(
            "{:?}",
            super::super::callee_legacy::returns(&files["case.js"], &first)
        )
    );
    for other in [
        id("case.js", "same", 1, 99),
        id("case.js", "empty", 2, 2),
        id("case.js", "missing", 1, 1),
        id("missing.js", "same", 1, 1),
        id("case.js", "same", 2, 2),
    ] {
        let value = cache.get(&other);
        assert!(
            !Arc::ptr_eq(&cell, &value),
            "complete FunctionId is the key"
        );
        assert!(
            Arc::ptr_eq(&value, &cache.get(&other)),
            "negative results must be retained"
        );
        if other.file == "case.js" {
            assert_eq!(
                facts(&value),
                format!(
                    "{:?}",
                    super::super::callee_legacy::returns(&files["case.js"], &other)
                )
            );
        } else {
            assert!(value.get().unwrap().is_none());
        }
    }
    assert!(cache
        .get(&id("case.js", "empty", 2, 2))
        .get()
        .unwrap()
        .as_ref()
        .unwrap()
        .is_empty());
    // No return inventory was populated for a function until its first demand.
    assert_eq!(cache.entries.lock().unwrap().len(), 6);
}

#[test]
fn syntax_inventories_match_legacy_all_languages() {
    for (ext, language, source) in super::super::callee_cases::languages() {
        let files = super::super::callee_cases::parsed(ext, language, source);
        let cache = ReturnCache::new(&files);
        for (file, parsed) in &files {
            for node in parsed.all_functions() {
                let Some(name) = parsed.language.function_name(&node) else {
                    continue;
                };
                let (start, end) = parsed.node_line_range(&node);
                let owner = id(file, parsed.node_text(&name), start, end);
                assert_eq!(
                    facts(&cache.get(&owner)),
                    format!("{:?}", super::super::callee_legacy::returns(parsed, &owner)),
                    "{language:?}: {owner:?}"
                );
            }
        }
    }
}

#[test]
fn concurrent_demands_and_later_builds_are_isolated() {
    let files = super::super::callee_cases::parsed(
        "js",
        Language::JavaScript,
        "function helper(x) { if (x) { return x; } return 0; }\nfunction unused() { return 9; }\n",
    );
    let owner = id("case.js", "helper", 1, 1);
    let cache = ReturnCache::new(&files);
    assert!(cache.entries.lock().unwrap().is_empty());
    std::thread::scope(|scope| {
        let threads: Vec<_> = (0..16).map(|_| scope.spawn(|| cache.get(&owner))).collect();
        let first = cache.get(&owner);
        for thread in threads {
            assert!(Arc::ptr_eq(&first, &thread.join().unwrap()));
        }
    });
    assert_eq!(cache.entries.lock().unwrap().len(), 1);
    let before = facts(&cache.get(&owner));
    let later = super::super::callee_cases::parsed(
        "js",
        Language::JavaScript,
        "function helper(x) { return 123; }\n",
    );
    let other = ReturnCache::new(&later);
    assert_ne!(before, facts(&other.get(&owner)));
    assert_eq!(before, facts(&cache.get(&owner)));
}

#[test]
#[ignore = "captured main demands; explicit scratch/count environment required"]
fn shared_return_demand_replay() {
    assert!(std::env::var_os("CALLEE_COUNT_REPLAY").is_some());
    let scratch = std::env::var("CALLEE_SCRATCH").unwrap();
    for name in ["xlike-1", "xlike-8", "xlike-32", "checker"] {
        let files = if name == "checker" {
            let source =
                std::fs::read_to_string(format!("{scratch}/fixtures-c1/checker/checker.ts"))
                    .unwrap();
            BTreeMap::from([(
                "checker.ts".into(),
                ParsedFile::parse("checker.ts", &source, Language::TypeScript).unwrap(),
            )])
        } else {
            super::super::callee_census_tests::synthetic(
                name.strip_prefix("xlike-").unwrap().parse().unwrap(),
            )
        };
        let before: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(format!("{scratch}/census-{name}.json")).unwrap(),
        )
        .unwrap();
        let mut owners: BTreeMap<String, BTreeSet<FunctionId>> = BTreeMap::new();
        for (file, parsed) in &files {
            for node in crate::anon_owner::owner_nodes(parsed) {
                if let Some(owner) = crate::anon_owner::emitted_identity(parsed, &node, file) {
                    owners
                        .entry(format!(
                            "{}:{}:{}",
                            owner.file, owner.start_line, owner.name
                        ))
                        .or_default()
                        .insert(owner);
                }
            }
        }
        super::super::callee_cost_tests::take_counts();
        let cache = ReturnCache::new(&files);
        for (key, demand) in before["returns_per_callee"].as_object().unwrap() {
            let ids = &owners[key];
            assert_eq!(ids.len(), 1, "saved key must identify a complete owner");
            for _ in 0..demand[0].as_u64().unwrap() {
                let _ = cache.get(ids.first().unwrap());
            }
        }
        let counts = super::super::callee_cost_tests::take_counts();
        let total = |kind: &str| {
            counts
                .iter()
                .filter(|((k, _), _)| k == kind)
                .map(|(_, n)| *n)
                .sum::<usize>()
        };
        assert_eq!(
            total("returns") as u64,
            before["return_distinct"].as_u64().unwrap()
        );
        assert_eq!(
            total("return-visits") as u64,
            before["return_distinct_nodes"].as_u64().unwrap()
        );
        assert_eq!(total("return-lookups"), 1);
        assert!(counts
            .iter()
            .filter(|((k, _), _)| k == "returns")
            .all(|(_, n)| *n == 1));
        let result = serde_json::json!({"builds_before":before["return_builds"],"builds_after":total("returns"),"visits_before":before["return_nodes_visited"],"visits_after":total("return-visits"),"all_functions_before":before["all_functions_5c"],"all_functions_after":total("return-lookups"),"per_callee":counts.iter().filter(|((k,_),_)| k == "returns").map(|((_,id),n)| (format!("{}:{}:{}",id.file,id.start_line,id.name),n)).collect::<BTreeMap<_,_>>()});
        std::fs::write(
            format!("{scratch}/after-returns-{name}.json"),
            serde_json::to_string_pretty(&result).unwrap(),
        )
        .unwrap();
        eprintln!("{name}: {result}");
    }
}
