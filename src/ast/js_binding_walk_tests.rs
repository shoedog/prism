//! S1b-3 unit rows (SPEC §7 C-9, C-15, C-16, C-24, C-29, C-32, C-35): the raw site walk answer
//! (`Walk`), covering unproven positions, `with`, decorators and dotted namespaces directly.
//! `js_binding_site_tests.rs` covers the full end-to-end classification through nested scopes.
use super::{Walk, E_TABLE};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

fn parse(path: &str, src: &str) -> ParsedFile {
    let lang = match path.rsplit('.').next() {
        Some("tsx") => Language::Tsx,
        Some("ts") => Language::TypeScript,
        _ => Language::JavaScript,
    };
    ParsedFile::parse(path, src, lang).unwrap()
}

/// The `n`-th (0-indexed) `identifier` node spelled `name`, in source order: the exact node a
/// real call or JSX site would hand the walk (never a container spanning extra tokens).
fn ident<'a>(p: &'a ParsedFile, name: &str, n: usize) -> Node<'a> {
    fn walk<'a>(node: Node<'a>, name: &str, text: &str, out: &mut Vec<Node<'a>>) {
        if matches!(node.kind(), "identifier" | "type_identifier")
            && &text[node.start_byte()..node.end_byte()] == name
        {
            out.push(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            walk(child, name, text, out);
        }
    }
    let mut out = Vec::new();
    walk(p.tree.root_node(), name, &p.source, &mut out);
    out.into_iter()
        .nth(n)
        .unwrap_or_else(|| panic!("no occurrence {n} of {name:?}"))
}

const BOTH: [&str; 2] = ["a.js", "a.tsx"];

#[test]
fn c9_unproven_position_for_an_unlisted_field() {
    // A function declaration's own `name` field is not a site position (SPEC §3.1a: it holds
    // a binding name, never a use).
    let src = "function f() {\n  return 1;\n}\n";
    for path in BOTH {
        let p = parse(path, src);
        let mut cache = super::JsBindingCache::default();
        let site = ident(&p, "f", 0);
        assert!(
            matches!(
                p.js_ts_binding_walk(site, "f", &mut cache),
                Walk::Unchecked("unproven_position")
            ),
            "{path}: expected an unproven position for the declaration's own name"
        );
    }
}

#[test]
fn unbound_at_the_program_scope() {
    let src = "g();\n";
    for path in BOTH {
        let p = parse(path, src);
        let mut cache = super::JsBindingCache::default();
        let site = ident(&p, "g", 0);
        assert!(matches!(
            p.js_ts_binding_walk(site, "g", &mut cache),
            Walk::Unbound
        ));
    }
}

#[test]
fn c_m5_with_body_is_an_explicit_object_environment() {
    let src = "with (o) {\n  f();\n}\n";
    for path in BOTH {
        let p = parse(path, src);
        let mut cache = super::JsBindingCache::default();
        let site = ident(&p, "f", 0);
        match p.js_ts_binding_walk(site, "f", &mut cache) {
            Walk::Found(scope, explicit) => {
                assert_eq!(scope.kind(), "with_statement", "{path}");
                assert!(explicit.is_some(), "{path}: with binds explicitly");
            }
            _ => panic!("{path}: expected Found(with_statement)"),
        }
    }
}

#[test]
fn with_object_walk_resumes_outside_the_with() {
    // The `with` object expression is Outside (J3): a name it references is looked up starting
    // from the `with` statement's own enclosing scope, not inside the `with` body.
    let src = "let q = 1;\nwith (q) {\n  f();\n}\n";
    let p = parse("a.js", src);
    let mut cache = super::JsBindingCache::default();
    let site = ident(&p, "q", 1); // the reference in `with (q)`, not its declaration.
    match p.js_ts_binding_walk(site, "q", &mut cache) {
        Walk::Found(scope, explicit) => {
            assert_eq!(scope.kind(), "program");
            assert!(explicit.is_none());
        }
        _ => panic!("expected Found(program)"),
    }
}

