use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::go_promoted_snapshot::{
    GoPromotedOwnerSnapshot, GoPromotedSelectorSnapshot, GoPromotedSnapshotVerdict,
};
use prism::languages::Language;
use prism::resolution::GoOwnerIdentity;
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

fn build_go_module(sources: &[(&str, &str)]) -> CallGraph {
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
        "module example.test/root\n\ngo 1.24\n",
    )
    .expect("write go.mod fixture");
    let inputs = prism::repo_loader::scope_graph_build_inputs(repo.path(), &files);
    CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
}

fn snapshot(cg: &CallGraph) -> &GoPromotedSelectorSnapshot {
    cg.go_promoted_selector_snapshot()
}

fn owner<'a>(
    cg: &'a CallGraph,
    package_dir: &str,
    package_clause: &str,
    name: &str,
) -> &'a GoPromotedOwnerSnapshot {
    snapshot(cg)
        .owners
        .get(&GoOwnerIdentity {
            package_dir: package_dir.to_string(),
            package_clause: package_clause.to_string(),
            name: name.to_string(),
        })
        .unwrap_or_else(|| panic!("missing snapshot owner {package_dir}:{package_clause}.{name}"))
}

fn assert_conflict(cg: &CallGraph, owner_name: &str) {
    assert_eq!(
        owner(cg, "", "p", owner_name).verdict,
        GoPromotedSnapshotVerdict::ProfileConflict
    );
}

fn base() -> (&'static str, &'static str) {
    (
        "base.go",
        "package p\ntype B struct{}\nfunc (B) M() {}\ntype C struct{}\ntype D struct{}\n",
    )
}

#[test]
fn package_qualifier_is_part_of_profile_uniqueness() {
    let cg = build_go_module(&[
        ("q/b.go", "package q\ntype B struct{}\nfunc (B) M() {}\n"),
        ("r/b.go", "package r\ntype B struct{}\nfunc (B) M() {}\n"),
        (
            "outer/s_linux.go",
            "package outer\nimport q \"example.test/root/q\"\ntype S struct{ q.B }\n",
        ),
        (
            "outer/s_windows.go",
            "package outer\nimport r \"example.test/root/r\"\ntype S struct{ r.B }\n",
        ),
    ]);
    assert_eq!(
        owner(&cg, "outer", "outer", "S").verdict,
        GoPromotedSnapshotVerdict::ProfileConflict
    );
}

#[test]
fn resolved_embedded_owner_identity_is_part_of_profile_uniqueness() {
    let base = base();
    let cg = build_go(&[
        base,
        ("s_linux.go", "package p\ntype S struct{ B }\n"),
        ("s_windows.go", "package p\ntype S struct{ C }\n"),
    ]);
    assert_conflict(&cg, "S");
}

#[test]
fn ordinary_field_selector_names_are_part_of_profile_uniqueness() {
    let base = base();
    let cg = build_go(&[
        base,
        ("s_linux.go", "package p\ntype S struct{ B }\n"),
        ("s_windows.go", "package p\ntype S struct{ B; M func() }\n"),
    ]);
    assert_conflict(&cg, "S");
}

#[test]
fn own_method_names_are_part_of_profile_uniqueness() {
    let base = base();
    let cg = build_go(&[
        base,
        (
            "s_linux.go",
            "package p\ntype S struct{ B }\nfunc (S) M() {}\n",
        ),
        ("s_windows.go", "package p\ntype S struct{ B }\n"),
    ]);
    assert_conflict(&cg, "S");
}

