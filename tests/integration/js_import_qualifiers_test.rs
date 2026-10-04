use super::js_paths_common::{graph, outcome, write};
use prism::call_graph::CallGraph;
use tempfile::TempDir;

fn fixture(grammar: &str, module: &str, caller: &str) -> TempDir {
    let d = TempDir::new().unwrap();
    write(
        d.path(),
        "tsconfig.json",
        &serde_json::json!({"compilerOptions":{
        "allowJs":true,"checkJs":true,"moduleResolution":"node","baseUrl":".",
        "paths":{"@lib":["m"]},"jsx":"preserve"},"include":["**/*"]})
        .to_string(),
    );
    write(d.path(), &format!("m.{grammar}"), module);
    write(d.path(), &format!("app.{grammar}"), caller);
    d
}

fn base(mut g: CallGraph) -> CallGraph {
    g.js_ts_qualifier_modules.clear();
    g.js_ts_qualifier_exports.clear();
    g
}

fn hit(g: &CallGraph, grammar: &str, member: &str, target: &str) {
    let r = outcome(g, &format!("app.{grammar}"), member);
    let ts = r["resolved_targets"].as_array().unwrap();
    assert_eq!(ts.len(), 1, "{r}");
    assert_eq!(ts[0]["function_id"]["file"], target, "{r}");
    assert_eq!(ts[0]["confidence"], "exact", "{r}");
    assert_eq!(ts[0]["kind"], "import_qualified", "{r}");
}

#[test]
fn class_statics_and_fields_are_new_exact_in_both_grammars() {
    for grammar in ["jsx", "tsx"] {
        for spec in ["./m", "@lib"] {
            let d = fixture(grammar, "export class C { static sm() { return 1; } static sf = () => 2; im() { return 3; } }\n",
                &format!("import {{ C as X }} from '{spec}'; export function run() {{ X.sm(); X.sf(); }}\n"));
            let g = graph(d.path());
            for member in ["sm", "sf"] {
                assert!(outcome(&base(g.clone()), &format!("app.{grammar}"), member)
                    ["resolved_targets"]
                    .as_array()
                    .unwrap()
                    .is_empty());
                hit(&g, grammar, member, &format!("m.{grammar}"));
            }
        }
    }
}

