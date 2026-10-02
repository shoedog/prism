use super::js_paths_common::*;
use prism::{call_graph::CallGraph, repo_loader::load_repo};
use std::path::Path;
use tempfile::TempDir;

fn setup(root: &Path, ext: &str, implementation: &str, options: serde_json::Value) {
    let mut cfg: serde_json::Value =
        serde_json::from_str(&config(serde_json::json!({"utils/format":["src/util"]}))).unwrap();
    cfg["compilerOptions"]["noEmit"] = true.into();
    cfg["compilerOptions"]["jsx"] = "preserve".into();
    for (k, v) in options.as_object().unwrap() {
        cfg["compilerOptions"][k] = v.clone();
    }
    write(root, "tsconfig.json", &cfg.to_string());
    write(
        root,
        &format!("app.{ext}"),
        "import {real as picked} from 'utils/format'; export function run(){picked();}",
    );
    write(
        root,
        &format!("src/util.{implementation}"),
        "export function real(){return 1;}",
    );
}
fn expect(root: &Path, ext: &str, implementation: &str, exact: bool) {
    let app = format!("app.{ext}");
    let g = graph(root);
    if exact {
        assert_hit(&g, &app, "picked", &format!("src/util.{implementation}"));
    } else {
        let r = load_repo(root).unwrap();
        assert_eq!(
            outcome(&g, &app, "picked"),
            outcome(&CallGraph::build(&r.files), &app, "picked")
        );
    }
}

#[test]
fn ancestor_automatic_named_and_referenced_types_keep_base() {
    for ext in ["jsx", "tsx"] {
        for arm in ["implicit", "types", "reference", "roots", "empty", "absent"] {
            let d = TempDir::new().unwrap();
            let root = d.path().join("project");
            let options = match arm {
                "types" => serde_json::json!({"types":["custom"]}),
                "roots" => serde_json::json!({"typeRoots":["../node_modules/@types"]}),
                "empty" | "reference" => serde_json::json!({"types":[]}),
                _ => serde_json::json!({}),
            };
            setup(&root, ext, "tsx", options);
            if arm != "absent" {
                write(
                    d.path(),
                    "node_modules/@types/custom/index.d.ts",
                    "declare module 'utils/format' { export function real(): number; }",
                );
            }
            if arm == "reference" {
                write(
                    &root,
                    "src/ref.ts",
                    "/// <reference types='custom' />\nexport {};",
                );
            }
            expect(&root, ext, "tsx", matches!(arm, "empty" | "absent"));
        }
    }
}

#[test]
fn uppercase_ambient_reference_files_and_type_package_keep_base() {
    for ext in ["jsx", "tsx"] {
        for arm in ["direct", "reference", "files", "types"] {
            for implementation in ["jsx", "tsx"] {
                let d = TempDir::new().unwrap();
                setup(d.path(), ext, implementation, serde_json::json!({}));
                let p = if arm == "types" {
                    "node_modules/@types/custom/index.D.TS"
                } else {
                    "types/ambient.D.TS"
                };
                write(
                    d.path(),
                    p,
                    "declare\u{feff}module 'utils/format' { export function real(): number; }",
                );
                let mut cfg: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(d.path().join("tsconfig.json")).unwrap())
                        .unwrap();
                if arm == "types" {
                    cfg["compilerOptions"]["types"] = serde_json::json!(["custom"]);
                }
                if arm == "files" {
                    cfg["files"] = serde_json::json!([format!("app.{ext}"), "types/ambient.d.ts"]);
                }
                if arm == "reference" {
                    write(
                        d.path(),
                        "src/ref.ts",
                        "/// <reference path='../types/ambient.d.ts' />\nexport {};",
                    );
                }
                write(d.path(), "tsconfig.json", &cfg.to_string());
                expect(d.path(), ext, implementation, false);
                std::fs::remove_file(d.path().join(p)).unwrap();
                expect(d.path(), ext, implementation, true);
            }
        }
    }
}

#[test]
fn declaration_module_dependencies_outside_root_keep_base() {
    for ext in ["jsx", "tsx"] {
        for implementation in ["jsx", "tsx"] {
            for body in [
                "import '../../../outside/global'; export interface Empty {}",
                "export * from '../../../outside/global';",
                "export type Empty = import('../../../outside/global').Value;",
                "import Empty = require('../../../outside/global');",
                "/// <reference path='../../../outside/global.d.ts' />\nexport interface Empty {}",
            ] {
                let d = TempDir::new().unwrap();
                let root = d.path().join("project");
                setup(
                    &root,
                    ext,
                    implementation,
                    serde_json::json!({"typeRoots":["types"]}),
                );
                write(
                    d.path(),
                    "outside/global.d.ts",
                    "declare module 'utils/format' { export function real(): number; }",
                );
                write(&root, "types/local/index.d.ts", body);
                expect(&root, ext, implementation, false);
                write(&root, "types/local/index.d.ts", "export interface Empty {}");
                expect(&root, ext, implementation, true);
                write(
                    &root,
                    "types/local/index.d.ts",
                    "import '../global'; export interface Empty {}",
                );
                write(&root, "types/global.d.ts", "export interface Value {}");
                expect(&root, ext, implementation, true);
            }
        }
    }
}

