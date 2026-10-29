use super::*;
use crate::ast::ParsedFile;
use crate::languages::Language;
use crate::name_resolution::rust_policy::{NS_MACRO, NS_TYPE, NS_VALUE, VIS_PUB};
use crate::name_resolution::rust_populator::{populate_rust, RustCrateConfig};
use crate::name_resolution::types::*;
use std::collections::BTreeMap;

pub(super) fn fixture() -> ScopeGraph {
    let mut graph = ScopeGraph::new();
    graph.file_paths = BTreeMap::from([
        ("a.rs".into(), FileId(0)),
        ("alias.rs".into(), FileId(0)),
        ("b.rs".into(), FileId(4)),
        ("m.rs".into(), FileId(2)),
        ("z.rs".into(), FileId(0)),
    ]);
    for scope in 0..3 {
        let extents: Vec<_> = (0..2)
            .map(|file| ScopeExtent {
                file: FileId(file),
                range: Span {
                    lo: SourceLoc {
                        file: FileId(file),
                        byte: 0,
                    },
                    hi: SourceLoc {
                        file: FileId(file),
                        byte: 100,
                    },
                },
                cond: None,
                occ: None,
            })
            .collect();
        graph.add_scope(Scope {
            id: ScopeId(scope),
            kind: ScopeKind::Module,
            parent: None,
            extents: extents.clone(),
            owner_item: None,
            cond: None,
        });
        for ns in [NS_TYPE, NS_VALUE, NS_MACRO] {
            // Deliberately non-key-sorted insertion order and duplicate keys.
            for name in ["z", "same", "a", "same"] {
                graph.add_binding(Binding {
                    scope: ScopeId(scope),
                    name: name.into(),
                    ns,
                    target: BindTarget::Resolved(Target::Item {
                        id: ItemId(ns as u32),
                        ns,
                        owns: None,
                        callable: ns == NS_VALUE,
                    }),
                    vis: Vis {
                        kind: VIS_PUB,
                        restrict: None,
                        payload: Default::default(),
                    },
                    cond: Some(CfgCond::Atom("feature".into(), Some(name.into()))),
                    vis_extents: extents.iter().map(|e| e.range.clone()).collect(),
                });
            }
        }
    }
    let mut pending = graph.bindings[0].clone();
    pending.target = BindTarget::Pending(RawPath(vec!["other".into()]), Anchor::default());
    graph.add_binding(pending);
    graph
}

fn check_equivalence(graph: &ScopeGraph, indexed: bool) {
    for scope in graph.scopes.keys().copied().chain([ScopeId(999)]) {
        for ns in [NS_TYPE, NS_VALUE, NS_MACRO, 99] {
            for name in graph
                .bindings
                .iter()
                .map(|b| b.name.as_str())
                .chain(["missing"])
            {
                let found = graph.binding_indices(scope, name, ns);
                assert_eq!(matches!(&found, BindingIndices::Indexed(_)), indexed);
                let expected: Vec<_> = graph
                    .bindings
                    .iter()
                    .enumerate()
                    .filter(|(_, b)| b.scope == scope && b.name == name && b.ns == ns)
                    .map(|(i, _)| i)
                    .collect();
                assert_eq!(found.collect::<Vec<_>>(), expected);
            }
        }
    }
    let absent = Target::Scope(ScopeId(999));
    for target in graph
        .bindings
        .iter()
        .filter_map(|b| match &b.target {
            BindTarget::Resolved(t) => Some(t),
            _ => None,
        })
        .chain([&absent])
    {
        let found = graph.resolved_binding_indices(target);
        assert_eq!(matches!(&found, BindingIndices::Indexed(_)), indexed);
        let expected: Vec<_> = graph
            .bindings
            .iter()
            .enumerate()
            .filter(|(_, b)| matches!(&b.target, BindTarget::Resolved(t) if t == target))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(found.collect::<Vec<_>>(), expected);
    }
    for file in [FileId(0), FileId(1), FileId(2), FileId(4), FileId(999)] {
        let expected = graph
            .file_paths
            .iter()
            .find_map(|(path, &id)| (id == file).then_some(path.as_str()));
        assert_eq!(graph.path_for_file(file), expected);
    }
}

