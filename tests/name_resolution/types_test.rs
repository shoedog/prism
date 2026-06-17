//! Task 1 TDD: scope-graph core data-model types.
//!
//! Step 1: this test must FAIL before types are implemented.
//! Step 2: implement types to make it pass.
use prism::name_resolution::types::{
    Anchor, BindTarget, Binding, Candidate, CfgCond, Edge, EdgeKindId, FileId, ItemId, NamespaceId,
    RawPath, ResStatus, Resolution, Scope, ScopeExtent, ScopeId, ScopeKind, SourceLoc, Span,
    Target, Vis, VisKindId,
};

/// Build a 2-scope graph by hand: a Root + a Module child with one Binding.
/// Assert field access and serde round-trip byte-stability.
#[test]
fn test_two_scope_graph_field_access_and_serde() {
    // ── leaf IDs ────────────────────────────────────────────────────────────
    let root_id = ScopeId(0);
    let module_id = ScopeId(1);
    let item_id = ItemId(42);
    let file_id = FileId(0);

    // ── Spans and SourceLoc ─────────────────────────────────────────────────
    let lo = SourceLoc {
        file: file_id,
        byte: 0,
    };
    let hi = SourceLoc {
        file: file_id,
        byte: 100,
    };
    let span = Span { lo, hi };

    // ── Extents ─────────────────────────────────────────────────────────────
    let extent = ScopeExtent {
        file: file_id,
        range: span.clone(),
        cond: None,
        occ: None,
    };

    // ── Root scope ──────────────────────────────────────────────────────────
    let root = Scope {
        id: root_id,
        kind: ScopeKind::Root,
        parent: None,
        extents: vec![extent.clone()],
        owner_item: None,
        cond: None,
    };
    assert_eq!(root.id, root_id);
    assert!(root.parent.is_none());
    assert_eq!(root.kind, ScopeKind::Root);

    // ── Module scope ────────────────────────────────────────────────────────
    let module_extent = ScopeExtent {
        file: file_id,
        range: Span {
            lo: SourceLoc {
                file: file_id,
                byte: 10,
            },
            hi: SourceLoc {
                file: file_id,
                byte: 90,
            },
        },
        cond: None,
        occ: None,
    };
    let module = Scope {
        id: module_id,
        kind: ScopeKind::Module,
        parent: Some(root_id),
        extents: vec![module_extent],
        owner_item: None,
        cond: None,
    };
    assert_eq!(module.parent, Some(root_id));
    assert_eq!(module.kind, ScopeKind::Module);

    // ── Binding in the Module ────────────────────────────────────────────────
    let ns: NamespaceId = 0; // Value namespace (policy-assigned constant)
    let vis_kind: VisKindId = 0;
    let vis = Vis {
        kind: vis_kind,
        restrict: None,
        payload: Default::default(),
    };
    let target = Target::Item {
        id: item_id,
        ns,
        owns: None,
        callable: true,
    };
    let binding = Binding {
        scope: module_id,
        name: "my_fn".to_string(),
        ns,
        target: BindTarget::Resolved(target.clone()),
        vis: vis.clone(),
        cond: None,
        vis_extents: vec![span.clone()],
    };
    assert_eq!(binding.scope, module_id);
    assert_eq!(binding.name, "my_fn");
    assert!(binding.cond.is_none());

    // ── Resolution wrapping it ───────────────────────────────────────────────
    let candidate = Candidate {
        target,
        cond: CfgCond::True,
        provenance: Default::default(),
    };
    let resolution = Resolution {
        candidates: vec![candidate],
        status: ResStatus::Resolved,
    };
    assert_eq!(resolution.status, ResStatus::Resolved);
    assert_eq!(resolution.candidates.len(), 1);

    // ── Serde round-trip (byte-stable) ──────────────────────────────────────
    let root_json = serde_json::to_string(&root).expect("root serialize");
    let root2: Scope = serde_json::from_str(&root_json).expect("root deserialize");
    assert_eq!(
        root, root2,
        "Root scope serde round-trip must be byte-stable"
    );

    let module_json = serde_json::to_string(&module).expect("module serialize");
    let module2: Scope = serde_json::from_str(&module_json).expect("module deserialize");
    assert_eq!(
        module, module2,
        "Module scope serde round-trip must be byte-stable"
    );

    let binding_json = serde_json::to_string(&binding).expect("binding serialize");
    let binding2: Binding = serde_json::from_str(&binding_json).expect("binding deserialize");
    assert_eq!(
        binding, binding2,
        "Binding serde round-trip must be byte-stable"
    );

    let resolution_json = serde_json::to_string(&resolution).expect("resolution serialize");
    let resolution2: Resolution =
        serde_json::from_str(&resolution_json).expect("resolution deserialize");
    assert_eq!(
        resolution, resolution2,
        "Resolution serde round-trip must be byte-stable"
    );

    // Re-serialise and compare strings (byte-stability check)
    let root_json2 = serde_json::to_string(&root2).expect("root re-serialize");
    assert_eq!(
        root_json, root_json2,
        "Root JSON must be byte-stable across round-trips"
    );
}