#[test]
fn c_m19_enum_body_is_a_scope_for_a_sibling_member() {
    // T10: an enum body declares its own member names; a later member's initializer sees an
    // earlier sibling directly, resolved at the enum body itself (not left unbound/leaving).
    let src = "enum E {\n  A,\n  B = A,\n}\n";
    let p = parse("a.ts", src);
    let mut cache = super::JsBindingCache::default();
    // The declaration is a `property_identifier`; only the reference is an `identifier`.
    let site = ident(&p, "A", 0);
    match p.js_ts_binding_walk(site, "A", &mut cache) {
        Walk::Found(scope, explicit) => {
            assert_eq!(scope.kind(), "enum_body");
            assert!(explicit.is_none());
        }
        _ => panic!("expected Found(enum_body)"),
    }
}

#[test]
fn c_m25_enum_member_names_compare_by_string_value_not_raw_text() {
    // C158: a string-literal member key (`'f' = 1`) declares `f` by StringValue, not the
    // quoted raw text `'f'`; a sibling member's initializer referencing `f` finds it.
    let src = "enum E {\n  'f' = 1,\n  g = f(),\n}\n";
    let p = parse("a.ts", src);
    let mut cache = super::JsBindingCache::default();
    let site = ident(&p, "f", 0);
    match p.js_ts_binding_walk(site, "f", &mut cache) {
        Walk::Found(scope, explicit) => {
            assert_eq!(scope.kind(), "enum_body");
            assert!(explicit.is_none());
        }
        _ => panic!("expected Found(enum_body): the string-literal key decodes to `f`"),
    }
}

#[test]
fn c_m15_j3_computed_method_name_is_outside_the_method() {
    // A `method_definition`'s computed `name` is Outside (J3): the key is evaluated where the
    // class is defined, not inside the method's own environment.
    let src = "let f = 1;\nclass C {\n  [f]() {\n    return 1;\n  }\n}\n";
    let p = parse("a.ts", src);
    let mut cache = super::JsBindingCache::default();
    let site = ident(&p, "f", 1); // occurrence 0 is the declaration; 1 is the computed key.
    match p.js_ts_binding_walk(site, "f", &mut cache) {
        Walk::Found(scope, explicit) => {
            assert_eq!(scope.kind(), "program");
            assert!(explicit.is_none());
        }
        _ => panic!("expected Found(program): J3 must skip the method's own environment"),
    }
}

#[test]
fn c_m10_j2_class_decorator_resumes_above_the_class() {
    // A decorator on the class node itself (a plain declaration, not a named expression) is
    // evaluated where the class is defined (J2): the walk resumes above the class.
    let src = "let d = 1;\n@d\nclass C {}\n";
    let p = parse("a.ts", src);
    let mut cache = super::JsBindingCache::default();
    let site = ident(&p, "d", 1); // occurrence 0 is the declaration; 1 is `@d`.
    match p.js_ts_binding_walk(site, "d", &mut cache) {
        Walk::Found(scope, explicit) => {
            assert_eq!(scope.kind(), "program");
            assert!(explicit.is_none());
        }
        _ => panic!("expected Found(program): J2 must resume above the class"),
    }
}

#[test]
fn c_m20_member_decorator_is_inside_the_class() {
    // A decorator under `class_body` (a member or parameter decorator) is evaluated in the
    // class scope (T8): the walk resumes at `class_body`, so the class's own inner name is
    // visible from it (unlike a class-node decorator, J2).
    let src = "class C {\n  @d(C) m() {}\n}\n";
    let p = parse("a.ts", src);
    let mut cache = super::JsBindingCache::default();
    let site = ident(&p, "C", 1); // occurrence 0 is the class's own name; 1 is `@d(C)`.
    match p.js_ts_binding_walk(site, "C", &mut cache) {
        Walk::Found(scope, explicit) => {
            assert_eq!(scope.kind(), "class_declaration");
            assert!(explicit.is_none());
        }
        _ => panic!("expected Found(class_declaration): a member decorator sees the class name"),
    }
}

