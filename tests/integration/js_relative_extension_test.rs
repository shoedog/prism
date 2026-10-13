//! I1: a relative JS/TS import that spells a `.js` suffix (`./x.js`) resolves
//! to the TypeScript file on disk (`x.ts`/`x.tsx`) exactly like its
//! extensionless twin (`./x`); bare `.`/`..` specifiers resolve to the
//! directory index; redundant `//`, `/./`, `/../` segments are normalized.
//! Every row compares the new spelling against the extensionless twin AND
//! pins the absolute target/confidence/kind, so a shared regression cannot
//! pass as parity.
//! Unit-level candidate rows live in
//! `src/call_graph/js_relative_candidates_test.rs`.

use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
use std::collections::BTreeMap;

const F_TS: &str = "export function f() { return 1; }\n";
const DISTRACTOR: &str = "export function f() { return 2; }\n";

fn graph(sources: &[(&str, &str)]) -> CallGraph {
    let files: BTreeMap<String, ParsedFile> = sources
        .iter()
        .map(|(path, src)| {
            let lang = Language::from_path(path).expect("known extension");
            (
                path.to_string(),
                ParsedFile::parse(path, src, lang).expect("parse fixture"),
            )
        })
        .collect();
    CallGraph::build(&files)
}

/// One row per call site `callee` made from a function named `caller`.
fn rows(cg: &CallGraph, caller: &str, callee: &str) -> Vec<String> {
    let mut out = Vec::new();
    for site in cg.calls.values().flatten() {
        if site.caller.name != caller || site.callee_name != callee {
            continue;
        }
        let res = cg.resolve_call_site_full(site);
        if let Some(drop) = res.drop {
            out.push(format!("drop {drop:?}"));
            continue;
        }
        for r in &res.resolved {
            out.push(format!(
                "{}:{}@{}-{} {:?}/{}",
                r.target.file,
                r.target.name,
                r.target.start_line,
                r.target.end_line,
                r.confidence,
                r.kind.as_str()
            ));
        }
    }
    out
}

/// Builds the same repo twice, with `{S}` replaced by `spelled` and by
/// `plain` in every source, and returns the rows for each spelling. `{I}`
/// is the barrel import `./index.js` for the spelled run and `./index` for
/// the plain run.
fn twin(
    sources: &[(&str, &str)],
    spelled: &str,
    plain: &str,
    caller: &str,
    callee: &str,
) -> (Vec<String>, Vec<String>) {
    let with = |spec: &str| -> Vec<String> {
        let fixed: Vec<(&str, String)> = sources
            .iter()
            .map(|(p, s)| {
                let index = if spec.ends_with(".js") {
                    "./index.js"
                } else {
                    "./index"
                };
                (*p, s.replace("{S}", spec).replace("{I}", index))
            })
            .collect();
        let refs: Vec<(&str, &str)> = fixed.iter().map(|(p, s)| (*p, s.as_str())).collect();
        rows(&graph(&refs), caller, callee)
    };
    (with(spelled), with(plain))
}

/// Asserts the `.js` spelling equals its extensionless twin and both equal
/// `expected`.
fn assert_parity(
    sources: &[(&str, &str)],
    spelled: &str,
    plain: &str,
    caller: &str,
    callee: &str,
    expected: &[&str],
) {
    let (js, ext_less) = twin(sources, spelled, plain, caller, callee);
    assert_eq!(ext_less, expected, "extensionless twin {plain:?}");
    assert_eq!(js, expected, "spelled {spelled:?}");
}

const EXACT_F: &str = "pkg/x.ts:f@1-1 Exact/import_member";

#[test]
fn named_import_with_js_suffix_resolves_exact_import_member() {
    assert_parity(
        &[
            ("pkg/x.ts", F_TS),
            ("other/x.ts", DISTRACTOR),
            ("pkg/other.ts", DISTRACTOR),
            (
                "pkg/app.ts",
                "import { f } from '{S}';\nexport function g() { f(); }\n",
            ),
        ],
        "./x.js",
        "./x",
        "g",
        "f",
        &[EXACT_F],
    );
}

#[test]
fn renamed_import_with_js_suffix_resolves() {
    assert_parity(
        &[
            ("pkg/x.ts", F_TS),
            ("other/x.ts", DISTRACTOR),
            (
                "pkg/app.ts",
                "import { f as h } from '{S}';\nexport function g() { h(); }\n",
            ),
        ],
        "./x.js",
        "./x",
        "g",
        "h",
        &[EXACT_F],
    );
}

#[test]
fn default_import_with_js_suffix_resolves() {
    assert_parity(
        &[
            ("pkg/x.ts", "export default function f() { return 1; }\n"),
            ("other/x.ts", "export default function f() { return 2; }\n"),
            (
                "pkg/app.ts",
                "import d from '{S}';\nexport function g() { d(); }\n",
            ),
        ],
        "./x.js",
        "./x",
        "g",
        "d",
        &[EXACT_F],
    );
}

