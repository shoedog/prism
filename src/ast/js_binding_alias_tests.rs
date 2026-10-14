//! Slice L unit rows: which `const` aliases `js_ts_follow_alias` follows, and what each chain
//! ends at. The site is always the LAST `name(` in the source. Every "not followed" row is a
//! condition the contract names; each must give `None` (the outcome stays what main gives).
use super::JsFollow;
use crate::ast::js_binding::{JsBinding, JsBindingCache, JsTerminal};
use crate::ast::js_binding_helper_tests::parse;

pub(super) const ANY: [&str; 3] = ["a.js", "a.ts", "a.tsx"];
const TS: [&str; 1] = ["a.ts"];
pub(super) const NS: &str = "import * as N from './m';\n";

fn callable(local: &str, line: usize, wrapped: bool) -> Option<JsFollow> {
    Some(JsFollow::Callable(JsTerminal {
        local: local.to_string(),
        start_line: line,
        end_line: line,
        wrapped,
    }))
}

fn import(local: &str) -> Option<JsFollow> {
    Some(JsFollow::ImportLocal(local.to_string()))
}

pub(super) fn member(module: &str, member: &str) -> Option<JsFollow> {
    Some(JsFollow::NamespaceMember {
        module_path: module.to_string(),
        member: member.to_string(),
    })
}

/// `(id, source, site name, want)`.
pub(super) fn check(paths: &[&str], rows: Vec<(&str, String, &str, Option<JsFollow>)>) {
    let mut wrong = Vec::new();
    for path in paths {
        for (id, src, name, want) in &rows {
            let p = parse(path, src);
            let at = src
                .rfind(&format!("{name}("))
                .unwrap_or_else(|| panic!("{id}: no site `{name}(`"));
            let site = p
                .tree
                .root_node()
                .descendant_for_byte_range(at, at + name.len())
                .unwrap();
            assert_eq!(site.kind(), "identifier", "{id}");
            let mut cache = JsBindingCache::default();
            let got = p.js_ts_follow_alias(site, name, &mut cache);
            if got != *want {
                wrong.push(format!("{id} {path}: got {got:?}, want {want:?}"));
            }
        }
    }
    assert!(wrong.is_empty(), "wrong follows:\n{}", wrong.join("\n"));
}

pub(super) fn row<'a>(id: &'a str, src: &str, name: &'a str, want: Option<JsFollow>) -> RowT<'a> {
    (id, src.to_string(), name, want)
}
pub(super) type RowT<'a> = (&'a str, String, &'a str, Option<JsFollow>);

#[test]
fn identifier_links_end_at_the_callable_or_the_import_local() {
    check(
        &ANY,
        vec![
            row(
                "fn",
                "function t() {}\nconst a = t;\na();\n",
                "a",
                callable("t", 1, false),
            ),
            row(
                "const arrow",
                "const t = () => 1;\nconst a = t;\na();\n",
                "a",
                callable("t", 1, false),
            ),
            row(
                "let arrow, never written",
                "let t = () => 1;\nconst a = t;\na();\n",
                "a",
                callable("t", 1, false),
            ),
            row(
                "named import",
                "import { f } from './m';\nconst a = f;\na();\n",
                "a",
                import("f"),
            ),
            row(
                "renamed import",
                "import { g as f } from './m';\nconst a = f;\na();\n",
                "a",
                import("f"),
            ),
            row(
                "default import",
                "import f from './m';\nconst a = f;\na();\n",
                "a",
                import("f"),
            ),
            row(
                "alias in a function body",
                "import { f } from './m';\nfunction c() {\n  const a = f;\n  a();\n}\n",
                "a",
                import("f"),
            ),
            row(
                "alias name equals the import local's spelling in an inner scope",
                "import { f } from './m';\nimport * as N from './m';\nfunction c() {\n  const f = N.g;\n  f();\n}\n",
                "f",
                member("./m", "g"),
            ),
        ],
    );
}

#[test]
fn a_wrapped_terminal_keeps_its_wrapped_flag() {
    check(
        &ANY,
        vec![row(
            "memo",
            "import { memo } from 'react';\nconst W = memo(() => 1);\nconst a = W;\na();\n",
            "a",
            callable("W", 2, true),
        )],
    );
}

