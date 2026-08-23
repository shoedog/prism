//! Slice 4 part B (roadmap #14 / #17-narrow R1(b) record): owner/profile-keyed
//! promoted-selector snapshot — FOUNDATION ONLY. Nothing here may change
//! resolution routing: no new edges, no removed edges. The snapshot records,
//! per outer owner P10 identity and PER DECLARING FILE/PROFILE: embedded
//! fields as (pointer-ness, RESOLVED embedded owner identity, selector name),
//! ordinary field selector names, own-method names, and per-promoted-method
//! facts, plus an explicit profile-conflict verdict computed over the FOUR
//! known axes (embed tuple, ordinary fields, own methods, embedded-alias
//! selector names).

use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
use std::collections::BTreeMap;

fn build_go(sources: &[(&str, &str)]) -> CallGraph {
    let files: BTreeMap<String, ParsedFile> = sources
        .iter()
        .map(|(path, source)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, source, Language::Go).expect("parse Go fixture"),
            )
        })
        .collect();
    CallGraph::build(&files)
}

fn build_go_with_module(sources: &[(&str, &str)], module_path: &str) -> CallGraph {
    let files: BTreeMap<String, ParsedFile> = sources
        .iter()
        .map(|(path, source)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, source, Language::Go).expect("parse Go fixture"),
            )
        })
        .collect();
    let repo = tempfile::tempdir().expect("temporary Go module root");
    std::fs::write(
        repo.path().join("go.mod"),
        format!("module {module_path}\n\ngo 1.22\n"),
    )
    .expect("write go.mod fixture");
    let inputs = prism::repo_loader::scope_graph_build_inputs(repo.path(), &files);
    CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
}

fn stats(cg: &CallGraph) -> serde_json::Value {
    prism::navigation::queries::call_stats(cg)
}

fn snapshot_counts(cg: &CallGraph) -> (usize, usize, usize) {
    let snapshot = cg.go_promoted_snapshot();
    (
        snapshot.owners.len(),
        snapshot
            .owners
            .values()
            .filter(|owner| {
                owner.verdict == prism::go_promoted_snapshot::GoPromotionVerdict::ProfileConflict
            })
            .count(),
        snapshot
            .owners
            .values()
            .map(|owner| owner.promoted.len())
            .sum(),
    )
}

#[test]
fn snapshot_embed_identity_axis_conflict_across_profiles() {
    // Axis 1: linux declares S{B}, windows declares S{C}.
    let cg = build_go(&[
        (
            "pkg/s_linux.go",
            "//go:build linux\npackage pkg\ntype B struct{}\ntype S struct { B }\n",
        ),
        (
            "pkg/s_windows.go",
            "//go:build windows\npackage pkg\ntype C struct{}\ntype S struct { C }\n",
        ),
        (
            "pkg/b_extra.go",
            "//go:build windows\npackage pkg\ntype B struct{}\n",
        ),
    ]);
    assert_eq!(snapshot_counts(&cg), (3, 1, 0));
    assert_eq!(stats(&cg)["go_promoted_snapshot_profile_conflicts"], 1);
}

#[test]
fn snapshot_ordinary_field_axis_conflict_across_profiles() {
    // Axis 2: S{B} vs S{B; M func()} — an extra ordinary field.
    let cg = build_go(&[
        ("pkg/base.go", "package pkg\ntype B struct{}\n"),
        (
            "pkg/s_plain.go",
            "//go:build !extra\npackage pkg\ntype S struct { B }\n",
        ),
        (
            "pkg/s_field.go",
            "//go:build extra\npackage pkg\ntype S struct { B; M func() }\n",
        ),
    ]);
    assert_eq!(snapshot_counts(&cg), (2, 1, 0));
}

#[test]
fn snapshot_own_method_axis_conflict_across_profiles() {
    // Axis 3: S{B} plus a profile-specific own method on S.
    let cg = build_go(&[
        ("pkg/base.go", "package pkg\ntype B struct{}\n"),
        (
            "pkg/s_base.go",
            "package pkg\ntype S struct { B }\nfunc (s S) Shared() {}\n",
        ),
        (
            "pkg/s_m_linux.go",
            "//go:build linux\npackage pkg\nfunc (s S) M() {}\n",
        ),
    ]);
    assert_eq!(snapshot_counts(&cg), (2, 1, 0));
}

