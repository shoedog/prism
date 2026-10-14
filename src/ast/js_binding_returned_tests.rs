//! Slice V unit rows: which `const` bindings `js_ts_follow_returned` follows, to which hook
//! terminal and which part of its result. The site is always the LAST `name(` in the source.
//! Every "not followed" row is a condition the contract names.
use super::JsBindingCache;
use crate::ast::js_binding_helper_tests::parse;
use crate::ast::{JsFollow, JsTerminal};
use crate::js_returns::JsReturnPath;

const ANY: [&str; 3] = ["a.js", "a.ts", "a.tsx"];
const TS: [&str; 2] = ["a.ts", "a.tsx"];

type Want = Option<(JsFollow, JsReturnPath)>;

fn local(name: &str, line: usize) -> JsFollow {
    JsFollow::Callable(JsTerminal {
        local: name.to_string(),
        start_line: line,
        end_line: line,
        wrapped: false,
    })
}

fn import(name: &str) -> JsFollow {
    JsFollow::ImportLocal(name.to_string())
}

fn key(k: &str) -> JsReturnPath {
    JsReturnPath::Key(k.to_string())
}

/// `(id, source, site name, want)`.
fn check(paths: &[&str], rows: Vec<(&str, &str, &str, Want)>) {
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
            let got = p.js_ts_follow_returned(site, name, &mut cache);
            if got != *want {
                wrong.push(format!("{id} {path}: got {got:?}, want {want:?}"));
            }
        }
    }
    assert!(wrong.is_empty(), "wrong follows:\n{}", wrong.join("\n"));
}

#[test]
fn the_three_declarator_forms_follow_a_same_file_hook() {
    check(
        &ANY,
        vec![
            (
                "form a",
                "function H() {}\nconst x = H();\nx();\n",
                "x",
                Some((local("H", 1), JsReturnPath::Whole)),
            ),
            (
                "form b shorthand",
                "function H() {}\nconst { save } = H();\nsave();\n",
                "save",
                Some((local("H", 1), key("save"))),
            ),
            (
                "form b renamed",
                "function H() {}\nconst { save: run, other } = H();\nrun();\n",
                "run",
                Some((local("H", 1), key("save"))),
            ),
            (
                "form c first slot",
                "function H() {}\nconst [a] = H();\na();\n",
                "a",
                Some((local("H", 1), JsReturnPath::Slot(0))),
            ),
            (
                "form c later slot with holes",
                "function H() {}\nconst [a, , c, d] = H();\nd();\n",
                "d",
                Some((local("H", 1), JsReturnPath::Slot(3))),
            ),
            (
                "const arrow hook",
                "const H = () => 1;\nconst { save } = H();\nsave();\n",
                "save",
                Some((local("H", 1), key("save"))),
            ),
            (
                "call with an inline function argument (may-call on the base)",
                "function H() {}\nconst { save } = H(() => 1);\nsave();\n",
                "save",
                Some((local("H", 1), key("save"))),
            ),
            (
                "call with other arguments",
                "function H() {}\nconst x = H(1, 'a');\nx();\n",
                "x",
                Some((local("H", 1), JsReturnPath::Whole)),
            ),
            (
                "parenthesized call",
                "function H() {}\nconst x = (H());\nx();\n",
                "x",
                Some((local("H", 1), JsReturnPath::Whole)),
            ),
            (
                "hook is an alias of a local function",
                "function real() {}\nconst H = real;\nconst x = H();\nx();\n",
                "x",
                Some((local("real", 1), JsReturnPath::Whole)),
            ),
        ],
    );
}

#[test]
fn the_hook_terminal_is_an_import_local_or_a_namespace_member() {
    check(
        &ANY,
        vec![
            (
                "named import",
                "import { H } from './m';\nconst { save } = H();\nsave();\n",
                "save",
                Some((import("H"), key("save"))),
            ),
            (
                "renamed import",
                "import { use as H } from './m';\nconst { save } = H();\nsave();\n",
                "save",
                Some((import("H"), key("save"))),
            ),
            (
                "default import",
                "import H from './m';\nconst { save } = H();\nsave();\n",
                "save",
                Some((import("H"), key("save"))),
            ),
            (
                "alias of an import",
                "import { use } from './m';\nconst H = use;\nconst x = H();\nx();\n",
                "x",
                Some((import("use"), JsReturnPath::Whole)),
            ),
            (
                "alias of a namespace member",
                "import * as N from './m';\nconst H = N.use;\nconst x = H();\nx();\n",
                "x",
                Some((
                    JsFollow::NamespaceMember {
                        module_path: "./m".to_string(),
                        member: "use".to_string(),
                    },
                    JsReturnPath::Whole,
                )),
            ),
        ],
    );
}

#[test]
fn typescript_wrappers_around_the_call_are_unwrapped() {
    check(
        &TS,
        vec![
            (
                "generic call",
                "function H<T>() {}\nconst { save } = H<number>();\nsave();\n",
                "save",
                Some((local("H", 1), key("save"))),
            ),
            (
                "as assertion",
                "function H() {}\nconst { save } = H() as any;\nsave();\n",
                "save",
                Some((local("H", 1), key("save"))),
            ),
            (
                "typed declarator",
                "function H() {}\nconst [a, b]: [number, number] = H();\nb();\n",
                "b",
                Some((local("H", 1), JsReturnPath::Slot(1))),
            ),
        ],
    );
}

