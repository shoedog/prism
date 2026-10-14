//! Slice L round-1 fold, items 1-3: where the alias route does not produce a proven hit, the
//! site keeps MAIN's result (same rungs, same outcome, same drop reason), never a new drop and
//! never a new wrong edge. Each expectation below is the record set `origin/main` gives for the
//! same source (captured by running these fixtures against main's `src/`), and the head must
//! equal it. Decoy functions are present so a fall-through to a same-named unrelated function
//! elsewhere would show.
use super::js_alias_follow_test::graph_n;
use super::js_paths_common::{graph, write};
use prism::call_graph::CallGraph;

/// Every record in `caller_file` (all origins), as `L<line> <callee> <origin>: <targets|drop>`.
pub(super) fn records_of(cg: &CallGraph, caller_file: &str) -> Vec<String> {
    let mut out = Vec::new();
    for row in prism::navigation::queries::call_site_dump(cg) {
        if row["caller"]["file"] != caller_file {
            continue;
        }
        let shown = match row["resolved_targets"].as_array().filter(|t| !t.is_empty()) {
            Some(targets) => targets
                .iter()
                .map(|t| {
                    format!(
                        "{}:{}@{}-{} {} {}",
                        t["function_id"]["file"].as_str().unwrap(),
                        t["function_id"]["name"].as_str().unwrap(),
                        t["function_id"]["start_line"],
                        t["function_id"]["end_line"],
                        t["kind"].as_str().unwrap(),
                        t["confidence"].as_str().unwrap()
                    )
                })
                .collect::<Vec<_>>()
                .join(", "),
            None => format!("drop {}", row["drop"].as_str().unwrap_or("?")),
        };
        out.push(format!(
            "L{} {} {}: {shown}",
            row["source_span"]["line"],
            row["callee_text"].as_str().unwrap(),
            row["origin"].as_str().unwrap()
        ));
    }
    out.sort();
    out
}

/// Asserts `got` equals `main` (the record set on `origin/main`); `ALIAS_PRINT=1` prints it.
pub(super) fn equals_main(label: &str, got: Vec<String>, main: &[&str]) {
    if std::env::var("ALIAS_PRINT").is_ok() {
        eprintln!("CASE {label}: {got:?}");
        return;
    }
    assert_eq!(got, main, "{label}: head must equal main");
}

const CALLEE: &str =
    "export function f() {\n  return 1;\n}\nexport function g() {\n  return 2;\n}\n";
const DECOY: &str =
    "export function h() {\n  return 3;\n}\nexport function a() {\n  return 4;\n}\n\
    export function legacy() {\n  return 5;\n}\nexport function fetchUser() {\n  return 6;\n}\n";

fn unproven_producer() -> &'static str {
    "export function h() {\n  return 1;\n}\nexport const x = h;\n"
}

#[test]
fn an_alias_of_an_import_whose_export_is_an_unproven_alias_equals_main() {
    let app = "import { x } from './producer';\nexport function outer() {\n  const h = x;\n  \
        return function run() {\n    return h();\n  };\n}\n";
    let cg = graph_n(&[("producer.ts", unproven_producer()), ("app.ts", app)]);
    equals_main(
        "import alias, unproven export",
        records_of(&cg, "app.ts"),
        &["L5 h Source: producer.ts:h@1-3 free_single exact"], // main: the name search binds the producer's own `h` (R5)
    );
}

#[test]
fn an_alias_of_a_namespace_member_whose_export_is_an_unproven_alias_equals_main() {
    let app = "import * as N from './producer';\nexport function outer() {\n  const h = N.x;\n  \
        return function run() {\n    return h();\n  };\n}\n";
    let cg = graph_n(&[("producer.ts", unproven_producer()), ("app.ts", app)]);
    equals_main(
        "namespace alias, unproven export",
        records_of(&cg, "app.ts"),
        &["L5 h Source: producer.ts:h@1-3 free_single exact"],
    );
}

#[test]
fn a_destructured_tsconfig_paths_namespace_equals_main() {
    use super::js_paths_common::{config, fixture};
    for ext in ["ts", "tsx"] {
        let d = fixture(
            ext,
            "@lib",
            &config(serde_json::json!({"@lib": ["lib/real"]})),
        );
        write(
            d.path(),
            &format!("app.{ext}"),
            "import * as api from '@lib';\nexport function outer() {\n  const { real } = api;\n  \
             return function handler() {\n    return real();\n  };\n}\n",
        );
        let cg = graph(d.path());
        equals_main(
            &format!("paths namespace destructure .{ext}"),
            records_of(&cg, &format!("app.{ext}")),
            &[&format!("L5 real Source: decoy/real.{ext}:real@1-1 free_multi name_only, lib/real.{ext}:real@1-1 free_multi name_only")],  // main: both same-named files, name only
        );
    }
}

