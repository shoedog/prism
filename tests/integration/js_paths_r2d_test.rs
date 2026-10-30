use super::js_paths_common::*;
use prism::{call_graph::CallGraph, repo_loader::load_repo};
use tempfile::TempDir;

fn setup(ext: &str, spec: &str, roots: Option<Vec<&str>>) -> TempDir {
    let d = TempDir::new().unwrap();
    let mut cfg: serde_json::Value =
        serde_json::from_str(&config(serde_json::json!({spec:["lib/real"]}))).unwrap();
    cfg["files"] = serde_json::json!([format!("src/deep/app.{ext}")]);
    cfg["include"] = serde_json::json!([]);
    if let Some(roots) = roots {
        cfg["compilerOptions"]["typeRoots"] = serde_json::json!(roots);
    }
    write(d.path(), "tsconfig.json", &cfg.to_string());
    write(
        d.path(),
        &format!("src/deep/app.{ext}"),
        &format!("import {{real as picked}} from '{spec}'; export function run(){{picked();}}"),
    );
    write(
        d.path(),
        "lib/real.jsx",
        "export function real(){return 1;}",
    );
    d
}
fn base(d: &TempDir, ext: &str) {
    let loaded = load_repo(d.path()).unwrap();
    let file = format!("src/deep/app.{ext}");
    assert_eq!(
        outcome(&graph(d.path()), &file, "picked"),
        outcome(&CallGraph::build(&loaded.files), &file, "picked")
    );
}
fn exact(d: &TempDir, ext: &str) {
    assert_hit(
        &graph(d.path()),
        &format!("src/deep/app.{ext}"),
        "picked",
        "lib/real.jsx",
    );
}

#[test]
fn js_only_first_pass_absent_binds_exact() {
    for ext in ["jsx", "tsx"] {
        for roots in [None, Some(vec![]), Some(vec!["./missing-types"])] {
            let d = setup(ext, "utils", roots);
            exact(&d, ext);
            write(
                d.path(),
                "node_modules/unrelated/index.d.ts",
                "export interface Empty {}",
            );
            exact(&d, ext);
        }
    }
}
#[test]
fn node_modules_file_directory_and_types_keep_base() {
    for ext in ["jsx", "tsx"] {
        for level in ["", "src/", "src/deep/"] {
            for (spec, path) in [
                ("utils", "utils.d.ts"),
                ("utils", "utils/index.d.ts"),
                ("utils", "@types/utils/index.d.ts"),
                ("utils/format", "utils/format.d.ts"),
                ("@scope/utils/format", "@types/scope__utils/format.d.ts"),
            ] {
                let d = setup(ext, spec, None);
                exact(&d, ext);
                write(
                    d.path(),
                    &format!("{level}node_modules/{path}"),
                    "export declare function real(): void;",
                );
                base(&d, ext);
            }
        }
    }
}
#[test]
fn package_fields_and_versions_keep_base() {
    for ext in ["jsx", "tsx"] {
        for field in ["types", "typings", "main", "typesVersions"] {
            for spec in ["utils", "utils/format"] {
                let d = setup(ext, spec, None);
                let package = if spec == "utils" {
                    "node_modules/utils"
                } else {
                    "node_modules/utils/format"
                };
                let value = if field == "typesVersions" {
                    serde_json::json!({"*":{"*":["decls/entry"]}})
                } else {
                    serde_json::json!("decls/entry.d.ts")
                };
                write(
                    d.path(),
                    &format!("{package}/package.json"),
                    &serde_json::json!({field:value}).to_string(),
                );
                exact(&d, ext);
                write(
                    d.path(),
                    &format!("{package}/decls/entry.d.ts"),
                    "export declare function real(): void;",
                );
                base(&d, ext);
            }
        }
        let d = setup(ext, "utils/format", None);
        write(
            d.path(),
            "node_modules/utils/package.json",
            r#"{"typesVersions":{">=5.9":{"format":["decls/entry"]}}}"#,
        );
        exact(&d, ext);
        write(
            d.path(),
            "node_modules/utils/decls/entry.d.ts",
            "export declare function real(): void;",
        );
        base(&d, ext);
    }
}
#[test]
fn custom_type_roots_file_directory_and_scoped_names_keep_base() {
    for ext in ["jsx", "tsx"] {
        for (spec, root, name) in [
            ("utils", "types", "utils.d.ts"),
            ("utils", "types", "utils/index.d.ts"),
            ("@scope/utils", "types", "@scope/utils/index.d.ts"),
            (
                "@scope/utils",
                "node_modules/@types",
                "scope__utils/index.d.ts",
            ),
        ] {
            let d = setup(ext, spec, Some(vec![root]));
            exact(&d, ext);
            write(
                d.path(),
                &format!("{root}/{name}"),
                "export declare function real(): void;",
            );
            base(&d, ext);
        }
    }
}
#[test]
fn first_pass_paths_keep_base() {
    for ext in ["jsx", "tsx"] {
        for path in ["lib/real.d.ts", "lib/real/index.ts"] {
            let d = setup(ext, "utils", None);
            exact(&d, ext);
            write(d.path(), path, "export declare function real(): void;");
            base(&d, ext);
        }
    }
}
#[test]
fn opaque_and_outside_package_candidates_keep_base() {
    for ext in ["jsx", "tsx"] {
        for field in ["types", "typings", "main", "typesVersions"] {
            let d = setup(ext, "utils", None);
            let value = if field == "typesVersions" {
                serde_json::json!({"*":{"*":["../../../outside.d.ts"]}})
            } else {
                serde_json::json!("../../../outside.d.ts")
            };
            write(
                d.path(),
                "node_modules/utils/package.json",
                &serde_json::json!({field:value}).to_string(),
            );
            base(&d, ext);
        }
        let d = setup(ext, "utils", None);
        write(d.path(), "node_modules/utils/package.json", "{");
        base(&d, ext);
        let d = setup(ext, "utils", None);
        std::fs::create_dir_all(d.path().join("node_modules/utils.d.ts")).unwrap();
        base(&d, ext);
    }
}