#[test]
fn snapshot_embedded_alias_selector_name_axis_is_recorded_separately_from_resolved_identity() {
    // Axis 4: `type A = B` embedded as S{A} exposes selector `A` while the
    // twin declares S{B}: resolved embedded OWNER IDENTITY is B on both sides,
    // so ONLY the carried selector name distinguishes them.
    let cg = build_go(&[
        ("pkg/base.go", "package pkg\ntype B struct{}\n"),
        (
            "pkg/s_alias.go",
            "//go:build aliasway\npackage pkg\ntype A = B\ntype S struct { A }\n",
        ),
        (
            "pkg/s_direct.go",
            "//go:build !aliasway\npackage pkg\ntype S struct { B }\n",
        ),
    ]);
    let (owners, conflicts, _promoted) = snapshot_counts(&cg);
    assert_eq!((owners, conflicts), (2, 1));
}

#[test]
fn snapshot_package_qualifier_embed_resolves_to_different_identities() {
    // `struct{ q.B }` vs `struct{ r.B }`: different package qualifiers are
    // different embedded owners — the resolution must carry the identity.
    let cg = build_go(&[
        ("q/q.go", "package q\ntype B struct{}\nfunc (B) M() {}\n"),
        ("r/r.go", "package r\ntype B struct{}\nfunc (B) M() {}\n"),
        (
            "pkg/s_q.go",
            "//go:build useq\npackage pkg\nimport q \"example.com/prism/q\"\ntype S struct { q.B }\n",
        ),
        (
            "pkg/s_r.go",
            "//go:build !useq\npackage pkg\nimport r \"example.com/prism/r\"\ntype S struct { r.B }\n",
        ),
    ]);
    assert_eq!(snapshot_counts(&cg), (3, 1, 0));
}

#[test]
fn snapshot_anonymous_struct_embed_fails_closed() {
    let cg = build_go(&[(
        "pkg/s.go",
        "package pkg\ntype S struct { struct{ M func() } }\n",
    )]);
    let (_owners, conflicts, promoted) = snapshot_counts(&cg);
    assert_eq!((conflicts, promoted), (1, 0));
}

#[test]
fn snapshot_depth2_path_owner_conflict_poisons_the_outer_owner() {
    let cg = build_go(&[
        ("pkg/mid_linux.go", "//go:build linux\npackage pkg\ntype Leaf struct{}\ntype Mid struct { Leaf }\nfunc (Leaf) Deep() {}\n"),
        ("pkg/mid_windows.go", "//go:build windows\npackage pkg\ntype Leaf struct{}\ntype Other struct{}\ntype Mid struct { Other }\nfunc (Leaf) Deep() {}\n"),
        ("pkg/top.go", "package pkg\ntype Top struct { Mid }\n"),
    ]);
    // Mid conflicts on the embed axis; Top sits on a promotion path through
    // Mid, so it must be conflicted too (every hop profile-unique).
    let (_owners, conflicts, promoted) = snapshot_counts(&cg);
    assert_eq!((conflicts, promoted), (2, 0));
}

#[test]
fn snapshot_duplicate_identical_declarations_are_not_a_conflict() {
    let cg = build_go(&[
        (
            "pkg/base.go",
            "package pkg\ntype B struct{}\nfunc (b *B) Write() {}\n",
        ),
        ("pkg/s_one.go", "package pkg\ntype S struct { B }\n"),
        ("pkg/s_two.go", "package pkg\ntype S struct { B }\n"),
    ]);
    let (owners, conflicts, promoted) = snapshot_counts(&cg);
    assert_eq!((owners, conflicts), (2, 0));
    // S promotes *B.Write at depth 1; pointer-receiver method reached through
    // a non-pointer embed is NOT in the value method set.
    let snapshot = cg.go_promoted_snapshot();
    let s_owner = snapshot
        .owners
        .iter()
        .find(|(owner, _)| owner.name == "S")
        .map(|(_, v)| v)
        .expect("S owner");
    let write = &s_owner.promoted["Write"];
    assert_eq!(write.depth, 1);
    assert!(!write.value_method_set);
    assert!(!write.shadowed_by_field);
    assert_eq!((owners, conflicts, promoted), (2, 0, 1));
}