#[test]
fn indexed_keys_preserve_linear_order_and_policy_inputs() {
    let graph = fixture();
    check_equivalence(&graph, true);
    assert_eq!(
        graph
            .binding_indices(ScopeId(0), "same", NS_TYPE)
            .collect::<Vec<_>>(),
        [1, 3]
    );
    assert_eq!(graph.path_for_file(FileId(0)), Some("a.rs"));
    let index = graph.binding_index.0.get().unwrap();
    assert_eq!(index.by_name.len(), graph.bindings.len());
    assert_eq!(index.by_target.len(), graph.bindings.len() - 1);
}

#[test]
fn append_invalidates_and_direct_push_uses_scan() {
    let mut graph = fixture();
    check_equivalence(&graph, true);
    let binding = graph.bindings[1].clone();
    graph.add_binding(binding.clone());
    assert!(graph.binding_index.0.get().is_none());
    check_equivalence(&graph, true);
    assert_eq!(
        graph
            .binding_indices(binding.scope, &binding.name, binding.ns)
            .last(),
        Some(graph.bindings.len() - 1)
    );
    graph.bindings.push(binding.clone());
    check_equivalence(&graph, false);
    assert_eq!(
        graph
            .binding_indices(binding.scope, &binding.name, binding.ns)
            .last(),
        Some(graph.bindings.len() - 1)
    );
    // Returning to add_binding also repairs a previously bypassed invalidation.
    graph.add_binding(binding);
    check_equivalence(&graph, true);
}

#[test]
fn path_append_falls_back_and_empty_graph_misses() {
    let mut graph = fixture();
    assert_eq!(graph.path_for_file(FileId(999)), None);
    graph.file_paths.insert("new.rs".into(), FileId(999));
    assert_eq!(graph.path_for_file(FileId(999)), Some("new.rs"));
    check_equivalence(&ScopeGraph::new(), true);
}

#[test]
fn populated_graph_keys_equal_scans_from_rayon_threads() {
    let files = BTreeMap::from([
        (
            "lib.rs".into(),
            ParsedFile::parse(
                "lib.rs",
                "mod util; use util::*; #[cfg(feature=\"a\")] fn f() {} fn g(){ let x = 1; }",
                Language::Rust,
            )
            .unwrap(),
        ),
        (
            "util.rs".into(),
            ParsedFile::parse(
                "util.rs",
                "pub struct S; impl S { pub fn f() {} }",
                Language::Rust,
            )
            .unwrap(),
        ),
    ]);
    let config = RustCrateConfig {
        crate_roots: vec!["lib.rs".into()],
        ..Default::default()
    };
    let graph = populate_rust(&files, &config, None);
    rayon::scope(|s| {
        for _ in 0..8 {
            s.spawn(|_| {
                check_equivalence(&graph, true);
                super::scope_tests::check_equivalence(&graph, true, true);
            });
        }
    });
}

#[test]
fn index_is_absent_from_serialization_and_identity() {
    let mut graph = fixture();
    super::scope_tests::add_entries(&mut graph);
    // The pre-index bincode layout, pinned independently of ScopeGraph's derive.
    let old_bytes = bincode::serialize(&(
        graph.complete,
        graph.edition,
        graph.edition_uniform,
        &graph.file_paths,
        &graph.crate_deps_by_root,
        &graph.scopes,
        &graph.bindings,
        &graph.local_facts,
        &graph.edges,
        &graph.macro_wildcards,
    ))
    .unwrap();
    let cold_json = serde_json::to_string(&graph).unwrap();
    check_equivalence(&graph, true);
    super::scope_tests::check_equivalence(&graph, true, true);
    assert_eq!(bincode::serialize(&graph).unwrap(), old_bytes);
    assert_eq!(serde_json::to_string(&graph).unwrap(), cold_json);
    let back: ScopeGraph = bincode::deserialize(&old_bytes).unwrap();
    assert!(back.binding_index.0.get().is_none());
    assert!(back.binding_index.1.get().is_none());
    assert!(back.binding_index.2.get().is_none());
    assert_eq!(graph, back);
    let clone = graph.clone();
    assert!(clone.binding_index.0.get().is_none());
    assert!(clone.binding_index.1.get().is_none());
    assert!(clone.binding_index.2.get().is_none());
    assert_eq!(graph, clone);
    check_equivalence(&back, true);
    super::scope_tests::check_equivalence(&back, true, true);
    super::scope_tests::check_equivalence(&clone, true, true);
}
