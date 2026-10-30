//! Lane A2 S1: navigation guards for anonymous call-graph nodes. `<anon@L:C>`
//! nodes are injected directly into the CPG before the index is built.

use super::{seed, NavigationIndex, NavigationSession};
use crate::cpg::anon_guard_tests::{add_anonymous_function, add_variable, function_idx, FILE, SRC};
use crate::cpg::{CodePropertyGraph, CpgContext, CpgNode};
use crate::navigation::queries;
use crate::navigation::types::{QueryError, SymbolRef};
use crate::reasoning::seeds::{self, SeedRole, SeedSpec};
use std::sync::Arc;

const ANON: &str = "<anon@2:18>";

fn session_with(
    inject: impl FnOnce(&mut CodePropertyGraph),
) -> (tempfile::TempDir, NavigationSession) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(FILE), SRC).unwrap();
    let repo = Arc::new(crate::repo_loader::load_repo(dir.path()).unwrap());
    let mut ctx = CpgContext::build_with_scope_graph_inputs(
        &repo.files,
        repo.type_db.as_ref(),
        repo.scope_graph_inputs.as_ref(),
    );
    inject(&mut ctx.cpg);
    let index = Arc::new(NavigationIndex::from_ctx(ctx));
    (dir, NavigationSession { repo, index })
}

fn with_anon_function() -> (tempfile::TempDir, NavigationSession) {
    session_with(|cpg| {
        add_anonymous_function(cpg, ANON, 2);
    })
}

fn named(s: &NavigationSession, idx: petgraph::graph::NodeIndex) -> String {
    match s.index.cpg().node(idx) {
        CpgNode::Function { name, .. } => name.clone(),
        other => panic!("not a function: {other:?}"),
    }
}

#[test]
fn enclosing_function_skips_anonymous_nodes_by_default() {
    let (_d, s) = with_anon_function();
    let (idx, name) = s.index.enclosing_function(FILE, 2).unwrap();
    assert_eq!(name, "outer");
    assert_eq!(named(&s, idx), "outer");
    let (idx, name) = s
        .index
        .enclosing_function_including_anonymous(FILE, 2)
        .unwrap();
    assert_eq!(name, ANON);
    assert_eq!(named(&s, idx), ANON);
    // Outside the anonymous range both variants agree.
    assert_eq!(s.index.enclosing_function(FILE, 3).unwrap().1, "outer");
    assert_eq!(
        s.index
            .enclosing_function_including_anonymous(FILE, 3)
            .unwrap()
            .1,
        "outer"
    );
}

#[test]
fn enclosing_function_named_control_is_unchanged() {
    let (_d, s) = session_with(|_| {});
    for line in 1..=3 {
        assert_eq!(s.index.enclosing_function(FILE, line).unwrap().1, "outer");
        assert_eq!(
            s.index
                .enclosing_function_including_anonymous(FILE, line)
                .unwrap()
                .1,
            if line == 2 { ANON } else { "outer" }
        );
    }
    assert!(s.index.enclosing_function(FILE, 9).is_none());
}

