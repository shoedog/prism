//! S1 guards (wrapped-export SPEC §4, §7): each refusal names the reason that fired, and the
//! importer's `<Island/>` never becomes an `import_member` edge. Guards pass on base too.
use super::js_wrapped_export_test::{app_sites, graph, run, APP};
use prism::ast::ParsedFile;
use prism::languages::Language;

const M: &str = "import { memo } from 'react';\n";
const FR: &str = "import { forwardRef } from 'react';\n";
const DROP: &[&str] = &["L3 Island: drop UnknownName"];

/// `pre` + `export const Island = <init>;` on one line.
fn w(pre: &str, init: &str) -> String {
    format!("{pre}export const Island = {init};\n")
}

/// The admitted `memo` export after the React import and `pre`.
fn memo(pre: &str) -> String {
    w(&format!("{M}{pre}"), "memo((p) => null)")
}

/// A `forwardRef` export after `pre` (which supplies, or fakes, the callee).
fn fr(pre: &str) -> String {
    w(pre, "forwardRef((p, r) => null)")
}

/// The lib's declarator counters: `(spanned_admitted, skipped_expr_count, reasons)`.
fn counters(lib: &str, ext: &str) -> (usize, usize, Vec<String>) {
    let path = format!("lib.{ext}");
    let lang = Language::from_path(&path).unwrap();
    let f = ParsedFile::parse(&path, lib, lang)
        .unwrap()
        .extract_js_ts_export_facts();
    let reasons = f
        .skipped_decl_reasons
        .iter()
        .map(|(r, n)| format!("{r}={n}"));
    (f.spanned_admitted, f.skipped_expr_count, reasons.collect())
}

/// Each row: the lib counts `reason` exactly once (and nothing else: T-O1, which these
/// tables cover for every reachable reason; R12 is unreachable) and `<Island/>` drops.
fn refused(rows: &[(String, &str)], exts: &[&str]) {
    for (i, (lib, reason)) in rows.iter().enumerate() {
        for ext in exts {
            let want = (0, 1, vec![format!("{reason}=1")]);
            assert_eq!(counters(lib, ext), want, "row {i} .{ext}");
            assert_eq!(run(lib, APP, ext), DROP, "row {i} .{ext}");
        }
    }
}

#[test]
fn t_n1_t_n2_t_n8_t_n15_shape_refusals() {
    let c10 = "import { forwardRef } from 'react';\nfunction IslandInner(props, ref) { return \
        null; }\nfunction helper() {\n  function Island() { return 1; }\n  return Island();\n}\n";
    let both = "import { memo, forwardRef } from 'react';\n";
    refused(
        &[
            (w(c10, "forwardRef(IslandInner)"), "first_arg_not_function"),
            (
                w(both, "memo(forwardRef((p, r) => null))"),
                "nested_wrapper",
            ),
            (
                w(FR, "Object.assign(forwardRef((p, r) => null), {})"),
                "callee_not_admitted",
            ),
            (w(FR, "forwardRef((p, r) => null, 1)"), "arity"),
            (w(M, "memo((p) => null, cmp, x)"), "arity"),
            (w(FR, "forwardRef(...args)"), "first_arg_not_function"),
            (w(FR, "forwardRef()"), "arity"),
            (
                w(M, "memo((p) => <div/>, (a, b) => true)"),
                "comparator_line_collision",
            ),
        ],
        &["jsx", "tsx"],
    );
    let cast = w(FR, "forwardRef((p, r) => null) as any");
    refused(&[(cast, "non_call_initializer")], &["tsx"]);
}

#[test]
fn t_n3_t_n4_t_n6_t_n18_callee_not_admitted() {
    let others = "import { create, createSelector, observer, debounce } from 'lib';\nimport \
        styled from 'styled';\n";
    let rows = [
        fr("function forwardRef(f) { return f; }\n"),
        fr("import { forwardRef } from './react-shim';\n"),
        fr("import { forwardRef } from 'preact/compat';\n"),
        w(others, "create((set) => null)"),
        w(others, "createSelector(a, (x) => x)"),
        w(others, "styled('div')((p) => null)"),
        w(others, "items.map((x) => x)"),
        w(others, "observer((p) => null)"),
        w(others, "debounce((x) => x, 1)"),
        fr("const { forwardRef } = require('react');\n"),
        w("import { memo } from \"react'\";\n", "memo((p) => null)"), // impl r2 (sol W1)
        w("import { memo } from 'react\"';\n", "memo((p) => null)"),
    ];
    refused(
        &rows.map(|lib| (lib, "callee_not_admitted")),
        &["jsx", "tsx"],
    );
}

#[test]
fn t_n5_t_n12_t_n17_declaration_and_parse_refusals() {
    let c29 = "import { forwardRef as fr ??? } from 'react';\n";
    refused(
        &[
            (fr(FR).replace("const Island", "let Island"), "not_const"),
            (fr(FR).replace("const Island", "var Island"), "not_const"),
            (w(M, "memo((p) => (null)"), "parse_recovery"),
            (w(M, "memo((p) => null, ???)"), "parse_recovery"),
            (w(c29, "fr((p, r) => null)"), "import_parse_recovery"),
        ],
        &["jsx", "tsx"],
    );
    // Impl r1 (sol W1): a malformed import recovered as a top-level ERROR sibling of a
    // valid React import is still a parse-recovered import (R4).
    let siblings = [
        "import { memo from \"./other\";",
        "import { memo } from;",
        "import memo from;",
        "import * as memo from ;",
        "} import { memo from './other';", // impl r2 (opus W1): V1
        "if (x) import { memo from './other';", // V5
        "export import { memo from './other';", // V6 (JSX recovers identifier:"import")
    ];
    let rows = siblings.map(|bad| (memo(&format!("{bad}\n")), "import_parse_recovery"));
    refused(&rows, &["jsx", "tsx"]);
    // T-N17 twin (and V11 `} x;`): a parse error outside every import does not refuse.
    let twin = w(
        "import { forwardRef as fr } from 'react';\nconst x = ???;\n",
        "fr((p, r) => null)",
    );
    for (lib, ext) in [&twin, &memo("} x;\n")]
        .into_iter()
        .flat_map(|l| [(l, "jsx"), (l, "tsx")])
    {
        let want = ["L3 Island: Exact import_member lib:Island@3-3"];
        assert_eq!(run(lib, APP, ext), want, ".{ext}");
    }
}