#[test]
fn namespace_member_links_name_the_member_of_the_module() {
    let shapes = [
        ("member", "const a = N.f;\na();\n", "a", "f"),
        ("different name", "const a = N.g;\na();\n", "a", "g"),
        ("parenthesized", "const a = (N.f);\na();\n", "a", "f"),
        ("shorthand pattern", "const { f } = N;\nf();\n", "f", "f"),
        ("renamed pattern", "const { g: a } = N;\na();\n", "a", "g"),
        (
            "parenthesized pattern value",
            "const { f } = (N);\nf();\n",
            "f",
            "f",
        ),
        (
            "two shorthand keys",
            "const { g, f } = N;\nf();\n",
            "f",
            "f",
        ),
        (
            "two renamed keys",
            "const { g: a, h: b } = N;\na();\n",
            "a",
            "g",
        ),
        (
            "comment inside parentheses",
            "const a = (/* c */ N.f);\na();\n",
            "a",
            "f",
        ),
    ];
    let rows = shapes
        .iter()
        .map(|(id, body, name, m)| row(id, &format!("{NS}{body}"), name, member("./m", m)))
        .collect();
    check(&ANY, rows);
}

#[test]
fn typescript_assertions_are_unwrapped() {
    let shapes = [
        ("as", "const a = N.f as any;\na();\n"),
        ("satisfies", "const a = N.f satisfies unknown;\na();\n"),
        ("non-null", "const a = N.f!;\na();\n"),
        ("angle", "const a = <any>N.f;\na();\n"),
        ("as in pattern", "const { f: a } = N as any;\na();\n"),
    ];
    let rows = shapes
        .iter()
        .map(|(id, body)| row(id, &format!("{NS}{body}"), "a", member("./m", "f")))
        .collect();
    check(&TS, rows);
    check(
        &TS,
        vec![row(
            "identifier under as",
            "function t() {}\nconst a = t as any;\na();\n",
            "a",
            callable("t", 1, false),
        )],
    );
}

#[test]
fn chains_mix_link_kinds_and_stop_after_eight_links() {
    let eight = "import { f as imp } from './m';\nconst a1 = imp; const a2 = a1; const a3 = a2; \
        const a4 = a3; const a5 = a4; const a6 = a5; const a7 = a6; const f = a7;\nf();\n";
    let nine = eight.replace("const f = a7;", "const a8 = a7; const f = a8;");
    check(
        &ANY,
        vec![
            row(
                "member then identifier",
                &format!("{NS}const a = N.f;\nconst b = a;\nb();\n"),
                "b",
                member("./m", "f"),
            ),
            row(
                "import then identifier",
                "import { f } from './m';\nconst a = f;\nconst b = a;\nb();\n",
                "b",
                import("f"),
            ),
            row("eight links", eight, "f", import("imp")),
            row("nine links", &nine, "f", None),
        ],
    );
}

#[test]
fn cycles_are_not_followed() {
    check(
        &ANY,
        vec![
            row("two-cycle", "const a = b;\nconst b = a;\na();\n", "a", None),
            row("self", "const a = a;\na();\n", "a", None),
            row(
                "three-cycle behind an import",
                "import { f } from './m';\nconst a = b;\nconst b = c;\nconst c = a;\na();\n",
                "a",
                None,
            ),
        ],
    );
}

#[test]
fn only_const_declarators_are_links() {
    check(
        &ANY,
        vec![
            row("let", "function t() {}\nlet a = t;\na();\n", "a", None),
            row("var", "function t() {}\nvar a = t;\na();\n", "a", None),
            row(
                "let in a pattern",
                &format!("{NS}let {{ f }} = N;\nf();\n"),
                "f",
                None,
            ),
            row(
                "var in a pattern",
                &format!("{NS}var {{ f }} = N;\nf();\n"),
                "f",
                None,
            ),
            row(
                "assignment",
                "function t() {}\nlet a;\na = t;\na();\n",
                "a",
                None,
            ),
        ],
    );
    check(
        &TS,
        vec![row(
            "using",
            "function t() {}\nusing a = t;\na();\n",
            "a",
            None,
        )],
    );
}

