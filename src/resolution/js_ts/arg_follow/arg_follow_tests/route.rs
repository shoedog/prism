//! The E2 route: what the resolver does with the index (lane 9 slice E2).

use super::*;
use crate::resolution::{DropReason, ResolutionKind, ResolutionOutcome, ResolvedCallee};

fn module(source: &str) -> Fixture {
    Fixture::build(format!("{source}\nexport {{}};\n"), Language::JavaScript)
}

fn full<'a>(fixture: &'a Fixture, invocation: &str) -> ResolutionOutcome<'a> {
    fixture
        .graph
        .resolve_call_site_full(fixture.site(invocation).expect("site"))
}

#[test]
fn indexed_targets_resolve_as_name_only_argument_follow() {
    let fixture = module(
        "function run(cb) { cb(1); }\nfunction handler(e) {}\nfunction main() { run(handler); }",
    );
    let outcome = full(&fixture, "cb(1)");
    let [one] = outcome.resolved.as_slice() else {
        panic!("{:?}", outcome.drop);
    };
    assert_eq!(one.target.name, "handler");
    assert_eq!(
        one.confidence,
        crate::resolution::ResolutionConfidence::NameOnly
    );
    assert_eq!(one.kind, ResolutionKind::ArgumentFollow);
    assert_eq!(one.kind.as_str(), "argument_follow");
    assert!(!outcome.via_alias_route && outcome.anonymous_certificate.is_none());
    assert!(outcome.drop.is_none());
}

#[test]
fn a_refused_site_keeps_its_original_drop() {
    // The caller passes an opaque call result: the index says `Empty`, so the
    // resolver answers exactly what it answered before the route existed.
    let fixture = module("function run(cb) { cb(1); }\nfunction main() { run(external()); }");
    assert!(matches!(
        fixture.outcome("cb(1)"),
        Some(ArgOutcome::Empty(_))
    ));
    let outcome = full(&fixture, "cb(1)");
    assert!(outcome.resolved.is_empty());
    assert_eq!(outcome.drop, Some(DropReason::UnknownName));
}

#[test]
fn only_an_empty_unknown_name_or_unproven_binding_outcome_changes() {
    let fixture = module(
        "function run(cb) { cb(1); }\nfunction handler(e) {}\nfunction main() { run(handler); }",
    );
    let site = fixture.site("cb(1)").unwrap();
    let target = fixture.graph.functions["handler"].first().unwrap();
    // An existing resolution survives untouched, whatever the index says.
    let exact = ResolutionOutcome::hit(vec![ResolvedCallee {
        target,
        confidence: crate::resolution::ResolutionConfidence::Exact,
        kind: ResolutionKind::LocalDef,
    }]);
    let kept = fixture.graph.js_ts_argument_follow(site, exact);
    assert_eq!(kept.resolved.len(), 1);
    assert_eq!(kept.resolved[0].kind, ResolutionKind::LocalDef);
    // Any other drop reason is not an argument-follow candidate.
    for reason in [
        DropReason::ExternalReceiver,
        DropReason::ImportExternal,
        DropReason::MultiOwnerCollision,
        DropReason::JsxIntrinsic,
    ] {
        let kept = fixture
            .graph
            .js_ts_argument_follow(site, ResolutionOutcome::dropped(reason));
        assert!(kept.resolved.is_empty(), "{reason:?}");
        assert_eq!(kept.drop, Some(reason));
    }
    // Both admitted reasons are replaced.
    for reason in [DropReason::UnknownName, DropReason::LocalBindingUnproven] {
        let changed = fixture
            .graph
            .js_ts_argument_follow(site, ResolutionOutcome::dropped(reason));
        assert_eq!(changed.resolved.len(), 1, "{reason:?}");
    }
    // A site the index does not hold is unchanged.
    let other = fixture
        .graph
        .calls
        .values()
        .flatten()
        .find(|s| s.callee_name == "run");
    let other = other.unwrap();
    assert!(fixture.index.lookup(other).is_none());
    let kept = fixture
        .graph
        .js_ts_argument_follow(other, ResolutionOutcome::dropped(DropReason::UnknownName));
    assert_eq!(kept.drop, Some(DropReason::UnknownName));
}

fn fan_in(count: usize) -> Fixture {
    let mut source = String::from("function run(cb) { cb(1); }\n");
    for i in 0..count {
        source.push_str(&format!("function t{i}(e) {{}}\nrun(t{i});\n"));
    }
    module(&source)
}

#[test]
fn eight_distinct_targets_resolve_and_nine_drop_as_arg_fanout() {
    let eight = fan_in(8);
    let outcome = full(&eight, "cb(1)");
    assert_eq!(outcome.resolved.len(), 8);
    assert!(outcome
        .resolved
        .iter()
        .all(|r| r.kind == ResolutionKind::ArgumentFollow));
    let nine = fan_in(9);
    let outcome = full(&nine, "cb(1)");
    assert!(outcome.resolved.is_empty());
    assert_eq!(outcome.drop, Some(DropReason::ArgFanout));
}

#[test]
fn call_stats_counts_the_fanout_drop_and_the_resolution_kind() {
    let nine = fan_in(9);
    let stats = crate::navigation::queries::call_stats(&nine.graph);
    assert_eq!(stats["dropped_arg_fanout"], 1);
    let eight = fan_in(8);
    let stats = crate::navigation::queries::call_stats(&eight.graph);
    assert_eq!(stats["dropped_arg_fanout"], 0);
}

