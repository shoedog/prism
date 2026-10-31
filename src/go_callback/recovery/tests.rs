use super::*;
use crate::go_type_alias::import_memo::parallel::with_serial_reference;

fn fixture() -> BTreeMap<String, ParsedFile> {
    let mut files: BTreeMap<_, _> = [
        (
            "ok/a.go",
            "package ok\nfunc invoke(cb func(int)) { cb(1) }\nfunc target(x int) {recover()}\n",
        ),
        ("ok/a_test.go", "package ok\nfunc broken( {"),
        ("poison/a.go", "package poison\nfunc clean(cb func(int)) {}"),
        ("poison/b.go", "package poison\nfunc broken( {"),
        ("missing/a.go", "package missing\nfunc incomplete() {"),
        (
            "tag/a_linux.go",
            "//go:build linux\n\npackage tag\nfunc f(cb func()) {}",
        ),
        ("empty/a.go", "package empty"),
        ("no_package/a.go", "var x int"),
    ]
    .into_iter()
    .map(|(p, s)| (p.into(), ParsedFile::parse(p, s, Language::Go).unwrap()))
    .collect();
    files.insert(
        "other.py".into(),
        ParsedFile::parse("other.py", "def f(): pass", Language::Python).unwrap(),
    );
    files
}

#[test]
fn recovery_facts_match_recursive_reference_and_do_not_rescan() {
    let mut files = fixture();
    let (profiles, _) = crate::go_build_profile::extract_go_file_profiles(&files);
    let (facts, scans) = measured(|| RecoveryFacts::new(&files, &profiles));
    assert_eq!(scans, 6);
    assert_eq!(facts.rows.len(), 6);
    for (path, parsed) in &files {
        assert_eq!(
            facts.rows.contains_key(path.as_str()),
            parsed.language == Language::Go
                && !path.ends_with("_test.go")
                && !path.starts_with("no_package/")
        );
        if parsed.language == Language::Go
            && !path.ends_with("_test.go")
            && !path.starts_with("no_package/")
        {
            assert_eq!(
                facts.contains(path, parsed),
                contains_recovery_unmeasured(parsed.tree.root_node()),
                "{path}"
            );
        }
    }
    assert!(!facts.contains("ok/a.go", &files["ok/a.go"]));
    assert!(facts.contains("poison/b.go", &files["poison/b.go"]));
    assert!(facts.contains("missing/a.go", &files["missing/a.go"]));
    assert!(facts.contains("absent.go", &files["ok/a.go"]));
    let (_, scans) = measured(|| {
        for _ in 0..5 {
            for (p, f) in &files {
                let _ = facts.contains(p, f);
            }
        }
    });
    assert_eq!(scans, 0);
    drop(facts);
    files.insert(
        "poison/b.go".into(),
        ParsedFile::parse(
            "poison/b.go",
            "package poison\nfunc repaired() {}",
            Language::Go,
        )
        .unwrap(),
    );
    assert!(!RecoveryFacts::new(&files, &profiles).contains("poison/b.go", &files["poison/b.go"]));
    assert!(RecoveryFacts::new(&BTreeMap::new(), &BTreeMap::new())
        .rows
        .is_empty());
}

#[test]
fn callback_indices_match_serial_and_scan_once_per_go_file() {
    let files = fixture();
    crate::build_pool::install(|| {
        let (expected, old_scans) = measured(|| {
            with_serial_reference(|| {
                super::super::extract_go_callback_indices(&files, &BTreeMap::new())
            })
        });
        let (actual, scans) =
            measured(|| super::super::extract_go_callback_indices(&files, &BTreeMap::new()));
        assert_eq!(actual, expected);
        assert_eq!(scans, 6, "one recovery scan per eligible Go input");
        assert!(
            old_scans > scans,
            "frozen serial control must repeat scans: {old_scans} vs {scans}"
        );
        assert!(actual.parameters.keys().any(|f| f.file == "ok/a.go"));
        assert!(!actual
            .free_functions
            .keys()
            .any(|f| f.file.starts_with("poison/")
                || f.file.ends_with("_test.go")
                || f.file.starts_with("missing/")));
    });
}

#[test]
fn preparation_keeps_both_path_and_profile_admission_routes() {
    let files = fixture();
    let (mut profiles, _) = crate::go_build_profile::extract_go_file_profiles(&files);
    profiles.get_mut("ok/a.go").unwrap().is_test_file = true;
    profiles.get_mut("ok/a_test.go").unwrap().is_test_file = false;
    let facts = RecoveryFacts::new(&files, &profiles);
    assert!(
        facts.rows.contains_key("ok/a.go"),
        "clean-package path check still needs this file"
    );
    assert!(
        facts.rows.contains_key("ok/a_test.go"),
        "admissible-file profile check still needs this file"
    );
    drop(facts);
    profiles.get_mut("ok/a.go").unwrap().build_unparsed = true;
    assert!(!RecoveryFacts::new(&files, &profiles)
        .rows
        .contains_key("ok/a.go"));
    profiles.remove("poison/b.go");
    assert!(!RecoveryFacts::new(&files, &profiles)
        .rows
        .contains_key("poison/b.go"));
    let non_go: BTreeMap<_, _> = files
        .into_iter()
        .filter(|(_, p)| p.language != Language::Go)
        .collect();
    assert!(RecoveryFacts::new(&non_go, &profiles).rows.is_empty());
}