#[test]
fn a_written_or_duplicated_alias_is_not_followed() {
    check(
        &ANY,
        vec![
            row(
                "written at its scope",
                "function t() {}\nconst a = t;\na = 1;\na();\n",
                "a",
                None,
            ),
            row(
                "written in a nested closure",
                "function t() {}\nconst a = t;\nfunction w() {\n  a = 2;\n}\na();\n",
                "a",
                None,
            ),
            row(
                "duplicate declaration",
                "function t() {}\nconst a = t;\nfunction a() {}\na();\n",
                "a",
                None,
            ),
            row(
                "link target written",
                "let t = () => 1;\nt = () => 2;\nconst a = t;\na();\n",
                "a",
                None,
            ),
            row(
                "namespace written",
                &format!("{NS}N = 3;\nconst a = N.f;\na();\n"),
                "a",
                None,
            ),
        ],
    );
}

#[test]
fn pattern_shapes_outside_the_flat_object_pattern_are_not_followed() {
    let shapes = [
        ("default", "const { f = 1 } = N;\nf();\n", "f"),
        (
            "default on another key",
            "const { g = 1, f } = N;\nf();\n",
            "f",
        ),
        (
            "default on a renamed key",
            "const { g: a = 1 } = N;\na();\n",
            "a",
        ),
        ("nested", "const { x: { f } } = N;\nf();\n", "f"),
        ("rest element", "const { ...f } = N;\nf();\n", "f"),
        ("computed key", "const { ['f']: a } = N;\na();\n", "a"),
        ("string key", "const { 'f': a } = N;\na();\n", "a"),
        ("numeric key", "const { 1: a } = N;\na();\n", "a"),
        (
            "same name bound twice",
            "const { f, g: f } = N;\nf();\n",
            "f",
        ),
        ("array pattern", "const [a] = N;\na();\n", "a"),
        ("array inside object", "const { x: [a] } = N;\na();\n", "a"),
        ("call value", "const { f } = N();\nf();\n", "f"),
        ("member value", "const { f } = N.x;\nf();\n", "f"),
    ];
    let rows = shapes
        .iter()
        .map(|(id, body, name)| row(id, &format!("{NS}{body}"), name, None))
        .collect();
    check(&ANY, rows);
}

#[test]
fn member_shapes_outside_the_plain_namespace_member_are_not_followed() {
    let shapes = [
        ("optional chain", "const a = N?.f;\na();\n"),
        ("computed access", "const a = N['f'];\na();\n"),
        ("deep member", "const a = N.x.f;\na();\n"),
        ("call result", "const a = N.f();\na();\n"),
        ("conditional", "const a = c ? N.f : N.g;\na();\n"),
        ("logical", "const a = N.f || N.g;\na();\n"),
        ("assignment chain", "let b;\nconst a = (b = N.f);\na();\n"),
        ("bind", "const a = N.f.bind(null);\na();\n"),
        ("array element", "const a = [N.f][0];\na();\n"),
    ];
    let rows = shapes
        .iter()
        .map(|(id, body)| row(id, &format!("{NS}{body}"), "a", None))
        .collect();
    check(&ANY, rows);
}

