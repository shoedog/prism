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
fn ancestor_automatic_types_do_not_decline_but_explicit_outside_inputs_do() {
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
                    if arm == "implicit" {
                        "export interface Empty {}"
                    } else {
                        "declare module 'utils/format' { export function real(): number; }"
                    },
                );
            }
            if arm == "reference" {
                write(
                    &root,
                    "src/ref.ts",
                    "/// <reference types='custom' />\nexport {};",
                );
            }
            expect(
                &root,
                ext,
                "tsx",
                matches!(arm, "implicit" | "empty" | "absent"),
            );
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
fn scanned_module_imports_do_not_close_the_repository_boundary() {
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
                write(d.path(), "outside/global.d.ts", "export interface Value {}");
                write(&root, "types/local/index.d.ts", body);
                expect(&root, ext, implementation, !body.starts_with("///"));
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
fn unrelated_absolute_imports_and_external_links_skip() {
    for ext in ["jsx", "tsx"] {
        let d = TempDir::new().unwrap();
        let root = d.path().join("project");
        setup(&root, ext, "tsx", serde_json::json!({"types":[]}));
        write(d.path(), "outside/global.d.ts", "export interface Empty {}");
        write(
            &root,
            "types/local.d.ts",
            &format!(
                "import {:?}; export interface Empty {{}}",
                d.path().join("outside/global").to_str().unwrap()
            ),
        );
        expect(&root, ext, "tsx", true);
        std::fs::remove_file(root.join("types/local.d.ts")).unwrap();
        std::os::unix::fs::symlink(d.path().join("outside"), root.join("types/link")).unwrap();
        expect(&root, ext, "tsx", true);
    }
}

#[test]
fn installed_bin_and_internal_workspace_links_recover() {
    for ext in ["jsx", "tsx"] {
        let d = TempDir::new().unwrap();
        setup(
            d.path(),
            ext,
            "tsx",
            serde_json::json!({"types":["./node_modules/workspace"]}),
        );
        write(d.path(), "node_modules/.bin/.keep", "");
        write(
            d.path(),
            "node_modules/.bin/ignored.d.ts",
            "declare module 'utils/format' {}",
        );
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
            expect(d.path(), ext, "tsx", true);
        }
    }
}

#[test]
fn unrelated_import_package_redirects_do_not_decline() {
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
            write(d.path(), "outside/global.d.ts", "export interface Empty {}");
            expect(&root, ext, "tsx", true);
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
                expect(&root, ext, "tsx", true);
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

#[test]
fn accepted_cost_outside_ambient_shadowing_can_still_produce_wrong_exact() {
    // SPEC §0: ambient declarations found only outside this repository, including
    // through unscanned transitive imports, are the owner's disclosed accepted cost.
    // The current Exact below is intentionally asserted, not closed by a new fence.
    for ext in ["jsx", "tsx"] {
        for shape in ["ancestor", "transitive"] {
            let d = TempDir::new().unwrap();
            let root = d.path().join("project");
            setup(&root, ext, "tsx", serde_json::json!({}));
            write(
                d.path(),
                "node_modules/@types/custom/index.d.ts",
                "declare module 'utils/format' { export function real(): number; }",
            );
            if shape == "transitive" {
                write(
                    &root,
                    "types/local.d.ts",
                    "import '../../node_modules/@types/custom'; export interface Empty {}",
                );
            }
            expect(&root, ext, "tsx", true);
        }
    }
}

#[test]
fn installed_odd_files_skip_without_declining_in_both_grammars() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    for ext in ["jsx", "tsx"] {
        for arm in [
            "bin",
            "executable",
            "json",
            "workspace",
            "outside",
            "unread",
            "unparseable",
            "main",
        ] {
            let d = TempDir::new().unwrap();
            let root = d.path().join("project");
            setup(&root, ext, "tsx", serde_json::json!({"types":[]}));
            match arm {
                "bin" => {
                    write(&root, "node_modules/.bin/.keep", "");
                    symlink("../missing/tool", root.join("node_modules/.bin/tool")).unwrap();
                }
                "executable" => {
                    write(
                        &root,
                        "node_modules/pkg/bin/tool",
                        "#!/usr/bin/env node\ndeclare module 'utils/format' {}",
                    );
                    std::fs::set_permissions(
                        root.join("node_modules/pkg/bin/tool"),
                        std::fs::Permissions::from_mode(0o755),
                    )
                    .unwrap();
                }
                "json" => write(
                    &root,
                    "node_modules/pkg/constants.json",
                    r#"{"text":"declare module 'utils/format' {}"}"#,
                ),
                "workspace" => {
                    write(
                        &root,
                        "packages/workspace/index.d.ts",
                        "export interface Empty {}",
                    );
                    std::fs::create_dir_all(root.join("node_modules")).unwrap();
                    symlink("../packages/workspace", root.join("node_modules/workspace")).unwrap();
                }
                "outside" => {
                    write(d.path(), "outside.d.ts", "export interface Empty {}");
                    symlink(d.path().join("outside.d.ts"), root.join("link.d.ts")).unwrap();
                }
                "unread" => {
                    std::fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
                    std::fs::write(root.join("node_modules/pkg/bad.d.ts"), [0xff]).unwrap();
                }
                "unparseable" => write(
                    &root,
                    "node_modules/pkg/bad.d.ts",
                    r#"declare module '\xZZ' {}"#,
                ),
                "main" => {
                    write(&root, "types/local.d.ts", "import 'terser';");
                    write(
                        &root,
                        "node_modules/terser/package.json",
                        r#"{"main":"bin/terser"}"#,
                    );
                    write(
                        &root,
                        "node_modules/terser/bin/terser",
                        "#!/usr/bin/env node",
                    );
                }
                _ => unreachable!(),
            }
            expect(&root, ext, "tsx", true);
            // A skipped file cannot suppress a different, valid matching declaration.
            write(
                &root,
                "node_modules/guard/ambient.D.TS",
                "declare module 'utils/format' {}",
            );
            expect(&root, ext, "tsx", false);
        }
    }
}

