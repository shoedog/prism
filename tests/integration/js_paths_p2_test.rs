//! P2 controls: alias -> TS/JS barrel -> relative JS implementation.
use super::js_paths_common::*;
use prism::{call_graph::CallGraph, repo_loader::load_repo};
use tempfile::TempDir;

fn case(grammar: &str, hop: &str, leaf: &str, write_kind: &str) -> TempDir {
    let d = TempDir::new().unwrap();
    write(
        d.path(),
        "tsconfig.json",
        &config(serde_json::json!({"@lib":["barrel"]})),
    );
    write(
        d.path(),
        &format!("app.{grammar}"),
        "import { real as picked } from '@lib';\nexport function run() { picked(); }\n",
    );
    write(d.path(), &format!("barrel.{grammar}"), hop);
    write(
        d.path(),
        leaf,
        &format!("export function real() {{ return 1; }}\n{write_kind}\n"),
    );
    d
}

fn base(d: &TempDir) -> CallGraph {
    CallGraph::build(&load_repo(d.path()).unwrap().files)
}

#[test]
fn relative_js_hops_bind_in_both_grammars() {
    for grammar in ["jsx", "tsx"] {
        for leaf in ["lib/real.js", "lib/real.jsx", "lib/real/index.js"] {
            for hop in [
                "export { real } from './lib/real';",
                "export * from './lib/real';",
                "import { real } from './lib/real'; export { real };",
            ] {
                for member_write in [
                    "",
                    "real.displayName = 'Real';",
                    "real['displayName'] = 'Real';",
                ] {
                    let d = case(grammar, hop, leaf, member_write);
                    assert_hit(&graph(d.path()), &format!("app.{grammar}"), "picked", leaf);
                }
            }
        }
        // An intermediate JS barrel also needs the priority-pass proof.
        let d = case(
            grammar,
            "export * from './lib/middle';",
            "lib/real.jsx",
            "real.displayName = 'Real';",
        );
        write(
            d.path(),
            "lib/middle.js",
            "export { real } from './real.jsx';\n",
        );
        assert_hit(
            &graph(d.path()),
            &format!("app.{grammar}"),
            "picked",
            "lib/real.jsx",
        );
    }
}

#[test]
fn explicit_js_hops_bind_after_local_priority_pass() {
    for grammar in ["jsx", "tsx"] {
        for extension in ["js", "jsx", "mjs", "cjs"] {
            let leaf = format!("lib/real.{extension}");
            let d = case(
                grammar,
                &format!("export {{ real }} from './{leaf}';"),
                &leaf,
                "real.displayName = 'Real';",
            );
            assert_hit(&graph(d.path()), &format!("app.{grammar}"), "picked", &leaf);
            // Unknown suffixes still append the JS-family extensions.
            let d = case(
                grammar,
                "export { real } from './lib/real.service';",
                "lib/real.service.js",
                "",
            );
            assert_hit(
                &graph(d.path()),
                &format!("app.{grammar}"),
                "picked",
                "lib/real.service.js",
            );
            // Replacement candidates for each JS-family spelling, including declarations.
            for priority in match extension {
                "mjs" => vec!["mts", "d.mts"],
                "cjs" => vec!["cts", "d.cts"],
                _ => vec!["ts", "tsx", "d.ts"],
            } {
                let d = case(
                    grammar,
                    &format!("export {{ real }} from './{leaf}';"),
                    &leaf,
                    "",
                );
                write(
                    d.path(),
                    &format!("lib/real.{priority}"),
                    "export function real() { return 2; }\n",
                );
                assert_eq!(
                    outcome(&graph(d.path()), &format!("app.{grammar}"), "picked"),
                    outcome(&base(&d), &format!("app.{grammar}"), "picked")
                );
            }
        }
    }
}

