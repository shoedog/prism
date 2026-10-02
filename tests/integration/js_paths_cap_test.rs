use super::js_paths_common::*;
use prism::call_graph::CallGraph;
use prism::repo_loader::load_repo;
use tempfile::TempDir;

fn reviewer_case(index: usize) {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/js_paths_cap.json")).unwrap();
    let case = &cases[index];
    let d = TempDir::new().unwrap();
    for (path, text) in case["payload"].as_object().unwrap() {
        write(d.path(), path, text.as_str().unwrap());
    }
    let loaded = load_repo(d.path()).unwrap();
    let app = case["app"].as_str().unwrap();
    let callee = if case["case"].as_str().unwrap().starts_with('N') {
        "real"
    } else {
        "picked"
    };
    if [10, 11, 24, 25].contains(&index) {
        let target = if index < 24 {
            "lib/utils.jsx"
        } else {
            "src/util.jsx"
        };
        assert_hit(&graph(d.path()), app, callee, target);
        return;
    }
    assert_eq!(
        outcome(&graph(d.path()), app, callee),
        outcome(&CallGraph::build(&loaded.files), app, callee),
        "{}",
        case["case"]
    );
}

#[test]
fn reviewer_n1_ambient_tripleslash_jsx() {
    reviewer_case(0);
}

#[test]
fn reviewer_n1_ambient_tripleslash_tsx() {
    reviewer_case(1);
}

#[test]
fn reviewer_n2_ambient_typeroots_jsx() {
    reviewer_case(2);
}

#[test]
fn reviewer_n2_ambient_typeroots_tsx() {
    reviewer_case(3);
}

#[test]
fn reviewer_n4_nm_file_form_jsx() {
    reviewer_case(4);
}

#[test]
fn reviewer_n4_nm_file_form_tsx() {
    reviewer_case(5);
}

#[test]
fn reviewer_n5_typeroots_file_jsx() {
    reviewer_case(6);
}

#[test]
fn reviewer_n5_typeroots_file_tsx() {
    reviewer_case(7);
}

#[test]
fn reviewer_n5b_typeroots_dir_jsx() {
    reviewer_case(8);
}

#[test]
fn reviewer_n5b_typeroots_dir_tsx() {
    reviewer_case(9);
}

#[test]
fn reviewer_n5c_control_no_typing_jsx() {
    reviewer_case(10);
}

#[test]
fn reviewer_n5c_control_no_typing_tsx() {
    reviewer_case(11);
}

#[test]
fn reviewer_n7_ambient_in_vendor_dir_jsx() {
    reviewer_case(12);
}

#[test]
fn reviewer_n7_ambient_in_vendor_dir_tsx() {
    reviewer_case(13);
}

#[test]
fn reviewer_n7b_ambient_in_build_dir_jsx() {
    reviewer_case(14);
}

#[test]
fn reviewer_n7b_ambient_in_build_dir_tsx() {
    reviewer_case(15);
}

#[test]
fn reviewer_ambient_bom_declare_jsx() {
    reviewer_case(16);
}

#[test]
fn reviewer_ambient_bom_declare_tsx() {
    reviewer_case(17);
}

#[test]
fn reviewer_ambient_bom_module_jsx() {
    reviewer_case(18);
}

#[test]
fn reviewer_ambient_bom_module_tsx() {
    reviewer_case(19);
}

#[test]
fn reviewer_ambient_name_unicode_jsx() {
    reviewer_case(20);
}

#[test]
fn reviewer_ambient_name_unicode_tsx() {
    reviewer_case(21);
}

#[test]
fn reviewer_ambient_unicode_jsx() {
    reviewer_case(22);
}

#[test]
fn reviewer_ambient_unicode_tsx() {
    reviewer_case(23);
}

#[test]
fn reviewer_typeroots_absent_jsx() {
    reviewer_case(24);
}

#[test]
fn reviewer_typeroots_absent_tsx() {
    reviewer_case(25);
}

#[test]
fn reviewer_typeroots_jsx() {
    reviewer_case(26);
}

#[test]
fn reviewer_typeroots_tsx() {
    reviewer_case(27);
}