#[test]
fn a_destructured_commonjs_namespace_equals_main() {
    let d = tempfile::TempDir::new().unwrap();
    write(d.path(), "api/cjs.js", "exports.legacy = function () {};\n");
    write(d.path(), "other/legacy.ts", DECOY);
    write(
        d.path(),
        "src/app.ts",
        "import * as C from '../api/cjs';\nexport function outer() {\n  const { legacy } = C;\n  \
         return function handler() {\n    return legacy();\n  };\n}\n",
    );
    let cg = graph(d.path());
    equals_main(
        "cjs namespace destructure",
        records_of(&cg, "src/app.ts"),
        &["L5 legacy Source: api/cjs.js:legacy@1-1 free_multi name_only, other/legacy.ts:legacy@7-9 free_multi name_only"],
    );
}

const SPANLESS: &str =
    "let w;\nw = compute();\nexport { w };\nfunction other() {\n  function w() {}\n}\n";

#[test]
fn an_export_without_a_span_is_not_a_proven_alias_terminal() {
    let ns = "import * as N from './producer';\nexport function run() {\n  const h = N.w;\n  \
        return h();\n}\n";
    let named = "import { w } from './producer';\nexport function run() {\n  const h = w;\n  \
        return h();\n}\n";
    let destructure = "import * as N from './producer';\nexport function run() {\n  \
        const { w: h } = N;\n  return h();\n}\n";
    for (label, app, want) in [
        ("ns member", ns, vec!["L4 h Source: drop UnknownName"]),
        // main also holds the textual function-pointer record (a wrong edge to the nested `w`);
        // the stand-down is decided by the alias route's proof, which is absent here, so the
        // head keeps main's complete record set.
        (
            "named import",
            named,
            vec![
                "L4 h Source: drop UnknownName",
                "L4 w IndirectResolution: producer.ts:w@5-5 import_member exact",
            ],
        ),
        (
            "destructure",
            destructure,
            vec!["L4 h Source: drop UnknownName"],
        ),
    ] {
        let cg = graph_n(&[("producer.ts", SPANLESS), ("app.ts", app)]);
        equals_main(
            &format!("span-less {label}"),
            records_of(&cg, "app.ts"),
            &want,
        );
    }
}

#[test]
fn an_export_with_a_span_still_binds_through_both_alias_routes() {
    let producer =
        "function w() {\n  return 1;\n}\nexport { w };\nfunction other() {\n  function w() {}\n}\n";
    let ns = "import * as N from './producer';\nexport function run() {\n  const h = N.w;\n  \
        return h();\n}\n";
    let named = "import { w } from './producer';\nexport function run() {\n  const h = w;\n  \
        return h();\n}\n";
    for (label, app) in [("ns member", ns), ("named import", named)] {
        let cg = graph_n(&[("producer.ts", producer), ("app.ts", app)]);
        assert_eq!(
            records_of(&cg, "app.ts"),
            ["L4 h Source: producer.ts:w@1-3 import_member exact"],
            "{label}"
        );
    }
}

#[test]
fn an_invalid_destructure_is_not_followed_and_equals_main() {
    let head = "import * as N from './callee';\nexport function run() {\n  ";
    for (label, decl, call) in [
        ("rest rebinds", "const { f, ...f } = N;", "f"),
        ("nested rebinds", "const { f, x: { f } } = N;", "f"),
        ("computed sibling", "const { f: a, ['x']: a } = N;", "a"),
        ("string sibling", "const { f: a, 'x': a } = N;", "a"),
    ] {
        let app = format!("{head}{decl}\n  return {call}();\n}}\n");
        let cg = graph_n(&[("callee.js", CALLEE), ("app.js", &app), ("decoy.js", DECOY)]);
        let want = format!("L4 {call} Source: drop UnknownName");
        equals_main(label, records_of(&cg, "app.js"), &[&want]);
    }
}

#[test]
fn valid_siblings_in_a_pattern_are_not_followed_and_equal_main() {
    let head = "import * as N from './callee';\nexport function run() {\n  ";
    for (label, decl) in [
        ("rest sibling", "const { f, ...rest } = N;"),
        ("nested sibling", "const { f, x: { y } } = N;"),
    ] {
        let app = format!("{head}{decl}\n  return f();\n}}\n");
        let cg = graph_n(&[("callee.js", CALLEE), ("app.js", &app), ("decoy.js", DECOY)]);
        equals_main(
            label,
            records_of(&cg, "app.js"),
            &["L4 f Source: drop UnknownName"],
        );
    }
}

const MEMO_C: &str =
    "import { memo } from 'react';\nconst C = memo(function Comp() {\n  return null;\n});\n";

