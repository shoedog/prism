//! S1b-2a unit rows (SPEC §7, IMPLEMENTOR "S1b-2a dispatch"): the module terminal's binding
//! for a name, not an edge (S1b-2b re-asserts these rows as edges). Every row runs in the
//! JavaScript grammar and the TSX grammar unless it is TS-only or a `.cjs` row.
use super::{JsBinding, JsBindingCache, JsTerminal, Strictness, CLASSIFIED};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

const BOTH: [&str; 2] = ["a.js", "a.tsx"];
const TS: [&str; 2] = ["a.ts", "a.tsx"];

fn parse(path: &str, src: &str) -> ParsedFile {
    let lang = match path.rsplit('.').next() {
        Some("tsx") => Language::Tsx,
        Some("ts" | "mts") => Language::TypeScript,
        _ => Language::JavaScript,
    };
    ParsedFile::parse(path, src, lang).unwrap()
}

/// The smallest node spanning the first occurrence of `at`.
fn node_at<'a>(p: &'a ParsedFile, at: &str) -> Node<'a> {
    let start = p.source.find(at).unwrap_or_else(|| panic!("no {at:?}"));
    let root = p.tree.root_node();
    root.descendant_for_byte_range(start, start + at.len())
        .unwrap()
}

fn callable(local: &str, start_line: usize, end_line: usize) -> JsBinding {
    JsBinding::Callable(JsTerminal {
        local: local.to_string(),
        start_line,
        end_line,
        wrapped: false,
    })
}

const DUP: JsBinding = JsBinding::Refused("duplicate_declaration");
const PARSE: JsBinding = JsBinding::Refused("parse_recovery");

/// `(control id, source, name, site text, expected)`.
type Row<'s> = (&'s str, &'s str, &'s str, &'s str, JsBinding);

/// Runs every row in every grammar and reports every wrong value at once. `errors` pins
/// whether the source must parse with an error (B1 rows) or cleanly (every other row), so a
/// row cannot pass by accident of parse recovery.
fn check(paths: &[&str], errors: bool, rows: Vec<Row<'_>>) {
    let mut wrong = Vec::new();
    for path in paths {
        for (id, src, name, at, want) in &rows {
            let p = parse(path, src);
            assert_eq!(p.tree.root_node().has_error(), errors, "{id} {path}: parse");
            let got = p.js_ts_module_binding(name, node_at(&p, at), &mut JsBindingCache::default());
            if got != *want {
                wrong.push(format!("{id} {path} `{name}`: got {got:?}, want {want:?}"));
            }
        }
    }
    assert!(wrong.is_empty(), "wrong bindings:\n{}", wrong.join("\n"));
}

#[test]
fn b4_duplicate_declarations_refuse() {
    check(
        &BOTH,
        false,
        vec![
            (
                "C74",
                "var f = () => 1;\nvar f = () => 2;\nexport { f };\n",
                "f",
                "export { f }",
                DUP,
            ),
            (
                "C75",
                "export function f() {\n  return 1;\n}\nif (globalThis.x) {\n  var f = 2;\n}\n",
                "f",
                "export function f",
                DUP,
            ),
        ],
    );
}

#[test]
fn b5_named_function_expression_binds_by_its_own_name() {
    let src = "const f = function g() {\n  return 1;\n};\nexport { f };\n";
    check(
        &BOTH,
        false,
        vec![("C78", src, "f", "export { f }", callable("g", 1, 3))],
    );
}

#[test]
fn b6_parse_recovery_refuses_unless_sealed() {
    let c79 = "export function f() {\n  return 1;\n}\nexport function broken() {\n  return <div>{</div>;\n}\n";
    let c149 = "export function f() {\n  return 1;\n}\nexport function broken() {\n  let x = ;\n  return x;\n}\n";
    let c150 = "export function f() {\n  return 1;\n}\nexport function broken() {\n  let f = ;\n  return f;\n}\n";
    let site = "export function f";
    check(
        &BOTH,
        true,
        vec![
            ("C79", c79, "f", site, PARSE),
            ("C149", c149, "f", site, callable("f", 1, 3)),
            ("C150", c150, "f", site, callable("f", 1, 3)),
        ],
    );
}