#[test]
fn relative_js_priority_candidates_preserve_complete_base() {
    for grammar in ["jsx", "tsx"] {
        for spec in ["./lib/real", "./lib/real.js", "./lib/real.jsx"] {
            let leaf = if spec.ends_with(".jsx") {
                "lib/real.jsx"
            } else {
                "lib/real.js"
            };
            for candidate in [
                "lib/real.ts",
                "lib/real.tsx",
                "lib/real.d.ts",
                "lib/real/index.d.ts",
                "lib/real.js.ts",
            ] {
                // These implicit candidates belong to the explicit .js spelling.
                if candidate.contains("real.js") && !spec.ends_with(".js") {
                    continue;
                }
                let d = case(
                    grammar,
                    &format!("export {{ real }} from '{spec}';"),
                    leaf,
                    "real.displayName = 'Real';",
                );
                write(
                    d.path(),
                    candidate,
                    "export function real() { return 2; }\n",
                );
                // Directory lookup uses the full candidate, not its suffix-stripped stem.
                if candidate == "lib/real/index.d.ts" && spec != "./lib/real" {
                    assert_hit(&graph(d.path()), &format!("app.{grammar}"), "picked", leaf);
                    continue;
                }
                assert_eq!(
                    outcome(&graph(d.path()), &format!("app.{grammar}"), "picked"),
                    outcome(&base(&d), &format!("app.{grammar}"), "picked"),
                    "{grammar} {spec} {candidate}"
                );
            }
        }
        let d = case(grammar, "export * from './lib/middle';", "lib/real.js", "");
        write(
            d.path(),
            "lib/middle.js",
            "export { real } from './real';\n",
        );
        write(
            d.path(),
            "lib/middle.d.ts",
            "export declare function real(): number;\n",
        );
        assert_eq!(
            outcome(&graph(d.path()), &format!("app.{grammar}"), "picked"),
            outcome(&base(&d), &format!("app.{grammar}"), "picked")
        );
    }
}

