use super::*;

pub(super) fn files(rows: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    rows.iter()
        .map(|(path, source)| {
            (
                (*path).into(),
                ParsedFile::parse(path, source, Language::Go).unwrap(),
            )
        })
        .collect()
}

pub(super) fn check(rows: &[(&str, &str)], paths: &BTreeMap<String, String>) -> GoTypeProvider {
    let files = files(rows);
    for aware in [false, true] {
        let empty = BTreeMap::new();
        let imports = if aware { paths } else { &empty };
        let old = GoTypeProvider::from_parsed_files_with_import_scan(
            &files,
            imports,
            !aware,
            ImportScan::EverySignature,
        );
        let new = if aware {
            GoTypeProvider::from_parsed_files_with_package_import_paths(&files, imports)
        } else {
            GoTypeProvider::from_parsed_files(&files)
        };
        assert_eq!(format!("{:#?}", old.data), format!("{:#?}", new.data));
    }
    GoTypeProvider::from_parsed_files_with_package_import_paths(&files, paths)
}

#[test]
fn aliases_generics_embedding_and_pointer_embeds_are_identical() {
    let rows = [
        (
            "p/a.go",
            r#"package p
        type Word = int
        type BrokenA = BrokenB
        type BrokenB = BrokenA
        type I interface { M(Word) }
        type Inner struct { Callback func(int) }
        func (Inner) M(x Word) {}
        type Outer struct { *Inner }
        type Unknown struct { *Missing }
        type Embedded interface { I }
        type Holder struct { I }
        type Box[T any] struct { Value T }
        func (b Box[T]) Get() T { return b.Value }
        type IntBox = Box[int]
        type Param[T any] = []T
        func (Inner) Slice(x Param[int]) {}
        func (Outer) Bad(x BrokenA) {}
        "#,
        ),
        (
            "q/a.go",
            "package q\ntype Inner struct { Other string }\nfunc (*Inner) M(x string) {}\n",
        ),
    ];
    let p = check(&rows, &BTreeMap::new());
    assert!(p.data.alias_unresolved.values().sum::<usize>() > 0);
    assert_eq!(p.data.structs["Inner"].file, "q/a.go");
}

#[test]
fn profile_variants_and_external_test_packages_are_identical() {
    check(&[
        ("p/a_linux.go", "//go:build linux\n\npackage p\ntype T struct { X int }\nfunc (T) M(x int) {}\n"),
        ("p/a_windows.go", "//go:build windows\n\npackage p\ntype T struct { X string }\nfunc (T) M(x string) {}\n"),
        ("p/b.go", "package p\ntype Outer struct {*T}\ntype I interface {M(int)}\n"),
        ("p/a_test.go", "package p_test\ntype T struct{}\nfunc (T) M(x bool) {}\n"),
        ("p/bad.go", "//go:build (\n\npackage p\ntype A = T\nfunc (T) Bad(x A) {}\n"),
    ], &BTreeMap::new());
}

#[test]
fn last_map_write_and_first_method_winner_keep_path_and_source_order() {
    let p = check(
        &[
            (
                "z/t.go",
                "package z\ntype T struct {Z string}\nfunc (T) M() {}\n",
            ),
            (
                "a/t.go",
                "package a\ntype T struct {A int}\nfunc (T) M() {}\nfunc (T) M() {}\n",
            ),
        ],
        &BTreeMap::new(),
    );
    assert_eq!(p.data.structs["T"].file, "z/t.go");
    let methods = &p.data.methods["T"];
    assert_eq!(
        methods.iter().map(|m| m.file.as_str()).collect::<Vec<_>>(),
        vec!["a/t.go", "a/t.go", "z/t.go"]
    );
    assert!(methods[0].start_line < methods[1].start_line);
    let winner = p.resolve_dispatch("T", "M", &BTreeSet::new());
    assert_eq!(winner[0].file, "a/t.go");
    assert_eq!(winner[0].start_line, methods[0].start_line);
}