#[test]
fn a_wrapped_callable_terminal_at_a_non_jsx_site_equals_main() {
    // The callable arm is a proof only when it binds exactly one Exact target; a wrapped
    // component called as a function is not one, so the site keeps main's outcome.
    let with_decoy = format!(
        "{MEMO_C}function h() {{\n  return 0;\n}}\nexport function run() {{\n  const h = C;\n  \
         return h();\n}}\n"
    );
    let without = format!("{MEMO_C}export function run() {{\n  const h = C;\n  return h();\n}}\n");
    for (label, src, want) in [
        // main binds the same-file decoy `h` by name (the wrong edge the next slice removes).
        (
            "with a same-named decoy",
            &with_decoy,
            vec!["L10 h Source: app.tsx:h@5-7 local_def exact"],
        ),
        (
            "without a decoy",
            &without,
            vec!["L7 h Source: drop UnknownName"],
        ),
    ] {
        let cg = graph_n(&[("app.tsx", src)]);
        equals_main(label, records_of(&cg, "app.tsx"), &want);
    }
}

#[test]
fn a_wrapped_alias_called_from_a_nested_function_equals_main() {
    let app = format!(
        "{MEMO_C}export function outer() {{\n  const helper = C;\n  return function inner() {{\n    \
         return helper();\n  }};\n}}\n"
    );
    let other = "export function helper() {\n  return 1;\n}\n";
    let cg = graph_n(&[("app.tsx", &app), ("other.ts", other)]);
    equals_main(
        "wrapped alias, nested call",
        records_of(&cg, "app.tsx"),
        &["L8 helper Source: other.ts:helper@1-3 free_single exact"],
    );
}

#[test]
fn same_name_functions_starting_on_one_line_bind_the_one_proven_target() {
    let src = "function f() { function f() {}\n  return 1;\n}\nexport function run() {\n  const h = f;\n  \
        return h();\n}\n";
    let cg = graph_n(&[("app.js", src)]);
    // main: `Source` drops and the textual record binds BOTH same-named functions (the nested
    // `f@1-1` is a wrong edge). The head's alias route proves exactly one target (`f@1-3`, by
    // start and end line), so the predicate is true: one `Source` record, no textual record.
    assert_eq!(
        records_of(&cg, "app.js"),
        ["L6 h Source: app.js:f@1-3 local_def exact"]
    );
}

#[test]
fn commonjs_exports_keep_mains_textual_record_through_an_alias() {
    // The exports are unproven on the alias routes (no span); main's textual record to the
    // CommonJS function is a correct edge and must survive.
    let lib = "function helper() {\n  return 1;\n}\nmodule.exports = { helper };\n";
    let app = "import { helper } from './lib';\nfunction f() {\n  const h1 = helper;\n  \
        return h1();\n}\n";
    let cg = graph_n(&[("lib.js", lib), ("app.js", app)]);
    equals_main(
        "commonjs object export",
        records_of(&cg, "app.js"),
        &[
            "L4 h1 Source: drop UnknownName",
            "L4 helper IndirectResolution: lib.js:helper@1-3 import_member exact",
        ],
    );
}

#[test]
fn a_destructured_namespace_alias_with_a_decoy_named_like_the_namespace_equals_main() {
    let producer = "export function f() {}\nexport const x = f;\n";
    let app = "import * as N from './producer';\nfunction decoy() {\n  function N() {}\n}\n\
        export function run() {\n  const { x: h } = N;\n  return h();\n}\n";
    let cg = graph_n(&[("producer.ts", producer), ("app.ts", app)]);
    equals_main(
        "namespace decoy N",
        records_of(&cg, "app.ts"),
        &[
            "L7 N IndirectResolution: app.ts:N@3-3 local_def exact",
            "L7 h Source: drop UnknownName",
        ],
    );
}

#[test]
fn an_alias_named_like_its_paths_import_does_not_leak_the_import_proof() {
    use super::js_paths_common::{config, fixture};
    let d = fixture(
        "tsx",
        "@lib",
        &config(serde_json::json!({"@lib": ["lib/real"]})),
    );
    write(
        d.path(),
        "lib/real.tsx",
        "import { memo } from 'react';\nexport const real = memo(() => null);\n",
    );
    write(
        d.path(),
        "app.tsx",
        "import { real } from '@lib';\nexport function outer() {\n  const t = real;\n  \
         return function mid() {\n    const real = t;\n    return function inner() {\n      \
         return real();\n    };\n  };\n}\n",
    );
    let cg = graph(d.path());
    equals_main(
        "paths wrapped alias named like the import",
        records_of(&cg, "app.tsx"),
        &["L7 real Source: drop UnknownName"],
    );
}

#[test]
fn a_pattern_binding_a_name_twice_is_not_followed_and_equals_main() {
    let app =
        "import * as N from './m';\nexport function run() {\n  const { f, x: a, f: a } = N;\n  \
        return f();\n}\n";
    let m = "export function f() {\n  return 1;\n}\n";
    let cg = graph_n(&[("m.ts", m), ("app.ts", app)]);
    equals_main(
        "duplicate bound name",
        records_of(&cg, "app.ts"),
        &["L4 f Source: drop UnknownName"],
    );
}