#[test]
fn b1_structural_braces_in_a_sealed_error_refuse() {
    let token = "function broken() {\n  let x = 1 {;\n  return x;\n}\nexport function f() {\n  return 1;\n}\n";
    let missing =
        "function broken() {\n  let x = ( { );\n}\nexport function f() {\n  return 1;\n}\n";
    let site = "export function f";
    check(
        &BOTH,
        true,
        vec![
            ("B1(i) ERROR token", token, "f", site, PARSE),
            ("B1(i) MISSING brace", missing, "f", site, PARSE),
        ],
    );
}

#[test]
fn b8_type_space_declares_nothing() {
    let c143 = "function f() {\n  return 1;\n}\ntype f = number;\ninterface g {}\nfunction g() {\n  return 2;\n}\nexport function run() {\n  return f() + g();\n}\n";
    let c144 = "import type { f } from './t';\nfunction f() {\n  return 1;\n}\nexport function run() {\n  return f();\n}\n";
    let c144b = "import { type f } from './t';\nfunction f() {\n  return 1;\n}\nexport function run() {\n  return f();\n}\n";
    let c146 = "export const Button = () => null;\nexport interface Button {\n  x: number;\n}\n";
    let site = "export function run";
    check(
        &TS,
        false,
        vec![
            ("C143 type", c143, "f", site, callable("f", 1, 3)),
            ("C143 interface", c143, "g", site, callable("g", 6, 8)),
            ("C144", c144, "f", site, callable("f", 2, 4)),
            ("C144 specifier", c144b, "f", site, callable("f", 2, 4)),
            (
                "C146",
                c146,
                "Button",
                "export interface",
                callable("Button", 1, 1),
            ),
        ],
    );
}

#[test]
fn b14_header_error_is_not_sealed() {
    // RP2-a: the error sits in an arrow's header, not in a delimited child; the first
    // delimited ancestor is `run`'s body, and `run` holds the site.
    let src = "function f() { return 1; }\nfunction run() { let f = 3 ) ` const g = () => { return 2; }; f(); }\n";
    check(&BOTH, true, vec![("RP2-a", src, "f", "f(); }", PARSE)]);
}

#[test]
fn b15_string_braces_never_break_the_brace_condition() {
    // RP2-b, both quote forms: the error holds a string whose text is a brace.
    let double = "function f() { return 1; }\nfunction broken() {\n  const x = \"{\" \"y\";\n  return x;\n}\nexport function run() {\n  return f();\n}\n";
    let single = "function f() { return 1; }\nfunction broken() {\n  const x = '{' 'y';\n  return x;\n}\nexport function run() {\n  return f();\n}\n";
    let site = "return f();";
    check(
        &BOTH,
        true,
        vec![
            ("RP2-b double", double, "f", site, callable("f", 1, 1)),
            ("RP2-b single", single, "f", site, callable("f", 1, 1)),
        ],
    );
}

#[test]
fn b0_escaped_identifier_refuses() {
    let src = "function f() {\n  return 1;\n}\nvar \\u0067 = 2;\nexport { f };\n";
    let want = JsBinding::Refused("escaped_identifier");
    check(
        &BOTH,
        false,
        vec![("C137 module", src, "f", "export { f }", want)],
    );
}

