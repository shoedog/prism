//! Task 4a TDD: the inert consumer-facing name-resolution helpers.
//!
//! These tests hand-build `ScopeGraph`s and synthetic `CallSite`s. They pin the
//! recall-safety contract before the helpers are wired into navigation or call
//! resolution consumers.

use std::collections::BTreeMap;

use prism::call_graph::{CallKind, CallSite, CallSiteOrigin, FunctionId};
use prism::name_resolution::consumer::{
    authoritative_for, graph_callable_edge, graph_module_dep_edge, GraphImport, ResolvedImport,
};
use prism::name_resolution::graph::ScopeGraph;
use prism::name_resolution::rust_policy::{self, NS_MACRO, NS_TYPE, NS_VALUE, VIS_PUB};
use prism::name_resolution::types::{
    Anchor, AnchorKind, BindTarget, Binding, BindingRef, CfgCond, Edge, ExternRef, FileId, ItemId,
    RawPath, Scope, ScopeExtent, ScopeId, ScopeKind, SourceLoc, Span, Target, Vis,
};

const MAIN: FileId = FileId(0);
const UTIL: FileId = FileId(1);
const TYPES: FileId = FileId(2);
const PRELUDE: FileId = FileId(3);
const ROOT_FOO: FileId = FileId(4);
const INNER_FOO: FileId = FileId(5);

fn loc(file: FileId, byte: usize) -> SourceLoc {
    SourceLoc { file, byte }
}

fn span(file: FileId, lo: usize, hi: usize) -> Span {
    Span {
        lo: loc(file, lo),
        hi: loc(file, hi),
    }
}

fn scope(
    id: u32,
    kind: ScopeKind,
    parent: Option<u32>,
    file: FileId,
    lo: usize,
    hi: usize,
) -> Scope {
    Scope {
        id: ScopeId(id),
        kind,
        parent: parent.map(ScopeId),
        extents: vec![ScopeExtent {
            file,
            range: span(file, lo, hi),
            cond: None,
            occ: None,
        }],
        owner_item: None,
        cond: None,
    }
}

fn vis() -> Vis {
    Vis {
        kind: VIS_PUB,
        restrict: None,
        payload: Default::default(),
    }
}

fn binding(scope: u32, name: &str, ns: u16, target: BindTarget) -> Binding {
    Binding {
        scope: ScopeId(scope),
        name: name.to_string(),
        ns,
        target,
        vis: vis(),
        cond: None,
        vis_extents: vec![span(MAIN, 0, 1000)],
    }
}

fn item(item: u32, ns: u16, callable: bool) -> Target {
    Target::Item {
        id: ItemId(item),
        ns,
        owns: None,
        callable,
    }
}

fn call_site(file: &str, name: &str, byte: usize) -> CallSite {
    CallSite {
        caller: FunctionId {
            file: file.to_string(),
            name: "caller".to_string(),
            start_line: 1,
            end_line: 1,
        },
        callee_name: name.to_string(),
        line: 1,
        kind: CallKind::Call,
        start_byte: byte,
        end_byte: byte + name.len(),
        qualifier: None,
        receiver_type: None,
        receiver_owner_identity: None,
        receiver_recovery: None,
        receiver_materialized: false,
        arg_count: None,
        arg_spread: false,
        receiver_outcome: None,
        origin: CallSiteOrigin::Source,
    }
}

fn macro_call_site(file: &str, name: &str, byte: usize) -> CallSite {
    CallSite {
        kind: CallKind::MacroInvocation,
        ..call_site(file, name, byte)
    }
}

fn base_graph() -> ScopeGraph {
    let mut g = ScopeGraph::new();
    g.complete = true;
    g.file_paths = BTreeMap::from([
        ("src/main.rs".to_string(), MAIN),
        ("src/util.rs".to_string(), UTIL),
        ("src/types.rs".to_string(), TYPES),
        ("src/prelude.rs".to_string(), PRELUDE),
        ("src/root_foo.rs".to_string(), ROOT_FOO),
        ("src/inner_foo.rs".to_string(), INNER_FOO),
    ]);
    g.add_scope(scope(0, ScopeKind::Root, None, MAIN, 0, 1000));
    g.add_scope(scope(1, ScopeKind::Module, Some(0), UTIL, 2000, 3000));
    g.add_scope(scope(2, ScopeKind::Module, Some(0), TYPES, 3000, 4000));
    g.add_scope(scope(3, ScopeKind::Module, Some(0), PRELUDE, 4000, 5000));
    g.add_binding(binding(
        0,
        "util",
        NS_TYPE,
        BindTarget::Resolved(Target::Scope(ScopeId(1))),
    ));
    g.add_binding(binding(
        0,
        "types",
        NS_TYPE,
        BindTarget::Resolved(Target::Scope(ScopeId(2))),
    ));
    g.add_binding(binding(
        0,
        "prelude",
        NS_TYPE,
        BindTarget::Resolved(Target::Scope(ScopeId(3))),
    ));
    g
}

