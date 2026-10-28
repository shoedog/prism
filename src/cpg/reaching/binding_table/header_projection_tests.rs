use super::*;

fn declaration(parsed: &ParsedFile) -> Node<'_> {
    let mut pending = vec![parsed.tree.root_node()];
    while let Some(node) = pending.pop() {
        if node.kind() == "declaration" {
            return node;
        }
        let mut cursor = node.walk();
        pending.extend(node.named_children(&mut cursor));
    }
    panic!("declaration node");
}

#[test]
fn c_source_and_header_use_the_same_e4b_rows() {
    let source = "void f(void) { int value; }";
    let c_source = ParsedFile::parse("test.c", source, Language::C).unwrap();
    let c_row = select_binding_row(&c_source, declaration(&c_source)).expect("C row");
    assert_eq!(c_row.regression, "c-local-without-initializer");
    assert_eq!(c_row.ruling, Ruling::Classified);

    let header = ParsedFile::parse("test.h", source, Language::C).unwrap();
    let header_row = select_binding_row(&header, declaration(&header)).expect("header row");
    assert_eq!(header_row, c_row);
    assert_eq!(header_row.regression, "c-local-without-initializer");
    assert_eq!(header_row.ruling, Ruling::Classified);
}