#[test]
fn namespace_import_with_js_suffix_resolves() {
    // The namespace route already had a `.js` -> `.ts` sibling probe; the
    // candidate-level rule must agree with it.
    assert_parity(
        &[
            ("pkg/x.ts", F_TS),
            ("other/x.ts", DISTRACTOR),
            (
                "pkg/app.ts",
                "import * as N from '{S}';\nexport function g() { N.f(); }\n",
            ),
        ],
        "./x.js",
        "./x",
        "g",
        "f",
        &["pkg/x.ts:f@1-1 Exact/import_qualified"],
    );
}

#[test]
fn named_reexport_barrel_with_js_suffix_resolves_to_the_declaration() {
    for barrel in [
        "export { f } from '{S}';\n",
        "export { f as f } from '{S}';\n",
    ] {
        assert_parity(
            &[
                ("pkg/x.ts", F_TS),
                ("other/x.ts", DISTRACTOR),
                ("pkg/index.ts", barrel),
                (
                    "pkg/app.ts",
                    "import { f } from '{I}';\nexport function g() { f(); }\n",
                ),
            ],
            "./x.js",
            "./x",
            "g",
            "f",
            &[EXACT_F],
        );
    }
}

#[test]
fn star_reexport_barrel_with_js_suffix_resolves_to_the_declaration() {
    assert_parity(
        &[
            ("pkg/x.ts", F_TS),
            ("other/x.ts", DISTRACTOR),
            ("pkg/index.ts", "export * from '{S}';\n"),
            (
                "pkg/app.ts",
                "import { f } from './index';\nexport function g() { f(); }\n",
            ),
        ],
        "./x.js",
        "./x",
        "g",
        "f",
        &[EXACT_F],
    );
}

#[test]
fn jsx_element_from_js_suffixed_tsx_resolves() {
    assert_parity(
        &[
            ("pkg/c.tsx", "export function C() { return <div/>; }\n"),
            ("other/c.tsx", "export function C() { return <p/>; }\n"),
            (
                "pkg/app.tsx",
                "import { C } from '{S}';\nexport function App() { return <C/>; }\n",
            ),
        ],
        "./c.js",
        "./c",
        "App",
        "C",
        &["pkg/c.tsx:C@1-1 Exact/import_member"],
    );
}

#[test]
fn commonjs_require_with_js_suffix_matches_its_twin() {
    // `require('./x.js')` flows through the same relative-module resolution
    // as ESM imports; the twin pins whichever behavior the extensionless
    // spelling has.
    let sources = [
        (
            "pkg/x.ts",
            "function f() { return 1; }\nmodule.exports = { f };\n",
        ),
        (
            "other/x.ts",
            "function f() { return 2; }\nmodule.exports = { f };\n",
        ),
        (
            "pkg/app.ts",
            "const { f } = require('{S}');\nexport function g() { f(); }\n",
        ),
    ];
    let (js, plain) = twin(&sources, "./x.js", "./x", "g", "f");
    assert_eq!(plain, ["pkg/x.ts:f@1-1 Exact/import_member"]);
    assert_eq!(js, plain);
}

#[test]
fn js_suffix_with_missing_export_matches_the_twin_and_adds_no_edge() {
    let sources = [
        ("pkg/x.ts", "export function other() { return 1; }\n"),
        ("elsewhere/y.ts", DISTRACTOR),
        (
            "pkg/app.ts",
            "import { f } from '{S}';\nexport function g() { f(); }\n",
        ),
    ];
    let (js, plain) = twin(&sources, "./x.js", "./x", "g", "f");
    assert_eq!(js, plain);
    assert!(
        js.iter().all(|row| !row.contains("elsewhere/y.ts")),
        "no edge to the unrelated same-named function: {js:?}"
    );
}

#[test]
fn type_only_import_with_js_suffix_matches_the_twin() {
    for import in [
        "import type { f } from '{S}';",
        "import { type f } from '{S}';",
    ] {
        let src = format!("{import}\nexport function g() {{ f(); }}\n");
        let sources = [("pkg/x.ts", F_TS), ("pkg/app.ts", src.as_str())];
        let (js, plain) = twin(&sources, "./x.js", "./x", "g", "f");
        assert_eq!(js, plain, "{import}");
        assert!(
            js.iter().all(|row| !row.contains("Exact/import_member")),
            "{import}: {js:?}"
        );
    }
}

#[test]
fn literal_js_file_wins_over_same_stem_ts() {
    let rows = {
        let cg = graph(&[
            ("pkg/x.js", F_TS),
            ("pkg/x.ts", DISTRACTOR),
            (
                "pkg/app.ts",
                "import { f } from './x.js';\nexport function g() { f(); }\n",
            ),
        ]);
        rows(&cg, "g", "f")
    };
    assert_eq!(rows, ["pkg/x.js:f@1-1 Exact/import_member"]);
}