#[test]
fn location_seed_on_a_callback_line_resolves_to_the_named_function() {
    let (_d, s) = with_anon_function();
    let resolved = seed::resolve_fn(&s, None, None, Some("a.js:2")).unwrap();
    match resolved.symbol {
        SymbolRef::Function { name, .. } => assert_eq!(name, "outer"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn symbol_seed_for_an_anonymous_owner_name_is_rejected_with_guidance() {
    let (_d, s) = with_anon_function();
    for name in [ANON, "<anon@2:21>"] {
        let err = seed::resolve_fn(&s, Some(name), None, None).unwrap_err();
        match &err {
            QueryError::SymbolNotFound { seed } => {
                assert!(seed.contains("location seed"), "{seed}");
                assert!(seed.contains(name), "{seed}");
            }
            other => panic!("{other:?}"),
        }
        for q in [queries::callers, queries::callees] {
            let err = q(&s, Some(name), None, None, 1).unwrap_err();
            assert!(format!("{err:?}").contains("location seed"), "{err:?}");
        }
        let err = seeds::resolve(
            &s,
            &[SeedSpec::Symbol {
                name: name.into(),
                file: None,
            }],
            SeedRole::Source,
        )
        .unwrap_err();
        assert!(format!("{err:?}").contains("location seed"), "{err:?}");
    }
}

#[test]
fn symbol_seed_for_a_named_function_still_resolves() {
    let (_d, s) = with_anon_function();
    let resolved = seed::resolve_fn(&s, Some("outer"), None, None).unwrap();
    assert_eq!(named(&s, resolved.idx), "outer");
    // A missing ordinary name keeps the plain not-found message.
    let err = seed::resolve_fn(&s, Some("missing"), None, None).unwrap_err();
    assert!(!format!("{err:?}").contains("location seed"), "{err:?}");
}

#[test]
fn nodes_at_hides_anonymous_identities_but_keeps_named_ones() {
    let (_d, s) = session_with(|cpg| {
        let outer = function_idx(cpg, "outer");
        add_anonymous_function(cpg, ANON, 2);
        add_variable(cpg, outer, ANON, 2, 2);
        add_variable(cpg, outer, "outer", 1, 2);
    });
    let json = serde_json::to_string(&queries::nodes_at(&s, FILE, 2)).unwrap();
    assert!(!json.contains("<anon@"), "{json}");
    assert!(!json.contains("<anon@"), "{json}");
    // Named control: the enclosing function and the named-owner Variable remain.
    assert!(json.contains("\"outer\""), "{json}");
}

#[test]
fn ego_contains_hops_never_surface_anonymous_variables() {
    let (_d, s) = session_with(|cpg| {
        let outer = function_idx(cpg, "outer");
        // Contains edges from a NAMED function to an anonymous-owned Variable.
        add_variable(cpg, outer, ANON, 2, 2);
        add_variable(cpg, outer, "<anon@2:21>", 2, 2);
        add_variable(cpg, outer, "outer", 1, 2);
    });
    let ego = queries::ego_graph(&s, Some("outer"), None, None, 1, &["Contains"]).unwrap();
    let json = serde_json::to_string(&ego).unwrap();
    assert!(!json.contains("<anon@"), "{json}");
    assert!(!json.contains("<anon@"), "{json}");
    // Named control: the Contains hop to a named-owner Variable is still there.
    let graph = ego.graph.unwrap();
    assert!(
        graph.nodes.iter().any(|n| matches!(
            &n.symbol,
            Some(SymbolRef::Variable { function, .. }) if function == "outer"
        )),
        "{graph:?}"
    );
}

#[test]
fn ego_hops_never_surface_anonymous_return_values() {
    let (_d, s) = session_with(|cpg| {
        let outer = function_idx(cpg, "outer");
        for (n, owner) in [ANON, "<anon@2:21>", "outer"].into_iter().enumerate() {
            let idx = cpg.graph.add_node(CpgNode::ReturnValue {
                file: FILE.to_string(),
                function: owner.to_string(),
                function_start_line: 2,
                line: 2,
                return_start_byte: 0,
                return_end_byte: 0,
                child_slot: 0,
                start_byte: 10 * (n + 1),
                end_byte: 10 * (n + 1) + 1,
            });
            cpg.graph
                .add_edge(outer, idx, crate::cpg::CpgEdge::Contains);
        }
    });
    let ego = queries::ego_graph(&s, Some("outer"), None, None, 1, &["Contains"]).unwrap();
    let json = serde_json::to_string(&ego).unwrap();
    assert!(!json.contains("<anon@"), "{json}");
    assert!(!json.contains("<anon@"), "{json}");
    // ReturnValue rows serialize without their owner, so count them: only the
    // named-owner row (bytes 30..31) may be reached.
    let graph = ego.graph.unwrap();
    let rv: Vec<_> = graph
        .nodes
        .iter()
        .filter_map(|n| match &n.symbol {
            Some(SymbolRef::Statement {
                kind, start_byte, ..
            }) if kind == "ReturnValue" => Some(*start_byte),
            _ => None,
        })
        .collect();
    assert_eq!(rv, vec![30], "{graph:?}");
}

#[test]
fn onboarding_function_count_excludes_anonymous_nodes() {
    let (_d, with_anon) = session_with(|cpg| {
        add_anonymous_function(cpg, ANON, 2);
        add_anonymous_function(cpg, "<anon@2:30>", 2);
    });
    let (_d2, plain) = session_with(|_| {});
    let counted = crate::navigation::onboarding::build_report(&with_anon)
        .unwrap()
        .inventory
        .functions;
    let control = crate::navigation::onboarding::build_report(&plain)
        .unwrap()
        .inventory
        .functions;
    assert_eq!(counted, control);
    assert!(control >= 1, "named functions are still counted");
}