fn ts_fixture(ext: &str) -> TempDir {
    let d = TempDir::new().unwrap();
    write(
        d.path(),
        "tsconfig.json",
        &config(serde_json::json!({"@lib":["lib/real.tsx"]})),
    );
    write(
        d.path(),
        &format!("app.{ext}"),
        "import { real as picked } from '@lib'; export function run() { picked(); }",
    );
    write(
        d.path(),
        "lib/real.tsx",
        "export function real() { return 1; }",
    );
    assert_hit(
        &graph(d.path()),
        &format!("app.{ext}"),
        "picked",
        "lib/real.tsx",
    );
    d
}
fn base_row(root: &std::path::Path, app: &str) {
    let loaded = load_repo(root).unwrap();
    assert_eq!(
        outcome(&graph(root), app, "picked"),
        outcome(&CallGraph::build(&loaded.files), app, "picked")
    );
}
#[test]
fn structural_js_module_and_hop_terminals() {
    for ext in ["jsx", "tsx"] {
        for js in ["js", "jsx", "mjs", "cjs"] {
            for form in ["direct", "named", "star", "forward"] {
                let d = ts_fixture(ext);
                let target = if form == "direct" { "leaf" } else { "barrel" };
                write(
                    d.path(),
                    "tsconfig.json",
                    &config(serde_json::json!({"@lib":[format!("lib/{target}")]})),
                );
                write(
                    d.path(),
                    &format!("lib/leaf.{js}"),
                    "export function real() { return 1; }",
                );
                if form != "direct" {
                    write(
                        d.path(),
                        "lib/barrel.tsx",
                        match form {
                            "named" => "export { real } from './leaf';",
                            "star" => "export * from './leaf';",
                            _ => "import { real } from './leaf'; export { real };",
                        },
                    );
                }
                if form == "direct" && ["js", "jsx"].contains(&js) {
                    assert_hit(
                        &graph(d.path()),
                        &format!("app.{ext}"),
                        "picked",
                        &format!("lib/leaf.{js}"),
                    );
                } else {
                    base_row(d.path(), &format!("app.{ext}"));
                }
            }
        }
    }
}
#[test]
fn structural_ambient_everywhere() {
    for ext in ["jsx", "tsx"] {
        for folder in [
            "vendor",
            "build",
            "dist",
            "target",
            ".hidden",
            "node_modules/@types/legacy",
        ] {
            for suffix in ["ts", "d.ts", "d.mts", "d.cts"] {
                let d = ts_fixture(ext);
                write(
                    d.path(),
                    &format!("{folder}/ambient.{suffix}"),
                    "declare module '@l*' { export function real(): number; }",
                );
                // Outside root membership still declines the alias, by disposition.
                write(d.path(), "tsconfig.json", &serde_json::json!({"compilerOptions":{"moduleResolution":"node","allowJs":true,"paths":{"@lib":["lib/real.tsx"]}},"files":[format!("app.{ext}")],"include":[]}).to_string());
                base_row(d.path(), &format!("app.{ext}"));
            }
        }
    }
}
#[test]
fn structural_ambient_lexical_boundaries() {
    for ext in ["jsx", "tsx"] {
        for whitespace in [
            '\u{feff}', '\u{a0}', '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}',
            '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200a}',
            '\u{202f}', '\u{205f}', '\u{3000}',
        ] {
            for slot in [0, 1] {
                let d = ts_fixture(ext);
                let text = if slot == 0 {
                    format!("declare{whitespace}module '@lib' {{}}")
                } else {
                    format!("declare module{whitespace}'@lib' {{}}")
                };
                write(d.path(), "types/ambient.d.ts", &text);
                base_row(d.path(), &format!("app.{ext}"));
            }
        }
        for text in [
            "declare/*x*/module/*y*/'@lib' {}",
            "declare\\\nmodule\\\r\n'@lib' {}",
            "declare module '\\u0040\\x6cib' {}",
            "declare module '\\u{40}lib' {}",
            "declare module '@l\\\nib' {}",
            "declare module",
            "declare module '\\uQQQQ' {}",
            "declare module Unparsed {}",
            "declare module '@lib",
        ] {
            let d = ts_fixture(ext);
            write(d.path(), "types/ambient.d.ts", text);
            base_row(d.path(), &format!("app.{ext}"));
        }
        let d = ts_fixture(ext);
        write(
            d.path(),
            "types/ambient.d.ts",
            "undeclare module '@lib'; declare moduleFoo '@lib'; declare module '@other' {}",
        );
        assert_hit(
            &graph(d.path()),
            &format!("app.{ext}"),
            "picked",
            "lib/real.tsx",
        );
    }
}
#[test]
fn structural_config_types_and_roots() {
    for ext in ["jsx", "tsx"] {
        for key in ["types", "typeRoots"] {
            for inherited in [false, true] {
                let d = ts_fixture(ext);
                let mut cfg: serde_json::Value =
                    serde_json::from_str(&config(serde_json::json!({"@lib":["lib/real.tsx"]})))
                        .unwrap();
                cfg["compilerOptions"][key] = serde_json::json!([]);
                if inherited {
                    write(d.path(), "tsconfig.parent.json", &cfg.to_string());
                    write(d.path(), "tsconfig.json", &serde_json::json!({"extends":"./tsconfig.parent.json","compilerOptions":{key:[]}}).to_string());
                } else {
                    write(d.path(), "tsconfig.json", &cfg.to_string());
                }
                assert_hit(
                    &graph(d.path()),
                    &format!("app.{ext}"),
                    "picked",
                    "lib/real.tsx",
                );
            }
        }
    }
}
#[test]
fn structural_project_triple_references() {
    for ext in ["jsx", "tsx"] {
        for key in ["path", "types"] {
            for location in ["app", "sibling"] {
                let d = ts_fixture(ext);
                let path = if location == "app" {
                    format!("app.{ext}")
                } else {
                    "lib/reference.ts".into()
                };
                let original = std::fs::read_to_string(d.path().join(&path)).unwrap_or_default();
                let outside = if location == "app" {
                    "../outside.d.ts"
                } else {
                    "../../outside.d.ts"
                };
                write(
                    d.path(),
                    &path,
                    &format!("/// <reference {key}=\"{outside}\" />\n{original}"),
                );
                base_row(d.path(), &format!("app.{ext}"));
            }
        }
        let d = ts_fixture(ext);
        write(
            d.path(),
            "excluded/reference.ts",
            "/// <reference path=\"../../outside.d.ts\" />",
        );
        write(d.path(), "tsconfig.json", &serde_json::json!({"compilerOptions":{"moduleResolution":"node","allowJs":true,"paths":{"@lib":["lib/real.tsx"]}},"include":["lib",format!("app.{ext}")]}).to_string());
        assert_hit(
            &graph(d.path()),
            &format!("app.{ext}"),
            "picked",
            "lib/real.tsx",
        );
    }
}
#[test]
fn structural_scan_dependency_add_remove_edit() {
    for ext in ["jsx", "tsx"] {
        for folder in ["vendor", ".hidden", "node_modules/@types/legacy"] {
            let d = ts_fixture(ext);
            let path = format!("{folder}/ambient.d.ts");
            let before = load_repo(d.path()).unwrap().manifest_hashes;
            write(d.path(), &path, "export interface Empty {}");
            let occupied = load_repo(d.path()).unwrap().manifest_hashes;
            assert_ne!(before, occupied);
            write(
                d.path(),
                &path,
                "declare module '@lib' { export function real(): number; }",
            );
            let bearing = load_repo(d.path()).unwrap().manifest_hashes;
            assert_ne!(occupied, bearing);
            base_row(d.path(), &format!("app.{ext}"));
            write(
                d.path(),
                &path,
                "declare module '@lib' { export function real(): string; }",
            );
            assert_ne!(bearing, load_repo(d.path()).unwrap().manifest_hashes);
            write(d.path(), &path, "declare module '@other' {}");
            assert_hit(
                &graph(d.path()),
                &format!("app.{ext}"),
                "picked",
                "lib/real.tsx",
            );
            std::fs::remove_file(d.path().join(&path)).unwrap();
            // Topology for the preexisting opaque directory differs from absence;
            // compare after retaining that directory on both sides.
            let removed = load_repo(d.path()).unwrap().manifest_hashes;
            write(d.path(), &path, "export interface Empty {}");
            assert_ne!(removed, load_repo(d.path()).unwrap().manifest_hashes);
            std::fs::remove_file(d.path().join(&path)).unwrap();
            assert_eq!(removed, load_repo(d.path()).unwrap().manifest_hashes);
        }
    }
}

#[test]
fn structural_opaque_ts_hop() {
    for ext in ["jsx", "tsx"] {
        let d = ts_fixture(ext);
        write(
            d.path(),
            "tsconfig.json",
            &config(serde_json::json!({"@lib":["lib/barrel.tsx"]})),
        );
        write(
            d.path(),
            "lib/barrel.tsx",
            "export * from './real'; export * from './opaque';",
        );
        // A TS-family opaque branch bypasses neither the terminal cut nor the
        // existing skipped-star provenance guard. This isolates I08 from JS refusal.
        write(
            d.path(),
            "lib/opaque.tsx",
            "module.exports = buildExports();",
        );
        base_row(d.path(), &format!("app.{ext}"));
    }
}
