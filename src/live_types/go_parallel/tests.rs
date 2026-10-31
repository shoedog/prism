use super::*;

#[test]
fn ordered_go_scans_match_serial_with_pointer_and_negative_cases() {
    let rows = [
        (
            "a.go",
            "package a\nfunc f(){ _ = A{}; _ = &B{}; _ = new(C); var d D; var e *E; _ = lower{} }",
            Language::Go,
        ),
        (
            "b.go",
            "package b\nfunc f(){ _ = pkg.A{}; _ = &pkg.F{} }",
            Language::Go,
        ),
        (
            "b_test.go",
            "package b\nfunc f(){ _ = TestOnly{} }",
            Language::Go,
        ),
        (
            "c_linux.go",
            "//go:build linux\n\npackage c\nvar x Tagged",
            Language::Go,
        ),
        ("empty.go", "package empty", Language::Go),
        ("other.py", "x = PythonOnly()", Language::Python),
    ];
    let files: BTreeMap<_, _> = rows
        .into_iter()
        .map(|(p, s, l)| (p.into(), ParsedFile::parse(p, s, l).unwrap()))
        .collect();
    let expected = reference(&files);
    assert_eq!(collect(&files), expected);
    let per_file: Vec<_> = files
        .values()
        .filter(|p| p.language == Language::Go)
        .map(|p| {
            let mut live = BTreeSet::new();
            scan_go(p, &mut live);
            live
        })
        .collect();
    assert_eq!(prepare(&files), per_file);
    for name in [
        "A", "B", "*B", "C", "*C", "D", "E", "*E", "F", "*F", "TestOnly", "Tagged",
    ] {
        assert!(expected.contains(name), "{name}");
    }
    for name in ["lower", "PythonOnly", "*A", "*D"] {
        assert!(!expected.contains(name), "{name}");
    }
    assert!(prepare(&BTreeMap::new()).is_empty());
    let non_go = BTreeMap::from([(
        "a.py".into(),
        ParsedFile::parse("a.py", "x = A()", Language::Python).unwrap(),
    )]);
    assert!(prepare(&non_go).is_empty());
    assert!(collect(&non_go).is_empty());
}
