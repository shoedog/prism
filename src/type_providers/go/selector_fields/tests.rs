use super::*;
use crate::{ast::ParsedFile, languages::Language, type_providers::go::GoTypeProvider};

#[test]
fn owner_range_matches_full_scan_with_empty_fields_and_unproven_embeds() {
    let files: std::collections::BTreeMap<_, _> = [
        ("a/a.go", "package a\ntype T struct{ Before int }"),
        ("m/a.go", "package m\ntype T struct{ Own int; *Missing; Inner }\ntype Inner struct{}\nfunc (Inner) M() {}"),
        ("z/a.go", "package z\ntype T struct{ After int }"),
    ].into_iter().map(|(p,s)| (p.into(), ParsedFile::parse(p,s,Language::Go).unwrap())).collect();
    let mut provider = GoTypeProvider::from_parsed_files(&files);
    let data = std::sync::Arc::get_mut(&mut provider.data).unwrap();
    let owner = GoOwnerIdentity {
        package_dir: "m".into(),
        package_clause: "m".into(),
        name: "T".into(),
    };
    // Empty strings are valid map keys, even though extraction cannot emit one.
    data.field_types
        .insert((owner.clone(), String::new()), "int".into());
    for dir in ["", "a", "b", "m", "n", "z", "zz"] {
        for clause in ["", "a", "m", "z"] {
            for name in ["", "Inner", "T", "Z"] {
                let query = GoOwnerIdentity {
                    package_dir: dir.into(),
                    package_clause: clause.into(),
                    name: name.into(),
                };
                assert_eq!(names(data, &query), reference(data, &query), "{query:?}");
            }
        }
    }
    let found = names(data, &owner);
    assert!(found.contains(""));
    assert!(found.contains("Own"));
    assert!(
        found.contains("Missing"),
        "unproven pointer embed must still shadow"
    );
    assert!(found.contains("Inner"));
    assert!(!found.contains("Before"));
    assert!(!found.contains("After"));
}