#[test]
fn embedded_alias_selector_name_is_preserved_separately_from_target_identity() {
    let base = base();
    let cg = build_go(&[
        base,
        ("alias.go", "package p\ntype A = B\n"),
        ("s_linux.go", "package p\ntype S struct{ A }\n"),
        ("s_windows.go", "package p\ntype S struct{ B }\n"),
    ]);
    let s = owner(&cg, "", "p", "S");
    assert_eq!(s.verdict, GoPromotedSnapshotVerdict::ProfileConflict);
    let linux = s
        .declarations
        .iter()
        .find(|declaration| declaration.defining_file == "s_linux.go")
        .expect("linux S declaration");
    let embedded = linux.embedded_fields.iter().next().expect("embedded alias");
    assert_eq!(embedded.selector, "A");
    assert_eq!(embedded.target.name, "B");
}

#[test]
fn anonymous_struct_embed_is_an_explicit_profile_conflict() {
    let cg = build_go(&[("s.go", "package p\ntype S struct { struct { X int } }\n")]);
    assert_conflict(&cg, "S");
}

#[test]
fn unresolvable_embedded_identity_is_an_explicit_profile_conflict() {
    let cg = build_go(&[("s.go", "package p\ntype S struct{ Missing }\n")]);
    assert_conflict(&cg, "S");
}

#[test]
fn resolved_embedded_interface_is_not_an_unresolvable_identity() {
    let cg = build_go(&[(
        "s.go",
        "package p\ntype I interface{ M() }\ntype S struct{ I }\n",
    )]);
    let s = owner(&cg, "", "p", "S");
    assert_eq!(s.verdict, GoPromotedSnapshotVerdict::ProfileUnique);
    let embedded = s.declarations[0]
        .embedded_fields
        .iter()
        .next()
        .expect("resolved embedded interface identity");
    assert_eq!(embedded.target.name, "I");
    assert!(s.declarations[0].promoted_methods.is_empty());
}

#[test]
fn embedded_interface_profile_method_divergence_conflicts_outer_owner() {
    let cg = build_go(&[
        ("b_linux.go", "package p\ntype B interface{ M() }\n"),
        ("b_windows.go", "package p\ntype B interface{ N() }\n"),
        ("s_linux.go", "package p\ntype S struct{ B }\n"),
        ("s_windows.go", "package p\ntype S struct{ B }\n"),
    ]);
    assert_conflict(&cg, "S");
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["go_promoted_snapshot_owners"], 1);
    assert_eq!(stats["go_promoted_snapshot_profile_conflicts"], 1);
}

#[test]
fn identical_embedded_interface_profiles_keep_outer_owner_unique() {
    let cg = build_go(&[
        ("b_linux.go", "package p\ntype B interface{ M() }\n"),
        ("b_windows.go", "package p\ntype B interface{ M() }\n"),
        ("s_linux.go", "package p\ntype S struct{ B }\n"),
        ("s_windows.go", "package p\ntype S struct{ B }\n"),
    ]);
    assert_eq!(
        owner(&cg, "", "p", "S").verdict,
        GoPromotedSnapshotVerdict::ProfileUnique
    );
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["go_promoted_snapshot_owners"], 1);
    assert_eq!(stats["go_promoted_snapshot_profile_conflicts"], 0);
}

#[test]
fn embedded_defined_type_contributes_its_promoted_method() {
    let cg = build_go(&[(
        "s.go",
        "package p\ntype D int\nfunc (D) M() {}\ntype S struct{ D }\n",
    )]);
    let s = owner(&cg, "", "p", "S");
    assert_eq!(s.verdict, GoPromotedSnapshotVerdict::ProfileUnique);
    let promoted = s.declarations[0]
        .promoted_methods
        .iter()
        .find(|method| method.method == "M")
        .expect("D.M promoted to S");
    assert_eq!(promoted.target_owner.name, "D");
    assert_eq!(promoted.depth, 1);
}

#[test]
fn depth_two_profile_conflict_taints_the_outer_owner() {
    let base = base();
    let cg = build_go(&[
        base,
        ("b_linux.go", "package p\ntype X struct{ C }\n"),
        ("b_windows.go", "package p\ntype X struct{ D }\n"),
        ("s.go", "package p\ntype S struct{ X }\n"),
    ]);
    assert_conflict(&cg, "X");
    assert_conflict(&cg, "S");
}

