//! S1b-3 unit rows (SPEC §7 C-45, C-48): the NoFn closed value class and the
//! destructuring-default test.
use super::{js_ts_has_default, js_ts_holds_no_function};
use crate::ast::ParsedFile;
use crate::languages::Language;
use tree_sitter::Node;

fn parse(src: &str) -> ParsedFile {
    ParsedFile::parse("a.js", src, Language::JavaScript).unwrap()
}

/// The initializer value of the first `variable_declarator` in `src`.
fn decl_value(p: &ParsedFile) -> Node<'_> {
    fn find<'a>(n: Node<'a>) -> Option<Node<'a>> {
        if n.kind() == "variable_declarator" {
            return n.child_by_field_name("value");
        }
        let mut cursor = n.walk();
        for c in n.named_children(&mut cursor) {
            if let Some(v) = find(c) {
                return Some(v);
            }
        }
        None
    }
    find(p.tree.root_node()).expect("a variable_declarator with a value")
}

/// The pattern (`name` field) of the first `variable_declarator` in `src`.
fn decl_pattern(p: &ParsedFile) -> Node<'_> {
    fn find<'a>(n: Node<'a>) -> Option<Node<'a>> {
        if n.kind() == "variable_declarator" {
            return n.child_by_field_name("name");
        }
        let mut cursor = n.walk();
        for c in n.named_children(&mut cursor) {
            if let Some(v) = find(c) {
                return Some(v);
            }
        }
        None
    }
    find(p.tree.root_node()).expect("a variable_declarator with a pattern")
}

/// C-45: NoFn covers a literal, and an object/array literal whose every member is itself NoFn.
#[test]
fn c45_nofn_literals() {
    let nofn = [
        "const x = 1;",
        "const x = 'a';",
        "const x = `t`;",
        "const x = true;",
        "const x = false;",
        "const x = null;",
        "const x = undefined;",
        "const x = /re/;",
        "const x = [1, 'a', [true, null]];",
        "const x = { a: 1, b: { c: 'd' } };",
    ];
    for src in nofn {
        let p = parse(src);
        assert!(
            js_ts_holds_no_function(decl_value(&p)),
            "{src:?} should be NoFn"
        );
    }
}

/// C-45: a shorthand property, a spread, a method or an identifier/call member is not NoFn.
#[test]
fn c45_not_nofn() {
    let not_nofn = [
        "const x = { a };",
        "const x = { ...a };",
        "const x = { a() {} };",
        "const x = [g()];",
        "const x = { a: g() };",
        "const x = a;",
        "const x = a.b;",
        "const x = g();",
        "const x = new X();",
    ];
    for src in not_nofn {
        let p = parse(src);
        assert!(
            !js_ts_holds_no_function(decl_value(&p)),
            "{src:?} should not be NoFn"
        );
    }
}

/// C-48: a destructuring default anywhere in the pattern is detected, whatever the value.
#[test]
fn c48_has_default() {
    let with_default = [
        "const { missing: x = fallback } = {};",
        "const [y = fallback] = [];",
        "const { z = fallback } = {};",
        "const { f = 0 } = {};",
    ];
    for src in with_default {
        let p = parse(src);
        assert!(
            js_ts_has_default(decl_pattern(&p)),
            "{src:?} should have a default"
        );
    }
    let without_default = ["const { a } = o;", "const [a] = o;", "const a = o;"];
    for src in without_default {
        let p = parse(src);
        assert!(
            !js_ts_has_default(decl_pattern(&p)),
            "{src:?} should not have a default"
        );
    }
}