#[test]
fn absolute_and_external_symlink_module_dependencies_keep_base() {
    for ext in ["jsx", "tsx"] {
        let d = TempDir::new().unwrap();
        let root = d.path().join("project");
        setup(&root, ext, "tsx", serde_json::json!({"types":[]}));
        write(
            d.path(),
            "outside/global.d.ts",
            "declare module 'utils/format' {}",
        );
        write(
            &root,
            "types/local.d.ts",
            &format!(
                "import {:?}; export interface Empty {{}}",
                d.path().join("outside/global").to_str().unwrap()
            ),
        );
        expect(&root, ext, "tsx", false);
        std::fs::remove_file(root.join("types/local.d.ts")).unwrap();
        std::os::unix::fs::symlink(d.path().join("outside"), root.join("types/link")).unwrap();
        expect(&root, ext, "tsx", false);
    }
}

#[test]
fn installed_bin_and_internal_workspace_links_recover() {
    for ext in ["jsx", "tsx"] {
        let d = TempDir::new().unwrap();
        setup(d.path(), ext, "tsx", serde_json::json!({}));
        write(d.path(), "node_modules/.bin/.keep", "");
        std::os::unix::fs::symlink("../missing/bin/tsc", d.path().join("node_modules/.bin/tsc"))
            .unwrap();
        write(
            d.path(),
            ".git/opaque.d.ts",
            "declare module 'utils/format' {}",
        );
        std::os::unix::fs::symlink(".git", d.path().join("git-alias")).unwrap();
        write(
            d.path(),
            "packages/workspace/index.d.ts",
            "export interface Empty {}",
        );
        std::os::unix::fs::symlink(
            "../packages/workspace",
            d.path().join("node_modules/workspace"),
        )
        .unwrap();
        std::os::unix::fs::symlink(".", d.path().join("packages/workspace/cycle")).unwrap();
        expect(d.path(), ext, "tsx", true);
        write(
            d.path(),
            "packages/workspace/index.d.ts",
            "declare module 'utils/format' { export function real(): number; }",
        );
        expect(d.path(), ext, "tsx", false);
    }
}

#[test]
fn large_installed_source_tree_under_byte_budget_recovers() {
    for ext in ["jsx", "tsx"] {
        let d = TempDir::new().unwrap();
        setup(d.path(), ext, "tsx", serde_json::json!({}));
        let text = format!(
            "/*{}*/\nexport interface Empty {{}}",
            "x".repeat(17 * 1024 * 1024)
        );
        write(d.path(), "node_modules/large/index.d.ts", &text);
        expect(d.path(), ext, "tsx", true);
    }
}

#[test]
fn case_variant_candidate_priority_and_m65_row_keep_base() {
    for ext in ["jsx", "tsx"] {
        for implementation in ["jsx", "tsx"] {
            for competitor in ["src/UTIL.TS", "src/Util.ts"] {
                let d = TempDir::new().unwrap();
                setup(d.path(), ext, implementation, serde_json::json!({}));
                write(d.path(), competitor, "export function real(){return 2;}");
                // This host's case-insensitive occupancy is the reviewer witness.
                if d.path().join("src/util.ts").exists() {
                    expect(d.path(), ext, implementation, false);
                    std::fs::remove_file(d.path().join(competitor)).unwrap();
                    expect(d.path(), ext, implementation, true);
                }
            }
        }
    }
}

#[test]
fn absent_scheme_module_is_not_an_absolute_boundary() {
    for ext in ["jsx", "tsx"] {
        let d = TempDir::new().unwrap();
        setup(d.path(), ext, "tsx", serde_json::json!({"types":[]}));
        for name in ["bun:test", "node:fs", "test:missing"] {
            write(
                d.path(),
                "types/local.d.ts",
                &format!("import {name:?}; export interface Empty {{}}"),
            );
            expect(d.path(), ext, "tsx", true);
        }
        for name in [
            "/outside/global",
            "C:/outside/global",
            "C:\\outside\\global",
        ] {
            write(
                d.path(),
                "types/local.d.ts",
                &format!("import {name:?}; export interface Empty {{}}"),
            );
            expect(d.path(), ext, "tsx", false);
        }
    }
}

