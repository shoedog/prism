use super::js_paths_common::*;
use prism::{call_graph::CallGraph, repo_loader::load_repo};
use tempfile::TempDir;

fn setup(ext: &str) -> TempDir {
    let d = TempDir::new().unwrap();
    write(
        d.path(),
        "tsconfig.json",
        &config(serde_json::json!({"@lib":["lib/real.tsx"]})),
    );
    write(
        d.path(),
        &format!("app.{ext}"),
        "import {real as picked} from '@lib'; export function run(){picked();}",
    );
    write(
        d.path(),
        "lib/real.tsx",
        "export function real(){return 1;}",
    );
    d
}
fn expect(d: &TempDir, ext: &str, exact: bool) {
    let app = format!("app.{ext}");
    if exact {
        assert_hit(&graph(d.path()), &app, "picked", "lib/real.tsx");
    } else {
        let loaded = load_repo(d.path()).unwrap();
        assert_eq!(
            outcome(&graph(d.path()), &app, "picked"),
            outcome(&CallGraph::build(&loaded.files), &app, "picked")
        );
    }
}

#[test]
fn covered_config_type_inputs() {
    for ext in ["jsx", "tsx"] {
        for key in ["typeRoots", "types"] {
            for inherited in [false, true] {
                for ambient in [false, true] {
                    let d = setup(ext);
                    write(
                        d.path(),
                        "node_modules/@types/local/index.d.ts",
                        if ambient {
                            "declare module '@lib' {}"
                        } else {
                            "export interface Empty {}"
                        },
                    );
                    let mut cfg: serde_json::Value = serde_json::from_str(
                        &std::fs::read_to_string(d.path().join("tsconfig.json")).unwrap(),
                    )
                    .unwrap();
                    cfg["compilerOptions"][key] = serde_json::json!([if key == "types" {
                        "local"
                    } else {
                        "./node_modules/@types"
                    }]);
                    if inherited {
                        write(d.path(), "tsconfig.parent.json", &cfg.to_string());
                        write(
                            d.path(),
                            "tsconfig.json",
                            r#"{"extends":"./tsconfig.parent.json"}"#,
                        );
                    } else {
                        write(d.path(), "tsconfig.json", &cfg.to_string());
                    }
                    expect(&d, ext, !ambient);
                }
            }
        }
    }
}

#[test]
fn uncovered_config_type_inputs() {
    for ext in ["jsx", "tsx"] {
        for key in ["typeRoots", "types"] {
            for input in ["../outside", "./missing", "/outside", "./unread.txt"] {
                for inherited in [false, true] {
                    let d = setup(ext);
                    write(d.path(), "unread.txt", "declare module '@lib' {}");
                    let mut cfg: serde_json::Value = serde_json::from_str(
                        &std::fs::read_to_string(d.path().join("tsconfig.json")).unwrap(),
                    )
                    .unwrap();
                    cfg["compilerOptions"][key] = serde_json::json!([input]);
                    if inherited {
                        write(d.path(), "tsconfig.parent.json", &cfg.to_string());
                        write(d.path(), "tsconfig.json", &serde_json::json!({"extends":"./tsconfig.parent.json","compilerOptions":{key:[]}}).to_string());
                    } else {
                        write(d.path(), "tsconfig.json", &cfg.to_string());
                    }
                    expect(&d, ext, input == "./missing");
                }
            }
        }
    }
}

#[test]
fn covered_triple_reference_inputs() {
    for ext in ["jsx", "tsx"] {
        for key in ["path", "types"] {
            for ambient in [false, true] {
                for folder in ["vendor", "node_modules/@types/local"] {
                    let d = setup(ext);
                    write(
                        d.path(),
                        &format!("{folder}/index.d.ts"),
                        if ambient {
                            "declare module '@lib' {}"
                        } else {
                            "export interface Empty {}"
                        },
                    );
                    let input = if key == "types" && folder.starts_with("node_modules") {
                        "local".to_owned()
                    } else {
                        format!("./{folder}/index.d.ts")
                    };
                    let original =
                        std::fs::read_to_string(d.path().join(format!("app.{ext}"))).unwrap();
                    write(
                        d.path(),
                        &format!("app.{ext}"),
                        &format!(
                            "/// <reference preserve=\"true\" {key} = '{input}' />\n{original}"
                        ),
                    );
                    expect(&d, ext, !ambient);
                }
            }
        }
    }
}

