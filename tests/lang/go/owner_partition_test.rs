use prism::ast::ParsedFile;
use prism::go_build_profile::unconstrained_profile;
use prism::languages::Language;
use prism::resolution::{resolve_go_owner_identity, GoOwnerIdentity};
use prism::type_providers::go::GoTypeProvider;
use std::collections::{BTreeMap, BTreeSet};

fn profile(package_clause: &str, is_test_file: bool) -> prism::go_build_profile::GoBuildProfile {
    let mut profile = unconstrained_profile();
    profile.package_clause = package_clause.to_string();
    profile.is_test_file = is_test_file;
    profile
}

fn go_provider(files: &[(&str, &str)]) -> GoTypeProvider {
    let parsed = files
        .iter()
        .map(|(path, source)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, source, Language::Go).expect("parse Go fixture"),
            )
        })
        .collect();
    GoTypeProvider::from_parsed_files(&parsed)
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

#[test]
fn provider_preserves_both_build_partition_field_declarations() {
    let provider = go_provider(&[
        (
            "pkg/a_linux.go",
            "package foo\ntype T struct { f LinuxConn }\n",
        ),
        (
            "pkg/z_windows.go",
            "package foo\ntype T struct { f WindowsConn }\n",
        ),
    ]);
    let owner = GoOwnerIdentity {
        package_dir: "pkg".to_string(),
        package_clause: "foo".to_string(),
        name: "T".to_string(),
    };
    let field_types: BTreeSet<String> = provider
        .go_struct_declarations()
        .into_iter()
        .filter(|(candidate, _)| candidate == &owner)
        .flat_map(|(_, declarations)| declarations)
        .filter_map(|declaration| declaration.fields.get("f").cloned())
        .collect();

    assert_eq!(
        field_types,
        BTreeSet::from(["LinuxConn".to_string(), "WindowsConn".to_string()])
    );
}

#[test]
fn provider_s4_route_does_not_cross_external_test_clause() {
    let provider = go_provider(&[
        (
            "pkg/a_prod.go",
            "package foo\ntype Doer interface { Prod() }\ntype Holder struct { Doer }\n",
        ),
        (
            "pkg/z_external_test.go",
            "package foo_test\ntype Doer interface { Test() }\n",
        ),
    ]);
    let owner = GoOwnerIdentity {
        package_dir: "pkg".to_string(),
        package_clause: "foo".to_string(),
        name: "Holder".to_string(),
    };
    let routes = provider.embedded_interface_method_routes();
    let methods = routes.get(&owner).expect("ordinary Holder routes");

    assert_eq!(methods.get("Prod"), Some(&"Doer".to_string()));
    assert!(!methods.contains_key("Test"));
}