#[test]
fn root_level_caller_with_js_suffix_resolves() {
    assert_parity(
        &[
            ("x.ts", F_TS),
            ("other/x.ts", DISTRACTOR),
            (
                "app.ts",
                "import { f } from '{S}';\nexport function g() { f(); }\n",
            ),
        ],
        "./x.js",
        "./x",
        "g",
        "f",
        &["x.ts:f@1-1 Exact/import_member"],
    );
}

#[test]
fn parent_directory_js_suffix_resolves() {
    assert_parity(
        &[
            ("lib/x.ts", F_TS),
            ("other/x.ts", DISTRACTOR),
            (
                "pkg/app.ts",
                "import { f } from '{S}';\nexport function g() { f(); }\n",
            ),
        ],
        "../lib/x.js",
        "../lib/x",
        "g",
        "f",
        &["lib/x.ts:f@1-1 Exact/import_member"],
    );
}

// Not modeled in this slice: pin that these spellings stay unresolved.
#[test]
fn jsx_mjs_cjs_suffixes_are_not_substituted() {
    for (suffix, target) in [
        ("./x.jsx", "pkg/x.tsx"),
        ("./x.mjs", "pkg/x.ts"),
        ("./x.cjs", "pkg/x.ts"),
    ] {
        let app = format!("import {{ f }} from '{suffix}';\nexport function g() {{ f(); }}\n");
        let cg = graph(&[(target, F_TS), ("pkg/app.ts", app.as_str())]);
        let got = rows(&cg, "g", "f");
        assert!(
            got.iter().all(|row| !row.contains("Exact/import_member")),
            "{suffix}: {got:?}"
        );
    }
}

#[test]
fn bare_dot_import_resolves_to_directory_index() {
    assert_parity(
        &[
            ("pkg/index.ts", F_TS),
            ("other/index.ts", DISTRACTOR),
            (
                "pkg/app.ts",
                "import { f } from '{S}';\nexport function g() { f(); }\n",
            ),
        ],
        ".",
        "./index",
        "g",
        "f",
        &["pkg/index.ts:f@1-1 Exact/import_member"],
    );
}

#[test]
fn bare_dot_dot_import_resolves_to_parent_index() {
    assert_parity(
        &[
            ("pkg/index.ts", F_TS),
            ("other/index.ts", DISTRACTOR),
            (
                "pkg/sub/app.ts",
                "import { f } from '{S}';\nexport function g() { f(); }\n",
            ),
        ],
        "..",
        "../index",
        "g",
        "f",
        &["pkg/index.ts:f@1-1 Exact/import_member"],
    );
}

#[test]
fn bare_dot_dot_from_a_root_level_file_resolves_nothing() {
    let cg = graph(&[
        ("index.ts", F_TS),
        (
            "app.ts",
            "import { f } from '..';\nexport function g() { f(); }\n",
        ),
    ]);
    let got = rows(&cg, "g", "f");
    assert!(
        got.iter().all(|row| !row.contains("Exact/import_member")),
        "{got:?}"
    );
}

#[test]
fn bare_dot_star_reexport_resolves_through_the_barrel() {
    // `export * from '.'` inside pkg/sub/index.ts re-exports pkg/sub/index.ts
    // itself, a cycle; the sibling form `..` reaches the parent barrel.
    assert_parity(
        &[
            ("pkg/x.ts", F_TS),
            ("pkg/index.ts", "export * from './x';\n"),
            ("pkg/sub/index.ts", "export * from '{S}';\n"),
            (
                "pkg/sub/app.ts",
                "import { f } from './index';\nexport function g() { f(); }\n",
            ),
        ],
        "..",
        "../index",
        "g",
        "f",
        &[EXACT_F],
    );
}

#[test]
fn redundant_segments_in_specifier_resolve() {
    for spelled in ["./a//b", "./a/./b", "./a/../a/b"] {
        assert_parity(
            &[
                ("pkg/a/b.ts", "export function f() { return 1; }\n"),
                ("other/a/b.ts", DISTRACTOR),
                (
                    "pkg/app.ts",
                    "import { f } from '{S}';\nexport function g() { f(); }\n",
                ),
            ],
            spelled,
            "./a/b",
            "g",
            "f",
            &["pkg/a/b.ts:f@1-1 Exact/import_member"],
        );
    }
}

#[test]
fn specifier_escaping_the_repo_root_resolves_nothing() {
    let cg = graph(&[
        ("b.ts", F_TS),
        ("pkg/b.ts", F_TS),
        (
            "pkg/app.ts",
            "import { f } from './a/../../../b';\nexport function g() { f(); }\n",
        ),
    ]);
    let got = rows(&cg, "g", "f");
    assert!(
        got.iter().all(|row| !row.contains("Exact/import_member")),
        "{got:?}"
    );
}