fn edition_decoy_graph(edition: u16) -> ScopeGraph {
    let mut g = base_graph();
    g.edition = edition;

    let root_foo = Target::Scope(ScopeId(4));
    let inner_foo = Target::Scope(ScopeId(6));
    let root_bar = item(200, NS_VALUE, true);
    let inner_bar = item(201, NS_VALUE, true);

    g.add_scope(scope(4, ScopeKind::Module, Some(0), ROOT_FOO, 0, 100));
    g.add_scope(scope(5, ScopeKind::Module, Some(0), MAIN, 100, 500));
    g.add_scope(scope(6, ScopeKind::Module, Some(5), INNER_FOO, 0, 100));
    g.add_binding(binding(0, "foo", NS_TYPE, BindTarget::Resolved(root_foo)));
    g.add_binding(binding(4, "bar", NS_VALUE, BindTarget::Resolved(root_bar)));
    g.add_binding(binding(5, "foo", NS_TYPE, BindTarget::Resolved(inner_foo)));
    g.add_binding(binding(6, "bar", NS_VALUE, BindTarget::Resolved(inner_bar)));
    g
}

#[test]
fn macro_invocation_callable_edge_routes_to_macro_namespace_only() {
    let call = call_site("src/main.rs", "foo", 50);
    let macro_site = macro_call_site("src/main.rs", "foo", 50);
    let value_fn = item(10, NS_VALUE, true);

    let mut value_only = base_graph();
    value_only.add_binding(binding(
        0,
        "foo",
        NS_VALUE,
        BindTarget::Resolved(value_fn.clone()),
    ));

    assert_eq!(graph_callable_edge(&value_only, &macro_site), None);
    assert_ne!(
        graph_callable_edge(&value_only, &macro_site),
        Some(value_fn.clone())
    );
    assert_eq!(graph_callable_edge(&value_only, &call), Some(value_fn));

    let macro_fn = item(11, NS_MACRO, true);
    let mut macro_only = base_graph();
    macro_only.add_binding(binding(
        0,
        "foo",
        NS_MACRO,
        BindTarget::Resolved(macro_fn.clone()),
    ));

    assert_eq!(
        graph_callable_edge(&macro_only, &macro_site),
        Some(macro_fn)
    );
}

#[test]
fn module_dep_use_path_uses_2015_crate_root_anchor_not_inner_decoy() {
    let g = edition_decoy_graph(2015);
    let import = binding(
        5,
        "bar",
        NS_VALUE,
        BindTarget::Pending(
            RawPath(vec!["foo".to_string(), "bar".to_string()]),
            Anchor::use_path_2015(),
        ),
    );

    assert_eq!(
        graph_module_dep_edge(&g, GraphImport::Named(&import)),
        ResolvedImport::File("src/root_foo.rs".to_string())
    );
    assert_ne!(
        graph_module_dep_edge(&g, GraphImport::Named(&import)),
        ResolvedImport::File("src/inner_foo.rs".to_string())
    );
}

#[test]
fn leading_colon_anchor_is_crate_root_in_2015_and_falls_through_in_2018() {
    let import = binding(
        5,
        "bar",
        NS_VALUE,
        BindTarget::Pending(
            RawPath(vec!["foo".to_string(), "bar".to_string()]),
            Anchor {
                kind: AnchorKind::LeadingColon,
                prelude: None,
            },
        ),
    );

    let g_2015 = edition_decoy_graph(2015);
    assert_eq!(
        graph_module_dep_edge(&g_2015, GraphImport::Named(&import)),
        ResolvedImport::File("src/root_foo.rs".to_string())
    );

    let g_2018 = edition_decoy_graph(2018);
    assert_eq!(
        graph_module_dep_edge(&g_2018, GraphImport::Named(&import)),
        ResolvedImport::Unresolved
    );
}

