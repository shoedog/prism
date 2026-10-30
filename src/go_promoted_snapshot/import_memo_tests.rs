use super::*;
use crate::go_type_alias::import_memo::measured;
use crate::languages::Language;
use crate::type_providers::go::GoTypeProvider;

#[test]
fn promoted_snapshot_matches_reference_and_scans_once_across_aliases_structs_and_interfaces() {
    for count in [1, 8, 32] {
        let mut source =
            "package shared\nimport (q \"example/q\"; _ \"blank/pkg\")\ntype A = q.T\ntype Direct struct {q.T}\n".to_string();
        for n in 0..count {
            source.push_str(&format!(
                "type S{n} struct {{A}}\ntype I{n} interface {{q.I}}\ntype H{n} struct {{I{n}}}\n"
            ));
        }
        let rows = [
            ("p/a.go", source.as_str()),
            ("p/b.go", "package shared\nimport . \"example/dot\"\ntype Dot struct {Missing}\ntype D interface {Unknown}\n"),
            ("other/a.go", "package shared\nimport q \"other/q\"\ntype S0 struct {q.T}\n"),
            ("q/a.go", "package q\ntype T struct{}\nfunc (T) M() {}\ntype I interface {M()}\n"),
        ];
        let files: BTreeMap<_, _> = rows
            .into_iter()
            .map(|(path, source)| {
                (
                    path.into(),
                    ParsedFile::parse(path, source, Language::Go).unwrap(),
                )
            })
            .collect();
        let (profiles, _) = crate::go_build_profile::extract_go_file_profiles(&files);
        for paths in [
            BTreeMap::new(),
            BTreeMap::from([
                ("p/a.go".into(), "example/p".into()),
                ("p/b.go".into(), "example/p".into()),
                ("other/a.go".into(), "example/other".into()),
                ("q/a.go".into(), "example/q".into()),
            ]),
        ] {
            let (provider, provider_scans) = measured(|| {
                GoTypeProvider::from_parsed_files_with_package_import_paths(&files, &paths)
            });
            assert_eq!(
                provider_scans,
                files.len(),
                "provider aliases and signatures share scans"
            );
            let types = provider.go_type_declaration_files();
            let structs = provider.go_struct_declarations();
            let interfaces = provider.go_interface_declarations();
            let methods = provider.go_method_declarations();
            let expected = import_reference::build_reference(
                &files,
                &paths,
                &profiles,
                &types,
                &structs,
                &interfaces,
                &methods,
            );
            let (actual, scans) = measured(|| {
                build(
                    &files,
                    &paths,
                    &profiles,
                    &types,
                    &structs,
                    &interfaces,
                    &methods,
                )
            });
            assert_eq!(actual, expected);
            assert_eq!(
                bincode::serialize(&actual).unwrap(),
                bincode::serialize(&expected).unwrap()
            );
            assert_eq!(
                scans,
                files.len(),
                "declaration count must not multiply import scans"
            );
            assert!(!actual.owners.is_empty());
        }
    }
}

#[test]
fn missing_declaration_files_match_the_old_empty_map_fallback() {
    let files = BTreeMap::from([(
        "a.go".into(),
        ParsedFile::parse(
            "a.go",
            "package a\ntype S struct {missing.T}\ntype I interface {missing.I}\n",
            Language::Go,
        )
        .unwrap(),
    )]);
    let provider = GoTypeProvider::from_parsed_files(&files);
    let types = provider.go_type_declaration_files();
    let structs = provider.go_struct_declarations();
    let interfaces = provider.go_interface_declarations();
    let methods = provider.go_method_declarations();
    let empty = BTreeMap::new();
    let paths = BTreeMap::new();
    let profiles = BTreeMap::new();
    let expected = import_reference::build_reference(
        &empty,
        &paths,
        &profiles,
        &types,
        &structs,
        &interfaces,
        &methods,
    );
    let (actual, scans) = measured(|| {
        build(
            &empty,
            &paths,
            &profiles,
            &types,
            &structs,
            &interfaces,
            &methods,
        )
    });
    assert_eq!(actual, expected);
    assert_eq!(scans, 0);
    assert!(actual
        .owners
        .values()
        .flat_map(|owner| &owner.declarations)
        .any(|decl| !decl.unresolved_embedded_fields.is_empty()));
}
