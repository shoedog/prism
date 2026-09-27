//! S1b-2a unit rows (SPEC §7, IMPLEMENTOR "S1b-2a dispatch"): the module terminal's binding
//! for a name, not an edge (S1b-2b re-asserts these rows as edges). Every row runs in the
//! JavaScript grammar and the TSX grammar unless it is TS-only or a `.cjs` row. Sources are
//! the controls' (`probes/controls_gen.py` ids), moved to module scope where the control's
//! site is nested.
use super::{JsBinding, JsBindingCache, JsTerminal, Strictness, CLASSIFIED};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

const BOTH: [&str; 2] = ["a.js", "a.tsx"];
const TS: [&str; 2] = ["a.ts", "a.tsx"];
const DUP: JsBinding = JsBinding::Refused("duplicate_declaration");
const PARSE: JsBinding = JsBinding::Refused("parse_recovery");

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
            let mut cache = JsBindingCache::default();
            let got = p.js_ts_module_binding(name, node_at(&p, at), &mut cache);
            if got != *want {
                wrong.push(format!("{id} {path} `{name}`: got {got:?}, want {want:?}"));
            }
        }
    }
    assert!(wrong.is_empty(), "wrong bindings:\n{}", wrong.join("\n"));
}

const C74: &str = "var f = () => 1;\nvar f = () => 2;\nexport { f };\n";
const C75: &str = "export function f() {\n  return 1;\n}\nif (globalThis.x) {\n  var f = 2;\n}\n";

#[test]
fn b4_duplicate_declarations_refuse() {
    let rows = vec![
        ("C74", C74, "f", "export { f }", DUP),
        ("C75", C75, "f", "export function f", DUP),
    ];
    check(&BOTH, false, rows);
}

#[test]
fn b5_named_function_expression_binds_by_its_own_name() {
    let c78 = "const f = function g() {\n  return 1;\n};\nexport { f };\n";
    check(
        &BOTH,
        false,
        vec![("C78", c78, "f", "export { f }", callable("g", 1, 3))],
    );
}

const F: &str = "export function f() {\n  return 1;\n}\n";
const C79: &str = "export function broken() {\n  return <div>{</div>;\n}\n";
const C149: &str = "export function broken() {\n  let x = ;\n  return x;\n}\n";
const C150: &str = "export function broken() {\n  let f = ;\n  return f;\n}\n";
/// B1 (i): an `ERROR` holding a `{` token, and a `MISSING` brace, both sealed by (ii).
const TOKEN: &str = "function broken() {\n  let x = 1 {;\n  return x;\n}\n";
const MISSING: &str = "function broken() {\n  let x = ( { );\n}\n";

#[test]
fn b6_parse_recovery_refuses_unless_sealed() {
    let (c79, c149, c150) = (F.to_owned() + C79, F.to_owned() + C149, F.to_owned() + C150);
    let site = "export function f";
    let rows = vec![
        ("C79", c79.as_str(), "f", site, PARSE),
        ("C149", &c149, "f", site, callable("f", 1, 3)),
        // r1 fold (Opus W2): the sealed function mentions `f`, so M2 keeps base behavior.
        ("C150", &c150, "f", site, MAY),
    ];
    check(&BOTH, true, rows);
}

#[test]
fn b1_structural_braces_in_a_sealed_error_refuse() {
    let (token, missing) = (TOKEN.to_owned() + F, MISSING.to_owned() + F);
    let site = "export function f";
    let rows = vec![
        ("B1(i) ERROR token", token.as_str(), "f", site, PARSE),
        ("B1(i) MISSING brace", &missing, "f", site, PARSE),
    ];
    check(&BOTH, true, rows);
}

const RUN: &str = "export function run() {\n  return f() + g();\n}\n";
const C143: &str = "function f() {\n  return 1;\n}\ntype f = number;\ninterface g {}\n\
    function g() {\n  return 2;\n}\n";
const C144: &str = "import type { f } from './t';\nfunction f() {\n  return 1;\n}\n";
const C144B: &str = "import { type f } from './t';\nfunction f() {\n  return 1;\n}\n";
const C146: &str =
    "export const Button = () => null;\nexport interface Button {\n  x: number;\n}\n";