#[test]
fn b10_classified_equals_the_pinned_grammars_named_kinds() {
    let mut grammar = BTreeSet::new();
    for lang in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let ts = lang.tree_sitter_language();
        for id in 0..ts.node_kind_count() as u16 {
            if ts.node_kind_is_named(id) && ts.node_kind_is_visible(id) {
                grammar.extend(ts.node_kind_for_id(id));
            }
        }
    }
    let runtime: BTreeSet<&str> = CLASSIFIED.split_whitespace().collect();
    let missing: Vec<_> = grammar.difference(&runtime).collect();
    let extra: Vec<_> = runtime.difference(&grammar).collect();
    assert_eq!((missing, extra, runtime.len()), (vec![], vec![], 186));
}

/// `(path, source, site text, expected)`; the node spelled by the site text starts the walk.
type StrictRow = (&'static str, &'static str, &'static str, Strictness);

#[test]
fn b18_strictness_predicate() {
    use Strictness::{Sloppy, Strict, Unknown};
    let plain = "function run() {\n  g();\n}\n";
    let rows: Vec<StrictRow> = vec![
        (
            "a.js",
            "import x from 'y';\nfunction run() {\n  g();\n}\n",
            "g();",
            Strict,
        ),
        (
            "a.tsx",
            "import x from 'y';\nfunction run() {\n  g();\n}\n",
            "g();",
            Strict,
        ),
        (
            "a.js",
            "function run() {\n  g();\n}\nexport {};\n",
            "g();",
            Strict,
        ),
        (
            "a.tsx",
            "function run() {\n  g();\n}\nexport {};\n",
            "g();",
            Strict,
        ),
        ("a.mjs", plain, "g();", Strict),
        ("a.mts", plain, "g();", Strict),
        (
            "a.js",
            "'use strict';\nfunction run() {\n  g();\n}\n",
            "g();",
            Strict,
        ),
        (
            "a.tsx",
            "\"use strict\";\nfunction run() {\n  g();\n}\n",
            "g();",
            Strict,
        ),
        (
            "a.js",
            "// header\n\"use strict\";\nfunction run() {\n  g();\n}\n",
            "g();",
            Strict,
        ),
        (
            "a.js",
            "function run() {\n  'use strict';\n  if (x) {\n    g();\n  }\n}\n",
            "g();",
            Strict,
        ),
        (
            "a.tsx",
            "function run() {\n  \"use strict\";\n  if (x) {\n    g();\n  }\n}\n",
            "g();",
            Strict,
        ),
        (
            "a.cjs",
            "class C {\n  m() {\n    g();\n  }\n}\n",
            "g();",
            Strict,
        ),
        (
            "a.tsx",
            "class C {\n  m() {\n    g();\n  }\n}\n",
            "g();",
            Strict,
        ),
        ("a.cjs", plain, "g();", Sloppy),
        (
            "a.cjs",
            "function run() {\n  'use\\x20strict';\n  g();\n}\n",
            "g();",
            Sloppy,
        ),
        ("a.js", plain, "g();", Unknown),
        ("a.tsx", plain, "g();", Unknown),
        ("a.ts", plain, "g();", Unknown),
        ("a.js", "g();\n'use strict';\n", "g();", Unknown),
        ("a.tsx", "g();\n'use strict';\n", "g();", Unknown),
    ];
    let mut wrong = Vec::new();
    for (path, src, at, want) in rows {
        let p = parse(path, src);
        assert!(!p.tree.root_node().has_error(), "{path} {src:?}: parse");
        let got = p.js_ts_strictness(node_at(&p, at));
        if got != want {
            wrong.push(format!("{path} {src:?}: got {got:?}, want {want:?}"));
        }
    }
    assert!(wrong.is_empty(), "wrong strictness:\n{}", wrong.join("\n"));
}

const BLOCK_FN: &str = "function inner() {\n  return 0;\n}\nfunction run(flag) {\n  if (flag) {\n    function inner() {\n      return 1;\n    }\n  }\n  return inner();\n}\n";

/// The declaration walk over `run`'s body, hoisting (as for a function body): the kinds of
/// the nodes it records for `inner`, and the unknown-strictness Annex-B names.
fn run_body_inner(path: &str, src: &str) -> (Vec<&'static str>, BTreeSet<String>) {
    let p = parse(path, src);
    assert!(!p.tree.root_node().has_error(), "{path}: parse");
    let run = p.all_functions().into_iter().find(|f| {
        let name = p.language.function_name(f);
        name.is_some_and(|n| p.node_text(&n) == "run")
    });
    let body = run
        .and_then(|f| f.child_by_field_name("body"))
        .expect("run's body");
    let (mut out, mut annex) = (super::Index::new(), BTreeSet::new());
    let mode = (true, true, p.js_ts_strictness(body));
    p.js_ts_declare_walk(body, mode, &BTreeSet::new(), &mut out, &mut annex);
    let kinds = out
        .get("inner")
        .map(|v| v.iter().map(|n| n.kind()).collect());
    (kinds.unwrap_or_default(), annex)
}

#[test]
fn b18_annex_b_marker_only_in_sloppy_code() {
    let module = format!("{BLOCK_FN}export {{}};\n");
    let directive = format!("'use strict';\n{BLOCK_FN}");
    let function_directive = BLOCK_FN.replace("(flag) {\n", "(flag) {\n  \"use strict\";\n");
    let generator = BLOCK_FN.replace("function inner", "function* inner");
    let generator = generator
        .replace("return 0", "yield 0")
        .replace("return 1", "yield 1");
    let asynchronous = BLOCK_FN.replace("    function inner", "    async function inner");
    let c159 = BLOCK_FN.replace("(flag)", "(flag: boolean)");
    let none = || (Vec::<&str>::new(), BTreeSet::new());
    let annex = || (Vec::<&str>::new(), BTreeSet::from(["inner".to_string()]));
    let rows = [
        (
            "C163",
            "a.cjs",
            BLOCK_FN,
            (vec!["statement_block"], BTreeSet::new()),
        ),
        ("C164", "a.cjs", generator.as_str(), none()),
        ("async", "a.cjs", asynchronous.as_str(), none()),
        ("C111", "a.js", module.as_str(), none()),
        ("C111", "a.tsx", module.as_str(), none()),
        ("C161", "a.js", directive.as_str(), none()),
        ("C161", "a.tsx", directive.as_str(), none()),
        ("C162", "a.js", function_directive.as_str(), none()),
        ("C162", "a.tsx", function_directive.as_str(), none()),
        ("C159", "a.ts", c159.as_str(), annex()),
        ("C160", "a.js", BLOCK_FN, annex()),
        ("C160", "a.tsx", BLOCK_FN, annex()),
    ];
    let mut wrong = Vec::new();
    for (id, path, src, want) in rows {
        let got = run_body_inner(path, src);
        if got != want {
            wrong.push(format!("{id} {path}: got {got:?}, want {want:?}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "wrong Annex-B records:\n{}",
        wrong.join("\n")
    );
}

#[test]
fn b18_module_terminal_consumes_markers_and_unknown_strictness() {
    let block = "function inner() {\n  return 0;\n}\nif (flag) {\n  function inner() {\n    return 1;\n  }\n}\n";
    let cjs = format!("{block}module.exports = {{ inner }};\n");
    let generator = cjs.replace("function inner", "function* inner");
    let site = "if (flag)";
    check(
        &["a.cjs"],
        false,
        vec![("C163 module scope", &cjs, "inner", site, DUP)],
    );
    let outer = callable("inner", 1, 3);
    check(
        &["a.cjs"],
        false,
        vec![("C164 module scope", &generator, "inner", site, outer)],
    );
    let unknown = JsBinding::Unchecked("annex_b_strictness");
    check(
        &BOTH,
        false,
        vec![("C160 module scope", block, "inner", site, unknown)],
    );
    let module = format!("{block}export {{ inner }};\n");
    check(
        &BOTH,
        false,
        vec![(
            "C111 module scope",
            &module,
            "inner",
            site,
            callable("inner", 1, 3),
        )],
    );
}
