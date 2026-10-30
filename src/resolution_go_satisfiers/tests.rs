use super::*;
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::sync::atomic::Ordering;

fn files() -> BTreeMap<String, ParsedFile> {
    let mut rows = vec![
        ("p/api.go", "package p\ntype I interface { M(int) int }\ntype Private interface { m() }\ntype Broken interface { Z() }\ntype A struct{}\nfunc (A) M(x int) int { return x }\nfunc (A) m() {}\ntype Pointer struct{}\nfunc (*Pointer) M(x int) int { return x }\ntype Embedded struct { A }\ntype Bad struct{}\nfunc (Bad) M(x string) string { return x }\ntype BadEmbedded struct { Bad }\ntype Conflict struct { A; Pointer }\n".to_string()),
        ("q/types.go", "package q\ntype A struct{}\nfunc (A) M(x int) int { return x }\nfunc (A) m() {}\n".into()),
        ("p/variant_linux.go", "package p\ntype Variant struct{}\nfunc (Variant) M(x int) int { return x }\n".into()),
        ("p/variant_windows.go", "package p\ntype Variant struct{}\nfunc (Variant) M(x int) int { return x+1 }\n".into()),
        ("p/local_test.go", "package p\ntype TestOnly struct{}\nfunc (TestOnly) M(x int) int { return x }\nfunc testUse(i I) { i.M(1) }\n".into()),
        ("p/external_test.go", "package p_test\ntype External struct{}\nfunc (External) M(x int) int { return x }\n".into()),
        ("p/broken.go", "package p\ntype Dup struct{}\nfunc (Dup) Z() {}\nfunc (Dup) Z() {}\n".into()),
        ("p/uncertain.go", "//go:build (broken\n\npackage p\ntype Uncertain struct{}\nfunc (Uncertain) Z() {}\n".into()),
        ("p/tagged.go", "//go:build custom\n\npackage p\ntype Tagged interface { M(int) int }\n".into()),
        ("p/use_linux.go", "package p\nfunc linuxUse(i I) { a := i.M(1); b := i.M(a); sink(b) }\n".into()),
        ("p/use_windows.go", "package p\nfunc windowsUse(i I) { a := i.M(1); sink(a) }\n".into()),
        ("p/cgo.go", "package p\nimport \"C\"\nfunc cgoUse(i I) { i.M(1) }\n".into()),
        ("vendor/v/types.go", "package v\ntype Vendor struct{}\nfunc (Vendor) M(x int) int { return x }\n".into()),
        ("q/internal/r/types.go", "package r\ntype Internal struct{}\nfunc (Internal) M(x int) int { return x }\n".into()),
    ];
    for name in ["many", "other"] {
        let mut source = format!("package p\nfunc {name}(i I) {{\n");
        for n in 0..20 {
            source.push_str(&format!("v{n} := i.M({n}); sink(v{n})\n"));
        }
        source.push_str("}\n");
        rows.push((
            if name == "many" {
                "p/many.go"
            } else {
                "p/other.go"
            },
            source,
        ));
    }
    rows.into_iter()
        .map(|(path, source)| {
            (
                path.into(),
                ParsedFile::parse(path, &source, Language::Go).unwrap(),
            )
        })
        .collect()
}

fn owner(name: &str) -> GoOwnerIdentity {
    GoOwnerIdentity {
        package_dir: "p".into(),
        package_clause: "p".into(),
        name: name.into(),
    }
}

fn names(value: &GoCallerScopedSatisfiers<'_>) -> Vec<String> {
    value
        .selection
        .value
        .as_ref()
        .into_iter()
        .flatten()
        .map(|(name, _)| name.clone())
        .collect()
}

