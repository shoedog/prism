use super::*;
use crate::name_resolution::rust_policy::{NS_MACRO, NS_TYPE};

#[derive(Clone, Copy)]
enum Family {
    Name,
    Target,
    Edge,
    Wildcard,
    Path,
}

fn fixture() -> ScopeGraph {
    let mut graph = super::tests::fixture();
    super::scope_tests::add_entries(&mut graph);
    graph
}

impl Family {
    fn lookup(self, graph: &ScopeGraph) -> Vec<usize> {
        match self {
            Self::Name => graph.binding_indices(ScopeId(0), "a", NS_TYPE).collect(),
            Self::Target => graph
                .resolved_binding_indices(&Target::Scope(ScopeId(999)))
                .collect(),
            Self::Edge => graph.edge_indices(ScopeId(0)).collect(),
            Self::Wildcard => graph.macro_wildcard_indices(ScopeId(0), NS_MACRO).collect(),
            // Compare paths separately; this branch only warms their shared index.
            Self::Path => {
                graph.path_for_file(FileId(0));
                vec![]
            }
        }
    }

    fn edit(self, graph: &mut ScopeGraph) {
        match self {
            Self::Name => graph.bindings[0].name = "a".into(),
            Self::Target => {
                graph.bindings.last_mut().unwrap().target =
                    BindTarget::Resolved(Target::Scope(ScopeId(999)))
            }
            Self::Edge => graph.edges[0].from = ScopeId(0),
            Self::Wildcard => graph.macro_wildcards[0].scope = ScopeId(0),
            Self::Path => *graph.file_paths.get_mut("a.rs").unwrap() = FileId(999),
        }
    }

    fn assert_linear(self, graph: &ScopeGraph) {
        let expected: Vec<_> = match self {
            Self::Name => graph
                .bindings
                .iter()
                .enumerate()
                .filter(|(_, b)| b.scope == ScopeId(0) && b.ns == NS_TYPE && b.name == "a")
                .map(|(i, _)| i)
                .collect(),
            Self::Target => graph
                .bindings
                .iter()
                .enumerate()
                .filter(|(_, b)| b.target == BindTarget::Resolved(Target::Scope(ScopeId(999))))
                .map(|(i, _)| i)
                .collect(),
            Self::Edge => graph
                .edges
                .iter()
                .enumerate()
                .filter(|(_, e)| e.from == ScopeId(0))
                .map(|(i, _)| i)
                .collect(),
            Self::Wildcard => graph
                .macro_wildcards
                .iter()
                .enumerate()
                .filter(|(_, m)| m.scope == ScopeId(0) && m.ns == NS_MACRO)
                .map(|(i, _)| i)
                .collect(),
            Self::Path => {
                let expected = graph
                    .file_paths
                    .iter()
                    .find_map(|(path, &id)| (id == FileId(0)).then_some(path.as_str()));
                assert_eq!(graph.path_for_file(FileId(0)), expected);
                return;
            }
        };
        assert_eq!(self.lookup(graph), expected);
    }
}

macro_rules! stale_test {
    ($name:ident, $family:ident, $message:literal) => {
        #[cfg(debug_assertions)]
        #[test]
        #[should_panic(expected = $message)]
        fn $name() {
            let mut graph = fixture();
            Family::$family.assert_linear(&graph);
            Family::$family.edit(&mut graph);
            Family::$family.lookup(&graph);
        }
    };
}

stale_test!(stale_name_panics, Name, "binding name lookup equivalence");
stale_test!(
    stale_target_panics,
    Target,
    "resolved target lookup equivalence"
);
stale_test!(stale_edge_panics, Edge, "edge lookup equivalence");
stale_test!(
    stale_wildcard_panics,
    Wildcard,
    "wildcard lookup equivalence"
);
stale_test!(stale_path_panics, Path, "file path lookup equivalence");

macro_rules! invalidation_test {
    ($name:ident, $family:ident) => {
        #[test]
        fn $name() {
            let mut graph = fixture();
            Family::$family.assert_linear(&graph);
            Family::$family.edit(&mut graph);
            graph.invalidate_lookup_indexes();
            Family::$family.assert_linear(&graph);
        }
    };
}