#[test]
fn matched_paths_miss_does_not_use_baseurl_and_versions_keep_order() {
    for ext in ["jsx", "tsx"] {
        let d = setup(ext, "utils", None);
        write(
            d.path(),
            "utils.d.ts",
            "export declare function real(): void;",
        );
        exact(&d, ext);
        let d = setup(ext, "utils/format", None);
        write(
            d.path(),
            "node_modules/utils/package.json",
            r#"{"typesVersions":{">=5":{"format":["absent"]},"*":{"format":["exists"]}}}"#,
        );
        write(
            d.path(),
            "node_modules/utils/exists.d.ts",
            "export declare function real(): void;",
        );
        exact(&d, ext);
        write(
            d.path(),
            "node_modules/utils/absent.d.ts",
            "export declare function real(): void;",
        );
        base(&d, ext);
        let d = setup(ext, "utils/format", None);
        write(
            d.path(),
            "node_modules/utils/package.json",
            r#"{"typesVersions":{"*":{"format":["decls"],"index":["shadow"]}}}"#,
        );
        exact(&d, ext);
        write(
            d.path(),
            "node_modules/utils/decls/shadow.d.ts",
            "export declare function real(): void;",
        );
        base(&d, ext);
    }
}

#[test]
fn types_versions_javascript_object_order_and_large_versions() {
    for ext in ["jsx", "tsx"] {
        for metadata in [
            r#"{"typesVersions":{"*":{"*":["absent"]},"5":{"*":["exists"]}}}"#,
            r#"{"typesVersions":{"*":{"*":["absent"]},"*":{"*":["exists"]}}}"#,
            r#"{"typesVersions":{"<=999999999999999999999999":{"*":["exists"]},"*":{"*":["absent"]}}}"#,
        ] {
            let d = setup(ext, "utils", None);
            write(d.path(), "node_modules/utils/package.json", metadata);
            exact(&d, ext);
            write(
                d.path(),
                "node_modules/utils/exists.d.ts",
                "export declare function real(): void;",
            );
            base(&d, ext);
        }
    }
}

