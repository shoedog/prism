use super::*;
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::sync::atomic::Ordering;

fn profiles() -> BTreeMap<String, GoBuildProfile> {
    let rows = [
        ("p/plain.go", "package p"),
        ("p/other.go", "package p"),
        ("q/plain.go", "package p"),
        ("p/local_test.go", "package p"),
        ("p/external_test.go", "package p_test"),
        ("q/local_test.go", "package p"),
        ("p/use_linux.go", "package p"),
        ("p/use_windows.go", "package p"),
        ("p/use_arm64.go", "package p"),
        ("p/use_amd64.go", "package p"),
        ("p/custom.go", "//go:build custom\n\npackage p"),
        ("p/notcustom.go", "//go:build !custom\n\npackage p"),
        ("p/cgo.go", "//go:build cgo\n\npackage p\nimport \"C\""),
        ("p/nocgo.go", "//go:build !cgo\n\npackage p"),
        ("p/broken.go", "//go:build (bad\n\npackage p"),
        (
            "p/capped.go",
            "//go:build a || b || c || d || e || f || g || h || i\n\npackage p",
        ),
        ("vendor/v/plain.go", "package v"),
        ("q/internal/r/plain.go", "package r"),
        ("root.go", "package root"),
    ];
    let mut profiles: BTreeMap<_, _> = rows
        .iter()
        .map(|(file, source)| {
            let parsed = ParsedFile::parse(file, source, Language::Go).unwrap();
            (
                (*file).into(),
                go_build_profile::extract_go_file_profile(file, &parsed).0,
            )
        })
        .collect();
    profiles.insert(
        "missing_package.go".into(),
        go_build_profile::unconstrained_profile(),
    );
    profiles
}

#[test]
fn every_pair_matches_frozen_helpers_including_missing_and_fallback() {
    let profiles = profiles();
    let memo = VisibilityMemo::new(&profiles);
    let mut uncached = VisibilityMemo::new(&profiles);
    uncached.capacity = 0;
    let files: Vec<_> = profiles
        .keys()
        .map(String::as_str)
        .chain(["missing.go"])
        .collect();
    for caller in &files {
        for defining in &files {
            let expected =
                super::super::reference_cross_package_visibility(caller, defining, &profiles);
            for _ in 0..2 {
                assert_eq!(
                    memo.for_caller(caller).get(defining),
                    expected,
                    "{caller}/{defining}"
                );
                assert_eq!(uncached.for_caller(caller).get(defining), expected);
                assert_eq!(
                    super::super::exact_cross_package_visibility(caller, defining, &profiles),
                    expected
                );
                assert_eq!(
                    super::super::exact_registration_visibility(caller, defining, &profiles),
                    expected
                );
            }
            // The new package-override entry point must preserve every detail,
            // including diagnostics and decisive flags (not just the bools).
            if let (Some(caller), Some(defining)) = (profiles.get(*caller), profiles.get(*defining))
            {
                let mut old = caller.clone();
                old.package_clause = defining.package_clause.clone();
                assert_eq!(
                    go_build_profile::go_visible_with_package(
                        caller,
                        defining,
                        &defining.package_clause
                    ),
                    go_build_profile::go_same_package_visible_detailed(&old, defining),
                );
            }
        }
    }
    assert!(uncached
        .entries
        .iter()
        .all(|shard| shard.lock().unwrap().is_empty()));
    assert!(
        memo.computations.load(Ordering::Relaxed) < uncached.computations.load(Ordering::Relaxed)
    );
    assert_eq!(
        memo.for_caller("missing.go").get("p/plain.go"),
        (true, false)
    );
    assert_eq!(
        memo.for_caller("p/plain.go").get("missing.go"),
        (true, false)
    );
    assert_eq!(
        memo.for_caller("p/broken.go").get("p/plain.go"),
        (true, false)
    );
    assert_eq!(
        memo.for_caller("p/plain.go").get("p/capped.go"),
        (true, false)
    );
}

#[test]
fn keys_retain_both_profiles_and_test_namespace_and_share_equal_profiles() {
    let profiles = profiles();
    let memo = VisibilityMemo::new(&profiles);
    for (caller, defining, expected) in [
        ("p/use_linux.go", "p/use_linux.go", (true, true)),
        ("p/use_windows.go", "p/use_linux.go", (false, false)),
        ("p/use_linux.go", "p/use_windows.go", (false, false)),
        ("p/local_test.go", "p/local_test.go", (true, true)),
        ("p/local_test.go", "q/local_test.go", (false, true)),
        ("q/local_test.go", "q/local_test.go", (true, true)),
        ("p/external_test.go", "p/local_test.go", (false, true)),
        ("p/plain.go", "p/local_test.go", (false, false)),
        ("p/custom.go", "p/notcustom.go", (false, false)),
        ("p/cgo.go", "p/nocgo.go", (false, false)),
        ("p/use_arm64.go", "p/use_amd64.go", (false, false)),
        ("missing_package.go", "p/plain.go", (true, false)),
    ] {
        assert_eq!(
            memo.for_caller(caller).get(defining),
            expected,
            "{caller}/{defining}"
        );
    }
    let before = memo.computations.load(Ordering::Relaxed);
    assert_eq!(
        memo.for_caller("p/plain.go").get("vendor/v/plain.go"),
        (true, true)
    );
    assert_eq!(
        memo.for_caller("p/other.go").get("vendor/v/plain.go"),
        (true, true)
    );
    assert_eq!(
        memo.for_caller("q/plain.go").get("vendor/v/plain.go"),
        (true, true)
    );
    assert_eq!(memo.computations.load(Ordering::Relaxed), before + 1);
}

#[test]
fn concurrent_pairs_initialize_once_and_saturation_preserves_existing_hits() {
    let profiles = profiles();
    let memo = VisibilityMemo::new(&profiles);
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let memo = &memo;
            scope.spawn(move || {
                for _ in 0..10 {
                    assert_eq!(
                        memo.for_caller("p/local_test.go").get("p/local_test.go"),
                        (true, true)
                    );
                    assert_eq!(
                        memo.for_caller("p/local_test.go").get("q/local_test.go"),
                        (false, true)
                    );
                }
            });
        }
    });
    assert_eq!(memo.computations.load(Ordering::Relaxed), 2);
    let mut bounded = VisibilityMemo::new(&profiles);
    bounded.capacity = 1;
    for caller in profiles.keys() {
        for defining in profiles.keys() {
            assert_eq!(
                bounded.for_caller(caller).get(defining),
                super::super::reference_cross_package_visibility(caller, defining, &profiles)
            );
        }
    }
    assert!(bounded
        .entries
        .iter()
        .all(|shard| shard.lock().unwrap().len() <= 1));
    // A full shard still serves retained entries, without recomputing.
    let before = bounded.computations.load(Ordering::Relaxed);
    assert_eq!(
        bounded
            .for_caller("missing_package.go")
            .get("missing_package.go"),
        (true, false)
    );
    assert_eq!(bounded.computations.load(Ordering::Relaxed), before);
}