#[test]
fn duplicate_identical_profile_declarations_are_not_a_conflict() {
    let base = base();
    let cg = build_go(&[
        base,
        ("s_linux.go", "package p\ntype S struct{ B }\n"),
        ("s_windows.go", "package p\ntype S struct{ B }\n"),
    ]);
    let s = owner(&cg, "", "p", "S");
    assert_eq!(s.verdict, GoPromotedSnapshotVerdict::ProfileUnique);
    assert_eq!(s.declarations.len(), 2);
    for declaration in &s.declarations {
        let promoted = declaration
            .promoted_methods
            .iter()
            .find(|method| method.method == "M")
            .expect("B.M promoted to S");
        assert_eq!(promoted.target_owner.name, "B");
        assert_eq!(promoted.depth, 1);
        assert!(!promoted.field_shadowed);
        assert!(promoted.value_method_set);
        assert_eq!(promoted.target.name, "M");
        assert_eq!(promoted.target.file, "base.go");
    }
}

#[test]
fn promoted_method_facts_preserve_shadow_and_value_method_set() {
    let cg = build_go(&[
        ("base.go", "package p\ntype B struct{}\nfunc (*B) P() {}\n"),
        ("value.go", "package p\ntype Value struct{ B; P int }\n"),
        ("pointer.go", "package p\ntype Pointer struct{ *B }\n"),
    ]);
    let value = owner(&cg, "", "p", "Value").declarations[0]
        .promoted_methods
        .iter()
        .find(|method| method.method == "P")
        .expect("pointer method candidate on value embed");
    assert!(value.field_shadowed);
    assert!(!value.value_method_set);

    let pointer = owner(&cg, "", "p", "Pointer").declarations[0]
        .promoted_methods
        .iter()
        .find(|method| method.method == "P")
        .expect("pointer method promoted through pointer embed");
    assert!(!pointer.field_shadowed);
    assert!(pointer.value_method_set);
}

#[test]
fn promoted_method_facts_preserve_depth_two_target_identity() {
    let cg = build_go(&[
        ("base.go", "package p\ntype C struct{}\nfunc (C) M() {}\n"),
        ("middle.go", "package p\ntype B struct{ C }\n"),
        ("outer.go", "package p\ntype S struct{ B }\n"),
    ]);
    let promoted = owner(&cg, "", "p", "S").declarations[0]
        .promoted_methods
        .iter()
        .find(|method| method.method == "M")
        .expect("C.M promoted through B to S");
    assert_eq!(promoted.target_owner.name, "C");
    assert_eq!(promoted.target.file, "base.go");
    assert_eq!(promoted.depth, 2);
}

#[test]
fn promoted_method_keeps_only_the_shallowest_selector_depth() {
    let cg = build_go(&[(
        "s.go",
        "package p\ntype D struct{}\nfunc (D) M() {}\ntype B struct{ D }\ntype C struct{}\nfunc (C) M() {}\ntype S struct{ B; C }\n",
    )]);
    let promoted = owner(&cg, "", "p", "S").declarations[0]
        .promoted_methods
        .iter()
        .filter(|method| method.method == "M")
        .collect::<Vec<_>>();

    assert_eq!(promoted.len(), 1, "only the shallowest M is selected");
    assert_eq!(promoted[0].target_owner.name, "C");
    assert_eq!(promoted[0].depth, 1);
    assert!(!promoted[0].field_shadowed);
}

#[test]
fn equal_depth_promoted_methods_are_omitted_and_flagged_ambiguous() {
    let cg = build_go(&[(
        "s.go",
        "package p\ntype D struct{}\nfunc (D) M() {}\ntype B struct{ D }\ntype F struct{}\nfunc (F) M() {}\ntype E struct{ F }\ntype S struct{ B; E }\n",
    )]);
    let declaration = &owner(&cg, "", "p", "S").declarations[0];

    assert!(
        declaration
            .promoted_methods
            .iter()
            .all(|method| method.method != "M"),
        "an ambiguous selector is not a promoted method"
    );
    let serialized = serde_json::to_value(declaration).expect("serialize profile snapshot");
    assert_eq!(
        serialized["ambiguous_promoted_methods"],
        serde_json::json!(["M"])
    );
}