invalidation_test!(invalidate_name_after_same_length_edit, Name);
invalidation_test!(invalidate_target_after_same_length_edit, Target);
invalidation_test!(invalidate_edge_after_same_length_edit, Edge);
invalidation_test!(invalidate_wildcard_after_same_length_edit, Wildcard);
invalidation_test!(invalidate_path_after_same_length_edit, Path);

#[test]
fn invalidate_drops_every_index_and_empty_invalidation_is_safe() {
    let mut graph = ScopeGraph::new();
    graph.invalidate_lookup_indexes();
    graph = fixture();
    for family in [
        Family::Name,
        Family::Target,
        Family::Edge,
        Family::Wildcard,
        Family::Path,
    ] {
        family.assert_linear(&graph);
    }
    assert!(graph.binding_index.0.get().is_some());
    assert!(graph.binding_index.1.get().is_some());
    assert!(graph.binding_index.2.get().is_some());
    graph.invalidate_lookup_indexes();
    assert!(graph.binding_index.0.get().is_none());
    assert!(graph.binding_index.1.get().is_none());
    assert!(graph.binding_index.2.get().is_none());
}

#[test]
fn navigation_mutation_hook_invalidates_warm_scope_graph() {
    let dir = tempfile::tempdir().unwrap();
    let repo = crate::repo_loader::load_repo(dir.path()).unwrap();
    let mut index = crate::navigation::NavigationIndex::build(&repo);
    // Exercise the absent-graph branch too.
    index = index.with_modified_cpg_for_testing(|cpg| cpg.call_graph.scope_graph = None);
    let graph = fixture();
    Family::Name.assert_linear(&graph);
    index.cpg.call_graph.scope_graph = Some(graph);
    let index = index.with_modified_cpg_for_testing(|cpg| {
        Family::Name.edit(cpg.call_graph.scope_graph.as_mut().unwrap());
    });
    Family::Name.assert_linear(index.call_graph().scope_graph.as_ref().unwrap());
}

// The cross-check compares whole index vectors, not just their lengths, and skips graphs above the cap.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "binding name lookup equivalence")]
fn cross_check_rejects_equal_length_different_indices() {
    assert_lookup_indices(
        &[1u32, 2].iter(),
        2,
        [1usize, 3].into_iter(),
        "binding name",
    );
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "edge lookup equivalence")]
fn cross_check_rejects_reordered_indices() {
    assert_lookup_indices(&[1u32, 2].iter(), 2, [2usize, 1].into_iter(), "edge");
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "wildcard lookup equivalence")]
fn cross_check_runs_at_the_cap() {
    assert_lookup_indices(
        &[1u32].iter(),
        CROSS_CHECK_MAX_LEN,
        [2usize].into_iter(),
        "wildcard",
    );
}

#[cfg(debug_assertions)]
#[test]
fn cross_check_skips_above_the_cap() {
    assert_lookup_indices(
        &[1u32].iter(),
        CROSS_CHECK_MAX_LEN + 1,
        [2usize].into_iter(),
        "wildcard",
    );
}

/// A graph with `n` file paths whose index is warm and then made stale by a same-length edit:
/// `FileId(0)` is remapped from `f0.rs` to `g0.rs` without invalidating.
#[cfg(debug_assertions)]
fn stale_paths(n: u32) -> ScopeGraph {
    let mut graph = ScopeGraph::new();
    for i in 0..n {
        graph.file_paths.insert(format!("f{i}.rs"), FileId(i));
    }
    assert_eq!(graph.path_for_file(FileId(0)), Some("f0.rs"));
    graph.file_paths.remove("f0.rs");
    graph.file_paths.insert("g0.rs".to_string(), FileId(0));
    graph
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "file path lookup equivalence")]
fn path_cross_check_runs_at_the_cap() {
    stale_paths(CROSS_CHECK_MAX_LEN as u32).path_for_file(FileId(0));
}

#[cfg(debug_assertions)]
#[test]
fn path_cross_check_skips_above_the_cap() {
    // Above the cap the debug check is skipped, so the stale index answers (as a release build would).
    assert_eq!(
        stale_paths(CROSS_CHECK_MAX_LEN as u32 + 1).path_for_file(FileId(0)),
        Some("f0.rs")
    );
}
