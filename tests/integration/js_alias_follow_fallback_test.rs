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
    for (label, app) in [
        ("ns member", ns),
        ("named import", named),
        ("destructure", destructure),
    ] {
        let cg = graph_n(&[("producer.ts", SPANLESS), ("app.ts", app)]);
        equals_main(
            &format!("span-less {label}"),
            records_of(&cg, "app.ts"),
            &["L4 h Source: drop UnknownName"], // main: also a textual record (named import only), same wrong edge to the nested `w`
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