#[test]
fn shallower_ordinary_field_keeps_the_method_shadowed_not_selected() {
    let cg = build_go(&[(
        "s.go",
        "package p\ntype D struct{}\nfunc (D) M() {}\ntype B struct{ D }\ntype C struct{ M int }\ntype S struct{ B; C }\n",
    )]);
    let promoted = owner(&cg, "", "p", "S").declarations[0]
        .promoted_methods
        .iter()
        .filter(|method| method.method == "M")
        .collect::<Vec<_>>();

    assert_eq!(
        promoted.len(),
        1,
        "retain the shadowing diagnostic candidate"
    );
    assert_eq!(promoted[0].target_owner.name, "D");
    assert_eq!(promoted[0].depth, 2);
    assert!(promoted[0].field_shadowed);
    assert!(promoted.iter().all(|method| method.field_shadowed));
}

#[test]
fn receiver_method_set_shape_is_a_fifth_profile_safety_axis() {
    let cg = build_go(&[
        (
            "b_linux.go",
            "package p\ntype B struct{}\nfunc (B) M() {}\n",
        ),
        (
            "b_windows.go",
            "package p\ntype B struct{}\nfunc (*B) M() {}\n",
        ),
        ("s_linux.go", "package p\ntype S struct{ B }\n"),
        ("s_windows.go", "package p\ntype S struct{ B }\n"),
    ]);
    assert_conflict(&cg, "B");
    assert_conflict(&cg, "S");
}

#[test]
fn snapshot_counts_reach_call_stats_and_the_manifest_diagnostic() {
    let base = base();
    let cg = build_go(&[
        base,
        ("s_linux.go", "package p\ntype S struct{ B }\n"),
        ("s_windows.go", "package p\ntype S struct{ B }\n"),
    ]);
    let snapshot = snapshot(&cg);
    let owners = snapshot.owners.len();
    let conflicts = snapshot
        .owners
        .values()
        .filter(|owner| owner.verdict == GoPromotedSnapshotVerdict::ProfileConflict)
        .count();
    let promoted = snapshot
        .owners
        .values()
        .flat_map(|owner| &owner.declarations)
        .map(|declaration| declaration.promoted_methods.len())
        .sum::<usize>();
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["go_promoted_snapshot_owners"], owners);
    assert_eq!(stats["go_promoted_snapshot_profile_conflicts"], conflicts);
    assert_eq!(stats["go_promoted_snapshot_promoted_methods"], promoted);
    let manifest = prism::navigation::queries::interface_dispatch_manifest(&cg);
    assert_eq!(manifest["go_promoted_snapshot"]["owners"], owners);
    assert_eq!(
        manifest["go_promoted_snapshot"]["profile_conflicts"],
        conflicts
    );
    assert_eq!(
        manifest["go_promoted_snapshot"]["promoted_methods"],
        promoted
    );
}

#[test]
fn non_go_graph_has_an_empty_snapshot() {
    let files = BTreeMap::from([(
        "main.py".to_string(),
        ParsedFile::parse("main.py", "def f():\n    return 1\n", Language::Python)
            .expect("parse Python fixture"),
    )]);
    let cg = CallGraph::build(&files);
    assert_eq!(snapshot(&cg), &GoPromotedSelectorSnapshot::default());
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["go_promoted_snapshot_owners"], 0);
    assert_eq!(stats["go_promoted_snapshot_profile_conflicts"], 0);
    assert_eq!(stats["go_promoted_snapshot_promoted_methods"], 0);
}