#[test]
fn authority_states_are_site_based() {
    let site = call_site("src/main.rs", "start", 50);
    let mut g = base_graph();
    g.add_binding(binding(
        0,
        "start",
        NS_VALUE,
        BindTarget::Resolved(item(10, NS_VALUE, true)),
    ));

    assert!(authoritative_for(&g, &site));
    assert_eq!(
        graph_callable_edge(&g, &site),
        Some(item(10, NS_VALUE, true))
    );

    let absent: Option<&ScopeGraph> = None;
    assert!(!absent
        .map(|graph| authoritative_for(graph, &site))
        .unwrap_or(false));
    assert_eq!(
        absent.and_then(|graph| graph_callable_edge(graph, &site)),
        None
    );

    let mut incomplete = g.clone();
    incomplete.complete = false;
    assert!(!authoritative_for(&incomplete, &site));
    assert_eq!(graph_callable_edge(&incomplete, &site), None);

    let mut poisoned_target = base_graph();
    poisoned_target.add_edge(Edge {
        from: ScopeId(0),
        kind: rust_policy::EK_GLOB,
        to: BindTarget::Pending(RawPath(vec![]), Anchor::default()),
        vis: vis(),
        cond: None,
        order: 0,
        vis_range: None,
    });
    assert!(authoritative_for(&poisoned_target, &site));
    assert_eq!(graph_callable_edge(&poisoned_target, &site), None);

    let unmodeled_site = call_site("src/missing.rs", "start", 50);
    assert!(!authoritative_for(&g, &unmodeled_site));
    assert_eq!(graph_callable_edge(&g, &unmodeled_site), None);
}

#[test]
fn callable_edge_falls_through_on_every_non_single_in_repo_callable() {
    let site = call_site("src/main.rs", "x", 50);

    let cases: Vec<(&str, Vec<Binding>, Vec<Edge>)> = vec![
        (
            "external",
            vec![binding(
                0,
                "x",
                NS_VALUE,
                BindTarget::Resolved(Target::External(ExternRef {
                    name: "std::x".to_string(),
                })),
            )],
            vec![],
        ),
        (
            "non-callable item",
            vec![binding(
                0,
                "x",
                NS_VALUE,
                BindTarget::Resolved(item(20, NS_VALUE, false)),
            )],
            vec![],
        ),
        (
            "scope",
            vec![binding(
                0,
                "x",
                NS_VALUE,
                BindTarget::Resolved(Target::Scope(ScopeId(1))),
            )],
            vec![],
        ),
        (
            "local",
            vec![binding(
                0,
                "x",
                NS_VALUE,
                BindTarget::Resolved(Target::Local(BindingRef {
                    scope: ScopeId(0),
                    ordinal: 0,
                })),
            )],
            vec![],
        ),
    ];

    for (label, bindings, edges) in cases {
        let mut g = base_graph();
        for b in bindings {
            g.add_binding(b);
        }
        for e in edges {
            g.add_edge(e);
        }
        assert_eq!(graph_callable_edge(&g, &site), None, "{label}");
    }

    let mut resolved_set = base_graph();
    let linux = CfgCond::Atom("target_os".to_string(), Some("linux".to_string()));
    let windows = CfgCond::Atom("target_os".to_string(), Some("windows".to_string()));
    let mut b1 = binding(
        0,
        "x",
        NS_VALUE,
        BindTarget::Resolved(item(30, NS_VALUE, true)),
    );
    b1.cond = Some(linux);
    let mut b2 = binding(
        0,
        "x",
        NS_VALUE,
        BindTarget::Resolved(item(31, NS_VALUE, true)),
    );
    b2.cond = Some(windows);
    resolved_set.add_binding(b1);
    resolved_set.add_binding(b2);
    assert_eq!(graph_callable_edge(&resolved_set, &site), None);

    let mut ambiguous = base_graph();
    ambiguous.add_binding(binding(
        0,
        "x",
        NS_VALUE,
        BindTarget::Resolved(item(40, NS_VALUE, true)),
    ));
    ambiguous.add_binding(binding(
        0,
        "x",
        NS_VALUE,
        BindTarget::Resolved(item(41, NS_VALUE, true)),
    ));
    assert_eq!(graph_callable_edge(&ambiguous, &site), None);

    let mut poisoned = base_graph();
    poisoned.add_edge(Edge {
        from: ScopeId(0),
        kind: rust_policy::EK_GLOB,
        to: BindTarget::Pending(RawPath(vec![]), Anchor::default()),
        vis: vis(),
        cond: None,
        order: 0,
        vis_range: None,
    });
    assert_eq!(graph_callable_edge(&poisoned, &site), None);

    let mut good = base_graph();
    good.add_binding(binding(
        0,
        "x",
        NS_VALUE,
        BindTarget::Resolved(item(50, NS_VALUE, true)),
    ));
    assert_eq!(
        graph_callable_edge(&good, &site),
        Some(item(50, NS_VALUE, true))
    );
}