#[test]
fn snapshot_records_field_shadowing_and_value_method_set_bit() {
    let cg = build_go(&[
        (
            "pkg/base.go",
            "package pkg\ntype B struct{}\nfunc (b B) Value() {}\nfunc (b *B) Ptr() {}\n",
        ),
        ("pkg/s.go", "package pkg\ntype S struct { B; Value int }\n"),
    ]);
    let snapshot = cg.go_promoted_snapshot();
    let s = snapshot
        .owners
        .iter()
        .find(|(owner, _)| owner.name == "S")
        .map(|(_, v)| v)
        .expect("S owner");
    assert_eq!(
        s.verdict,
        prism::go_promoted_snapshot::GoPromotionVerdict::Consistent
    );
    // `Value` is shadowed by the ordinary field at depth 0.
    assert!(s.promoted["Value"].shadowed_by_field);
    assert!(!s.promoted["Ptr"].shadowed_by_field);
    // Value-receiver methods promote into the value set; pointer-receiver do not.
    assert!(s.promoted["Value"].value_method_set);
    assert!(!s.promoted["Ptr"].value_method_set);
}

#[test]
fn snapshot_foundation_does_not_change_resolution_and_survives_caches_byte_identically() {
    let sources: &[(&str, &str)] = &[
        ("pkg/base.go", "package pkg\ntype B struct{}\nfunc (b B) Act() {}\n"),
        ("pkg/s.go", "package pkg\ntype Doer interface { Act() }\ntype Holder struct { Doer }\ntype S struct { B }\nfunc invoke(h Holder, s S) { h.Act(); s.Act() }\n"),
    ];
    let files: BTreeMap<String, ParsedFile> = sources
        .iter()
        .map(|(path, src)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, src, Language::Go).unwrap(),
            )
        })
        .collect();
    let fresh = CallGraph::build(&files);

    // Exact CPG cache hit: serialize/deserialize the whole graph bincode-wise
    // and require byte-equal re-serialization (full state parity).
    let fresh_bytes = bincode::serialize(&fresh).expect("serialize CallGraph");
    let restored: CallGraph = bincode::deserialize(&fresh_bytes).expect("deserialize CallGraph");
    let restored_bytes = bincode::serialize(&restored).expect("re-serialize CallGraph");
    assert_eq!(
        fresh_bytes, restored_bytes,
        "cache round-trip must be byte-equal"
    );

    // Resolution leaves identical to a graph whose snapshot was cleared:
    // foundation only — the snapshot must not influence routing.
    let mut stripped = fresh.clone();
    *stripped.go_promoted_snapshot_mut() = Default::default();
    let fresh_sites: Vec<_> = fresh.calls.values().flatten().collect();
    let stripped_sites: Vec<_> = stripped.calls.values().flatten().collect();
    assert_eq!(fresh_sites.len(), stripped_sites.len());
    for (index, (site_a, site_b)) in fresh_sites.iter().zip(&stripped_sites).enumerate() {
        assert_eq!(site_a.caller, site_b.caller);
        assert_eq!(site_a.line, site_b.line);
        assert_eq!(site_a.receiver_type, site_b.receiver_type);
        let a = fresh.resolve_call_site_full(site_a);
        let b = stripped.resolve_call_site_full(site_b);
        assert_eq!(
            a.resolved
                .iter()
                .map(|r| (r.target.clone(), r.confidence))
                .collect::<Vec<_>>(),
            b.resolved
                .iter()
                .map(|r| (r.target.clone(), r.confidence))
                .collect::<Vec<_>>(),
            "resolution must not depend on the snapshot (foundation only); pair {index}"
        );
    }

    let json = stats(&fresh);
    assert_eq!(json["go_promoted_snapshot_owners"], 3);
    assert_eq!(json["go_promoted_snapshot_profile_conflicts"], 0);
    assert_eq!(json["go_promoted_snapshot_promoted_methods"], 1);
}

