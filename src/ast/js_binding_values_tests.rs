//! S1b-3 unit rows (SPEC §7 C-45, C-48): the NoFn closed value class and the
//! destructuring-default test.
use super::{js_ts_has_default, js_ts_holds_no_function};
use crate::ast::js_binding::{JsBinding, JsBindingCache};
use crate::ast::js_binding_helper_tests::parse;
use crate::ast::ParsedFile;
use tree_sitter::Node;

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
        let p = parse("a.js", src);
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
        let p = parse("a.js", src);
        assert!(
            !js_ts_holds_no_function(decl_value(&p)),
            "{src:?} should not be NoFn"
        );
    }
}

/// C-39 (OQ11 a): a pattern declarator whose unwrapped value is a call with a direct function
/// argument is may-call, like an identifier declarator (M1); one with no function argument is
/// not_callable when the value is NoFn, `Alias` otherwise (never specially may-call).
#[test]
fn c39_pattern_declarator_call_with_function_argument_is_may_call() {
    let p = parse(
        "a.js",
        "const [s, setS] = useState(() => 0);\nexport { s };\n",
    );
    let mut cache = JsBindingCache::default();
    let site = p.tree.root_node();
    let got = p.js_ts_module_binding("s", site, &mut cache);
    assert_eq!(
        got,
        JsBinding::MayCall,
        "a function-argument call keeps base (M1)"
    );
}

/// C-47 (spec r1 sol W1), C180: a non-ASCII BoundName (`é` with a combining acute accent,
/// U+0301) is a real declarator, shadowing a same-named function; the shared P1 helper trusts
/// any unescaped spelling (mutant C-M42, `is_plain_ident` restored, would drop it as
/// unclassifiable and wrongly keep the outer function visible).
#[test]
fn c47_c180_non_ascii_bound_name_shadows() {
    let src = "function \u{e9}\u{301}() {\n  return 1;\n}\nconst \u{e9}\u{301} = 0;\n\
        \u{e9}\u{301}();\nexport {};\n";
    let p = parse("a.js", src);
    let mut cache = JsBindingCache::default();
    let name = "\u{e9}\u{301}";
    let site = p.tree.root_node();
    let got = p.js_ts_module_binding(name, site, &mut cache);
    assert_eq!(
        got,
        JsBinding::Refused("duplicate_declaration"),
        "the non-ASCII declarator must be seen as a second declaration of the same name (C180)"
    );
}

#[test]
fn a_d4_import_alias_and_require_export_stay_unproven_with_a_nested_decoy() {
    // Fold A (Opus r1 W1): an exported `import f = M.g` or `import f = require('./x')` is a
    // single declaration whose kind is `import_alias`/`import_statement` (JsBinding::Import);
    // D4's `imported` set (extract_import_bindings + type-only) does not recognize either
    // form, so before the fix `Import` fell through to base `Local(name)` (unpoisoned),
    // letting R4c bind a nested same-file decoy `function f` as Exact — re-opening the edge
    // 2b had closed. Both stay `UnprovenLocal`, counted `not_callable`.
    let import_alias =
        "import f = M.g;\nfunction outer() {\n  function f() {\n    return 1;\n  }\n  \
        return f;\n}\nexport { f, outer };\n";
    let import_require =
        "import f = require('./x');\nfunction outer() {\n  function f() {\n    return 1;\n  \
        }\n  return f;\n}\nexport { f, outer };\n";
    for (id, src) in [
        ("IA1 import_alias", import_alias),
        ("IA2 require", import_require),
    ] {
        for path in ["a.ts", "a.tsx"] {
            let p = parse(path, src);
            let site = p.tree.root_node();
            let mut facts = crate::js_exports::JsExportFacts::default();
            let mut scope: (std::collections::BTreeSet<String>, JsBindingCache<'_>) =
                (std::collections::BTreeSet::new(), JsBindingCache::default());
            let got = p.js_ts_local_export_target("f".to_string(), site, &mut facts, &mut scope);
            assert_eq!(
                got,
                crate::js_exports::JsExportTarget::UnprovenLocal("f".to_string()),
                "{id} {path}"
            );
            assert_eq!(
                facts.local_export_refusals.get("not_callable"),
                Some(&1),
                "{id} {path}"
            );
        }
    }
}

#[test]
fn c_m40_d4_keeps_unproven_local_for_an_alias_export() {
    // SPEC §3.8 (11): "D4 is unchanged": an alias value at an export occurrence keeps the
    // landed 2b answer (`UnprovenLocal`, counted `not_callable`), not base `Local` (mutant
    // C-M40).
    let src = "const t = i18n.t;\nexport { t };\n";
    let p = parse("a.js", src);
    let root = p.tree.root_node();
    let mut facts = crate::js_exports::JsExportFacts::default();
    let mut scope: (std::collections::BTreeSet<String>, JsBindingCache<'_>) =
        (std::collections::BTreeSet::new(), JsBindingCache::default());
    let got = p.js_ts_local_export_target("t".to_string(), root, &mut facts, &mut scope);
    assert_eq!(
        got,
        crate::js_exports::JsExportTarget::UnprovenLocal("t".to_string())
    );
    assert_eq!(facts.local_export_refusals.get("not_callable"), Some(&1));
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
        let p = parse("a.js", src);
        assert!(
            js_ts_has_default(decl_pattern(&p)),
            "{src:?} should have a default"
        );
    }
    let without_default = ["const { a } = o;", "const [a] = o;", "const a = o;"];
    for src in without_default {
        let p = parse("a.js", src);
        assert!(
            !js_ts_has_default(decl_pattern(&p)),
            "{src:?} should not have a default"
        );
    }
}