#[test]
fn a_binding_or_call_the_contract_excludes_is_not_followed() {
    check(
        &ANY,
        vec![
            ("let", "function H() {}\nlet x = H();\nx();\n", "x", None),
            ("var", "function H() {}\nvar x = H();\nx();\n", "x", None),
            (
                "written const",
                "function H() {}\nconst x = H();\nx = null;\nx();\n",
                "x",
                None,
            ),
            (
                "tagged template",
                "function H() {}\nconst x = H`a`;\nx();\n",
                "x",
                None,
            ),
            (
                "written const-like let",
                "function H() {}\nlet x = H();\nx = null;\nx();\n",
                "x",
                None,
            ),
            (
                "new call",
                "function H() {}\nconst x = new H();\nx();\n",
                "x",
                None,
            ),
            (
                "optional call",
                "function H() {}\nconst x = H?.();\nx();\n",
                "x",
                None,
            ),
            (
                "awaited call",
                "async function H() {}\nasync function r() {\n  const x = await H();\n  x();\n}\n",
                "x",
                None,
            ),
            (
                "member call",
                "const o = { H() {} };\nconst x = o.H();\nx();\n",
                "x",
                None,
            ),
            (
                "call on a call result",
                "function H() {}\nconst x = H()();\nx();\n",
                "x",
                None,
            ),
            (
                "hook is a parameter",
                "function r(H) {\n  const { save } = H();\n  save();\n}\n",
                "save",
                None,
            ),
            (
                "hook is not declared",
                "const { save } = H();\nsave();\n",
                "save",
                None,
            ),
            (
                "hook is a let that is written",
                "let H = () => 1;\nH = () => 2;\nconst x = H();\nx();\n",
                "x",
                None,
            ),
            (
                "hook is a call result",
                "function mk() {}\nconst H = mk();\nconst x = H();\nx();\n",
                "x",
                None,
            ),
            (
                "hook is a non-program import",
                "function r() {\n  const H = 1;\n  const x = H();\n  x();\n}\n",
                "x",
                None,
            ),
            (
                "not a call",
                "function H() {}\nconst x = H;\nx();\n",
                "x",
                None,
            ),
            (
                "non-identifier hook expression",
                "const o = {};\nconst x = (0, o.H)();\nx();\n",
                "x",
                None,
            ),
        ],
    );
}

#[test]
fn an_object_pattern_with_any_unmodeled_entry_is_not_followed() {
    let shapes = [
        ("rest sibling", "const { save, ...rest } = H();\nsave();\n"),
        (
            "nested sibling",
            "const { save, x: { y } } = H();\nsave();\n",
        ),
        ("default", "const { save = 1 } = H();\nsave();\n"),
        ("renamed default", "const { a: save = 1 } = H();\nsave();\n"),
        (
            "computed sibling",
            "const { save, ['k']: z } = H();\nsave();\n",
        ),
        (
            "string key sibling",
            "const { save, 'k': z } = H();\nsave();\n",
        ),
        (
            "numeric key sibling",
            "const { save, 1: z } = H();\nsave();\n",
        ),
        (
            "name bound twice",
            "const { a: save, b: save } = H();\nsave();\n",
        ),
    ];
    let rows = shapes
        .iter()
        .map(|(id, body)| (*id, body.to_string(), "save"))
        .collect::<Vec<_>>();
    let owned: Vec<String> = rows
        .iter()
        .map(|(_, body, _)| format!("function H() {{}}\n{body}"))
        .collect();
    let rows: Vec<_> = rows
        .iter()
        .zip(&owned)
        .map(|((id, _, name), src)| (*id, src.as_str(), *name, None))
        .collect();
    check(&ANY, rows);
}

#[test]
fn an_array_pattern_with_any_unmodeled_element_is_not_followed() {
    let shapes = [
        ("rest sibling", "const [a, ...rest] = H();\na();\n", "a"),
        ("rest before", "const [...rest] = H();\nrest();\n", "rest"),
        ("default", "const [a = 1] = H();\na();\n", "a"),
        ("default sibling", "const [a, b = 1] = H();\na();\n", "a"),
        ("nested sibling", "const [a, [b]] = H();\na();\n", "a"),
        (
            "nested object sibling",
            "const [a, { b }] = H();\na();\n",
            "a",
        ),
        ("name bound twice", "const [a, a] = H();\na();\n", "a"),
    ];
    let owned: Vec<String> = shapes
        .iter()
        .map(|(_, body, _)| format!("function H() {{}}\n{body}"))
        .collect();
    let rows: Vec<_> = shapes
        .iter()
        .zip(&owned)
        .map(|((id, _, name), src)| (*id, src.as_str(), *name, None))
        .collect();
    check(&ANY, rows);
}

#[test]
fn an_admitted_react_wrapper_stays_a_callable_and_is_never_a_returned_value() {
    check(
        &ANY,
        vec![(
            "useCallback",
            "import { useCallback } from 'react';\nfunction H() {}\nconst x = useCallback(() => 1, []);\nx();\n",
            "x",
            None,
        )],
    );
}