#[test]
fn s4ox_embedded_alias_resolves_to_its_owner_identity() {
    // SOL-W6: `type A = B` embedded as S{A} records the exact key
    // (is_pointer=false, resolved owner B, selector "A").
    let cg = build_go(&[
        (
            "pkg/base.go",
            "package pkg\ntype B struct{}\nfunc (b B) Act() {}\n",
        ),
        (
            "pkg/alias.go",
            "package pkg\ntype A = B\ntype S struct { A }\n",
        ),
    ]);
    let snapshot = cg.go_promoted_snapshot();
    let s = snapshot
        .owners
        .iter()
        .find(|(owner, _)| owner.name == "S")
        .map(|(_, v)| v)
        .expect("S owner");
    assert_eq!(
        s.verdict,
        prism::go_promoted_snapshot::GoPromotionVerdict::Consistent
    );
    let embed = s
        .declarations
        .values()
        .next()
        .unwrap()
        .embeds
        .iter()
        .next()
        .unwrap();
    assert_eq!(embed.selector, "A");
    assert!(!embed.is_pointer);
    assert!(!embed.is_interface);
    let resolved = embed.resolved_owner.as_ref().expect("resolved to B");
    assert_eq!(resolved.name, "B");
    // Promotion through the ALIAS still walks: B.Act promotes to S at depth 1.
    assert_eq!(s.promoted.len(), 1);
    assert_eq!(s.promoted["Act"].depth, 1);
}

#[test]
fn s4ox_qualified_embedded_interface_is_deferred_not_conflicted() {
    // SOL-W7 converse: `struct{ q.I }` resolves q.I and must reclassify it as
    // an INTERFACE (deferral), never a ProfileConflict.
    // Qualified resolution needs the module-graph path proof, so this fixture
    // runs under a go.mod (P10 identity), not the bare builder.
    let cg = build_go_with_module(
        &[
            ("q/q.go", "package q\ntype I interface { M() }\n"),
            (
                "pkg/s.go",
                "package pkg\nimport q \"example.com/prism/q\"\ntype S struct { q.I }\n",
            ),
        ],
        "example.com/prism",
    );
    let snapshot = cg.go_promoted_snapshot();
    let s = snapshot
        .owners
        .iter()
        .find(|(owner, _)| owner.name == "S")
        .map(|(_, v)| v)
        .expect("S owner");
    let embed = s
        .declarations
        .values()
        .next()
        .unwrap()
        .embeds
        .iter()
        .next()
        .unwrap();
    assert!(embed.is_interface);
    assert_eq!(
        s.verdict,
        prism::go_promoted_snapshot::GoPromotionVerdict::Consistent
    );
}

#[test]
fn s4ox_profile_divergent_embedded_interface_conflicts() {
    // fix-3 / SOL-W7: I's method surface varies by profile; untagged S{I}
    // must be conflicted even though S itself has one declaration.
    let divergent = build_go(&[
        (
            "pkg/i_linux.go",
            "//go:build linux\npackage pkg\ntype I interface { M() }\n",
        ),
        (
            "pkg/i_windows.go",
            "//go:build windows\npackage pkg\ntype I interface { N() }\n",
        ),
        ("pkg/s.go", "package pkg\ntype S struct { I }\n"),
    ]);
    let (_owners, conflicts, _promoted) = snapshot_counts(&divergent);
    assert_eq!(conflicts, 1);

    // Control: identical I on both profiles stays Consistent.
    let identical = build_go(&[
        (
            "pkg/i_linux.go",
            "//go:build linux\npackage pkg\ntype I interface { M() }\n",
        ),
        (
            "pkg/i_windows.go",
            "//go:build windows\npackage pkg\ntype I interface { M() }\n",
        ),
        ("pkg/s.go", "package pkg\ntype S struct { I }\n"),
    ]);
    let (_owners, conflicts, _promoted) = snapshot_counts(&identical);
    assert_eq!(conflicts, 0);
}

