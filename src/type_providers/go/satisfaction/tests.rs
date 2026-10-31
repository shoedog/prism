use super::*;
use crate::go_type_alias::import_memo::parallel::with_serial_reference;

fn compare(rows: &[(&str, &str)]) -> GoTypeProvider {
    let files = super::super::extraction_equivalence_tests::files(rows);
    let expected = with_serial_reference(|| GoTypeProvider::from_parsed_files(&files));
    // Drop the reference provider before constructing the candidate.
    let snapshot = format!("{:#?}", expected.data);
    drop(expected);
    let actual = GoTypeProvider::from_parsed_files(&files);
    assert!(format!("{:#?}", actual.data) == snapshot);
    actual
}

#[test]
fn ordered_satisfaction_matches_serial_for_value_pointer_gaps_and_embedding() {
    let provider = compare(&[
        (
            "p/a.go",
            r#"package p
        type I interface { M(int) }
        type Both interface { M(int); N() }
        type Empty interface {}
        type Generic[T any] interface { M(T) }
        type Gapped interface { A(interface {N()}); B(missing.T) }
        type Unknown interface {missing.External}
        type A struct{}
        func (A) M(x int) {}
        func (A) M(x int) {}
        type P struct{}
        func (*P) M(x int) {}
        type No struct{}
        func (No) M(x string) {}
        type E struct{I}
        type Promoted struct{A}
        type Complete struct{}
        func (Complete) M(x int) {}
        func (*Complete) N() {}
        "#,
        ),
        (
            "q/z.go",
            "package q\ntype A struct{X int}\nfunc (A) M(x int) {}\n",
        ),
        (
            "p/z_test.go",
            "package p\ntype Z struct{}\nfunc (Z) M(x int) {}\n",
        ),
    ]);
    let data = &provider.data;
    assert!(data.satisfaction["I"].contains("P"));
    assert!(!data.satisfaction["I"].contains("No"));
    assert!(data.satisfaction["I"].contains("E"));
    assert!(!data.satisfaction["I"].contains("Promoted"));
    assert!(data.satisfaction["Generic"].is_empty());
    assert!(data.satisfaction["Gapped"].is_empty());
    assert!(data.satisfaction["Unknown"].is_empty());
    assert!(!data.dispatch_gaps.is_empty());
    let keys = &data.sat_keys[&("I".into(), "M".into())];
    let names: Vec<_> = keys.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["A", "*A", "Complete", "*Complete", "*P", "Z", "*Z"]);
    // Duplicate methods still choose the first source declaration, even though
    // the bare struct map uses last-file-wins across packages.
    assert_eq!(data.structs["A"].file, "q/z.go");
    assert_eq!(keys[0].1.file, "p/a.go");
    assert_eq!(keys[0].1.start_line, 9);
    assert!(data.sat_keys.keys().all(|(iface, _)| iface != "Empty"));
    assert_eq!(
        data.sat_keys[&("Both".into(), "M".into())][0].0,
        "*Complete"
    );
    drop(provider);
    // The duplicate A.M declarations above make its promotion ambiguous in
    // the original provider too. A unique target must still promote normally.
    let unique = compare(&[("p/a.go", "package p\ntype I interface{M(int)}\ntype Base struct{}\nfunc (Base) M(x int) {}\ntype Outer struct{Base}\n")]);
    assert!(unique.data.satisfaction["I"].contains("Outer"));
    assert_eq!(
        unique.data.sat_keys[&("I".into(), "M".into())]
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["Base", "*Base", "Outer", "*Outer"]
    );
}

#[test]
fn empty_inputs_empty_interfaces_and_no_concrete_types_match_serial() {
    let empty = compare(&[]);
    assert!(empty.data.satisfaction.is_empty());
    drop(empty);
    let only_interfaces = compare(&[(
        "p/a.go",
        "package p\ntype I interface{M()}\ntype Empty interface{}\n",
    )]);
    assert!(only_interfaces
        .data
        .satisfaction
        .values()
        .all(BTreeSet::is_empty));
    assert!(only_interfaces.data.sat_keys.is_empty());
    drop(only_interfaces);
    let vacuous = compare(&[(
        "p/a.go",
        "package p\ntype Empty interface{}\ntype T struct{}\n",
    )]);
    assert_eq!(
        vacuous.data.satisfaction["Empty"],
        BTreeSet::from(["T".into()])
    );
    assert!(vacuous.data.sat_keys.is_empty());
}

#[test]
fn matching_jobs_use_build_pool_and_serial_control_does_not() {
    use crate::go_type_alias::import_memo::parallel::work_sites::measured;
    assert!(crate::build_pool::build_pool()
        .current_thread_index()
        .is_none());
    for source in ["package p\n", "package p\ntype I interface{M(int)}\ntype Empty interface{}\ntype Bad interface{missing.External}\ntype T struct{}\nfunc (T) M(x int) {}\ntype P struct{}\nfunc (*P) M(x int) {}\ntype No struct{}\nfunc (No) M(x string) {}\n"] {
        let files = super::super::extraction_equivalence_tests::files(&[("p/a.go",source)]);
        let (old, serial) = measured(|| with_serial_reference(|| GoTypeProvider::from_parsed_files(&files)));
        let expected = format!("{:#?}", old.data);
        drop(old);
        let (new, parallel) = measured(|| GoTypeProvider::from_parsed_files(&files));
        assert!(format!("{:#?}", new.data) == expected);
        let pairs = if source == "package p\n" { 0 } else { 6 };
        assert_eq!(serial.caller_pairs, pairs);
        assert_eq!(serial.pool_pairs, 0);
        assert_eq!(parallel.pool_pairs, pairs);
        assert_eq!(parallel.caller_pairs, 0);
    }
}