#[test]
fn full_values_match_reference_and_repeated_queries_compute_once() {
    let files = files();
    let graph = CallGraph::build(&files);
    let memo = GoSatisfierMemo::new(&graph);
    let mut queries = 0;
    for name in ["I", "Private", "Broken", "Tagged", "Missing"] {
        for method in ["M", "m", "Z", "absent"] {
            for file in files.keys().map(String::as_str).chain(["missing.go"]) {
                for promoted in [false, true] {
                    let expected = graph.go_reference_caller_scoped_satisfiers(
                        &owner(name),
                        method,
                        file,
                        promoted,
                    );
                    for _ in 0..3 {
                        assert!(
                            memo.get(&owner(name), method, file, promoted) == expected,
                            "full satisfier value differs: {name}/{method}/{file}/{promoted}"
                        );
                    }
                    queries += 1;
                }
            }
        }
    }
    assert_eq!(memo.computations.load(Ordering::Relaxed), queries);
    assert!(
        !memo
            .get(&owner("Missing"), "M", "p/many.go", true)
            .interface_selected
    );
    let broken = memo.get(&owner("Broken"), "Z", "p/many.go", true);
    assert!(broken.interface_selected);
    assert!(broken.selection.value.is_none());
    assert!(broken.selection.evidence.conflict);
    assert!(broken.selection.evidence.uncertain);
    let private = memo.get(&owner("Private"), "m", "p/many.go", true);
    assert!(!names(&private).is_empty());
    assert!(private
        .selection
        .value
        .unwrap()
        .iter()
        .all(|(_, target)| target.file.starts_with("p/")));
    assert!(
        memo.get(&owner("I"), "M", "p/many.go", true)
            .promoted
            .signature_drop
            > 0
    );
}

#[test]
fn caller_file_and_promoted_and_method_and_owner_are_discriminators() {
    let graph = CallGraph::build(&files());
    let memo = GoSatisfierMemo::new(&graph);
    let linux = memo.get(&owner("I"), "M", "p/use_linux.go", false);
    let windows = memo.get(&owner("I"), "M", "p/use_windows.go", false);
    assert!(
        linux != windows,
        "same-directory build profiles must not share entries"
    );
    assert_eq!(
        windows,
        graph.go_reference_caller_scoped_satisfiers(&owner("I"), "M", "p/use_windows.go", false)
    );
    let test = memo.get(&owner("I"), "M", "p/local_test.go", false);
    let external = memo.get(&owner("I"), "M", "p/external_test.go", false);
    assert!(names(&test).contains(&"TestOnly".into()));
    assert!(!names(&external).contains(&"TestOnly".into()));
    let ordinary = memo.get(&owner("I"), "M", "p/many.go", false);
    let promoted = memo.get(&owner("I"), "M", "p/many.go", true);
    assert!(!names(&ordinary).contains(&"Embedded".into()));
    assert!(names(&promoted).contains(&"Embedded".into()));
    assert_eq!(
        promoted,
        graph.go_reference_caller_scoped_satisfiers(&owner("I"), "M", "p/many.go", true)
    );
    assert!(memo
        .get(&owner("I"), "absent", "p/many.go", true)
        .selection
        .value
        .unwrap_or_default()
        .is_empty());
    assert!(
        !memo
            .get(&owner("Missing"), "M", "p/many.go", true)
            .interface_selected
    );
    // Same bare interface name in another namespace is a different key too.
    let mut other = owner("I");
    other.package_dir = "q".into();
    other.package_clause = "q".into();
    assert!(!memo.get(&other, "M", "p/many.go", true).interface_selected);
}

#[test]
fn concurrent_hits_share_initialization_and_graph_lifetimes_do_not_leak() {
    let mut graph = CallGraph::build(&files());
    let other = CallGraph::empty();
    {
        let memo = GoSatisfierMemo::new(&graph);
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let memo = &memo;
                let graph = &graph;
                scope.spawn(move || {
                    for file in ["p/many.go", "p/use_linux.go", "missing.go"] {
                        for promoted in [false, true] {
                            let expected = graph.go_reference_caller_scoped_satisfiers(
                                &owner("I"),
                                "M",
                                file,
                                promoted,
                            );
                            for _ in 0..10 {
                                assert_eq!(memo.get(&owner("I"), "M", file, promoted), expected);
                            }
                        }
                    }
                });
            }
        });
        assert_eq!(memo.computations.load(Ordering::Relaxed), 6);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            other.go_caller_scoped_satisfiers(&owner("I"), "M", "p/many.go", false, Some(&memo))
        }))
        .is_err());
    }
    graph.go_method_declarations.clear();
    let memo = GoSatisfierMemo::new(&graph);
    assert_eq!(
        memo.get(&owner("I"), "M", "p/many.go", false),
        graph.go_reference_caller_scoped_satisfiers(&owner("I"), "M", "p/many.go", false)
    );
    assert!(memo
        .get(&owner("I"), "M", "p/many.go", false)
        .selection
        .value
        .unwrap()
        .is_empty());
    // Unwinding and nesting restore the independent reference seam.
    let _ = std::panic::catch_unwind(|| with_reference(|| panic!("fixture unwind")));
    assert!(!reference_enabled());
    with_reference(|| {
        with_reference(|| assert!(reference_enabled()));
        assert!(reference_enabled());
    });
    assert!(!reference_enabled());
}