#[test]
fn module_dep_edge_resolves_named_type_reexport_external_glob_and_unresolved() {
    let mut g = base_graph();
    g.add_binding(binding(
        1,
        "helper",
        NS_VALUE,
        BindTarget::Resolved(item(100, NS_VALUE, true)),
    ));
    g.add_binding(binding(
        2,
        "Config",
        NS_TYPE,
        BindTarget::Resolved(item(101, NS_TYPE, false)),
    ));
    g.add_binding(binding(
        3,
        "thing",
        NS_VALUE,
        BindTarget::Resolved(item(102, NS_VALUE, true)),
    ));

    let util_import = binding(
        0,
        "helper",
        NS_VALUE,
        BindTarget::Pending(
            RawPath(vec!["util".to_string(), "helper".to_string()]),
            Anchor::crate_root(),
        ),
    );
    assert_eq!(
        graph_module_dep_edge(&g, GraphImport::Named(&util_import)),
        ResolvedImport::File("src/util.rs".to_string())
    );

    let type_import = binding(
        0,
        "Config",
        NS_TYPE,
        BindTarget::Pending(
            RawPath(vec!["types".to_string(), "Config".to_string()]),
            Anchor::crate_root(),
        ),
    );
    assert_eq!(
        graph_module_dep_edge(&g, GraphImport::Named(&type_import)),
        ResolvedImport::File("src/types.rs".to_string())
    );

    let reexport = binding(
        0,
        "facade_helper",
        NS_VALUE,
        BindTarget::Pending(
            RawPath(vec!["util".to_string(), "helper".to_string()]),
            Anchor::crate_root(),
        ),
    );
    let reexport_import = binding(
        0,
        "facade_helper",
        NS_VALUE,
        BindTarget::Pending(
            RawPath(vec!["facade_helper".to_string()]),
            Anchor::self_mod(),
        ),
    );
    g.add_binding(reexport);
    assert_eq!(
        graph_module_dep_edge(&g, GraphImport::Named(&reexport_import)),
        ResolvedImport::File("src/util.rs".to_string())
    );

    let external = binding(
        0,
        "fmt",
        NS_TYPE,
        BindTarget::Resolved(Target::External(ExternRef {
            name: "std::fmt".to_string(),
        })),
    );
    assert_eq!(
        graph_module_dep_edge(&g, GraphImport::Named(&external)),
        ResolvedImport::External
    );

    let pending_std = binding(
        0,
        "BTreeMap",
        NS_TYPE,
        BindTarget::Pending(
            RawPath(vec![
                "std".to_string(),
                "collections".to_string(),
                "BTreeMap".to_string(),
            ]),
            Anchor::use_path_2015(),
        ),
    );
    assert_eq!(
        graph_module_dep_edge(&g, GraphImport::Named(&pending_std)),
        ResolvedImport::External
    );

    let glob = Edge {
        from: ScopeId(0),
        kind: rust_policy::EK_GLOB,
        to: BindTarget::Resolved(Target::Scope(ScopeId(3))),
        vis: vis(),
        cond: None,
        order: 0,
        vis_range: None,
    };
    assert_eq!(
        graph_module_dep_edge(&g, GraphImport::Glob(&glob)),
        ResolvedImport::File("src/prelude.rs".to_string())
    );

    let unresolved = binding(
        0,
        "missing",
        NS_VALUE,
        BindTarget::Pending(
            RawPath(vec!["nope".to_string(), "missing".to_string()]),
            Anchor::crate_root(),
        ),
    );
    assert_eq!(
        graph_module_dep_edge(&g, GraphImport::Named(&unresolved)),
        ResolvedImport::Unresolved
    );
}