#[test]
fn s4ox_own_method_axis_includes_receiver_kind_and_target_identity() {
    // SOL-W8 (the fifth axis): linux `func (B) M()` vs windows `func (*B) M()`
    // have the same NAME but different method-set shape => conflict, and the
    // promoted map must not depend on file insertion order.
    let cg = build_go(&[
        ("pkg/base.go", "package pkg\ntype B struct{}\n"),
        (
            "pkg/m_linux.go",
            "//go:build linux\npackage pkg\nfunc (b B) M() {}\n",
        ),
        (
            "pkg/m_windows.go",
            "//go:build windows\npackage pkg\nfunc (b *B) M() {}\n",
        ),
        ("pkg/s.go", "package pkg\ntype S struct { B }\n"),
    ]);
    let (_owners, conflicts, promoted) = snapshot_counts(&cg);
    assert_eq!((conflicts, promoted), (2, 0));
}

#[test]
fn s4ox_alias_to_local_interface_is_deferred_not_conflicted() {
    // terra-r2-2 / sol-r2-5: `type A = I` embedded as S{A} must reclassify
    // the RESOLVED owner I as an interface and defer, staying Consistent.
    let cg = build_go(&[
        ("pkg/i.go", "package pkg\ntype I interface { M() }\ntype A = I\ntype S struct { A }\n"),
    ]);
    let snapshot = cg.go_promoted_snapshot();
    let s = snapshot
        .owners
        .iter()
        .find(|(owner, _)| owner.name == "S")
        .map(|(_, v)| v)
        .expect("S");
    assert_eq!(s.verdict, prism::go_promoted_snapshot::GoPromotionVerdict::Consistent);
    let embed = s.declarations.values().next().unwrap().embeds.iter().next().unwrap();
    assert!(embed.is_interface);
    assert_eq!(embed.selector, "A");
}

#[test]
fn s4ox_qualified_alias_embed_resolves_to_target() {
    // sol-r2-6 (alias half) + terra-r2-3: `type A = q.B` embedded as S{A}
    // resolves through BOTH hops; S promotes q.B's method at depth 1.
    fn build_mod(sources: &[(&str, &str)]) -> CallGraph {
        let files: BTreeMap<String, ParsedFile> = sources
            .iter()
            .map(|(path, source)| {
                (
                    (*path).to_string(),
                    ParsedFile::parse(path, source, Language::Go).expect("parse"),
                )
            })
            .collect();
        let repo = tempfile::tempdir().expect("tempdir");
        std::fs::write(repo.path().join("go.mod"), "module example.com/prism\n\ngo 1.22\n").unwrap();
        let inputs = prism::repo_loader::scope_graph_build_inputs(repo.path(), &files);
        CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
    }
    let concrete = build_mod(&[
        ("q/q.go", "package q\ntype B struct{}\nfunc (b B) M() {}\n"),
        ("pkg/s.go", "package pkg\nimport q \"example.com/prism/q\"\ntype S struct { q.B }\n"),
    ]);
    {
        let snapshot = concrete.go_promoted_snapshot();
        let s = snapshot.owners.iter().find(|(o, _)| o.name == "S").map(|(_, v)| v).expect("S");
        assert_eq!(s.verdict, prism::go_promoted_snapshot::GoPromotionVerdict::Consistent);
        let m = &s.promoted["M"];
        assert_eq!(m.target_owner.name, "B");
        assert_eq!(m.depth, 1);
        assert_eq!(m.function_id.file, "q/q.go");
    }

    let aliased = build_mod(&[
        ("q/q.go", "package q\ntype B struct{}\nfunc (b B) M() {}\n"),
        ("pkg/s.go", "package pkg\nimport q \"example.com/prism/q\"\ntype A = q.B\ntype S struct { A }\n"),
    ]);
    let snapshot = aliased.go_promoted_snapshot();
    let s = snapshot.owners.iter().find(|(o, _)| o.name == "S").map(|(_, v)| v).expect("S");
    assert_eq!(s.verdict, prism::go_promoted_snapshot::GoPromotionVerdict::Consistent);
    let embed = s.declarations.values().next().unwrap().embeds.iter().next().unwrap();
    assert_eq!(embed.resolved_owner.as_ref().expect("resolved q.B").name, "B");
    assert_eq!(embed.selector, "A");
    assert_eq!(s.promoted["M"].depth, 1);
}