#[test]
fn resolved_edges_telemetry_and_call_stats_json_match_reference() {
    crate::build_pool::install(|| {
        let files = files();
        let graph = CallGraph::build(&files);
        let memo = GoSatisfierMemo::new(&graph);
        let sites: Vec<_> = graph.calls.values().flatten().collect();
        assert!(sites.len() > 40);
        for _ in 0..3 {
            for site in &sites {
                let actual = memo.resolve_call_site_full(site);
                let expected = with_reference(|| graph.resolve_call_site_full(site));
                assert_eq!(format!("{actual:?}"), format!("{expected:?}"), "{site:?}");
            }
        }
        assert!(
            memo.computations.load(Ordering::Relaxed) > 0,
            "real resolver must use the memo"
        );
        assert!(
            memo.computations.load(Ordering::Relaxed) < 40,
            "many sites must share entries"
        );
        let (candidate, counts) = observe_phases(files.keys().cloned().collect(), || {
            crate::cpg::CodePropertyGraph::build(&files)
        });
        assert_eq!(counts.len(), 3, "steps 5, 5b, 5c each own a memo");
        assert!(
            counts.iter().all(|count| *count > 0 && *count < 40),
            "phase computations: {counts:?}"
        );
        let (reference, counts) = observe_phases(files.keys().cloned().collect(), || {
            with_reference(|| crate::cpg::CodePropertyGraph::build(&files))
        });
        assert_eq!(
            counts,
            vec![0, 0, 0],
            "reference must bypass every phase memo, including Rayon workers"
        );
        assert_eq!(candidate.call_graph.calls, reference.call_graph.calls);
        assert_eq!(
            crate::cpg::dfg_observation::fieldwise_payload(&candidate),
            crate::cpg::dfg_observation::fieldwise_payload(&reference)
        );
        assert_eq!(candidate.return_flow_stats, reference.return_flow_stats);
        assert_eq!(candidate.dfg_label_stats, reference.dfg_label_stats);
        let actual = crate::navigation::queries::call_stats(&candidate.call_graph);
        let expected =
            with_reference(|| crate::navigation::queries::call_stats(&reference.call_graph));
        assert_eq!(
            serde_json::to_vec_pretty(&actual).unwrap(),
            serde_json::to_vec_pretty(&expected).unwrap()
        );
    });
}

#[test]
fn phase_observation_excludes_unrelated_graphs() {
    crate::build_pool::install(|| {
        let files = files();
        let graph = CallGraph::build(&files);
        let other = CallGraph::empty();
        let (_, counts) = observe_phases(files.keys().cloned().collect(), || {
            drop(GoSatisfierMemo::new(&other));
            let (_, nested) = observe_phases(Vec::new(), || drop(GoSatisfierMemo::new(&other)));
            assert_eq!(nested, vec![0]);
            drop(GoSatisfierMemo::new(&graph));
        });
        assert_eq!(
            counts,
            vec![0],
            "only the fixture graph belongs to this observer"
        );
        let _ =
            std::panic::catch_unwind(|| observe_phases(Vec::new(), || panic!("observer unwind")));
        assert!(PHASE_COUNTS.with(|slot| slot.borrow().is_none()));
    });
}