#[test]
fn relative_js_guards_preserve_base() {
    for grammar in ["jsx", "tsx"] {
        for guard in [
            "allow-off",
            "binding-write",
            "star-hole",
            "nonrelative",
            "shadow",
            "package-field",
            "unknown-declaration",
            "forward-arrow",
        ] {
            let d = case(
                grammar,
                "export { real } from './lib/real';",
                "lib/real.js",
                "",
            );
            match guard {
                "allow-off" => write(d.path(), "tsconfig.json", r#"{"compilerOptions":{"moduleResolution":"node10","paths":{"@lib":["barrel"]}}}"#),
                "binding-write" => write(d.path(), "lib/real.js", "export function real() { return 1; }\nreal = () => 2;\n"),
                "star-hole" => write(d.path(), &format!("barrel.{grammar}"), "export * from './lib/real'; export * from './missing';\n"),
                "nonrelative" => write(d.path(), &format!("barrel.{grammar}"), "export { real } from '@leaf';\n"),
                "shadow" => write(d.path(), &format!("app.{grammar}"), "import { real as picked } from '@lib';\nexport function run(picked) { picked(); }\n"),
                "forward-arrow" => {
                    write(d.path(), &format!("barrel.{grammar}"), "import { real } from './lib/real'; export { real };\n");
                    write(d.path(), "lib/real.js", "export const real = () => 1;\n");
                }
                "package-field" => write(d.path(), "lib/real/package.json", r#"{"types":"../guard.d.ts"}"#),
                "unknown-declaration" => {
                    write(d.path(), &format!("barrel.{grammar}"), "export { real } from './lib/real.service';\n");
                    write(d.path(), "lib/real.service.js", "export function real() { return 1; }\n");
                    write(d.path(), "lib/real.d.service.ts", "export declare function real(): number;\n");
                }
                _ => unreachable!(),
            }
            assert_eq!(
                outcome(&graph(d.path()), &format!("app.{grammar}"), "picked"),
                outcome(&base(&d), &format!("app.{grammar}"), "picked"),
                "{grammar} {guard}"
            );
        }
    }
}

#[test]
fn relative_hops_do_not_search_nonrelative_type_roots() {
    for grammar in ["jsx", "tsx"] {
        let d = case(
            grammar,
            "export { real } from './lib/real.js';",
            "lib/real.js",
            "",
        );
        // This external ancestor would block a non-relative JS priority proof.
        // A TS entry barrel does not consult it, nor does the relative hop.
        std::fs::remove_file(d.path().join(format!("barrel.{grammar}"))).unwrap();
        write(
            d.path(),
            "barrel.ts",
            "export { real } from './lib/real.js';\n",
        );
        write(
            d.path(),
            "node_modules/@types/@lib/index.d.ts",
            "export declare function real(): number;\n",
        );
        write(
            d.path(),
            "types/@lib/index.d.ts",
            "export declare function real(): number;\n",
        );
        let mut cfg: serde_json::Value =
            serde_json::from_str(&config(serde_json::json!({"@lib":["barrel.ts"]}))).unwrap();
        cfg["compilerOptions"]["typeRoots"] = serde_json::json!(["types"]);
        write(d.path(), "tsconfig.json", &cfg.to_string());
        assert_hit(
            &graph(d.path()),
            &format!("app.{grammar}"),
            "picked",
            "lib/real.js",
        );
    }
}

#[test]
fn e5_member_writes_match_local_and_legacy_relative_routes() {
    for grammar in ["jsx", "tsx"] {
        for member in [
            "real.displayName = 'Real';",
            "real['displayName'] = 'Real';",
        ] {
            let d = case(
                grammar,
                "export { real } from './lib/real';",
                "lib/real.js",
                member,
            );
            write(d.path(), &format!("local.{grammar}"), &format!("function real() {{ return 1; }}\n{member}\nexport function run() {{ real(); }}\n"));
            write(d.path(), &format!("relative.{grammar}"), "import { real as picked } from './lib/real';\nexport function run() { picked(); }\n");
            write(d.path(), &format!("local-jsx.{grammar}"), &format!("function Real() {{ return null; }}\n{}\nexport function render() {{ return <Real />; }}\n", member.replace("real", "Real")));
            write(d.path(), &format!("relative-jsx.{grammar}"), "import { real as Real } from './lib/real';\nexport function render() { return <Real />; }\n");
            let g = base(&d);
            let local = outcome(&g, &format!("local.{grammar}"), "real");
            assert_eq!(
                local["resolved_targets"][0]["confidence"], "exact",
                "{local}"
            );
            assert_hit(&g, &format!("relative.{grammar}"), "picked", "lib/real.js");
            for file in [
                format!("local-jsx.{grammar}"),
                format!("relative-jsx.{grammar}"),
            ] {
                let row = outcome(&g, &file, "Real");
                assert_eq!(row["resolved_targets"][0]["confidence"], "exact", "{row}");
            }
        }
    }
}

#[test]
fn nonrelative_hops_reuse_p1_alias_proof() {
    for grammar in ["jsx", "tsx"] {
        for (first, second) in [
            ("export { real } from '@/x';", None),
            ("import { real } from '@/x'; export { real };", None),
            ("export * from '@/x';", None),
            (
                "export { real } from '@/middle';",
                Some("export { real } from '@/x';"),
            ),
            (
                "export { real } from './middle.js';",
                Some("export { real } from '@/x';"),
            ),
            (
                "export { real } from '@/middle';",
                Some("export { real } from './lib/real';"),
            ),
        ] {
            for member in [
                "",
                "real.displayName = 'Real';",
                "real['displayName'] = 'Real';",
            ] {
                let d = case(grammar, first, "lib/real.js", member);
                write(
                    d.path(),
                    "tsconfig.json",
                    &config(serde_json::json!({
                        "@lib":["barrel"], "@/x":["lib/real"], "@/middle":["middle"]
                    })),
                );
                if let Some(second) = second {
                    write(d.path(), "middle.js", second);
                }
                assert_hit(
                    &graph(d.path()),
                    &format!("app.{grammar}"),
                    "picked",
                    "lib/real.js",
                );
            }
        }
    }
}

#[test]
fn nonrelative_hop_refusals_preserve_base() {
    for grammar in ["jsx", "tsx"] {
        for guard in [
            "bare-package",
            "package-directory",
            "declaration",
            "competition",
            "ambient",
            "unsupported-extension",
            "binding-write",
            "forward-arrow",
        ] {
            let d = case(grammar, "export { real } from '@/x';", "lib/real.js", "");
            write(
                d.path(),
                "tsconfig.json",
                &config(serde_json::json!({"@lib":["barrel"],"@/x":["lib/real"]})),
            );
            match guard {
                "bare-package" => {
                    write(
                        d.path(),
                        &format!("barrel.{grammar}"),
                        "export { real } from 'pkg';",
                    );
                    write(
                        d.path(),
                        "node_modules/pkg/index.js",
                        "export function real() { return 1; }",
                    );
                }
                "package-directory" => write(
                    d.path(),
                    "lib/real/package.json",
                    r#"{"main":"../real.js"}"#,
                ),
                "declaration" => write(
                    d.path(),
                    "lib/real.d.ts",
                    "export declare function real(): number;",
                ),
                "competition" => write(
                    d.path(),
                    "lib/real/index.js",
                    "export function real() { return 2; }",
                ),
                "ambient" => write(
                    d.path(),
                    "ambient.d.ts",
                    "declare module '@/x' { export function real(): number; }",
                ),
                "unsupported-extension" => write(
                    d.path(),
                    "tsconfig.json",
                    &config(serde_json::json!({"@lib":["barrel"],"@/x":["lib/real.js"]})),
                ),
                "binding-write" => write(
                    d.path(),
                    "lib/real.js",
                    "export function real() { return 1; }\nreal = () => 2;",
                ),
                "forward-arrow" => {
                    write(
                        d.path(),
                        &format!("barrel.{grammar}"),
                        "import { real } from '@/x'; export { real };",
                    );
                    write(d.path(), "lib/real.js", "export const real = () => 1;");
                }
                _ => unreachable!(),
            }
            assert_eq!(
                outcome(&graph(d.path()), &format!("app.{grammar}"), "picked"),
                outcome(&base(&d), &format!("app.{grammar}"), "picked"),
                "{grammar} {guard}"
            );
        }
    }
}

#[test]
fn hop_options_belong_to_caller_program() {
    for grammar in ["jsx", "tsx"] {
        let d = TempDir::new().unwrap();
        for project in ["a", "b"] {
            write(
                d.path(),
                &format!("{project}/tsconfig.json"),
                &config(serde_json::json!({"@lib":["../shared/barrel.ts"],"@/x":["real"]})),
            );
            write(
                d.path(),
                &format!("{project}/app.{grammar}"),
                "import { real as picked } from '@lib';\nexport function run() { picked(); }",
            );
            write(
                d.path(),
                &format!("{project}/real.js"),
                "export function real() { return 1; }\nreal.displayName = 'Real';",
            );
        }
        // The barrel's nearer config intentionally disagrees with both callers.
        write(
            d.path(),
            "shared/tsconfig.json",
            &config(serde_json::json!({"@/x":["decoy"]})),
        );
        write(
            d.path(),
            "shared/decoy.js",
            "export function real() { return 2; }",
        );
        write(d.path(), "shared/barrel.ts", "export { real } from '@/x';");
        let g = graph(d.path());
        for project in ["a", "b"] {
            assert_hit(
                &g,
                &format!("{project}/app.{grammar}"),
                "picked",
                &format!("{project}/real.js"),
            );
        }
    }
}

#[test]
fn alias_hops_retain_ancestor_priority_pass() {
    for grammar in ["jsx", "tsx"] {
        let d = case(grammar, "export { real } from '@/x';", "lib/real.js", "");
        std::fs::remove_file(d.path().join(format!("barrel.{grammar}"))).unwrap();
        write(d.path(), "barrel.ts", "export { real } from '@/x';");
        write(
            d.path(),
            "tsconfig.json",
            &config(serde_json::json!({"@lib":["barrel.ts"],"@/x":["lib/real"]})),
        );
        assert_hit(
            &graph(d.path()),
            &format!("app.{grammar}"),
            "picked",
            "lib/real.js",
        );
        write(
            d.path(),
            "node_modules/@types/__x/index.d.ts",
            "export declare function real(): number;",
        );
        assert_eq!(
            outcome(&graph(d.path()), &format!("app.{grammar}"), "picked"),
            outcome(&base(&d), &format!("app.{grammar}"), "picked")
        );
    }
}

#[test]
fn legacy_star_nonrelative_forwards_preserve_complete_main_rows() {
    // Frozen complete c50de85a rows, from same-environment A1/A4 controls.
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/js_paths_p2_legacy_refusal.json")).unwrap();
    let mut mismatches = Vec::new();
    for grammar in ["jsx", "tsx"] {
        for module in ["pkg", "@k"] {
            for branches in [["a", "b"], ["b", "a"]] {
                let d = TempDir::new().unwrap();
                write(
                    d.path(),
                    &format!("a.{grammar}"),
                    &format!("import {{K}} from '{module}'; export {{K}};\n"),
                );
                write(
                    d.path(),
                    &format!("b.{grammar}"),
                    "export function K(){ return null; }\n",
                );
                write(
                    d.path(),
                    &format!("barrel.{grammar}"),
                    &branches
                        .map(|b| format!("export * from './{b}';\n"))
                        .concat(),
                );
                write(
                    d.path(),
                    &format!("app.{grammar}"),
                    "import {K} from './barrel';\nexport function run() { K(); return <K />; }\n",
                );
                if module == "pkg" {
                    write(
                        d.path(),
                        "node_modules/pkg/index.d.ts",
                        "export declare function K(): unknown;\n",
                    );
                    write(
                        d.path(),
                        "node_modules/pkg/index.js",
                        "export function K(){ return null; }\n",
                    );
                } else {
                    write(
                        d.path(),
                        "tsconfig.json",
                        &config(serde_json::json!({"@k":["k2"]})),
                    );
                    write(
                        d.path(),
                        &format!("k2.{grammar}"),
                        "export function K(){ return null; }\n",
                    );
                }
                for layout in ["A1/A4", "K1", "K2"] {
                    let stars = branches
                        .map(|b| format!("export * from './{b}';\n"))
                        .concat();
                    let barrel = match layout {
                        "K1" => {
                            write(
                                d.path(),
                                &format!("mid.{grammar}"),
                                "export * from './a';\n",
                            );
                            stars.replace("'./a'", "'./mid'")
                        }
                        "K2" => {
                            write(d.path(), &format!("inner.{grammar}"), &stars);
                            "export {K} from './inner';\n".to_owned()
                        }
                        _ => stars,
                    };
                    write(d.path(), &format!("barrel.{grammar}"), &barrel);
                    let rows: Vec<_> = prism::navigation::queries::call_site_dump(&graph(d.path()))
                        .into_iter()
                        .filter(|r| r["caller"]["file"] == format!("app.{grammar}"))
                        .collect();
                    let rows = serde_json::json!(rows);
                    if rows != expected[grammar] {
                        mismatches.push((grammar, module, branches, layout, rows));
                    }
                }
            }
        }
    }
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn legacy_star_other_unresolved_claims_keep_existing_behavior() {
    // W1 is specific to non-relative imported-local forwards. Do not expand
    // this repair into the inherited unresolved ReExport/relative treatment.
    for grammar in ["jsx", "tsx"] {
        for first in [
            "import {real} from './missing'; export {real};",
            "export {real} from 'pkg';",
        ] {
            let d = case(
                grammar,
                "export * from './a'; export * from './lib/real';",
                "lib/real.js",
                "",
            );
            write(d.path(), &format!("a.{grammar}"), first);
            // Use the legacy relative consumer, independent of paths projection.
            write(
                d.path(),
                &format!("app.{grammar}"),
                "import {real as picked} from './barrel';\nexport function run(){picked();}\n",
            );
            assert_hit(
                &graph(d.path()),
                &format!("app.{grammar}"),
                "picked",
                "lib/real.js",
            );
        }
    }
}