#[test]
fn package_entries_normalize_inside_the_captured_root() {
    for ext in ["jsx", "tsx"] {
        for field in ["types", "typings", "main", "typesVersions"] {
            let d = setup(ext, "utils", None);
            let entry = d.path().join("decls/entry.d.ts");
            let value = if field == "typesVersions" {
                serde_json::json!({"*":{"*":[entry]}})
            } else {
                serde_json::json!(entry)
            };
            write(
                d.path(),
                "node_modules/utils/package.json",
                &serde_json::json!({field:value}).to_string(),
            );
            exact(&d, ext);
            write(
                d.path(),
                "decls/entry.d.ts",
                "export declare function real(): void;",
            );
            base(&d, ext);
            std::fs::remove_file(&entry).unwrap();
            let outside = d.path().parent().unwrap().join("outside-r2d.d.ts");
            let value = if field == "typesVersions" {
                serde_json::json!({"*":{"*":[outside]}})
            } else {
                serde_json::json!(outside)
            };
            write(
                d.path(),
                "node_modules/utils/package.json",
                &serde_json::json!({field:value}).to_string(),
            );
            base(&d, ext);
        }
        let d = setup(ext, "utils", None);
        write(
            d.path(),
            "node_modules/utils/package.json",
            r#"{"types":"../../decls/entry.d.ts","typesVersions":{"*":{"*":["shadow"]}}}"#,
        );
        write(
            d.path(),
            "node_modules/utils/shadow.d.ts",
            "export declare function real(): void;",
        );
        exact(&d, ext); // An out-of-package entry skips typesVersions.
        write(
            d.path(),
            "decls/entry.d.ts",
            "export declare function real(): void;",
        );
        base(&d, ext);
        let d = setup(ext, "utils", None);
        write(
            d.path(),
            "node_modules/utils/package.json",
            r#"{"types":".","typesVersions":{"*":{"": ["shadow"]}}}"#,
        );
        write(
            d.path(),
            "node_modules/utils/shadow.d.ts",
            "export declare function real(): void;",
        );
        exact(&d, ext); // Empty exact mapping keys are falsy in TypeScript.
        write(
            d.path(),
            "node_modules/utils/index.d.ts",
            "export declare function real(): void;",
        );
        base(&d, ext);
    }
}

#[test]
fn types_versions_ecmascript_whitespace() {
    for ext in ["jsx", "tsx"] {
        for (range, binds) in [
            ("\u{feff}*", false),
            ("\u{85}*", true),
            ("* || \u{feff} || >=10", true),
            ("* || \u{2000} || >=10", true),
        ] {
            let d = setup(ext, "utils", None);
            write(
                d.path(),
                "node_modules/utils/package.json",
                &serde_json::json!({"typesVersions":{range:{"*":["exists"]}}}).to_string(),
            );
            exact(&d, ext);
            write(
                d.path(),
                "node_modules/utils/exists.d.ts",
                "export declare function real(): void;",
            );
            if binds {
                exact(&d, ext);
            } else {
                base(&d, ext);
            }
        }
    }
}

#[test]
fn package_root_file_candidates_are_outside_capture() {
    for ext in ["jsx", "tsx"] {
        for field in ["types", "typings", "main", "typesVersions"] {
            let d = setup(ext, "utils", None);
            exact(&d, ext);
            let root = d.path().to_str().unwrap();
            let value = if field == "typesVersions" {
                serde_json::json!({"*":{"*":[root]}})
            } else {
                serde_json::json!(root)
            };
            write(
                d.path(),
                "node_modules/utils/package.json",
                &serde_json::json!({field:value}).to_string(),
            );
            // Even with no sibling installed, absence outside the captured
            // root is unavailable. No content outside the capture is read.
            base(&d, ext);
            let sibling = format!("{root}.d.ts");
            std::fs::write(&sibling, "export declare function real(): void;").unwrap();
            base(&d, ext);
            std::fs::remove_file(sibling).unwrap();
        }
    }
}

#[test]
fn first_pass_filesystem_aliases_are_occupied() {
    for ext in ["jsx", "tsx"] {
        for (written, requested) in [
            ("node_modules/UTILS.d.ts", "node_modules/utils.d.ts"),
            ("node_modules/UTILS.D.TS", "node_modules/utils.d.ts"),
            ("NODE_MODULES/utils.d.ts", "node_modules/utils.d.ts"),
            (
                "node_modules/UTILS/index.d.ts",
                "node_modules/utils/index.d.ts",
            ),
        ] {
            let d = setup(ext, "utils", None);
            exact(&d, ext);
            write(d.path(), written, "export declare function real(): void;");
            if d.path().join(requested).exists() {
                base(&d, ext);
            } else {
                exact(&d, ext); // Case-sensitive filesystems have no alias.
            }
        }
    }
}
