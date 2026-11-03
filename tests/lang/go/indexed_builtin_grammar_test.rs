//! Parse-only reductions of the three error_NEW files in the #635 differential.
//! Builtin spellings are ordinary identifiers when used as containers.
use prism::ast::ParsedFile;
use prism::languages::Language;

fn assert_clean_index(source: &str, expected_indexes: usize) {
    let parsed = ParsedFile::parse("indexed.go", source, Language::Go).unwrap();
    assert_eq!(
        parsed.parse_error_count,
        0,
        "{}",
        parsed.tree.root_node().to_sexp()
    );
    let mut pending = vec![parsed.tree.root_node()];
    let mut indexes = 0;
    while let Some(node) = pending.pop() {
        if node.kind() == "index_expression" {
            indexes += 1;
            assert!(node.child_by_field_name("operand").is_some());
            assert!(node.child_by_field_name("index").is_some());
        }
        let mut cursor = node.walk();
        pending.extend(node.named_children(&mut cursor));
    }
    assert_eq!(
        indexes, expected_indexes,
        "indexing must retain its tree shape"
    );
}

#[test]
fn shadowed_new_index_in_call_argument_is_clean() {
    assert_clean_index(
        "package p\nfunc f(new []string, i int) { sink(new[i]) }\n",
        1,
    );
}

#[test]
fn shadowed_new_index_before_selector_in_case_is_clean() {
    assert_clean_index(
        "package p\ntype item struct { name string }\n\
         func f(new []item) { switch { case new[0].name < new[1].name: sink() } }\n",
        2,
    );
}

#[test]
fn shadowed_new_map_index_in_if_initializer_is_clean() {
    assert_clean_index(
        "package p\nfunc f(new map[string]bool, key string) {\n\
         if _, ok := new[key]; !ok { sink() }\n}\n",
        1,
    );
}

#[test]
fn shadowed_make_slice_and_map_indices_are_clean() {
    for source in [
        "package p\nfunc f(make []string, i int) { sink(make[i]) }\n",
        "package p\nfunc f(make map[string]bool, key string) {\n\
         if _, ok := make[key]; !ok { sink() }\n}\n",
    ] {
        assert_clean_index(source, 1);
    }
}

#[test]
fn ordinary_slice_and_map_index_calls_remain_indices() {
    assert_clean_index(
        "package p\nfunc f(handlers []func(), m map[string]func(), i int, k string) {\n\
         handlers[i]()\nm[k]()\n}\n",
        2,
    );
}
