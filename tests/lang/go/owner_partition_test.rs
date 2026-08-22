use prism::go_build_profile::unconstrained_profile;
use prism::resolution::resolve_go_owner_identity;
use std::collections::{BTreeMap, BTreeSet};

fn profile(package_clause: &str, is_test_file: bool) -> prism::go_build_profile::GoBuildProfile {
    let mut profile = unconstrained_profile();
    profile.package_clause = package_clause.to_string();
    profile.is_test_file = is_test_file;
    profile
}

#[test]
fn go_owner_identity_records_bare_callers_package_clause() {
    let imports = BTreeMap::new();
    let package_basenames: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut profiles = BTreeMap::new();
    profiles.insert(
        "pkg/blackbox_test.go".to_string(),
        profile("foo_test", true),
    );

    let owner = resolve_go_owner_identity(
        "T",
        "pkg/blackbox_test.go",
        &imports,
        &package_basenames,
        &profiles,
    )
    .expect("bare owner");
    let wire = serde_json::to_value(owner).expect("serialize owner");

    assert_eq!(wire["package_clause"], "foo_test");
}

#[test]
fn qualified_owner_chooses_the_single_ordinary_clause() {
    let imports = BTreeMap::from([(
        "pkg/blackbox_test.go".to_string(),
        BTreeMap::from([("foo".to_string(), "example/foo".to_string())]),
    )]);
    let package_basenames =
        BTreeMap::from([("foo".to_string(), BTreeSet::from(["pkg".to_string()]))]);
    let profiles = BTreeMap::from([
        (
            "pkg/blackbox_test.go".to_string(),
            profile("foo_test", true),
        ),
        ("pkg/prod.go".to_string(), profile("foo", false)),
        ("pkg/internal_test.go".to_string(), profile("foo", true)),
        (
            "pkg/external_test.go".to_string(),
            profile("foo_test", true),
        ),
    ]);

    let owner = resolve_go_owner_identity(
        "foo.T",
        "pkg/blackbox_test.go",
        &imports,
        &package_basenames,
        &profiles,
    )
    .expect("qualified ordinary owner");

    assert_eq!(owner.package_dir, "pkg");
    assert_eq!(owner.package_clause, "foo");
    assert_eq!(owner.name, "T");
}

#[test]
fn owner_identity_fails_closed_without_one_proven_clause() {
    let imports = BTreeMap::from([(
        "caller/main.go".to_string(),
        BTreeMap::from([("foo".to_string(), "example/foo".to_string())]),
    )]);
    let package_basenames =
        BTreeMap::from([("foo".to_string(), BTreeSet::from(["pkg".to_string()]))]);
    let profiles = BTreeMap::from([
        ("caller/main.go".to_string(), profile("", false)),
        ("pkg/a.go".to_string(), profile("foo", false)),
        ("pkg/b.go".to_string(), profile("bar", false)),
    ]);

    assert!(resolve_go_owner_identity(
        "T",
        "caller/main.go",
        &imports,
        &package_basenames,
        &profiles,
    )
    .is_none());
    assert!(resolve_go_owner_identity(
        "foo.T",
        "caller/main.go",
        &imports,
        &package_basenames,
        &profiles,
    )
    .is_none());
}