#[test]
fn declaration_import_package_redirects_cannot_leave_root() {
    for ext in ["jsx", "tsx"] {
        for (name, field) in [
            ("./local", "types"),
            ("pkg/sub", "types"),
            ("@scope/pkg/sub", "types"),
            (".", "types"),
            ("./local", "main"),
            ("pkg", "main"),
            ("@scope/pkg/sub", "main"),
            (".", "main"),
        ] {
            let d = TempDir::new().unwrap();
            let root = d.path().join("project");
            setup(&root, ext, "tsx", serde_json::json!({"types":[]}));
            write(
                &root,
                if name == "." {
                    "input.d.ts"
                } else {
                    "types/input.d.ts"
                },
                &format!("import {name:?}; export interface Empty {{}}"),
            );
            let package = match name {
                "./local" => "types/local",
                "." => "",
                "pkg" | "pkg/sub" => "node_modules/pkg",
                _ => "node_modules/@scope/pkg",
            };
            let escapes = if name == "." {
                "../outside/global.d.ts"
            } else if name == "@scope/pkg/sub" {
                "../../../../outside/global.d.ts"
            } else {
                "../../../outside/global.d.ts"
            };
            write(
                &root,
                format!("{package}/package.json").trim_start_matches('/'),
                &serde_json::json!({field:escapes}).to_string(),
            );
            write(
                d.path(),
                "outside/global.d.ts",
                "declare module 'utils/format' { export function real(): number; }",
            );
            expect(&root, ext, "tsx", false);
            write(
                &root,
                format!("{package}/package.json").trim_start_matches('/'),
                &serde_json::json!({field:"index.d.ts"}).to_string(),
            );
            write(
                &root,
                format!("{package}/index.d.ts").trim_start_matches('/'),
                "export interface Empty {}",
            );
            expect(&root, ext, "tsx", true);
            if root
                .join(format!("{package}/PACKAGE.JSON").trim_start_matches('/'))
                .exists()
            {
                std::fs::rename(
                    root.join(format!("{package}/package.json").trim_start_matches('/')),
                    root.join(format!("{package}/PACKAGE.JSON").trim_start_matches('/')),
                )
                .unwrap();
                expect(&root, ext, "tsx", false);
            }
        }
    }
}

#[test]
fn declaration_js_spelling_does_not_load_reference_strings() {
    for ext in ["jsx", "tsx"] {
        let d = TempDir::new().unwrap();
        setup(
            d.path(),
            ext,
            "tsx",
            serde_json::json!({"types":["./types/server.d.ts"]}),
        );
        let config = d.path().join("tsconfig.json");
        let mut cfg: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&config).unwrap()).unwrap();
        cfg["include"] = serde_json::json!([format!("app.{ext}"), "src"]);
        std::fs::write(config, cfg.to_string()).unwrap();
        write(
            d.path(),
            "types/server.d.ts",
            "export * from './implementation.js';",
        );
        write(
            d.path(),
            "types/implementation.d.ts",
            "export interface Empty {}",
        );
        write(
            d.path(),
            "types/implementation.js",
            "const text = '/// <reference path=\"/outside/global.d.ts\" />';",
        );
        expect(d.path(), ext, "tsx", true);
        // Including the JS file still does not turn its string into a directive.
        cfg["include"] = serde_json::json!(["**/*"]);
        write(d.path(), "tsconfig.json", &cfg.to_string());
        expect(d.path(), ext, "tsx", true);
        write(
            d.path(),
            "types/implementation.d.ts",
            "/// <reference path='/outside/global.d.ts' />\nexport interface Empty {}",
        );
        expect(d.path(), ext, "tsx", false);
    }
}

#[test]
fn installed_declaration_documentation_does_not_create_wildcard_ambient() {
    for ext in ["jsx", "tsx"] {
        let d = TempDir::new().unwrap();
        setup(d.path(), ext, "tsx", serde_json::json!({"types":[]}));
        for source in [
            "/** Includes `declare namespace`, `declare module` and `declare global`. */\nexport declare function isAmbientModuleBlock(): boolean;",
            "/* declare module 'utils/format' {} */\nexport interface Empty {}",
            "const text=\"declare module 'utils/format' {}\";",
            "const text=`declare module 'utils/format' {}`;",
            "const text=/declare module 'utils/format'/;",
        ] {
            write(d.path(), "node_modules/pkg/input.d.ts", source);
            expect(d.path(), ext, "tsx", true);
        }
        write(
            d.path(),
            "node_modules/pkg/input.d.ts",
            "declare module 'utils/format' { export function real(): number; }",
        );
        expect(d.path(), ext, "tsx", false);
    }
}

#[test]
fn first_pass_physical_unicode_alias_keeps_base() {
    for ext in ["jsx", "tsx"] {
        let d = TempDir::new().unwrap();
        setup(d.path(), ext, "jsx", serde_json::json!({"types":[]}));
        let config = d.path().join("tsconfig.json");
        let mut cfg: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&config).unwrap()).unwrap();
        cfg["compilerOptions"]["paths"] = serde_json::json!({"café":["src/util"]});
        std::fs::write(config, cfg.to_string()).unwrap();
        write(
            d.path(),
            &format!("app.{ext}"),
            "import {real as picked} from 'café'; export function run(){picked();}",
        );
        write(
            d.path(),
            "node_modules/cafe\u{301}.d.ts",
            "export declare function real(): number;",
        );
        if d.path().join("node_modules/café.d.ts").exists() {
            expect(d.path(), ext, "jsx", false);
            std::fs::remove_file(d.path().join("node_modules/cafe\u{301}.d.ts")).unwrap();
            expect(d.path(), ext, "jsx", true);
        }
    }
}