#[test]
fn b8_type_space_declares_nothing() {
    let (c143, c144, c144b) = (
        C143.to_owned() + RUN,
        C144.to_owned() + RUN,
        C144B.to_owned() + RUN,
    );
    let site = "export function run";
    let rows = vec![
        ("C143 type", c143.as_str(), "f", site, callable("f", 1, 3)),
        ("C143 interface", &c143, "g", site, callable("g", 6, 8)),
        ("C144", &c144, "f", site, callable("f", 2, 4)),
        ("C144 specifier", &c144b, "f", site, callable("f", 2, 4)),
        (
            "C146",
            C146,
            "Button",
            "export interface",
            callable("Button", 1, 1),
        ),
    ];
    check(&TS, false, rows);
}

/// RP2-a: the error sits in an arrow's header, not in a delimited child; the first delimited
/// ancestor is `run`'s body, and `run` holds the site.
const RP2A: &str = "function f() { return 1; }\n\
    function run() { let f = 3 ) ` const g = () => { return 2; }; f(); }\n";

#[test]
fn b14_header_error_is_not_sealed() {
    check(&BOTH, true, vec![("RP2-a", RP2A, "f", "f(); }", PARSE)]);
}

/// RP2-b: an error holding a string whose text is a brace.
const RP2B: &str = "function f() { return 1; }\nfunction broken() {\n  const x = \"{\" \"y\";\n\
    return x;\n}\nexport function run() {\n  return f();\n}\n";