#[test]
fn no_route_leaves_a_script_global_terminal_unresolved() {
    // A classic-script global function is a writable global property binding, so
    // the accept-list refuses it as a terminal and the route changes nothing.
    let fixture = Fixture::build(
        "function run(cb) { cb(1); }\nfunction handler(e) {}\nfunction main() { run(handler); }"
            .into(),
        Language::JavaScript,
    );
    let outcome = full(&fixture, "cb(1)");
    assert!(outcome.resolved.is_empty());
    assert_eq!(outcome.drop, Some(DropReason::UnknownName));
}

/// Complete site outcomes and labelled data-flow edges of a graph, as strings.
fn observed(cpg: &crate::cpg::CodePropertyGraph) -> (Vec<String>, Vec<String>) {
    use petgraph::visit::EdgeRef;
    let mut sites: Vec<String> = cpg
        .call_graph
        .calls
        .values()
        .flatten()
        .map(|site| {
            let outcome = cpg.call_graph.resolve_call_site_full(site);
            let targets: Vec<_> = outcome
                .resolved
                .iter()
                .map(|r| {
                    format!(
                        "{}:{}:{:?}:{}",
                        r.target.file,
                        r.target.name,
                        r.confidence,
                        r.kind.as_str()
                    )
                })
                .collect();
            format!(
                "{}:{}..{}:{}:{targets:?}:{:?}",
                site.caller.file, site.start_byte, site.end_byte, site.callee_name, outcome.drop
            )
        })
        .collect();
    sites.sort();
    let mut edges: Vec<String> = cpg
        .graph
        .edge_references()
        .filter(|edge| matches!(edge.weight(), crate::cpg::CpgEdge::DataFlow(_)))
        .map(|edge| {
            format!(
                "{:?}->{:?}:{:?}",
                cpg.graph[edge.source()],
                cpg.graph[edge.target()],
                edge.weight()
            )
        })
        .collect();
    edges.sort();
    (sites, edges)
}

#[test]
fn activated_route_parity() {
    use crate::cpg::CodePropertyGraph;
    use crate::cpg_cache::{compute_file_hashes, load_cache, save_cache, CacheResult, SaveOutcome};

    let temp = tempfile::tempdir().unwrap();
    let mut sources = BTreeMap::from([
        (
            "holder.js".to_string(),
            "export function holder(cb, value) { cb(value); }".to_string(),
        ),
        (
            "caller-base.js".to_string(),
            "import { holder } from './holder';\n".to_string(),
        ),
        ("caller-ninth.js".to_string(), String::new()),
    ]);
    for i in 0..8 {
        sources
            .get_mut("caller-base.js")
            .unwrap()
            .push_str(&format!(
                "function target{i}(item) {{ use(item); }}\nholder(target{i}, {i});\n"
            ));
    }
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
    let credited = |cpg: &CodePropertyGraph| {
        observed(cpg)
            .0
            .iter()
            .filter(|site| site.contains(":cb:"))
            .cloned()
            .collect::<Vec<_>>()
    };
    let initial = CodePropertyGraph::build(&files_of(&sources));
    let cold = credited(&initial);
    assert_eq!(cold.len(), 1);
    assert!(cold[0].matches("argument_follow").count() == 8, "{cold:?}");
    let edges = observed(&initial).1;
    assert!(
        edges
            .iter()
            .any(|edge| edge.contains("NameOnly(CallNameOnly)")),
        "payload edges carry the call_nameonly label"
    );
    let hashes = compute_file_hashes(&sources);
    assert_eq!(
        save_cache(&initial, &hashes, false, temp.path()).unwrap(),
        SaveOutcome::Saved
    );
    let CacheResult::Hit(restored) = load_cache(&hashes, false, temp.path()) else {
        panic!("expected full hit");
    };
    assert_eq!(observed(&initial), observed(&restored), "full hit");

    let ninth = "import { holder } from './holder';\nfunction added(item) { use(item); }\nholder(added, 9);";
    let mut previous = restored;
    for (caller, resolved) in [(ninth, false), ("", true), (ninth, false)] {
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
        let partial = CodePropertyGraph::build_incremental(
            cached_call_graph,
            cached_dfg,
            &changed_files,
            &files,
            None,
        );
        let fresh = CodePropertyGraph::build(&files);
        assert_eq!(observed(&partial), observed(&fresh), "partial vs cold");
        let site = &credited(&partial)[0];
        if resolved {
            assert!(site.matches("argument_follow").count() == 8, "{site}");
        } else {
            assert!(site.contains("ArgFanout"), "{site}");
            assert!(!site.contains("argument_follow"), "{site}");
        }
        assert_eq!(
            save_cache(&partial, &hashes, false, temp.path()).unwrap(),
            SaveOutcome::Saved
        );
        previous = partial;
    }
    // Caller deletion rebuilds the same graph as a cold build.
    sources.remove("caller-ninth.js");
    let files = files_of(&sources);
    let deleted = CodePropertyGraph::build_incremental(
        previous.call_graph,
        previous.dfg,
        &BTreeSet::from(["caller-ninth.js".to_string()]),
        &files,
        None,
    );
    assert_eq!(
        observed(&deleted),
        observed(&CodePropertyGraph::build(&files))
    );
}