#[test]
fn s4ox_promotion_follows_go_shallowest_selector_rule() {
    // sol-r2-7: B.M (depth 1) shadows C.M (depth 2); equal-depth ambiguity
    // records NO method (fail closed), never an arbitrary one.
    let shadowed = build_go(&[
        ("pkg/c.go", "package pkg\ntype C struct{}\nfunc (c C) M() {}\n"),
        ("pkg/b.go", "package pkg\ntype B struct { C }\nfunc (b B) M() {}\n"),
        ("pkg/s.go", "package pkg\ntype S struct { B }\n"),
    ]);
    {
        let snapshot = shadowed.go_promoted_snapshot();
        let s = snapshot.owners.iter().find(|(o, _)| o.name == "S").map(|(_, v)| v).expect("S");
        assert_eq!(s.promoted.len(), 1);
        assert_eq!(s.promoted["M"].target_owner.name, "B");
        assert_eq!(s.promoted["M"].depth, 1);
    }

    let ambiguous = build_go(&[
        ("pkg/b.go", "package pkg\ntype B struct{}\nfunc (b B) M() {}\n"),
        ("pkg/c.go", "package pkg\ntype C struct{}\nfunc (c C) M() {}\n"),
        ("pkg/s.go", "package pkg\ntype S struct { B; C }\n"),
    ]);
    {
        let snapshot = ambiguous.go_promoted_snapshot();
        let s = snapshot.owners.iter().find(|(o, _)| o.name == "S").map(|(_, v)| v).expect("S");
        assert!(!s.promoted.contains_key("M"), "equal-depth ambiguity fails closed");
    }
}

#[test]
fn s4ox_embedded_interface_profile_check_includes_signatures() {
    // terra-r2-4 / sol-r2-8: I{M(int)} vs I{M(string)} is a profile-dependent
    // surface even though the NAME sets agree.
    let divergent = build_go(&[
        ("pkg/i_linux.go", "//go:build linux\npackage pkg\ntype I interface { M(int) }\n"),
        ("pkg/i_windows.go", "//go:build windows\npackage pkg\ntype I interface { M(string) }\n"),
        ("pkg/s.go", "package pkg\ntype S struct { I }\n"),
    ]);
    let (_owners, conflicts, _promoted) = snapshot_counts(&divergent);
    assert_eq!(conflicts, 1);
}

#[test]
fn s4ox_qualified_alias_lookup_regression_tuple_range() {
    // SMELL-r2-1: targeted regression for the clause-range root cause — a
    // qualified package holding the requested alias PLUS unrelated types in
    // several files must not pollute the leaf's variant set.
    fn build_mod(sources: &[(&str, &str)]) -> CallGraph {
        let files: BTreeMap<String, ParsedFile> = sources
            .iter()
            .map(|(path, source)| {
                (
                    (*path).to_string(),
                    ParsedFile::parse(path, source, Language::Go).expect("parse"),
                )
            })
            .collect();
        let repo = tempfile::tempdir().expect("tempdir");
        std::fs::write(repo.path().join("go.mod"), "module example.com/prism\n\ngo 1.22\n").unwrap();
        let inputs = prism::repo_loader::scope_graph_build_inputs(repo.path(), &files);
        CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
    }
    let cg = build_mod(&[
        (
            "base/a.go",
            "package base\ntype Decoy struct{}\nfunc (Decoy) Act(t T) {}\n",
        ),
        ("base/t.go", "package base\ntype T struct{}\ntype AliasT = int\n"),
        ("base/more.go", "package base\ntype Other struct{}\n"),
        (
            "app/use.go",
            "package app\nimport \"example.com/prism/base\"\ntype Doer interface { Act(base.T) }\ntype Holder struct { Doer }\nfunc invoke(h Holder) { h.Act(base.T{}) }\n",
        ),
    ]);
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|s| s.caller.name == "invoke" && s.callee_name == "Act")
        .expect("site");
    let owners: Vec<_> = cg
        .resolve_call_site_full(site)
        .resolved
        .iter()
        .filter_map(|r| cg.method_owners.get(r.target).cloned())
        .collect();
    assert_eq!(owners, vec!["Decoy".to_string()]);
}