#[test]
fn b15_string_braces_never_break_the_brace_condition() {
    let single = RP2B.replace('"', "'");
    let rows = vec![
        (
            "RP2-b double",
            RP2B,
            "f",
            "return f();",
            callable("f", 1, 1),
        ),
        (
            "RP2-b single",
            &single,
            "f",
            "return f();",
            callable("f", 1, 1),
        ),
    ];
    check(&BOTH, true, rows);
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

const PLAIN: &str = "function run() {\n  g();\n}\n";
const IMPORTS: &str = "import x from 'y';\nfunction run() {\n  g();\n}\n";
const EXPORTS: &str = "function run() {\n  g();\n}\nexport {};\n";
const DIRECTIVE: &str = "'use strict';\nfunction run() {\n  g();\n}\n";
const COMMENTED: &str = "// header\n\"use strict\";\nfunction run() {\n  g();\n}\n";
const FN_DIRECTIVE: &str = "function run() {\n  'use strict';\n  if (x) {\n    g();\n  }\n}\n";
const CLASS: &str = "class C {\n  m() {\n    g();\n  }\n}\n";
const ESCAPED: &str = "function run() {\n  'use\\x20strict';\n  g();\n}\n";
const LATE: &str = "g();\n'use strict';\n";

#[test]
fn b18_strictness_predicate() {
    use Strictness::{Sloppy, Strict, Unknown};
    let quoted = FN_DIRECTIVE.replace('\'', "\"");
    let rows = [
        ("a.js", IMPORTS, Strict),
        ("a.tsx", IMPORTS, Strict),
        ("a.js", EXPORTS, Strict),
        ("a.tsx", EXPORTS, Strict),
        ("a.mjs", PLAIN, Strict),
        ("a.mts", PLAIN, Strict),
        ("a.js", DIRECTIVE, Strict),
        ("a.tsx", DIRECTIVE, Strict),
        ("a.js", COMMENTED, Strict),
        ("a.tsx", COMMENTED, Strict),
        ("a.js", FN_DIRECTIVE, Strict),
        ("a.tsx", &quoted, Strict),
        ("a.cjs", CLASS, Strict),
        ("a.tsx", CLASS, Strict),
        ("a.cjs", PLAIN, Sloppy),
        ("a.cjs", ESCAPED, Sloppy),
        ("a.js", PLAIN, Unknown),
        ("a.tsx", PLAIN, Unknown),
        ("a.ts", PLAIN, Unknown),
        ("a.js", LATE, Unknown),
        ("a.tsx", LATE, Unknown),
    ];
    let mut wrong = Vec::new();
    for (path, src, want) in rows {
        let p = parse(path, src);
        assert!(!p.tree.root_node().has_error(), "{path} {src:?}: parse");
        let got = p.js_ts_strictness(node_at(&p, "g();"));
        if got != want {
            wrong.push(format!("{path} {src:?}: got {got:?}, want {want:?}"));
        }
    }
    assert!(wrong.is_empty(), "wrong strictness:\n{}", wrong.join("\n"));
}

const BLOCK_FN: &str =
    "function inner() {\n  return 0;\n}\nfunction run(flag) {\n  if (flag) {\n    \
    function inner() {\n      return 1;\n    }\n  }\n  return inner();\n}\n";

/// The declaration walk over `run`'s body, hoisting (as for a function body): the kinds of
/// the nodes it records for `inner`, and the unknown-strictness Annex-B names.
fn run_body_inner(path: &str, src: &str) -> (Vec<&'static str>, BTreeSet<String>) {
    let p = parse(path, src);
    assert!(!p.tree.root_node().has_error(), "{path}: parse");
    let run = p.all_functions().into_iter().find(|f| {
        let name = p.language.function_name(f);
        name.is_some_and(|n| p.node_text(&n) == "run")
    });
    let body = run.and_then(|f| f.child_by_field_name("body"));
    let body = body.expect("run's body");
    let (mut out, mut annex) = (super::Index::new(), BTreeSet::new());
    let mode = (true, true, p.js_ts_strictness(body));
    p.js_ts_declare_walk(body, mode, &BTreeSet::new(), &mut out, &mut annex);
    let kinds = out.get("inner").map(|v| v.iter().map(|n| n.kind()));
    (kinds.map(Iterator::collect).unwrap_or_default(), annex)
}

#[test]
fn b18_annex_b_marker_only_in_sloppy_code() {
    let module = format!("{BLOCK_FN}export {{}};\n");
    let directive = format!("'use strict';\n{BLOCK_FN}");
    let function_directive = BLOCK_FN.replace("(flag) {\n", "(flag) {\n  \"use strict\";\n");
    let generator = BLOCK_FN.replace("function inner", "function* inner");
    let asynchronous = BLOCK_FN.replace("    function inner", "    async function inner");
    let c159 = BLOCK_FN.replace("(flag)", "(flag: boolean)");
    let none = || (Vec::<&str>::new(), BTreeSet::new());
    let annex = || (Vec::<&str>::new(), BTreeSet::from(["inner".to_string()]));
    let marker = (vec!["statement_block"], BTreeSet::new());
    let rows = [
        ("C163", "a.cjs", BLOCK_FN, marker),
        ("C164", "a.cjs", &generator, none()),
        ("async", "a.cjs", &asynchronous, none()),
        ("C111", "a.js", &module, none()),
        ("C111", "a.tsx", &module, none()),
        ("C161", "a.js", &directive, none()),
        ("C161", "a.tsx", &directive, none()),
        ("C162", "a.js", &function_directive, none()),
        ("C162", "a.tsx", &function_directive, none()),
        ("C159", "a.ts", &c159, annex()),
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

const BLOCK_AT_MODULE: &str = "function inner() {\n  return 0;\n}\nif (flag) {\n\
    function inner() {\n    return 1;\n  }\n}\n";

#[test]
fn b18_module_terminal_consumes_markers_and_unknown_strictness() {
    let cjs = format!("{BLOCK_AT_MODULE}module.exports = {{ inner }};\n");
    let generator = cjs.replace("function inner", "function* inner");
    let module = format!("{BLOCK_AT_MODULE}export {{ inner }};\n");
    let (site, outer) = ("if (flag)", callable("inner", 1, 3));
    let unknown = JsBinding::Unchecked("annex_b_strictness");
    check(
        &["a.cjs"],
        false,
        vec![("C163 module scope", &cjs, "inner", site, DUP)],
    );
    check(
        &["a.cjs"],
        false,
        vec![("C164 module scope", &generator, "inner", site, outer)],
    );
    check(
        &BOTH,
        false,
        vec![("C160 module scope", BLOCK_AT_MODULE, "inner", site, unknown)],
    );
    let outer = callable("inner", 1, 3);
    check(
        &BOTH,
        false,
        vec![("C111 module scope", &module, "inner", site, outer)],
    );
}

const NC: JsBinding = JsBinding::Refused("not_callable");
const MAY: JsBinding = JsBinding::MayCall;
const UNBOUND: JsBinding = JsBinding::Refused("unbound");

/// Rows for the name `f`, sited at the program node (these sources parse cleanly).
fn check_f(paths: &[&str], rows: Vec<(&str, &str, JsBinding)>) {
    let rows = rows
        .into_iter()
        .map(|(id, src, want)| (id, src, "f", src, want));
    check(paths, false, rows.collect());
}

const CATCH_VAR: &str = "function f() {}\ntry {} catch (f) {\n  var f = () => 2;\n}\n";
const WITH_VAR: &str = "function f() {}\nwith (o) {\n  var f = () => 2;\n}\n";
const MEMO: &str = "import { memo } from 'react';\nconst f = memo(() => 1);\n";

#[test]
fn table_declarations_at_module_scope() {
    let fe = callable("f", 1, 1);
    let wrapped = JsBinding::Callable(JsTerminal {
        local: "f".to_string(),
        start_line: 2,
        end_line: 2,
        wrapped: true,
    });
    let let_memo = MEMO.replace("const", "let");
    let rows = vec![
        ("D1 function", "function f() {}", fe.clone()),
        ("D1 generator", "function* f() {}", fe.clone()),
        ("D1 async", "async function f() {}", fe.clone()),
        ("D2 class", "class f {}", NC),
        ("D3 labelled", "l: function f() {}", fe.clone()),
        ("D4 let arrow", "let f = () => 1;", fe.clone()),
        ("D4 const fe", "const f = function () {};", fe.clone()),
        ("D4 block lexical", "{\n  const f = () => 1;\n}\n", UNBOUND),
        ("D4 value", "const f = 1;", NC),
        ("D5 named", "import { f } from './x';", NC),
        ("D5 renamed", "import { g as f } from './x';", NC),
        ("D5 default", "import f from './x';", NC),
        ("D5 namespace", "import * as f from './x';", NC),
        (
            "D7 block var",
            "if (x) {\n  var f = () => 1;\n}\n",
            callable("f", 2, 2),
        ),
        ("D7 for-in var", "for (var f in o) {}", NC),
        ("D7 for-of var", "for (var f of o) {}", NC),
        ("D7 catch marker", CATCH_VAR, DUP),
        ("D7 with marker", WITH_VAR, DUP),
        ("P1 object", "const { f } = o;", NC),
        ("P1 array", "const [f] = a;", NC),
        ("P1 pair", "const { a: f } = o;", NC),
        ("P1 default", "const { f = () => 1 } = o;", NC),
        ("P1 rest", "const { ...f } = o;", NC),
        ("P1 var pattern", "function f() {}\nvar { f } = o;", DUP),
        (
            "B3 assignment chain",
            "var f = (M.g = function () {});",
            callable("g", 1, 1),
        ),
        (
            "B3 parenthesized",
            "const f = (() => 1);",
            JsBinding::Refused("unindexed"),
        ),
        ("B3 wrapper", MEMO, wrapped),
        ("M1 let wrapper", &let_memo, MAY),
        ("M1 function argument", "const f = throttle(() => 1);", MAY),
        ("M1 no function argument", "const f = make();", NC),
        ("unbound", "g();", UNBOUND),
    ];
    check_f(&BOTH, rows);
}

#[test]
fn table_writes_make_may_call() {
    let rows = vec![
        ("W1 assignment", "function f() {}\nf = g;", MAY),
        ("W1 update", "let f = () => 1;\nf++;", MAY),
        ("W1 augmented", "let f = () => 1;\nf += 1;", MAY),
        ("W1 destructuring", "let f = () => 1;\n({ f } = o);", MAY),
        ("W1 for-in head", "let f = () => 1;\nfor (f in o) {}", MAY),
        (
            "W1 shadowed",
            "let f = () => 1;\nfunction g(f) {\n  f = 1;\n}\n",
            callable("f", 1, 1),
        ),
    ];
    check_f(&BOTH, rows);
    let rows = vec![
        ("W1 as", "let f = () => 1;\n(f as any) = g;", MAY),
        ("W1 non-null", "let f = () => 1;\nf! = g;", MAY),
    ];
    check_f(&TS, rows);
}

#[test]
fn table_typescript_value_space() {
    let rows = vec![
        ("D2 abstract class", "abstract class f {}", NC),
        ("D6 enum", "enum f {\n  A,\n}\n", NC),
        ("D6 namespace", "namespace f {}", NC),
        ("D6 dotted namespace", "namespace f.g {}", NC),
        ("D6 declare function", "declare function f(): void;", NC),
        ("D6 declare const", "declare const f: () => void;", NC),
        (
            "D6 overload",
            "function f(a: string): void;\nfunction f(a: any) {}",
            callable("f", 2, 2),
        ),
        ("D5 import alias", "import f = M.g;", NC),
        ("D5 import require", "import f = require('x');", NC),
        // D4: a `using` binding is a declaration, never a W1 write (S1b-2b filters it out of
        // the module write scan), so it classifies like a `const` declarator.
        ("D4 using", "using f = res();", NC),
        (
            "D4 using function argument",
            "using f = wrap(() => 1);",
            MAY,
        ),
        ("D4 using arrow", "using f = () => 1;", callable("f", 1, 1)),
        ("D4 using written", "using f = () => 1;\nf = g;", MAY),
    ];
    check_f(&TS, rows);
}

// Implementation review r1 fold (Opus W1-W6, sol W1-W2).

/// Writes the base scan misses: evaluated before a function's own environments (J1 parameter
/// lists, J2 decorators, J3 computed keys), so the function's parameters and body `var`s do
/// not shadow them.
const PARAM_WRITE: &str = "function f() {\n  return 1;\n}\nfunction g(a = (f = () => 2)) {\n\
    var f;\n  function f() {\n    return 3;\n  }\n  return a;\n}\nexport { f };\n";
const KEY_WRITE: &str =
    "function f() {\n  return 1;\n}\nclass C {\n  [f = 2]() {\n    var f;\n  }\n}\nexport { f };\n";
const OBJECT_KEY_WRITE: &str = "const f = () => 1;\nconst o = { [f = g](f) {} };\nexport { f };\n";
const DECORATOR_WRITE: &str = "function f() {}\nclass C {\n  @d(f = 1) m(f) {}\n}\nexport { f };\n";

#[test]
fn fold_writes_before_function_environments_are_may_call() {
    let twin = OBJECT_KEY_WRITE.replace("](f)", "](x)");
    let rows = vec![
        ("Opus W1 parameter", PARAM_WRITE, MAY),
        ("Opus W1 class key", KEY_WRITE, MAY),
        ("sol W1 object key", OBJECT_KEY_WRITE, MAY),
        ("sol W1 twin (x)", &twin, MAY),
        ("J2 decorator", DECORATOR_WRITE, MAY),
    ];
    check_f(&BOTH, rows);
}

const SEALED_WRITE: &str = "function f() {\n  return 1;\n}\nfunction g() {\n  f = () => 2;\n\
    let x = ;\n}\nexport { f };\n";
const MANGLED_WRITE: &str =
    "function f() {\n  return 1;\n}\nfunction g() {\n  f = ;\n}\nexport { f };\n";

#[test]
fn fold_sealed_error_mentioning_the_name_is_may_call() {
    let rows = vec![
        ("Opus W2 intact", SEALED_WRITE, "f", "export { f }", MAY),
        ("Opus W2 mangled", MANGLED_WRITE, "f", "export { f }", MAY),
    ];
    check(&BOTH, true, rows);
}

#[test]
fn fold_written_pattern_declarator_is_may_call() {
    let src = "let { f } = o;\nf = () => 1;\nexport { f };\n";
    check_f(&BOTH, vec![("Opus W3", src, MAY)]);
}

#[test]
fn fold_declare_global_is_not_a_module_binding() {
    let src = "function f() {}\ndeclare global {\n  var f: any;\n}\nexport { f };\n";
    check_f(&TS, vec![("Opus W4", src, callable("f", 1, 1))]);
}

#[test]
fn fold_b0_precedes_annex_b_uncertainty() {
    let escaped = format!("{BLOCK_AT_MODULE}var \\u0067 = 1;\n");
    let want = JsBinding::Refused("escaped_identifier");
    check(
        &BOTH,
        false,
        vec![("sol W2 B0", &escaped, "inner", "if (flag)", want)],
    );
}

#[test]
fn fold_b1_precedes_annex_b_uncertainty() {
    let unsealed = format!("{BLOCK_AT_MODULE}let x = ;\n");
    check(
        &BOTH,
        true,
        vec![("sol W2 B1", &unsealed, "inner", "if (flag)", PARSE)],
    );
}

#[test]
fn fold_b3_unwraps_assertions_around_a_may_call_wrapper() {
    let rows = vec![("parenthesized", "const f = (throttle(() => 1));", MAY)];
    check_f(&BOTH, rows);
    let rows = vec![
        ("as", "const f = throttle(() => 1) as any;", MAY),
        (
            "satisfies",
            "const f = throttle(() => 1) satisfies unknown;",
            MAY,
        ),
        ("non-null", "const f = throttle(() => 1)!;", MAY),
    ];
    check_f(&TS, rows);
    check_f(
        &["a.ts"],
        vec![("type assertion", "const f = <any>throttle(() => 1);", MAY)],
    );
}

#[test]
fn fold_missing_parenthesis_is_not_delimited() {
    let src = "function g(a, b {\n  return 1;\n}\nexport function f() {\n  return 1;\n}\n";
    check(
        &BOTH,
        true,
        vec![("Opus W6", src, "f", "export function f", PARSE)],
    );
}
