use super::*;
use crate::ast::ParsedFile;
use crate::languages::Language;

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
fn every_pair_matches_frozen_clone_and_override_helpers() {
    let profiles = profiles();
    let files: Vec<_> = profiles
        .keys()
        .map(String::as_str)
        .chain(["missing.go"])
        .collect();
    for caller in &files {
        for defining in &files {
            let expected =
                super::super::reference_cross_package_visibility(caller, defining, &profiles);
            assert_eq!(uncached(caller, defining, &profiles), expected);
            assert_eq!(
                super::super::exact_cross_package_visibility(caller, defining, &profiles),
                expected,
                "{caller}/{defining}"
            );
            assert_eq!(
                super::super::exact_registration_visibility(caller, defining, &profiles),
                expected,
                "{caller}/{defining}"
            );
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
    assert_eq!(
        uncached("missing.go", "p/plain.go", &profiles),
        (true, false)
    );
    assert_eq!(
        uncached("p/plain.go", "missing.go", &profiles),
        (true, false)
    );
    assert_eq!(
        uncached("p/broken.go", "p/plain.go", &profiles),
        (true, false)
    );
    assert_eq!(
        uncached("p/plain.go", "p/capped.go", &profiles),
        (true, false)
    );
}

#[test]
fn profiles_and_test_namespace_preserve_visibility_and_exactness() {
    let profiles = profiles();
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
            uncached(caller, defining, &profiles),
            expected,
            "{caller}/{defining}"
        );
    }
}