#[test]
fn native_decoding_keeps_ambient_in_both_grammars() {
    for ext in ["jsx", "tsx"] {
        for encoding in ["latin1", "le", "be", "utf8", "utf8-bom"] {
            let d = TempDir::new().unwrap();
            setup(d.path(), ext, "tsx", serde_json::json!({"types":[]}));
            let path = d.path().join("types/ambient.d.ts");
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let encode = |source: &str| match encoding {
                "latin1" => [b"// caf\xe9\n".as_slice(), source.as_bytes()].concat(),
                "le" | "be" => {
                    let mut bytes = if encoding == "le" {
                        vec![255, 254]
                    } else {
                        vec![254, 255]
                    };
                    for unit in source.encode_utf16() {
                        bytes.extend(if encoding == "le" {
                            unit.to_le_bytes()
                        } else {
                            unit.to_be_bytes()
                        });
                    }
                    bytes
                }
                "utf8-bom" => [b"\xef\xbb\xbf".as_slice(), source.as_bytes()].concat(),
                _ => source.as_bytes().to_vec(),
            };
            std::fs::write(
                &path,
                encode("declare module 'utils/format' { export function real(): number; }"),
            )
            .unwrap();
            expect(d.path(), ext, "tsx", false);
            // Decoding alone must not create a declaration from documentation.
            std::fs::write(
                &path,
                encode("/* declare module 'utils/format' {} */\nexport interface Empty {}"),
            )
            .unwrap();
            expect(d.path(), ext, "tsx", true);
        }
    }
}

