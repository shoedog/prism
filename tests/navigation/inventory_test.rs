use prism::navigation::inventory::functions_inventory;

#[test]
fn test_python_decorated_function_emits_one_record() {
    // §2.3 dedup: queries.rs captures BOTH (function_definition) and
    // (decorated_definition) for Python — without dedup this is two records.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("app.py"),
        "import functools\n\n@functools.cache\ndef handler(x):\n    return x\n",
    )
    .unwrap();
    let recs = functions_inventory(dir.path()).unwrap();
    assert_eq!(recs.len(), 1, "expected exactly one record, got {recs:?}");
    assert_eq!(recs[0].name.as_deref(), Some("handler"));
}

#[test]
fn test_python_nested_same_name_functions_both_emit_records() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("app.py"),
        "def f():\n    def f():\n        return 1\n    return f()\n",
    )
    .unwrap();
    let recs = functions_inventory(dir.path()).unwrap();
    assert_eq!(
        recs.len(),
        2,
        "expected nested same-name records, got {recs:?}"
    );
    assert!(recs.iter().all(|r| r.name.as_deref() == Some("f")));
    assert_eq!(recs[0].start_line, 1);
    assert_eq!(recs[1].start_line, 2);
}

#[test]
fn test_sorted_with_resolved_kind_names() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.rs"), "fn alpha() {}\nfn beta() {}\n").unwrap();
    let recs = functions_inventory(dir.path()).unwrap();
    assert_eq!(recs.len(), 2);
    assert!(
        recs[0].start_line < recs[1].start_line,
        "sorted by (file, start_line)"
    );
    assert_eq!(
        recs[0].kind, "function_item",
        "kind_id resolved to grammar kind name"
    );
    assert_eq!(recs[0].name.as_deref(), Some("alpha"));
}