#[test]
fn uncovered_triple_reference_inputs() {
    for ext in ["jsx", "tsx"] {
        for key in ["path", "types"] {
            for input in [
                "../outside.d.ts",
                "./missing.d.ts",
                "/outside.d.ts",
                "missing-package",
                "./unread.txt",
                "",
            ] {
                let d = setup(ext);
                write(d.path(), "unread.txt", "declare module '@lib' {}");
                let original =
                    std::fs::read_to_string(d.path().join(format!("app.{ext}"))).unwrap();
                write(
                    d.path(),
                    &format!("app.{ext}"),
                    &format!("/// <reference {key}=\"{input}\" />\n{original}"),
                );
                expect(
                    &d,
                    ext,
                    input == "./missing.d.ts" || input == "missing-package",
                );
            }
        }
    }
}

#[test]
fn type_input_origins_files_scoped_packages_and_absolute_paths() {
    for ext in ["jsx", "tsx"] {
        for input in ["./vendor/empty.d.ts", "@scope/local", "absolute"] {
            let d = setup(ext);
            write(d.path(), "vendor/empty.d.ts", "export interface Empty {}");
            write(
                d.path(),
                "node_modules/@types/scope__local/index.d.ts",
                "export interface Empty {}",
            );
            let value = if input == "absolute" {
                d.path()
                    .join("vendor/empty.d.ts")
                    .to_str()
                    .unwrap()
                    .to_owned()
            } else {
                input.to_owned()
            };
            let mut cfg: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(d.path().join("tsconfig.json")).unwrap(),
            )
            .unwrap();
            cfg["compilerOptions"]["types"] = serde_json::json!([value]);
            write(d.path(), "tsconfig.json", &cfg.to_string());
            expect(&d, ext, true);
        }
        let d = setup(ext);
        write(
            d.path(),
            "config/types/local/index.d.ts",
            "export interface Empty {}",
        );
        write(
            d.path(),
            "config/tsconfig.base.json",
            r#"{"compilerOptions":{"typeRoots":["./types"],"types":["local"]}}"#,
        );
        let mut cfg: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(d.path().join("tsconfig.json")).unwrap())
                .unwrap();
        cfg["extends"] = serde_json::json!("./config/tsconfig.base.json");
        write(d.path(), "tsconfig.json", &cfg.to_string());
        expect(&d, ext, true);
    }
}

#[test]
fn transitive_references_and_cycles() {
    for ext in ["jsx", "tsx"] {
        for arm in ["path", "types", "typeRoots"] {
            for external in [false, true] {
                let d = setup(ext);
                write(
                    d.path(),
                    "vendor/local/index.d.ts",
                    "/// <reference path='./next.d.ts' />",
                );
                write(
                    d.path(),
                    "vendor/local/next.d.ts",
                    if external {
                        "/// <reference path='../../../outside.d.ts' />"
                    } else {
                        "/// <reference path='./index.d.ts' />"
                    },
                );
                let mut cfg: serde_json::Value = serde_json::from_str(
                    &std::fs::read_to_string(d.path().join("tsconfig.json")).unwrap(),
                )
                .unwrap();
                cfg["files"] = serde_json::json!([format!("app.{ext}")]);
                cfg["include"] = serde_json::json!([]);
                if arm == "path" {
                    let original =
                        std::fs::read_to_string(d.path().join(format!("app.{ext}"))).unwrap();
                    write(
                        d.path(),
                        &format!("app.{ext}"),
                        &format!("/// <reference path='./vendor/local/index.d.ts' />\n{original}"),
                    );
                } else {
                    cfg["compilerOptions"]["typeRoots"] = serde_json::json!(["./vendor"]);
                    if arm == "types" {
                        cfg["compilerOptions"]["types"] = serde_json::json!(["local"]);
                    }
                }
                write(d.path(), "tsconfig.json", &cfg.to_string());
                expect(&d, ext, !external);
            }
        }
    }
}