#[test]
#[cfg(unix)]
fn lexical_source_link_to_text_keeps_ambient_in_both_grammars() {
    for ext in ["jsx", "tsx"] {
        let d = TempDir::new().unwrap();
        setup(d.path(), ext, "tsx", serde_json::json!({"types":[]}));
        write(
            d.path(),
            "types/body.txt",
            "declare module 'utils/format' { export function real(): number; }",
        );
        std::os::unix::fs::symlink("body.txt", d.path().join("types/ambient.d.ts")).unwrap();
        let mut cfg: serde_json::Value =
            serde_json::from_slice(&std::fs::read(d.path().join("tsconfig.json")).unwrap())
                .unwrap();
        cfg.as_object_mut().unwrap().remove("include");
        cfg["files"] =
            serde_json::json!([format!("app.{ext}"), "src/util.tsx", "types/ambient.d.ts"]);
        write(d.path(), "tsconfig.json", &cfg.to_string());
        expect(d.path(), ext, "tsx", false);
        write(d.path(), "types/body.txt", "export interface Empty {}");
        expect(d.path(), ext, "tsx", true);
        // A non-source link name does not make non-source target bytes eligible.
        std::fs::remove_file(d.path().join("types/ambient.d.ts")).unwrap();
        std::os::unix::fs::symlink("body.txt", d.path().join("types/ambient.txt")).unwrap();
        write(
            d.path(),
            "types/body.txt",
            "declare module 'utils/format' {}",
        );
        expect(d.path(), ext, "tsx", true);

        // Both aliases must retain their own reference directory, even if
        // the eligible physical source was scheduled first.
        write(
            d.path(),
            "data/deep/body.d.ts",
            "/// <reference path='./guard.d.ts' />\nexport interface Empty {}",
        );
        write(
            d.path(),
            ".git/guard.d.ts",
            "declare module 'utils/format' { export function real(): number; }",
        );
        std::os::unix::fs::symlink("../data/deep/body.d.ts", d.path().join(".git/link.d.ts"))
            .unwrap();
        cfg["files"] = serde_json::json!([format!("app.{ext}"), "src/util.tsx", ".git/link.d.ts"]);
        write(d.path(), "tsconfig.json", &cfg.to_string());
        expect(d.path(), ext, "tsx", false);
        write(d.path(), ".git/guard.d.ts", "export interface Empty {}");
        expect(d.path(), ext, "tsx", true);

        // Direct type classification canonicalizes the file. Its lexical
        // alias's outside reference must still decline the config.
        cfg["files"] = serde_json::json!([format!("app.{ext}"), "src/util.tsx"]);
        cfg["compilerOptions"]["types"] = serde_json::json!(["./.git/link.d.ts"]);
        write(d.path(), "tsconfig.json", &cfg.to_string());
        write(
            d.path(),
            "data/deep/body.d.ts",
            "/// <reference path='../../outside.d.ts' />\nexport interface Empty {}",
        );
        expect(d.path(), ext, "tsx", false);
        write(d.path(), "data/deep/body.d.ts", "export interface Empty {}");
        expect(d.path(), ext, "tsx", true);
    }
}

#[test]
fn loaded_excluded_sources_keep_ambient_in_both_grammars() {
    for ext in ["jsx", "tsx"] {
        for directory in [".git", "node_modules/.bin"] {
            for mode in [
                "files",
                "include",
                "include-name-glob",
                "include-recursive",
                "reference",
                "types",
                "roots",
            ] {
                let d = TempDir::new().unwrap();
                setup(d.path(), ext, "tsx", serde_json::json!({"types":[]}));
                let input = format!("{directory}/ambient.d.ts");
                write(
                    d.path(),
                    &input,
                    "declare module 'utils/format' { export function real(): number; }",
                );
                // An unloaded excluded source retains the harmless tooling skip.
                expect(d.path(), ext, "tsx", true);
                let mut cfg: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(d.path().join("tsconfig.json")).unwrap())
                        .unwrap();
                match mode {
                    "files" => cfg["files"] = serde_json::json!([input]),
                    "include" | "include-name-glob" | "include-recursive" => {
                        let pattern = match mode {
                            "include-name-glob" if directory == ".git" => ".g*/**/*".into(),
                            "include-name-glob" => "node_*/.b*/**/*".into(),
                            "include-recursive" => format!("**/{directory}/**/*"),
                            _ => format!("{directory}/**/*"),
                        };
                        cfg["include"] = serde_json::json!([format!("app.{ext}"), "src", pattern])
                    }
                    "reference" => write(
                        d.path(),
                        "src/ref.ts",
                        &format!("/// <reference path='../{input}' />\nexport {{}};"),
                    ),
                    "types" => {
                        cfg["compilerOptions"]["types"] = serde_json::json!([format!("./{input}")])
                    }
                    "roots" => cfg["compilerOptions"]["typeRoots"] = serde_json::json!([directory]),
                    _ => unreachable!(),
                }
                write(d.path(), "tsconfig.json", &cfg.to_string());
                expect(d.path(), ext, "tsx", false);
                write(d.path(), &input, "export interface Empty {}");
                expect(d.path(), ext, "tsx", true);
            }
        }
    }
}