#[test]
fn the_qualifier_must_be_one_program_scope_value_namespace_import() {
    check(
        &ANY,
        vec![
            row(
                "object literal",
                "const N = { f() {} };\nconst a = N.f;\na();\n",
                "a",
                None,
            ),
            row(
                "parameter shadows",
                &format!("{NS}function c(N) {{\n  const a = N.f;\n  a();\n}}\n"),
                "a",
                None,
            ),
            row(
                "inner declaration shadows",
                &format!("{NS}function c() {{\n  const N = {{}};\n  const a = N.f;\n  a();\n}}\n"),
                "a",
                None,
            ),
            row(
                "require result",
                "const N = require('./m');\nconst a = N.f;\na();\n",
                "a",
                None,
            ),
            row(
                "default import",
                "import N from './m';\nconst a = N.f;\na();\n",
                "a",
                None,
            ),
            row(
                "named import",
                "import { N } from './m';\nconst a = N.f;\na();\n",
                "a",
                None,
            ),
            row(
                "duplicated name",
                &format!("{NS}var N = 1;\nconst a = N.f;\na();\n"),
                "a",
                None,
            ),
            row(
                "alias of a namespace",
                &format!("{NS}const M = N;\nconst a = M.f;\na();\n"),
                "a",
                None,
            ),
            row("undeclared", "const a = N.f;\na();\n", "a", None),
            row(
                "pattern over an object literal",
                "const N = { f() {} };\nconst { f } = N;\nf();\n",
                "f",
                None,
            ),
            row(
                "pattern over a parameter",
                &format!("{NS}function c(N) {{\n  const {{ f }} = N;\n  f();\n}}\n"),
                "f",
                None,
            ),
        ],
    );
    check(
        &TS,
        vec![
            row(
                "import alias of a namespace body is not a program import",
                "namespace M {\n  import f = Other.g;\n  const a = f;\n  a();\n}\n",
                "a",
                None,
            ),
            row(
                "type-only namespace import",
                "import type * as N from './m';\nconst a = N.f;\na();\n",
                "a",
                None,
            ),
            row(
                "import equals",
                "import N = require('./m');\nconst a = N.f;\na();\n",
                "a",
                None,
            ),
        ],
    );
}

#[test]
fn an_identifier_link_needs_a_callable_an_import_or_another_link() {
    check(
        &ANY,
        vec![
            row(
                "parameter",
                "function c(p) {\n  const a = p;\n  a();\n}\n",
                "a",
                None,
            ),
            row("unbound", "const a = missing;\na();\n", "a", None),
            row(
                "call result",
                "const t = make();\nconst a = t;\na();\n",
                "a",
                None,
            ),
            row(
                "conditional value",
                "function t() {}\nfunction u() {}\nconst a = c ? t : u;\na();\n",
                "a",
                None,
            ),
            row(
                "require destructure",
                "const { f } = require('./m');\nconst a = f;\na();\n",
                "a",
                None,
            ),
            row(
                "member call result",
                "const t = o.make();\nconst a = t;\na();\n",
                "a",
                None,
            ),
            row(
                "import written",
                "import { f } from './m';\nf = 1;\nconst a = f;\na();\n",
                "a",
                None,
            ),
            row("class", "class t {}\nconst a = t;\na();\n", "a", None),
            row(
                "object literal",
                "const t = { f() {} };\nconst a = t;\na();\n",
                "a",
                None,
            ),
            row(
                "catch binding",
                "try {} catch (t) {\n  const a = t;\n  a();\n}\n",
                "a",
                None,
            ),
            row(
                "import in an inner scope is a parameter",
                "import { f } from './m';\nfunction c(f) {\n  const a = f;\n  a();\n}\n",
                "a",
                None,
            ),
        ],
    );
}

#[test]
fn a_parse_error_in_the_scope_stops_the_follow() {
    let broken = "export function broken() {\n  return <div>{</div>;\n}\n";
    check(
        &["a.js", "a.tsx"],
        vec![row(
            "unsealed error at module scope",
            &format!(
                "function t() {{}}\nconst a = t;\nexport function c() {{\n  a();\n}}\n{broken}"
            ),
            "a",
            None,
        )],
    );
}

#[test]
fn a_site_that_is_not_an_alias_follows_nothing() {
    check(
        &ANY,
        vec![
            row("function", "function a() {}\na();\n", "a", None),
            row("arrow", "const a = () => 1;\na();\n", "a", None),
            row("import", "import { a } from './m';\na();\n", "a", None),
        ],
    );
}

#[test]
fn module_export_classification_of_an_alias_is_unchanged() {
    // D4 reads `js_ts_module_binding`; an alias link stays `Alias` there.
    for path in ANY {
        for src in [
            "function t() {}\nexport const a = t;\n",
            "import * as N from './m';\nexport const a = N.f;\n",
            "import * as N from './m';\nexport const { f: a } = N;\n",
        ] {
            let p = parse(path, src);
            let site = crate::ast::js_binding_helper_tests::ident(&p, "a", 0);
            let mut cache = JsBindingCache::default();
            assert_eq!(
                p.js_ts_module_binding("a", site, &mut cache),
                JsBinding::Alias,
                "{path} {src:?}"
            );
        }
    }
}