#[test]
fn c29_named_class_expression_decorator_is_unproven() {
    let src = "const x = @d class Named {};\n";
    let p = parse("a.ts", src);
    let mut cache = super::JsBindingCache::default();
    let site = ident(&p, "d", 0);
    assert!(matches!(
        p.js_ts_binding_walk(site, "d", &mut cache),
        Walk::Unchecked("decorated_class_expression")
    ));
}

#[test]
fn c32_dotted_namespace_non_first_segment_is_explicit() {
    // `B`, a non-first dotted segment, denotes a namespace (not callable) whether or not the
    // file merges: the special case fires before the leave predicate.
    let src = "namespace A.B {\n  B();\n}\n";
    let p = parse("a.ts", src);
    let mut cache = super::JsBindingCache::default();
    let site = ident(&p, "B", 0); // the only standalone `B` token: the call.
    match p.js_ts_binding_walk(site, "B", &mut cache) {
        Walk::Found(scope, explicit) => {
            assert_eq!(scope.kind(), "internal_module");
            assert!(
                explicit.is_some(),
                "the segment binds explicitly, not by index"
            );
        }
        _ => panic!("expected Found(internal_module)"),
    }
}

#[test]
fn c32_first_segment_is_not_explicit() {
    // `A` (the first, outermost segment) is D6's ordinary declaration in the enclosing scope,
    // not the dotted-segment special case: in a module with no merge partner, the walk leaves
    // the body and finds it there.
    let src = "export {};\nnamespace A.B {\n  A();\n}\n";
    let p = parse("a.ts", src);
    let mut cache = super::JsBindingCache::default();
    let site = ident(&p, "A", 1); // occurrence 0 is the `A.B` name; 1 is the call.
    match p.js_ts_binding_walk(site, "A", &mut cache) {
        Walk::Found(scope, explicit) => {
            assert_eq!(scope.kind(), "program");
            assert!(explicit.is_none());
        }
        _ => panic!("expected Found(program)"),
    }
}

/// The leave predicate (SPEC §3.1a): leaving a namespace/enum body unbound is unproven when a
/// merge partner shares the first segment, or the file is a script.
#[test]
fn c25_c28_leave_predicate() {
    let with_partner = "namespace A {\n  f();\n}\nnamespace A {\n  function f() {}\n}\n";
    let no_partner_module = "export {};\nnamespace A {\n  f();\n}\n";
    let script = "namespace A {\n  f();\n}\n";
    let cases: [(&str, bool); 3] = [
        (with_partner, true),
        (no_partner_module, false),
        (script, true),
    ];
    for (src, want_unproven) in cases {
        let p = parse("a.ts", src);
        let mut cache = super::JsBindingCache::default();
        let site = ident(&p, "f", 0);
        let got = matches!(
            p.js_ts_binding_walk(site, "f", &mut cache),
            Walk::Unchecked("namespace_leave")
        );
        assert_eq!(got, want_unproven, "{src:?}");
    }
}

#[test]
fn c_m17_partner_index_compares_first_segments_not_full_dotted_names() {
    // Two namespaces that differ in their full dotted spelling but share the first segment are
    // still merge partners (the leave predicate compares first segments, C-M17).
    let src = "export {};\nnamespace A.B {\n  f();\n}\nnamespace A.C {\n  function f() {}\n}\n";
    let p = parse("a.ts", src);
    let mut cache = super::JsBindingCache::default();
    let site = ident(&p, "f", 0);
    assert!(
        matches!(
            p.js_ts_binding_walk(site, "f", &mut cache),
            Walk::Unchecked("namespace_leave")
        ),
        "A.B and A.C share the first segment A and must be counted as partners"
    );
}

/// C-35: the shipped `E_TABLE` names every field of every Σ′ kind exactly once, with no
/// duplicate `(kind, field)` pair; `probes/grammar_closure.py --rust src/ast` (run in
/// verification) is the authority that it equals the grammars' derived table.
#[test]
fn c35_e_table_has_no_duplicate_rows() {
    let mut seen = BTreeSet::new();
    for (k, f, _) in E_TABLE {
        assert!(seen.insert((*k, *f)), "duplicate E_TABLE row ({k}, {f})");
    }
    assert_eq!(E_TABLE.len(), 74, "E_TABLE row count drifted");
}