#[test]
fn declared_namespace_requires_unique_unwritten_callable() {
    let d = fixture(
        "tsx",
        "export { N } from './leaf';\n",
        "import { N as X } from './m'; export function run() { X.nf(); }\n",
    );
    write(
        d.path(),
        "leaf.tsx",
        "export namespace N { export function nf() { return 1; } }",
    );
    let g = graph(d.path());
    assert!(
        outcome(&base(g.clone()), "app.tsx", "nf")["resolved_targets"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    hit(&g, "tsx", "nf", "leaf.tsx");
    for module in [
        "export namespace N { export function nf() {} nf = () => 2; }",
        "export namespace N { function nf() {} }",
        "export namespace N { export function nf() {} } namespace N { export function nf() {} }",
        "export namespace N { export function nf() {} } N.nf = () => 2;",
        "export declare namespace N { function nf(): void; }",
    ] {
        let d = fixture(
            "tsx",
            "export { N } from './leaf';",
            "import { N as X } from './m'; export function run() { X.nf(); }",
        );
        write(d.path(), "leaf.tsx", module);
        let g = graph(d.path());
        assert_eq!(
            outcome(&g, "app.tsx", "nf"),
            outcome(&base(g.clone()), "app.tsx", "nf"),
            "{module}"
        );
    }
    // TypeScript-only syntax in the JavaScript grammar cannot grant authority.
    let d = fixture(
        "jsx",
        "export namespace N { export function nf() {} }",
        "import { N as X } from './m'; export function run() { X.nf(); }",
    );
    let g = graph(d.path());
    assert_eq!(
        outcome(&g, "app.jsx", "nf"),
        outcome(&base(g.clone()), "app.jsx", "nf")
    );
}

#[test]
fn reexported_namespace_reuses_callable_export_core() {
    for grammar in ["jsx", "tsx"] {
        let d = fixture(
            grammar,
            "export * as N from './leaf';",
            "import { N as X } from './m'; export function run() { X.nf(); }",
        );
        write(
            d.path(),
            &format!("leaf.{grammar}"),
            "export function nf() { return 1; }",
        );
        let g = graph(d.path());
        assert!(
            outcome(&base(g.clone()), &format!("app.{grammar}"), "nf")["resolved_targets"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        hit(&g, grammar, "nf", &format!("leaf.{grammar}"));
        for leaf in [
            "export function nf() {} nf = () => 2;",
            "export const nf = register(() => 2);",
            "export const nf = other;",
        ] {
            write(d.path(), &format!("leaf.{grammar}"), leaf);
            let g = graph(d.path());
            assert_eq!(
                outcome(&g, &format!("app.{grammar}"), "nf"),
                outcome(&base(g.clone()), &format!("app.{grammar}"), "nf"),
                "{leaf}"
            );
        }
    }
}

#[test]
fn namespace_this_uses_require_the_same_closed_whitelist() {
    for grammar in ["jsx", "tsx"] {
        for declared in [false, true] {
            if declared && grammar == "jsx" {
                continue; // Declared namespaces are TypeScript-only syntax.
            }
            for body in [
                "const A=this; A.nf=other;",
                "return this;",
                "consume(this);",
                "this.nf=other;",
                "this[key]();",
                "const method=this.nf;",
            ] {
                let d = fixture(
                    grammar,
                    "export { N } from './leaf';",
                    "import { N as X } from './m'; export function run() { X.nf(); }",
                );
                let leaf = if declared {
                    format!("export namespace N {{ export function nf() {{ {body} }} }}")
                } else {
                    write(
                        d.path(),
                        &format!("member.{grammar}"),
                        &format!("export function nf() {{ {body} }}"),
                    );
                    "export * as N from './member';".into()
                };
                write(d.path(), &format!("leaf.{grammar}"), &leaf);
                let g = graph(d.path());
                assert_eq!(
                    outcome(&g, &format!("app.{grammar}"), "nf"),
                    outcome(&base(g.clone()), &format!("app.{grammar}"), "nf"),
                    "{grammar} declared={declared}: {body}"
                );
            }
            // A literal direct receiver call is on the whitelist. Refusing all
            // implicit receivers would discard this positive without evidence.
            let d = fixture(
                grammar,
                "export { N } from './leaf';",
                "import { N as X } from './m'; export function run() { X.nf(); }",
            );
            let target = if declared {
                write(d.path(), "leaf.tsx", "export namespace N { export function nf() { this.other(); } export function other() {} }");
                "leaf.tsx".into()
            } else {
                write(
                    d.path(),
                    &format!("leaf.{grammar}"),
                    "export * as N from './member';",
                );
                let target = format!("member.{grammar}");
                write(
                    d.path(),
                    &target,
                    "export function nf() { this.other(); } export function other() {}",
                );
                target
            };
            hit(&graph(d.path()), grammar, "nf", &target);
        }
    }
}

#[test]
fn object_functions_require_literal_unique_keys() {
    for grammar in ["jsx", "tsx"] {
        let caller = "import { obj as X } from './m'; export function run() { X.of(); }";
        for value in [
            "{ of() { return 1; } }",
            "{ of: () => 1 }",
            "{ of: function named() { return 1; } }",
        ] {
            let d = fixture(grammar, "export { obj } from './leaf';", caller);
            write(
                d.path(),
                &format!("leaf.{grammar}"),
                &format!("export const obj = {value};"),
            );
            let g = graph(d.path());
            assert!(
                outcome(&base(g.clone()), &format!("app.{grammar}"), "of")["resolved_targets"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            hit(&g, grammar, "of", &format!("leaf.{grammar}"));
        }
        for value in [
            "{ of() {}, of: 1 }",
            "{ of() {}, ...unknown }",
            "{ get of() { return () => 1; } }",
            "register({ of() {} })",
            "{ ['of']() {} }",
        ] {
            let d = fixture(grammar, "export { obj } from './leaf';", caller);
            write(
                d.path(),
                &format!("leaf.{grammar}"),
                &format!("export const obj = {value};"),
            );
            let g = graph(d.path());
            assert_eq!(
                outcome(&g, &format!("app.{grammar}"), "of"),
                outcome(&base(g.clone()), &format!("app.{grammar}"), "of"),
                "{value}"
            );
        }
    }
}

#[test]
fn member_and_binding_writes_keep_complete_base() {
    for grammar in ["jsx", "tsx"] {
        for write_text in [
            "C.sm = other;",
            "C['sm'] = other;",
            "delete C.sm;",
            "C.sm++;",
            "C = other;",
        ] {
            let d = fixture(
                grammar,
                &format!("export class C {{ static sm() {{}} }} {write_text}"),
                "import { C as X } from './m'; export function run() { X.sm(); }",
            );
            let g = graph(d.path());
            assert_eq!(
                outcome(&g, &format!("app.{grammar}"), "sm"),
                outcome(&base(g.clone()), &format!("app.{grammar}"), "sm"),
                "{write_text}"
            );
        }
        for write_text in ["X.sm = other;", "X['sm'] = other;", "X = other;"] {
            let d=fixture(grammar,"export class C { static sm() {} }",
                &format!("import {{ C as X }} from './m'; {write_text} export function run() {{ X.sm(); }}"));
            let g = graph(d.path());
            assert_eq!(
                outcome(&g, &format!("app.{grammar}"), "sm"),
                outcome(&base(g.clone()), &format!("app.{grammar}"), "sm"),
                "{write_text}"
            );
        }
        // Owner-authorized lexical over-approximation also refuses shadow writes.
        let d=fixture(grammar,"export class C { static sm() {} }",
            "import { C as X } from './m'; function mutate(X) { X.sm = other; } export function run() { X.sm(); }");
        let g = graph(d.path());
        assert_eq!(
            outcome(&g, &format!("app.{grammar}"), "sm"),
            outcome(&base(g.clone()), &format!("app.{grammar}"), "sm")
        );
    }
}

#[test]
fn unproven_qualifiers_instances_duplicates_and_shadow_keep_base() {
    for grammar in ["jsx", "tsx"] {
        for module in [
            "export class C { im() {} }",
            "export class C { static sm() {} static sm() {} }",
            "export class C { static sm() {} } class C { static sm() {} }",
            "export class C { static sm() { this.sm = other; } }",
            "export const C = new Factory();",
        ] {
            let d = fixture(
                grammar,
                module,
                "import { C as X } from './m'; export function run() { X.sm(); }",
            );
            let g = graph(d.path());
            assert_eq!(
                outcome(&g, &format!("app.{grammar}"), "sm"),
                outcome(&base(g.clone()), &format!("app.{grammar}"), "sm"),
                "{module}"
            );
        }
        for caller in [
            "import { C as X } from './m'; export function run(X) { X.sm(); }",
            "import { C as X } from './m'; export function run() { const X = unknown; X.sm(); }",
            "const X = require('./m'); export function run() { X.sm(); }",
        ] {
            let d = fixture(grammar, "export class C { static sm() {} }", caller);
            let g = graph(d.path());
            assert_eq!(
                outcome(&g, &format!("app.{grammar}"), "sm"),
                outcome(&base(g.clone()), &format!("app.{grammar}"), "sm")
            );
        }
    }
}

#[test]
fn explicit_js_sibling_and_barrel_proof() {
    for grammar in ["jsx", "tsx"] {
        let d = fixture(
            grammar,
            "export { C } from './leaf';",
            "import { C as X } from './m'; export function run() { X.sm(); }",
        );
        write(
            d.path(),
            &format!("leaf.{grammar}"),
            "export class C { static sm() {} }",
        );
        hit(&graph(d.path()), grammar, "sm", &format!("leaf.{grammar}"));
        write(
            d.path(),
            &format!("m.{grammar}"),
            "export * from './leaf'; export * from './missing';",
        );
        let g = graph(d.path());
        assert_eq!(
            outcome(&g, &format!("app.{grammar}"), "sm"),
            outcome(&base(g.clone()), &format!("app.{grammar}"), "sm")
        );
    }
    let d = fixture(
        "tsx",
        "export class C { static sm() {} }",
        "import { C as X } from './m.js'; export function run() { X.sm(); }",
    );
    hit(&graph(d.path()), "tsx", "sm", "m.tsx");
}

#[test]
fn any_already_bound_row_is_byte_preserved() {
    for grammar in ["jsx", "tsx"] {
        let d = fixture(
            grammar,
            "export const obj = { f }; export function f() {}",
            "import { obj as X } from './m'; export function run() { X.f(); }",
        );
        let g = graph(d.path());
        let before = outcome(&base(g.clone()), &format!("app.{grammar}"), "f");
        assert!(!before["resolved_targets"].as_array().unwrap().is_empty());
        assert_eq!(outcome(&g, &format!("app.{grammar}"), "f"), before);
        let d = fixture(
            grammar,
            "export const obj = {\n f() {}\n};\nexport function f() {}\n",
            "import { obj as X } from './m'; export function run() { X.f(); }",
        );
        let g = graph(d.path());
        let before = outcome(&base(g.clone()), &format!("app.{grammar}"), "f");
        assert!(
            before["resolved_targets"].as_array().unwrap().len() > 1,
            "{before}"
        );
        assert_eq!(outcome(&g, &format!("app.{grammar}"), "f"), before);
    }
}

#[test]
fn forwarding_member_writes_and_namespace_identity_conflicts_keep_base() {
    for grammar in ["jsx", "tsx"] {
        let d = fixture(
            grammar,
            "import { C } from './leaf'; C.sm = other; export { C };",
            "import { C as X } from './m'; export function run() { X.sm(); }",
        );
        write(
            d.path(),
            &format!("leaf.{grammar}"),
            "export class C { static sm() {} }",
        );
        let g = graph(d.path());
        assert_eq!(
            outcome(&g, &format!("app.{grammar}"), "sm"),
            outcome(&base(g.clone()), &format!("app.{grammar}"), "sm")
        );
        let d = fixture(
            grammar,
            "export * from './left'; export * from './right';",
            "import { N as X } from './m'; export function run() { X.nf(); }",
        );
        for (p, text) in [
            ("left", "export * as N from './one';"),
            ("right", "export * as N from './two';"),
            ("one", "export { nf } from './real';"),
            ("two", "export { nf } from './real';"),
            ("real", "export function nf() {}"),
        ] {
            write(d.path(), &format!("{p}.{grammar}"), text);
        }
        let g = graph(d.path());
        assert_eq!(
            outcome(&g, &format!("app.{grammar}"), "nf"),
            outcome(&base(g.clone()), &format!("app.{grammar}"), "nf")
        );
    }
}

#[test]
fn s2_caller_ownership_and_declaration_priority_guards() {
    for grammar in ["jsx", "tsx"] {
        let d = fixture(
            grammar,
            "export class C { static sm() {} }",
            "import { C as X } from './m'; export function run() { X.sm(); }",
        );
        for cfg in [
            serde_json::json!({"compilerOptions":{"allowJs":true,"moduleResolution":"node"},"references":[{"path":"./other"}]}),
            serde_json::json!({"compilerOptions":{"allowJs":true,"moduleSuffixes":[".custom",""]}}),
            serde_json::json!({"compilerOptions":{"allowJs":true,"rootDirs":[".","./other"]}}),
            serde_json::json!({"compilerOptions":{"allowJs":true},"exclude":["app.*"]}),
        ] {
            write(d.path(), "tsconfig.json", &cfg.to_string());
            let g = graph(d.path());
            assert_eq!(
                outcome(&g, &format!("app.{grammar}"), "sm"),
                outcome(&base(g.clone()), &format!("app.{grammar}"), "sm")
            );
        }
    }
    let d = fixture(
        "tsx",
        "export class C { static sm() {} }",
        "import { C as X } from './m.js'; export function run() { X.sm(); }",
    );
    write(d.path(), "m.tsx", "export class C { static sm() {} }");
    write(
        d.path(),
        "m.ts",
        "export declare class C { static sm(): void; }",
    );
    let g = graph(d.path());
    assert_eq!(
        outcome(&g, "app.tsx", "sm"),
        outcome(&base(g.clone()), "app.tsx", "sm")
    );
    std::fs::remove_file(d.path().join("m.ts")).unwrap();
    write(d.path(),"tsconfig.json","{\"references\":[],\"compilerOptions\":{\"moduleResolution\":\"nodenext\"},\"include\":[\"**/*\"]}");
    hit(&graph(d.path()), "tsx", "sm", "m.tsx");
}

#[test]
fn default_class_is_proven_and_decorators_keep_base() {
    for grammar in ["jsx", "tsx"] {
        let d = fixture(
            grammar,
            "export default class C { static sm() {} }",
            "import X from './m'; export function run() { X.sm(); }",
        );
        hit(&graph(d.path()), grammar, "sm", &format!("m.{grammar}"));
        write(
            d.path(),
            &format!("m.{grammar}"),
            "@decorate export class C { static sm() {} }",
        );
        write(
            d.path(),
            &format!("app.{grammar}"),
            "import { C as X } from './m'; export function run() { X.sm(); }",
        );
        let g = graph(d.path());
        assert_eq!(
            outcome(&g, &format!("app.{grammar}"), "sm"),
            outcome(&base(g.clone()), &format!("app.{grammar}"), "sm")
        );
    }
}

#[test]
fn all_statically_joined_qualifier_writes_keep_base() {
    let mut failures = Vec::new();
    for grammar in ["jsx", "tsx"] {
        // A distinct importer writes the same declaration-backed qualifier.
        let d = fixture(
            grammar,
            "export class C { static sm() {} }",
            "import { C as X } from './m'; export function run() { X.sm(); }",
        );
        write(
            d.path(),
            &format!("writer.{grammar}"),
            "import { C as Y } from './m'; Y.sm = other;",
        );
        let g = graph(d.path());
        if outcome(&g, &format!("app.{grammar}"), "sm")
            != outcome(&base(g.clone()), &format!("app.{grammar}"), "sm")
        {
            failures.push(format!("{grammar}:imported-write"));
        }
        // Destructuring assignment still writes a member, not its root binding.
        let d = fixture(
            grammar,
            "export class C { static sm() {} } ({ value: C.sm } = other);",
            "import { C as X } from './m'; export function run() { X.sm(); }",
        );
        let g = graph(d.path());
        if outcome(&g, &format!("app.{grammar}"), "sm")
            != outcome(&base(g.clone()), &format!("app.{grammar}"), "sm")
        {
            failures.push(format!("{grammar}:destructuring-write"));
        }
        let d = fixture(
            grammar,
            "export { obj } from './leaf';",
            "import { obj as X } from './m'; export function run() { X.of(); }",
        );
        write(
            d.path(),
            &format!("leaf.{grammar}"),
            "export const obj = { of() { this.of = other; } };",
        );
        let g = graph(d.path());
        if outcome(&g, &format!("app.{grammar}"), "of")
            != outcome(&base(g.clone()), &format!("app.{grammar}"), "of")
        {
            failures.push(format!("{grammar}:object-this-write"));
        }
    }
    assert!(failures.is_empty(), "complete E5 population: {failures:?}");
}

#[test]
fn explicit_suffix_priority_and_declaration_refusals_in_both_grammars() {
    let mut failures = Vec::new();
    for grammar in ["jsx", "tsx"] {
        for (suffix, expected, declaration) in [
            ("js", Some("m.ts"), "m.d.ts"),
            ("jsx", Some("m.tsx"), "m.d.ts"),
            ("mjs", None, "m.d.mts"),
            ("cjs", None, "m.d.cts"),
        ] {
            let caller = format!(
                "import {{ C as X }} from './m.{suffix}'; export function run() {{ X.sm(); }}"
            );
            let d = fixture(grammar, "", &caller);
            std::fs::remove_file(d.path().join(format!("m.{grammar}"))).unwrap();
            for file in ["m.ts", "m.tsx", "m.mts", "m.cts"] {
                write(d.path(), file, "export class C { static sm() {} }");
            }
            let g = graph(d.path());
            let before = outcome(&base(g.clone()), &format!("app.{grammar}"), "sm");
            let after = outcome(&g, &format!("app.{grammar}"), "sm");
            let correct = match expected {
                Some(expected) => {
                    after["resolved_targets"].as_array().unwrap().len() == 1
                        && after["resolved_targets"][0]["function_id"]["file"] == expected
                        && after["resolved_targets"][0]["confidence"] == "exact"
                }
                None => after == before,
            };
            if !before["resolved_targets"].as_array().unwrap().is_empty() || !correct {
                failures.push(format!("{grammar}:{suffix}:{after}"));
            }
            let d = fixture(grammar, "", &caller);
            std::fs::remove_file(d.path().join(format!("m.{grammar}"))).unwrap();
            write(
                d.path(),
                declaration,
                "export declare class C { static sm(): void; }",
            );
            write(
                d.path(),
                &format!("m.{suffix}"),
                "export class C { static sm() {} }",
            );
            let g = graph(d.path());
            if outcome(&g, &format!("app.{grammar}"), "sm")
                != outcome(&base(g.clone()), &format!("app.{grammar}"), "sm")
            {
                failures.push(format!("{grammar}:{suffix}:declaration"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "complete suffix population: {failures:?}"
    );
}

#[test]
fn qualifier_identity_whitelist_refuses_all_value_escapes() {
    let mut failures = Vec::new();
    for grammar in ["jsx", "tsx"] {
        for usage in [
            "const A = Q; A.sm = replacement;",
            "let A; A = Q;",
            "const { sm } = Q;",
            "const [A] = [Q];",
            "consume(Q);",
            "function escaped() { return Q; }",
            "const holder = { value: Q };",
            "const holder = { Q };",
            "const array = [Q];",
            "new Map([[0, Q]]);",
            "const spread = { ...Q };",
            "consume(...Q);",
            "Q.sm = replacement;",
            "Q[key] = replacement;",
            "Object.assign(Q, {});",
            "Object.defineProperty(Q, 'sm', {});",
            "Reflect.get(Q, 'sm');",
            "Q[key]();",
            "const method = Q.sm;",
            "export { Q as Renamed };",
            "export default Q; Q.sm();",
            "const A = Q; A[key] = replacement;",
            "const {value: A} = {value: Q}; A.sm = replacement;",
        ] {
            for place in ["provider", "caller", "importer", "forwarder"] {
                let provider = if place == "provider" {
                    format!(
                        "export class C {{ static sm() {{}} }} {}",
                        usage.replace('Q', "C")
                    )
                } else {
                    "export class C { static sm() {} }".into()
                };
                let caller = format!(
                    "import {{ C as X }} from './m'; export function run() {{ X.sm(); }} {}",
                    if place == "caller" {
                        usage.replace('Q', "X")
                    } else {
                        String::new()
                    }
                );
                let d = fixture(grammar, &provider, &caller);
                if matches!(place, "importer" | "forwarder") {
                    write(
                        d.path(),
                        &format!("other.{grammar}"),
                        &format!(
                            "import {{ C as Q }} from './m'; {} {}",
                            usage,
                            if place == "forwarder" {
                                "export { Q };"
                            } else {
                                ""
                            }
                        ),
                    );
                }
                let g = graph(d.path());
                if outcome(&g, &format!("app.{grammar}"), "sm")
                    != outcome(&base(g.clone()), &format!("app.{grammar}"), "sm")
                {
                    failures.push(format!("{grammar}:{place}:{usage}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "whole escape population: {failures:?}");
}

#[test]
fn qualifier_whitelist_namespace_paths_reexports_and_this_keep_base() {
    let mut failures = Vec::new();
    for grammar in ["jsx", "tsx"] {
        for extra in [
            "export { C as D } from './m';",
            "import * as ns from './m'; const A = ns.C; A.sm = other;",
            "import * as ns from './m'; ns.C.sm = other;",
            "export async function mutate() { const { C: A } = await import('./m'); A.sm = other; }",
            "export async function mutate() { const { C } = await import('./m'); C.sm = other; }",
            "export async function mutate() { const A = (await import('./m')).C; A.sm = other; }",
        ] {
            let d = fixture(
                grammar,
                "export class C { static sm() {} }",
                "import { C as X } from './m'; export function run() { X.sm(); }",
            );
            write(d.path(), &format!("other.{grammar}"), extra);
            let g = graph(d.path());
            if outcome(&g, &format!("app.{grammar}"), "sm")
                != outcome(&base(g.clone()), &format!("app.{grammar}"), "sm")
            {
                failures.push(format!("{grammar}:{extra}"));
            }
        }
        for provider in [
            "export class C { static sm() { return this; } }",
            "export class C { static sm() { consume(this); } }",
            "export class C { static sm() { return this[key](); } }",
            "export const C = { sm() { return this; } };",
            "export const C = { sm() { consume(this); } };",
        ] {
            let d = fixture(
                grammar,
                "export { C } from './leaf';",
                "import { C as X } from './m'; export function run() { X.sm(); }",
            );
            write(d.path(), &format!("leaf.{grammar}"), provider);
            let g = graph(d.path());
            if outcome(&g, &format!("app.{grammar}"), "sm")
                != outcome(&base(g.clone()), &format!("app.{grammar}"), "sm")
            {
                failures.push(format!("{grammar}:{provider}"));
            }
        }
        // A writer outside the proven caller configuration is still visible.
        let d = fixture(
            grammar,
            "export class C { static sm() {} }",
            "import { C as X } from './m'; export function run() { X.sm(); }",
        );
        write(
            d.path(),
            "outside/tsconfig.json",
            "{\"compilerOptions\":{\"noResolve\":true},\"include\":[\"**/*\"]}",
        );
        write(
            d.path(),
            &format!("outside/other.{grammar}"),
            "import { C as Q } from '../m'; consume(Q);",
        );
        let g = graph(d.path());
        if outcome(&g, &format!("app.{grammar}"), "sm")
            != outcome(&base(g.clone()), &format!("app.{grammar}"), "sm")
        {
            failures.push(format!("{grammar}:unproved-writer-owner"));
        }
    }
    assert!(
        failures.is_empty(),
        "identity closure population: {failures:?}"
    );
}

#[test]
fn qualifier_whitelist_allows_declarations_calls_types_and_own_export() {
    for grammar in ["jsx", "tsx"] {
        let types = if grammar == "tsx" {
            "type T = C; let instance: C; type U = typeof C;"
        } else {
            ""
        };
        let d = fixture(
            grammar,
            &format!("class C {{ static sm() {{}} }} export {{ C }}; C.sm(); {types}"),
            "import { C as X } from './m'; export function run() { X.sm(); } export function shadow(X) { X.sm(); }",
        );
        let g = graph(d.path());
        let row = |graph: &CallGraph, caller: &str| {
            prism::navigation::queries::call_site_dump(graph)
                .into_iter()
                .find(|r| {
                    r["caller"]["file"] == format!("app.{grammar}")
                        && r["caller"]["name"] == caller
                        && r["callee_text"] == "sm"
                })
                .unwrap()
        };
        assert_eq!(
            row(&g, "run")["resolved_targets"].as_array().unwrap().len(),
            1
        );
        assert_eq!(row(&g, "shadow"), row(&base(g.clone()), "shadow"));
        let d = fixture(
            grammar,
            "class C { static sm() {} } export default C;",
            "import X from './m'; export function run() { X.sm(); }",
        );
        hit(&graph(d.path()), grammar, "sm", &format!("m.{grammar}"));
        // An unresolved bare package stays unavailable, with no in-repo targets.
        write(
            d.path(),
            &format!("other.{grammar}"),
            "import * as external from 'opaque-package'; consume(external);",
        );
        let g = graph(d.path());
        hit(&g, grammar, "sm", &format!("m.{grammar}"));
    }
}

#[test]
fn repair_r1_non_owned_calls_keep_base() {
    let mut failures = Vec::new();
    for grammar in ["jsx", "tsx"] {
        let cases = [
            ("provider-unused-non-owned", "export class C { static sm() { return 1; } } C.valueOf();", "", ""),
            ("heritage-unused", "class B {} export class C extends B { static sm() { return 1; } }", "", ""),
            ("valueOf-alias", "export class C { static sm() { return 1; } }", "", "import { C as Q } from './m'; const v = Q.valueOf(); v.sm = () => 2;"),
            ("valueOf-argument", "export class C { static sm() { return 1; } }", "", "import { C as Q } from './m'; Object.assign(Q.valueOf(), { sm: () => 2 });"),
            ("defineGetter", "export class C { static sm() { return 1; } }", "", "import { C as Q } from './m'; Q.__defineGetter__('sm', () => () => 2);"),
            ("defineSetter", "export class C { static sm() { return 1; } }", "", "import { C as Q } from './m'; Q.__defineSetter__('sm', () => {});"),
            ("inherited-self", "import { B } from './base'; export class C extends B { static sm() { return 1; } }", "export class B { static self() { return this; } }", "import { C as Q } from './m'; Object.assign(Q.self(), { sm: () => 2 });"),
            ("inherited-write-provider", "class B { static install(f) { this.sm = f; } } export class C extends B { static sm() { return 1; } } C.install(() => 2);", "", ""),
            ("inherited-write-importer", "import { B } from './base'; export class C extends B { static sm() { return 1; } }", "export class B { static install(f) { this.sm = f; } }", "import { C as Q } from './m'; Q.install(() => 2);"),
            ("provider-non-owned", "export class C { static sm() { return 1; } } const v = C.valueOf(); v.sm = () => 2;", "", ""),
            ("receiver-non-owned", "export class C { static sm() { return 1; } static leak() { return this.valueOf(); } } const v = C.leak(); v.sm = () => 2;", "", ""),
            ("forwarded-non-owned", "export class C { static sm() { return 1; } }", "", "import { C as Q } from './barrel'; Object.assign(Q.valueOf(), { sm: () => 2 });"),
        ];
        for (label, provider, ancestor, writer) in cases {
            let d = fixture(
                grammar,
                provider,
                "import { C as X } from './m'; export function run() { X.sm(); }",
            );
            write(d.path(), &format!("base.{grammar}"), ancestor);
            write(
                d.path(),
                &format!("barrel.{grammar}"),
                "export { C } from './m';",
            );
            write(d.path(), &format!("other.{grammar}"), writer);
            let g = graph(d.path());
            let before = outcome(&base(g.clone()), &format!("app.{grammar}"), "sm");
            assert!(
                before["resolved_targets"].as_array().unwrap().is_empty(),
                "{grammar}:{label}: base must drop: {before}"
            );
            if outcome(&g, &format!("app.{grammar}"), "sm") != before {
                failures.push(format!("{grammar}:{label}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "R1 non-owned-call population: {failures:?}"
    );
}

#[test]
fn repair_r1_owned_calls_remain_exact() {
    for grammar in ["jsx", "tsx"] {
        let d = fixture(grammar,
            "export class C { static sm() { return 1; } static own() { return this.sm(); } } C.own();",
            "import { C as X } from './m'; export function run() { X.sm(); X.own(); }");
        write(
            d.path(),
            &format!("other.{grammar}"),
            "import { C as Q } from './m'; Q.own();",
        );
        let g = graph(d.path());
        for name in ["sm", "own"] {
            assert!(
                outcome(&base(g.clone()), &format!("app.{grammar}"), name)["resolved_targets"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            hit(&g, grammar, name, &format!("m.{grammar}"));
        }
    }
}

#[test]
fn repair_r1_construction_keep_base() {
    let mut failures = Vec::new();
    for grammar in ["jsx", "tsx"] {
        for writer in [
            "import { C as Q } from './m'; new Q().constructor.sm = () => 2;",
            "import { C as Q } from './m'; const A = (new Q()).constructor; A.sm = () => 2;",
            "import { C as Q } from './m'; Object.getPrototypeOf(new Q()).constructor.sm = () => 2;",
            "import { C as Q } from './m'; const inst = new Q(); inst['constructor'].sm = () => 2;",
            "import { C as Q } from './m'; const inst = new Q(); const key = String.fromCharCode(99,111,110,115,116,114,117,99,116,111,114); inst[key].sm = () => 2;",
            "import { C as Q } from './m'; new Q();",
        ] {
            let d = fixture(grammar, "export class C { static sm() { return 1; } }", "import { C as X } from './m'; export function run() { X.sm(); }");
            write(d.path(), &format!("other.{grammar}"), writer);
            let g = graph(d.path());
            let before = outcome(&base(g.clone()), &format!("app.{grammar}"), "sm");
            assert!(before["resolved_targets"].as_array().unwrap().is_empty());
            if outcome(&g, &format!("app.{grammar}"), "sm") != before {
                failures.push(format!("{grammar}:{writer}"));
            }
        }
        let d = fixture(grammar,
            "function replacement() { return 2; } export class C { static sm() { return 1; } } const A = (new C()).constructor; A.sm = replacement;",
            "import { C as X } from './m'; export function run() { X.sm(); }");
        let g = graph(d.path());
        if outcome(&g, &format!("app.{grammar}"), "sm")
            != outcome(&base(g.clone()), &format!("app.{grammar}"), "sm")
        {
            failures.push(format!("{grammar}:provider-instance-constructor"));
        }
    }
    assert!(
        failures.is_empty(),
        "R1 construction population: {failures:?}"
    );
}

#[test]
fn repair_r1_receiver_carriers_keep_base() {
    let mut failures = Vec::new();
    for grammar in ["jsx", "tsx"] {
        {
            let d = fixture(grammar, "function replacement(){return 1;} export class C {constructor(){new.target.sm=replacement;} static sm(){return 0;}} new C();", "import { C as X } from './m'; export function run() { X.sm(); }");
            write(d.path(), &format!("other.{grammar}"), "");
            let g = graph(d.path());
            let before = outcome(&base(g.clone()), &format!("app.{grammar}"), "sm");
            assert!(before["resolved_targets"].as_array().unwrap().is_empty());
            if outcome(&g, &format!("app.{grammar}"), "sm") != before {
                failures.push(format!("{grammar}:F5-newtarget"));
            }
        }
        {
            let d = fixture(grammar, "import {B} from \"./b\"; export class C extends B {static sm(){return 0;}} new C();", "import { C as X } from './m'; export function run() { X.sm(); }");
            write(d.path(), &format!("other.{grammar}"), "");
            write(
                d.path(),
                &format!("b.{grammar}"),
                "export class B {constructor(){new.target.sm=()=>1;}}",
            );
            let g = graph(d.path());
            let before = outcome(&base(g.clone()), &format!("app.{grammar}"), "sm");
            assert!(before["resolved_targets"].as_array().unwrap().is_empty());
            if outcome(&g, &format!("app.{grammar}"), "sm") != before {
                failures.push(format!("{grammar}:F5-inherited"));
            }
        }
        {
            let d = fixture(grammar, "class B {static sm(){return 2;}} export class C extends B {static sm(){return 0;} static init(){super.sm=()=>1;}} C.init();", "import { C as X } from './m'; export function run() { X.sm(); }");
            write(d.path(), &format!("other.{grammar}"), "");
            let g = graph(d.path());
            let before = outcome(&base(g.clone()), &format!("app.{grammar}"), "sm");
            assert!(before["resolved_targets"].as_array().unwrap().is_empty());
            if outcome(&g, &format!("app.{grammar}"), "sm") != before {
                failures.push(format!("{grammar}:F5-super"));
            }
        }
        {
            let d = fixture(
                grammar,
                "export class C {constructor(){const self=new.target;} static sm(){return 0;}}",
                "import { C as X } from './m'; export function run() { X.sm(); }",
            );
            write(d.path(), &format!("other.{grammar}"), "");
            let g = graph(d.path());
            let before = outcome(&base(g.clone()), &format!("app.{grammar}"), "sm");
            assert!(before["resolved_targets"].as_array().unwrap().is_empty());
            if outcome(&g, &format!("app.{grammar}"), "sm") != before {
                failures.push(format!("{grammar}:F5-uncalled"));
            }
            if !g.js_ts_exports[&format!("m.{grammar}")]
                .qualifiers
                .written
                .contains("C")
            {
                failures.push(format!("{grammar}:explicit-new-target-carrier"));
            }
        }
    }
    assert!(failures.is_empty(), "F5 population: {failures:?}");
}

#[test]
fn repair_r1_nested_this_keep_base() {
    let mut failures = Vec::new();
    for grammar in ["jsx", "tsx"] {
        {
            let d = fixture(grammar, "function replacement(){return 1;} export class C {static sm(){const box={value:this}; return box.value;}} const A=C.sm(); A.sm=replacement;", "import { C as X } from './m'; export function run() { X.sm(); }");
            write(d.path(), &format!("other.{grammar}"), "");
            let g = graph(d.path());
            let before = outcome(&base(g.clone()), &format!("app.{grammar}"), "sm");
            assert!(before["resolved_targets"].as_array().unwrap().is_empty());
            if outcome(&g, &format!("app.{grammar}"), "sm") != before {
                failures.push(format!("{grammar}:F6-value"));
            }
        }
        {
            let d = fixture(grammar, "function replacement(){return 1;} export class C {static sm(){const box={leak:()=>this}; return box.leak();}} const A=C.sm(); A.sm=replacement;", "import { C as X } from './m'; export function run() { X.sm(); }");
            write(d.path(), &format!("other.{grammar}"), "");
            let g = graph(d.path());
            let before = outcome(&base(g.clone()), &format!("app.{grammar}"), "sm");
            assert!(before["resolved_targets"].as_array().unwrap().is_empty());
            if outcome(&g, &format!("app.{grammar}"), "sm") != before {
                failures.push(format!("{grammar}:F6-arrow"));
            }
        }
        {
            let d = fixture(grammar, "export class C {static sm(){return 0;} static install(){const box={leak:()=>{this.sm=()=>1;}};box.leak();}} C.install();", "import { C as X } from './m'; export function run() { X.sm(); }");
            write(d.path(), &format!("other.{grammar}"), "");
            let g = graph(d.path());
            let before = outcome(&base(g.clone()), &format!("app.{grammar}"), "sm");
            assert!(before["resolved_targets"].as_array().unwrap().is_empty());
            if outcome(&g, &format!("app.{grammar}"), "sm") != before {
                failures.push(format!("{grammar}:F6-write-arrow"));
            }
        }
    }
    assert!(failures.is_empty(), "F6 population: {failures:?}");
}

#[test]
fn repair_r1_parse_incomplete_keep_base() {
    let mut failures = Vec::new();
    for grammar in ["jsx", "tsx"] {
        {
            let d = fixture(
                grammar,
                "export class C { static sm() { return 0; } }",
                "import { C as X } from './m'; export function run() { X.sm(); }",
            );
            write(d.path(), &format!("other.{grammar}"), "unterminated(");
            let g = graph(d.path());
            let before = outcome(&base(g.clone()), &format!("app.{grammar}"), "sm");
            assert!(before["resolved_targets"].as_array().unwrap().is_empty());
            if outcome(&g, &format!("app.{grammar}"), "sm") != before {
                failures.push(format!("{grammar}:F8-incomplete"));
            }
            if !g
                .js_ts_exports
                .get(&format!("other.{grammar}"))
                .is_some_and(|f| f.qualifiers.syntax_incomplete)
            {
                failures.push(format!("{grammar}:incomplete-facts-retained"));
            }
        }
    }
    assert!(failures.is_empty(), "F8 population: {failures:?}");
}

#[test]
fn repair_r1b_error_refusals_preserve_legacy_star_opacity() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/js_paths_r1.json")).unwrap();
    let mut failures = Vec::new();
    for case in cases
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["case"].as_str().unwrap().starts_with("C57-"))
    {
        let d = TempDir::new().unwrap();
        for (file, text) in case["payload"].as_object().unwrap() {
            write(d.path(), file, text.as_str().unwrap());
        }
        let g = graph(d.path());
        let loaded = prism::repo_loader::load_repo(d.path()).unwrap();
        let before = CallGraph::build(&loaded.files);
        let app = case["app"].as_str().unwrap();
        if outcome(&g, app, "picked") != outcome(&before, app, "picked") {
            failures.push(case["case"].clone());
        }
    }
    assert!(failures.is_empty(), "legacy opacity: {failures:?}");
}

// S2-O7: these runtime channels do not change the static-binding grade.
// Pin the observed Exact so a future contract expansion must change the test.
#[test]
fn repair_r2_out_of_model_dynamic_import() {
    for grammar in ["jsx", "tsx"] {
        for writer in [
            "export const ready=import('./m').then(ns=>{ns['C'].sm=()=>1;});",
            "const path='./m'; export const ready=import(path).then(ns=>{ns['C'].sm=()=>1;});",
        ] {
            let d = fixture(
                grammar,
                "export class C {static sm(){return 0;}}",
                "import {C as X} from './m'; export function run(){X.sm();}",
            );
            write(d.path(), &format!("other.{grammar}"), writer);
            hit(&graph(d.path()), grammar, "sm", &format!("m.{grammar}"));
        }
    }
}

#[test]
fn repair_r2_out_of_model_require_and_ts_import_require() {
    for grammar in ["jsx", "tsx"] {
        let d = fixture(
            grammar,
            "export class C {static sm(){return 0;}}",
            "import {C as X} from './m'; export function run(){X.sm();}",
        );
        write(
            d.path(),
            &format!("other.{grammar}"),
            "const ns=require('./m'); ns['C'].sm=()=>1;",
        );
        hit(&graph(d.path()), grammar, "sm", &format!("m.{grammar}"));
        // import = require is TS syntax; exercise it with both provider grammars.
        write(d.path(), &format!("other.{grammar}"), "");
        write(
            d.path(),
            "writer.ts",
            "import ns = require('./m'); ns['C'].sm=()=>1;",
        );
        hit(&graph(d.path()), grammar, "sm", &format!("m.{grammar}"));
    }
}

#[test]
fn repair_r2_out_of_model_default_reacquisition() {
    for grammar in ["jsx", "tsx"] {
        let d = fixture(
            grammar,
            "export default class C {static sm(){return 0;}}",
            "import X from './m'; export function run(){X.sm();}",
        );
        write(
            d.path(),
            &format!("other.{grammar}"),
            "export const ready=import('./m').then(ns=>{ns.default.sm=()=>1;});",
        );
        hit(&graph(d.path()), grammar, "sm", &format!("m.{grammar}"));
    }
}

#[test]
fn repair_r3_statically_bound_namespace_enumeration_keeps_base() {
    for grammar in ["jsx", "tsx"] {
        let d = fixture(
            grammar,
            "export class C {static sm(){return 0;}}",
            "import {C as X} from './m'; export function run(){X.sm();}",
        );
        write(
            d.path(),
            &format!("barrel.{grammar}"),
            "export * as M from './m';",
        );
        write(
            d.path(),
            &format!("other.{grammar}"),
            "import {M} from './barrel'; Object.values(M).forEach(c=>{c.sm=()=>1;});",
        );
        let g = graph(d.path());
        assert_eq!(
            outcome(&g, &format!("app.{grammar}"), "sm"),
            outcome(&base(g.clone()), &format!("app.{grammar}"), "sm")
        );
    }
}

#[test]
fn repair_r2_out_of_model_eval_function_and_reflected_codegen() {
    for grammar in ["jsx", "tsx"] {
        for writer in [
            "export const ready=eval(\"import('./m').then(ns=>{ns.C.sm=()=>1})\");",
            "export const ready=new Function(\"return import('./m').then(ns=>{ns.C.sm=()=>1})\")();",
            "export const ready=[].filter.constructor(\"return import('./m').then(ns=>{ns.C.sm=()=>1})\")();",
            "with (scope) { indirect.sm=replacement; }",
        ] {
            let d=fixture(grammar,"export class C {static sm(){return 0;}}","import {C as X} from './m'; export function run(){X.sm();}");
            write(d.path(), &format!("other.{grammar}"), writer);
            hit(&graph(d.path()),grammar,"sm",&format!("m.{grammar}"));
        }
    }
}

#[test]
fn repair_r2_out_of_model_host_globals_and_reflective_lookup() {
    for grammar in ["jsx", "tsx"] {
        let d = fixture(
            grammar,
            "export class C {static sm(){return 0;}}",
            "import {C as X} from './m'; export function run(){X.sm();}",
        );
        write(
            d.path(),
            &format!("other.{grammar}"),
            "Reflect.get(globalThis,'registry')['C'].sm=()=>1; window['registry']['C'].sm=()=>1;",
        );
        hit(&graph(d.path()), grammar, "sm", &format!("m.{grammar}"));
    }
}

#[test]
fn repair_r2_static_alias_and_reflective_writes_keep_base() {
    for grammar in ["jsx", "tsx"] {
        for writer in [
            "import {C as Y} from './m'; const A=Y; A.sm=()=>1;",
            "import {C as Y} from './m'; Y.sm=()=>1;",
            "import {C as Y} from './m'; Reflect.set(Y,'sm',()=>1);",
        ] {
            let d = fixture(
                grammar,
                "export class C {static sm(){return 0;}}",
                "import {C as X} from './m'; export function run(){X.sm();}",
            );
            write(d.path(), &format!("other.{grammar}"), writer);
            let g = graph(d.path());
            assert_eq!(
                outcome(&g, &format!("app.{grammar}"), "sm"),
                outcome(&base(g.clone()), &format!("app.{grammar}"), "sm"),
                "{grammar}: {writer}"
            );
        }
    }
}

#[test]
fn repair_r3_reviewer_static_refusals_keep_base() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/s2_r3_static_refusals.json")).unwrap();
    let mut failures = Vec::new();
    for case in cases.as_array().unwrap() {
        let d = TempDir::new().unwrap();
        let files = case["files"].as_object().unwrap();
        for (file, text) in files {
            write(d.path(), file, text.as_str().unwrap());
        }
        let app = if files.contains_key("app.jsx") {
            "app.jsx"
        } else {
            "app.tsx"
        };
        let loaded = prism::repo_loader::load_repo(d.path()).unwrap();
        let main = CallGraph::build(&loaded.files);
        let before = outcome(&main, app, "sm");
        assert!(
            before["resolved_targets"].as_array().unwrap().is_empty(),
            "{}: main",
            case["case"]
        );
        let after = outcome(&graph(d.path()), app, "sm");
        if after != before {
            failures.push(case["case"].as_str().unwrap().to_string());
        }
    }
    assert!(failures.is_empty(), "complete R3 population: {failures:?}");
}

#[test]
fn repair_r3_clean_namespace_and_unrelated_closure_controls() {
    for grammar in ["jsx", "tsx"] {
        let d = fixture(
            grammar,
            "export class C {static sm(){return 0;}}",
            "import {C as X} from './m'; export function run(){X.sm();}",
        );
        write(
            d.path(),
            &format!("barrel.{grammar}"),
            "export * as M from './m';",
        );
        hit(&graph(d.path()), grammar, "sm", &format!("m.{grammar}"));
        // An opaque writer whose fully resolved static closure cannot expose C
        // must not revoke the separate m identity. Cycles terminate.
        write(
            d.path(),
            &format!("other.{grammar}"),
            "import Y from './opaque'; Y.sm=()=>1;",
        );
        write(
            d.path(),
            &format!("opaque.{grammar}"),
            r"export {\u0044 as default} from './leaf'; export * from './cycle';",
        );
        write(
            d.path(),
            &format!("leaf.{grammar}"),
            "export class D {static sm(){return 2;}}",
        );
        write(
            d.path(),
            &format!("cycle.{grammar}"),
            "export * from './opaque';",
        );
        hit(&graph(d.path()), grammar, "sm", &format!("m.{grammar}"));
    }
}

#[test]
fn repair_r3b_unresolved_package_without_workspace_targets_is_scoped() {
    for grammar in ["jsx", "tsx"] {
        for writer in [
            "import * as Y from '#unavailable'; Y.default.sm=()=>1;",
            "import Y from '#unavailable'; Y.sm=()=>1;",
        ] {
            let d = fixture(
                grammar,
                "export class C {static sm(){return 0;}}",
                "import {C as X} from './m'; export function run(){X.sm();}",
            );
            write(d.path(), &format!("other.{grammar}"), writer);
            let g = graph(d.path());
            hit(&g, grammar, "sm", &format!("m.{grammar}"));
        }
    }
}

#[test]
fn repair_r3_writer_project_and_absence_are_scoped() {
    for grammar in ["jsx", "tsx"] {
        let d = fixture(
            grammar,
            "export class C {static sm(){return 0;}}",
            "import {C as X} from './m'; export function run(){X.sm();}",
        );
        write(
            d.path(),
            &format!("leaf.{grammar}"),
            "export default class Api {static sm(){return 1;}}",
        );
        write(d.path(), "w/tsconfig.json", &serde_json::json!({"compilerOptions":{"allowJs":true,"moduleResolution":"node","baseUrl":".","paths":{"#leaf":["../leaf"]},"jsx":"preserve"},"include":["**/*"]}).to_string());
        write(
            d.path(),
            &format!("w/other.{grammar}"),
            "import * as Y from '#leaf'; Y.default.sm=()=>2;",
        );
        hit(&graph(d.path()), grammar, "sm", &format!("m.{grammar}"));
        write(
            d.path(),
            &format!("absent.{grammar}"),
            "export const unrelated=1;",
        );
        write(
            d.path(),
            &format!("other.{grammar}"),
            "import {missing as Z} from './absent'; Z.sm=()=>2;",
        );
        hit(&graph(d.path()), grammar, "sm", &format!("m.{grammar}"));
    }
}

#[test]
fn repair_r3_namespace_own_export_is_refused() {
    for grammar in ["jsx", "tsx"] {
        for forward in [
            "import * as N from './m'; export {N};",
            "import * as N from './m'; export default N;",
        ] {
            let d = fixture(
                grammar,
                "export default class Api {static sm(){return 0;}}",
                "import X from './m'; export function run(){X.sm();}",
            );
            write(d.path(), &format!("forward.{grammar}"), forward);
            let g = graph(d.path());
            assert_eq!(
                outcome(&g, &format!("app.{grammar}"), "sm"),
                outcome(&base(g.clone()), &format!("app.{grammar}"), "sm")
            );
        }
    }
}

#[test]
fn repair_r3b_resolved_non_qualifiers_and_missing_names_preserve_unrelated_gains() {
    for grammar in ["jsx", "tsx"] {
        for leaf in [
            "export const value=1;",
            "export function value(){}",
            "export const value=()=>{};",
        ] {
            for spec in ["./other", "@other"] {
                let d = fixture(
                    grammar,
                    "export class C {static sm(){return 0;}}",
                    "import {C as X} from './m'; export function run(){X.sm();}",
                );
                write(d.path(), &format!("leaf.{grammar}"), leaf);
                write(d.path(), &format!("other.{grammar}"), "export {value} from './leaf'; import Y from 'external-dependency'; consume(Y);");
                write(d.path(), "tsconfig.json", &serde_json::json!({"compilerOptions":{"allowJs":true,"checkJs":true,"moduleResolution":"node","baseUrl":".","paths":{"@other":["other"]},"jsx":"preserve"},"include":["**/*"]}).to_string());
                write(d.path(), &format!("writer.{grammar}"), &format!("import {{value as Y, missing as Z}} from '{spec}'; consume(Y); Z.sm=()=>1;"));
                hit(&graph(d.path()), grammar, "sm", &format!("m.{grammar}"));
            }
        }
    }
}

#[test]
fn repair_r3b_unavailable_relative_closure_is_scoped_and_fail_closed() {
    for grammar in ["jsx", "tsx"] {
        let d = fixture(
            grammar,
            "export class C {static sm(){return 0;}}",
            "import {C as X} from './m'; export function run(){X.sm();}",
        );
        write(
            d.path(),
            &format!("other.{grammar}"),
            "import Y from './missing'; Y.sm=()=>1;",
        );
        hit(&graph(d.path()), grammar, "sm", &format!("m.{grammar}"));
        write(
            d.path(),
            &format!("other.{grammar}"),
            "import Y from './broken'; Y.sm=()=>1;",
        );
        write(
            d.path(),
            &format!("broken.{grammar}"),
            r"export {\u0044 as default} from './leaf'; import E from './missing';",
        );
        write(
            d.path(),
            &format!("leaf.{grammar}"),
            "export class D {static sm(){return 2;}}",
        );
        hit(&graph(d.path()), grammar, "sm", &format!("m.{grammar}"));
        write(
            d.path(),
            &format!("broken.{grammar}"),
            r"export {\u0043 as default} from './m'; import E from './missing';",
        );
        let g = graph(d.path());
        assert_eq!(
            outcome(&g, &format!("app.{grammar}"), "sm"),
            outcome(&base(g.clone()), &format!("app.{grammar}"), "sm")
        );
    }
}

#[test]
fn repair_r3b_unresolved_bare_package_scopes_workspace_identities() {
    for grammar in ["jsx", "tsx"] {
        let d = fixture(
            grammar,
            "export class C {static sm(){return 0;}}",
            "import {C as X} from './m'; export function run(){X.sm();}",
        );
        write(d.path(), "package.json", r#"{"name":"in-repo-package"}"#);
        write(
            d.path(),
            &format!("other.{grammar}"),
            "import Y from 'external-dependency'; Y.sm=()=>1;",
        );
        hit(&graph(d.path()), grammar, "sm", &format!("m.{grammar}"));
        write(
            d.path(),
            &format!("other.{grammar}"),
            "import Y from 'in-repo-package'; Y.sm=()=>1;",
        );
        let g = graph(d.path());
        assert_eq!(
            outcome(&g, &format!("app.{grammar}"), "sm"),
            outcome(&base(g.clone()), &format!("app.{grammar}"), "sm")
        );
    }
}