#[test]
fn child_roots_recheck_inherited_types_and_path_requires_file() {
    for ext in ["jsx", "tsx"] {
        let d = setup(ext);
        write(
            d.path(),
            "old/local/index.d.ts",
            "export interface Empty {}",
        );
        std::fs::create_dir(d.path().join("new")).unwrap();
        write(
            d.path(),
            "tsconfig.parent.json",
            r#"{"compilerOptions":{"typeRoots":["./old"],"types":["local"]}}"#,
        );
        let mut cfg: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(d.path().join("tsconfig.json")).unwrap())
                .unwrap();
        cfg["extends"] = serde_json::json!("./tsconfig.parent.json");
        cfg["compilerOptions"]["typeRoots"] = serde_json::json!(["./new"]);
        write(d.path(), "tsconfig.json", &cfg.to_string());
        expect(&d, ext, true);
        let d = setup(ext);
        let original = std::fs::read_to_string(d.path().join(format!("app.{ext}"))).unwrap();
        write(
            d.path(),
            &format!("app.{ext}"),
            &format!("/// <reference path='./lib' />\n{original}"),
        );
        expect(&d, ext, true);
    }
}

#[test]
fn type_package_redirects_must_reach_scanned_inputs() {
    for ext in ["jsx", "tsx"] {
        for arm in ["typeRoots", "types", "reference-types"] {
            for key in ["types", "typings", "exports", "typesVersions"] {
                for outside in [false, true] {
                    let d = setup(ext);
                    let external = tempfile::TempDir::new().unwrap();
                    write(
                        external.path(),
                        "outside.d.ts",
                        "declare module '@lib' { export function real(): void; }",
                    );
                    write(
                        d.path(),
                        "node_modules/@types/local/index.d.ts",
                        "export interface Empty {}",
                    );
                    let target = if outside {
                        external
                            .path()
                            .join("outside.d.ts")
                            .to_str()
                            .unwrap()
                            .to_owned()
                    } else {
                        "./index.d.ts".into()
                    };
                    let value = match key {
                        "exports" => serde_json::json!({".":{"types":target}}),
                        "typesVersions" => {
                            serde_json::json!({"*":{"*":[if outside {target} else {"./*.d.ts".into()}]}})
                        }
                        _ => serde_json::json!(target),
                    };
                    write(
                        d.path(),
                        "node_modules/@types/local/package.json",
                        &serde_json::json!({key:value}).to_string(),
                    );
                    let mut cfg: serde_json::Value = serde_json::from_str(
                        &std::fs::read_to_string(d.path().join("tsconfig.json")).unwrap(),
                    )
                    .unwrap();
                    if arm == "reference-types" {
                        let original =
                            std::fs::read_to_string(d.path().join(format!("app.{ext}"))).unwrap();
                        write(
                            d.path(),
                            &format!("app.{ext}"),
                            &format!("/// <reference types='local' />\n{original}"),
                        );
                    } else {
                        cfg["compilerOptions"][arm] = serde_json::json!([if arm == "types" {
                            "local"
                        } else {
                            "./node_modules/@types"
                        }]);
                    }
                    write(d.path(), "tsconfig.json", &cfg.to_string());
                    expect(&d, ext, !outside || key == "exports");
                }
            }
        }
    }
}

#[test]
fn opaque_type_metadata_declines() {
    for ext in ["jsx", "tsx"] {
        for raw in [
            "{".to_owned(),
            r#"{"types":123}"#.to_owned(),
            r#"{"types":"./missing.d.ts"}"#.to_owned(),
            " ".repeat(262_145),
        ] {
            let d = setup(ext);
            write(
                d.path(),
                "node_modules/@types/local/index.d.ts",
                "export interface Empty {}",
            );
            write(d.path(), "node_modules/@types/local/package.json", &raw);
            let mut cfg: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(d.path().join("tsconfig.json")).unwrap(),
            )
            .unwrap();
            cfg["compilerOptions"]["types"] = serde_json::json!(["local"]);
            write(d.path(), "tsconfig.json", &cfg.to_string());
            expect(&d, ext, raw == r#"{"types":"./missing.d.ts"}"#);
        }
    }
}