/// CfgCond::compatible / exclusive semantics.
#[test]
fn test_cfg_cond_compatible_exclusive() {
    // True is compatible with everything and exclusive with nothing
    assert!(CfgCond::True.compatible(&CfgCond::True));
    assert!(!CfgCond::True.exclusive(&CfgCond::True));

    // Identical atoms are compatible, not exclusive
    let unix = CfgCond::Atom("target_os".to_string(), Some("linux".to_string()));
    let unix2 = CfgCond::Atom("target_os".to_string(), Some("linux".to_string()));
    assert!(unix.compatible(&unix2));
    assert!(!unix.exclusive(&unix2));

    // Two distinct values for the same key ARE exclusive
    let windows = CfgCond::Atom("target_os".to_string(), Some("windows".to_string()));
    assert!(!unix.compatible(&windows));
    assert!(unix.exclusive(&windows));

    // Unknown / different keys: conservative — compatible, not exclusive
    let feat_a = CfgCond::Atom("feature".to_string(), Some("a".to_string()));
    let feat_b = CfgCond::Atom("feature".to_string(), Some("b".to_string()));
    // Heuristic: same key + different Some value ⇒ declared exclusive.
    // This is OVER-APPROXIMATE for cfg(feature="…") (features can co-exist),
    // but conservative for target_os="…" / target_arch="…" etc.
    // A false-positive exclusive is safe: the engine keeps both candidates rather
    // than wrongly merging; it does NOT cause a silent drop.
    assert!(feat_a.exclusive(&feat_b));

    // Atom with no value vs atom with value: same key different shape → conservative compatible
    let key_only = CfgCond::Atom("target_os".to_string(), None);
    // conservative: unknown whether they conflict → compatible=true, exclusive=false
    assert!(key_only.compatible(&unix));
    assert!(!key_only.exclusive(&unix));

    // Not(unix) is exclusive with unix
    let not_unix = CfgCond::Not(Box::new(unix.clone()));
    assert!(unix.exclusive(&not_unix));
    assert!(not_unix.exclusive(&unix));

    // And(unix, windows) is never compatible with itself if unix and windows are exclusive
    // (we just check that And wrapping works without panic)
    let and_cond = CfgCond::And(vec![unix.clone(), windows.clone()]);
    let _ = and_cond.compatible(&CfgCond::True); // must not panic

    // And/Or fall through to the conservative false (not provably exclusive)
    assert!(!and_cond.exclusive(&CfgCond::True));
    assert!(and_cond.compatible(&CfgCond::True));
    let or_cond = CfgCond::Or(vec![unix.clone(), windows.clone()]);
    assert!(!or_cond.exclusive(&CfgCond::True));
    // Not(x) is NOT exclusive with Not(x) — both hold when x is false
    let not_unix2 = CfgCond::Not(Box::new(unix.clone()));
    assert!(!not_unix.exclusive(&not_unix2));
}

/// `Edge` field access + `BindTarget::Pending` construction + serde round-trip.
#[test]
fn test_edge_and_bind_target_pending_serde() {
    let file_id = FileId(0);
    let from_scope = ScopeId(10);
    let to_scope = ScopeId(11);
    let kind: EdgeKindId = 1; // Glob (conventional)

    let vis = Vis {
        kind: 0u16,
        restrict: None,
        payload: Default::default(),
    };

    // BindTarget::Pending wraps a RawPath + Anchor
    let raw_path = RawPath(vec!["std".to_string(), "collections".to_string()]);
    let anchor = Anchor::default();
    let pending = BindTarget::Pending(raw_path.clone(), anchor.clone());

    // Edge with real field values
    let lo = SourceLoc {
        file: file_id,
        byte: 5,
    };
    let hi = SourceLoc {
        file: file_id,
        byte: 20,
    };
    let span = Span { lo, hi };
    let edge = Edge {
        from: from_scope,
        kind,
        to: BindTarget::Resolved(Target::Scope(to_scope)),
        vis: vis.clone(),
        cond: None,
        order: 0,
        vis_range: Some(span.clone()),
    };

    // field access
    assert_eq!(edge.from, from_scope);
    assert_eq!(edge.kind, kind);
    assert_eq!(edge.order, 0);
    assert!(edge.vis_range.is_some());

    // BindTarget::Pending field access
    if let BindTarget::Pending(ref rp, _) = pending {
        assert_eq!(rp.0, vec!["std".to_string(), "collections".to_string()]);
    } else {
        panic!("expected Pending");
    }

    // serde round-trip for Edge
    let edge_json = serde_json::to_string(&edge).expect("edge serialize");
    let edge2: Edge = serde_json::from_str(&edge_json).expect("edge deserialize");
    assert_eq!(edge, edge2, "Edge serde round-trip must be stable");

    // serde round-trip for BindTarget::Pending
    let pending_json = serde_json::to_string(&pending).expect("pending serialize");
    let pending2: BindTarget = serde_json::from_str(&pending_json).expect("pending deserialize");
    assert_eq!(
        pending, pending2,
        "BindTarget::Pending serde round-trip must be stable"
    );
}
