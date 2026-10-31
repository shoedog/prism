use super::*;
use crate::go_type_alias::import_memo::parallel::with_serial_reference;

fn fixture() -> BTreeMap<String, ParsedFile> {
    let rows = [
        ("p/a.go", "package p\nimport q \"example/q\"\ntype (T = q.T; T = int; Chain = Other; Other = Chain; Box[X any] = []X)\n"),
        ("p/b.go", "package p\nimport q \"different/q\"\ntype T = q.T\ntype Defined int\n"),
        ("p/c_linux.go", "//go:build linux\n\npackage p\ntype T = string\n"),
        ("p/d_test.go", "package p\ntype T = bool\n"),
        ("q/a.go", "package q\nimport (\"example/widget/v2\"; _ \"side/effect\")\ntype T = widget.T\n"),
        ("r/a.go", "package r\nimport . \"example/dot\"\ntype T = Missing\n"),
        ("empty/a.go", "package empty\n"),
    ];
    let mut files: BTreeMap<_, _> = rows
        .into_iter()
        .map(|(p, s)| (p.into(), ParsedFile::parse(p, s, Language::Go).unwrap()))
        .collect();
    files.insert(
        "other.py".into(),
        ParsedFile::parse("other.py", "T = int", Language::Python).unwrap(),
    );
    files
}

#[test]
fn declarations_preserve_duplicates_source_order_profiles_and_imports() {
    let files = fixture();
    let (profiles, _) = crate::go_build_profile::extract_go_file_profiles(&files);
    let paths = BTreeMap::from([("p/a.go".into(), "example/p".into())]);
    // Missing strict overrides must fall back to that file's own scan.
    let strict = BTreeMap::from([(
        "p/a.go".into(),
        BTreeMap::from([("q".into(), "strict/q".into())]),
    )]);
    for overrides in [None, Some(&strict)] {
        let build = || {
            GoAliasResolver::build_inner(
                &files,
                &paths,
                &paths,
                &profiles,
                overrides.map(SignatureImportSource::Maps),
            )
        };
        let expected = with_serial_reference(build);
        let (actual, scans) = crate::go_type_alias::import_memo::measured(build);
        assert!(format!("{:?}", actual.declarations) == format!("{:?}", expected.declarations));
        assert_eq!(scans, if overrides.is_some() { 6 } else { 7 });
        assert_eq!(
            actual.packages_by_import_path,
            expected.packages_by_import_path
        );
        assert_eq!(actual.profiles, expected.profiles);
        assert_eq!(actual.telemetry(), expected.telemetry());
        let owner = GoOwnerIdentity {
            package_dir: "p".into(),
            package_clause: "p".into(),
            name: "T".into(),
        };
        let declarations = &actual.declarations[&owner];
        assert_eq!(
            declarations
                .iter()
                .map(|d| d.defining_file.as_str())
                .collect::<Vec<_>>(),
            ["p/a.go", "p/a.go", "p/b.go", "p/c_linux.go", "p/d_test.go"]
        );
        assert_ne!(
            format!("{:?}", declarations[0].kind),
            format!("{:?}", declarations[1].kind)
        );
        assert_ne!(
            format!("{:?}", declarations[0].kind),
            format!("{:?}", declarations[2].kind)
        );
    }
}

#[test]
fn missing_and_empty_profiles_non_go_and_rebuilds_do_not_leak() {
    let files = fixture();
    let (mut profiles, _) = crate::go_build_profile::extract_go_file_profiles(&files);
    profiles.remove("p/a.go");
    profiles.get_mut("p/b.go").unwrap().package_clause.clear();
    let paths = BTreeMap::new();
    let build = || GoAliasResolver::build(&files, &paths, &paths, &profiles);
    let expected = with_serial_reference(build);
    let actual = build();
    assert!(format!("{:?}", actual.declarations) == format!("{:?}", expected.declarations));
    assert!(!actual
        .declarations
        .values()
        .flatten()
        .any(|d| matches!(d.defining_file.as_str(), "p/a.go" | "p/b.go" | "other.py")));
    assert!(prepare(&BTreeMap::new(), &profiles, &paths, None).is_empty());
    let non_go = BTreeMap::from([(
        "other.py".into(),
        ParsedFile::parse("other.py", "T = int", Language::Python).unwrap(),
    )]);
    assert!(prepare(&non_go, &profiles, &paths, None).is_empty());
    let changed = BTreeMap::from([(
        "p/a.go".into(),
        ParsedFile::parse("p/a.go", "package p\ntype New = int\n", Language::Go).unwrap(),
    )]);
    let (profiles, _) = crate::go_build_profile::extract_go_file_profiles(&changed);
    let changed = GoAliasResolver::build(&changed, &paths, &paths, &profiles);
    assert_eq!(changed.declarations.len(), 1);
    assert_eq!(changed.declarations.keys().next().unwrap().name, "New");
}

#[test]
fn extraction_jobs_use_build_pool_and_serial_control_does_not() {
    use crate::go_type_alias::import_memo::parallel::work_sites::measured;
    assert!(crate::build_pool::build_pool()
        .current_thread_index()
        .is_none());
    for files in [BTreeMap::new(), fixture()] {
        let (profiles, _) = crate::go_build_profile::extract_go_file_profiles(&files);
        let paths = BTreeMap::new();
        let build = || GoAliasResolver::build(&files, &paths, &paths, &profiles);
        let (old, serial) = measured(|| with_serial_reference(build));
        let (new, parallel) = measured(build);
        assert!(format!("{:?}", old.declarations) == format!("{:?}", new.declarations));
        let go_files = files
            .values()
            .filter(|p| p.language == Language::Go)
            .count();
        assert_eq!(serial.caller_files, go_files);
        assert_eq!(serial.pool_files, 0);
        assert_eq!(parallel.pool_files, go_files);
        assert_eq!(parallel.caller_files, 0);
    }
}
