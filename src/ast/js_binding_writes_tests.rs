//! S1b-3 unit rows (SPEC §7 C-19, C-36 mangled write): the scoped write resolver and its memo.
use super::JsBindingCache;
use crate::ast::js_binding_helper_tests::parse;

/// C-37: a unit test (inside `src/ast`, where the cache fields are visible) binds N sites of
/// one file with one `JsBindingCache` and asserts `write_targets` is filled and `written` holds
/// one entry per distinct `(scope, name)` queried.
#[test]
fn c37_memo_fills_write_targets_and_caches_per_scope_and_name() {
    let src = "let f = () => 1;\nf = () => 2;\nfunction g() {\n  return f();\n}\n\
        function h() {\n  return f();\n}\n";
    let p = parse("a.js", src);
    let root = p.tree.root_node();
    let mut cache = JsBindingCache::default();
    assert!(cache.write_targets.is_none());
    assert!(cache.written.is_empty());
    let first = p.js_ts_scoped_written(root, "f", &mut cache);
    assert!(first, "the module-scope write should be found");
    assert!(
        cache.write_targets.is_some(),
        "write index built on first use"
    );
    assert_eq!(cache.written.len(), 1, "one (scope, name) memo entry");
    // A second query for the same (scope, name) reuses the memo (no growth).
    let second = p.js_ts_scoped_written(root, "f", &mut cache);
    assert_eq!(first, second);
    assert_eq!(cache.written.len(), 1);
    // A different name adds a distinct memo entry, without rebuilding the write index.
    p.js_ts_scoped_written(root, "g", &mut cache);
    assert_eq!(cache.written.len(), 2);
}

#[test]
fn write_in_a_different_scope_does_not_count() {
    let src = "let f = () => 1;\nfunction g() {\n  let f = () => 2;\n  f = () => 3;\n}\n\
        f();\n";
    let p = parse("a.js", src);
    let root = p.tree.root_node();
    let mut cache = JsBindingCache::default();
    // The module-scope `f` is never written; only `g`'s own local `f` is.
    assert!(!p.js_ts_scoped_written(root, "f", &mut cache));
}

#[test]
fn a_write_through_with_counts_for_every_scope() {
    let src = "let f = () => 1;\nwith (o) {\n  f = 2;\n}\nf();\n";
    let p = parse("a.js", src);
    let root = p.tree.root_node();
    let mut cache = JsBindingCache::default();
    assert!(p.js_ts_scoped_written(root, "f", &mut cache));
}

#[test]
fn c36_mangled_write_beside_an_error_still_counts() {
    // `f = ;` parses as `f` beside an `ERROR` holding `=`: recovery may have mangled a write.
    let src = "let f = () => 1;\nfunction g() {\n  f = ;\n}\nf();\n";
    let p = parse("a.js", src);
    assert!(p.tree.root_node().has_error());
    let root = p.tree.root_node();
    let mut cache = JsBindingCache::default();
    assert!(p.js_ts_scoped_written(root, "f", &mut cache));
}
