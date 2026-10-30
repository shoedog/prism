use super::extraction_equivalence_tests::files;
use super::*;

#[test]
fn repeated_signatures_scan_imports_once_per_file_not_once_per_signature() {
    for count in [1, 8, 32] {
        let mut source = "package p\ntype T struct{}\ntype I interface {I(int)}\n".to_string();
        for i in 0..count {
            source.push_str(&format!("func (T) M{i}(x int) int {{return x}}\n"));
        }
        let parsed = files(&[("a/t.go", &source), ("b/t.go", &source)]);
        for paths in [
            BTreeMap::new(),
            BTreeMap::from([
                ("a/t.go".into(), "example/a".into()),
                ("b/t.go".into(), "example/b".into()),
            ]),
        ] {
            let (old, old_count) = signature_inputs::measured(|| {
                GoTypeProvider::from_parsed_files_with_import_scan(
                    &parsed,
                    &paths,
                    true,
                    ImportScan::EverySignature,
                )
            });
            let (new, new_count) = signature_inputs::measured(|| {
                GoTypeProvider::from_parsed_files_inner(&parsed, &paths, true)
            });
            assert_eq!(format!("{:#?}", old.data), format!("{:#?}", new.data));
            assert_eq!(old_count, 2 * (count + 2), "aliases plus every signature");
            assert_eq!(new_count, 2, "one scan in each file extraction");
        }
    }
}

#[test]
fn files_without_signatures_only_scan_for_aliases_and_non_go_files_do_not_scan() {
    let mut parsed = files(&[("a.go", "package a\ntype S struct{X int}\n")]);
    parsed.insert(
        "a.rs".into(),
        ParsedFile::parse("a.rs", "fn f() {}", Language::Rust).unwrap(),
    );
    for (parsed, expected) in [(BTreeMap::new(), 0), (parsed, 1)] {
        let (_, count) = signature_inputs::measured(|| GoTypeProvider::from_parsed_files(&parsed));
        assert_eq!(count, expected);
    }
}