#[test]
fn t_r6_p1_to_p5_callee_provenance() {
    let escaped = "import { forwardRef as forw\\u0061rdRef } from 'react';\n";
    let rows = [
        memo("import { memo } from './other';\n"),
        memo("memo = other;\n"),
        w(escaped, "forw\\u0061rdRef((p, r) => null)"),
    ];
    refused(&rows.map(|lib| (lib, "callee_provenance")), &["jsx", "tsx"]);
    // T-R6-P2: sol's collision fixture (C55); the type-only-only import stays R5.
    let type_only = "import type { memo } from 'react';\n";
    refused(
        &[
            (
                memo("import type { T as memo } from './types';\n"),
                "callee_provenance",
            ),
            (w(type_only, "memo((p) => null)"), "callee_not_admitted"),
        ],
        &["tsx"],
    );
}

#[test]
fn t_r6_p4_and_p4b_module_scope_competitors() {
    let competitors = [
        "function memo(x) { return x; }\n",     // C64
        "class memo {}\n",                      // C65
        "const { memo } = x;\n",                // C66
        "if (flag) {\n  var memo = fake;\n}\n", // C57 (sol r3 W2)
        "for (var memo of [1]) {}\n",           // C58
        "try {\n  for (var memo = 0; memo < 1; memo++) {}\n} catch (e) {}\n", // C59
    ];
    refused(
        &competitors.map(|pre| (memo(pre), "callee_provenance")),
        &["jsx", "tsx"],
    );
    // Positive twins: not module-scope competitors (C67, C60, C61).
    for (pre, line) in [
        (
            "export function Other() {\n  const memo = 1;\n  return memo;\n}\n",
            6,
        ),
        ("if (flag) {\n  let memo = 1;\n}\n", 5),
        (
            "function h() {\n  var memo = 1;\n}\nclass K { m() { var memo = 2; } }\n",
            6,
        ),
    ] {
        let lib = memo(pre);
        let want = format!("L3 Island: Exact import_member lib:Island@{line}-{line}");
        for ext in ["jsx", "tsx"] {
            assert_eq!(run(&lib, APP, ext), [want.as_str()]);
        }
    }
}

#[test]
fn t_n7_t_n9_t_n11_t_n13_resolution_refusals() {
    // T-N7 (C18): an importer parameter shadows the imported local.
    let shadow = "import { Island } from './lib';\nexport function App(Island) {\n  return \
        <Island/>;\n}\n";
    let got = run(&memo(""), shadow, "tsx");
    assert!(got.iter().all(|s| !s.contains("import_member")), "{got:?}");
    // T-N9: forwarding a wrapped export stays refused, even with a same-named top-level
    // function beside a named inner function expression.
    let bridge = "import { Island } from './lib';\nexport { Island };\n";
    let app = APP.replace("./lib", "./bridge");
    let named = w(
        &format!("{FR}function IslandImpl() {{ return null; }}\n"),
        "forwardRef(function IslandImpl(p, r) { return null; })",
    );
    for lib in [fr(FR), named] {
        let cg = graph(&[("lib.jsx", &lib), ("bridge.jsx", bridge), ("app.jsx", &app)]);
        assert_eq!(app_sites(&cg), DROP);
    }
    // T-N11: two same-span `X` callables (arrow and nested declaration) bind nothing.
    let one_line = "import { forwardRef } from 'react';\nexport const X = forwardRef((p, r) => { \
        function X() {} return null; });\n";
    let app_x = "import { X } from './lib';\nexport function App() {\n  return <X/>;\n}\n";
    assert_eq!(run(one_line, app_x, "jsx"), ["L3 X: drop UnknownName"]);
    // T-N13: a later `export { other as Island }` poisons the admitted name.
    let poisoned = format!(
        "{}function other() {{}}\nexport {{ other as Island }};\n",
        memo("")
    );
    assert_eq!(counters(&poisoned, "tsx"), (1, 0, vec![]));
    assert_eq!(run(&poisoned, APP, "tsx"), DROP);
}

#[test]
fn t_n16_star_barrel_span_conflict() {
    // C39: two `Same` callables with different spans behind one barrel name.
    let imp = "import { memo } from 'react';\nexport const A = memo(function Same(p) {\n  return \
        <div/>;\n});\nexport const B = memo(function Same(p) {\n  return <span/>;\n});\n";
    let cg = graph(&[
        ("impl.tsx", imp),
        ("a.ts", "export { A as X } from './impl';\n"),
        ("b.ts", "export { B as X } from './impl';\n"),
        ("index.ts", "export * from './a';\nexport * from './b';\n"),
        (
            "app.tsx",
            "import { X } from './index';\nexport function App() {\n  return <X/>;\n}\n",
        ),
    ]);
    assert_eq!(app_sites(&cg), ["L3 X: drop UnknownName"]);
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["js_export_barrel_conflicts"], 1);
}
