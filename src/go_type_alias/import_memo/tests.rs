use super::*;
use crate::languages::Language;
use reference::signature_imports_reference;

fn files(rows: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    rows.iter()
        .map(|(path, source)| {
            (
                (*path).into(),
                ParsedFile::parse(path, source, Language::Go).unwrap(),
            )
        })
        .collect()
}

#[test]
fn maps_match_frozen_reference_for_each_file_and_local_path() {
    let files = files(&[
        ("a/a.go", r#"package shared
import (named "one/aliased"; _ "blank/sideeffect"; "example/widget/v2"; "gopkg.in/yaml.v3")
type A struct {named.T}
"#),
        ("a/b.go", "package shared\nimport . \"example/dot\"\ntype B struct {X}\n"),
        ("b/a.go", "package shared\nimport named \"other/aliased\"\ntype A struct {named.T}\n"),
        ("b/b.go", "package shared\nimport (\"first/widget/v2\"; \"second/widget/v3\"; widget \"explicit/widget\")\n"),
        ("b/ambiguous.go", "package shared\nimport (\"first/widget/v2\"; \"second/widget/v3\")\n"),
        ("empty/a.go", "package empty\n"),
    ]);
    for local in [None, Some(""), Some("example/local"), Some("another/local")] {
        let paths = local
            .map(|local| {
                files
                    .keys()
                    .map(|path| (path.clone(), local.into()))
                    .collect()
            })
            .unwrap_or_default();
        let memo = SignatureImportMemo::new(&files, &paths);
        for (file, parsed) in &files {
            let expected = signature_imports_reference(parsed, local);
            assert_eq!(memo.get(file), &expected, "{file}: {local:?}");
            assert!(std::ptr::eq(memo.get(file), memo.get(file)));
        }
        assert_ne!(
            memo.get("a/a.go").get("named"),
            memo.get("b/a.go").get("named")
        );
        assert!(memo.get("a/b.go").contains_key("."));
        assert!(!memo.get("a/a.go").contains_key("."));
        assert!(!memo.get("a/a.go").contains_key("_"));
        assert_eq!(
            memo.get("a/a.go").get("widget").map(String::as_str),
            Some("example/widget/v2")
        );
        assert!(!memo.get("b/ambiguous.go").contains_key("widget"));
    }
}

#[test]
fn empty_and_missing_maps_are_lazy_and_reused() {
    let files = files(&[
        ("empty.go", "package empty\n"),
        ("unused.go", "package unused\n"),
    ]);
    let paths = BTreeMap::new();
    let memo = SignatureImportMemo::new(&files, &paths);
    let (_, scans) = measured(|| {
        for _ in 0..8 {
            assert!(memo.get("missing.go").is_empty());
            assert!(memo.get("empty.go").is_empty());
        }
    });
    assert_eq!(
        scans, 1,
        "empty results are cached; missing and unused files never scan"
    );
}

#[test]
fn rebuilding_same_file_with_changed_source_or_local_path_is_isolated() {
    let a = files(&[("a.go", "package a\nimport q \"example/a\"\n")]);
    let b = files(&[("a.go", "package a\nimport q \"example/b\"\n")]);
    let paths_a = BTreeMap::from([("a.go".into(), "local/a".into())]);
    let paths_b = BTreeMap::from([("a.go".into(), "local/b".into())]);
    let first = SignatureImportMemo::new(&a, &paths_a);
    let changed_path = SignatureImportMemo::new(&a, &paths_b);
    let changed_source = SignatureImportMemo::new(&b, &paths_a);
    assert_eq!(first.get("a.go")[""], "local/a");
    assert_eq!(changed_path.get("a.go")[""], "local/b");
    assert_eq!(first.get("a.go")["q"], "example/a");
    assert_eq!(changed_source.get("a.go")["q"], "example/b");
}

#[test]
fn strict_override_maps_preserve_file_keys_and_missing_fallback() {
    let maps = BTreeMap::from([
        (
            "a.go".into(),
            BTreeMap::from([("q".into(), "one/q".into())]),
        ),
        (
            "b.go".into(),
            BTreeMap::from([("q".into(), "two/q".into())]),
        ),
    ]);
    let source = SignatureImportSource::Maps(&maps);
    assert_eq!(source.get("a.go"), maps.get("a.go"));
    assert_eq!(source.get("b.go"), maps.get("b.go"));
    assert_eq!(source.get("missing.go"), None);
}
