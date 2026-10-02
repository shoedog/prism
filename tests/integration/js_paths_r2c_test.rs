use super::js_paths_common::*;
use prism::{call_graph::CallGraph, repo_loader::load_repo};
use tempfile::TempDir;

fn case(ext: &str, arm: &str, input: &str, declaration: Option<&str>, spec: &str) -> TempDir {
    let d = TempDir::new().unwrap();
    let mut cfg: serde_json::Value =
        serde_json::from_str(&config(serde_json::json!({spec:["lib/real.tsx"]}))).unwrap();
    cfg["files"] = serde_json::json!([format!("app.{ext}")]);
    cfg["include"] = serde_json::json!([]);
    let reference = match arm {
        "types" | "typeRoots" => {
            cfg["compilerOptions"][arm] = serde_json::json!([input]);
            String::new()
        }
        _ => format!(
            "/// <reference {}='{input}' />\n",
            if arm == "path" { "path" } else { "types" }
        ),
    };
    write(d.path(), "tsconfig.json", &cfg.to_string());
    write(d.path(), &format!("app.{ext}"), &format!("{reference}import {{real as picked}} from '{spec}'; export function run(){{picked();}}"));
    write(
        d.path(),
        "lib/real.tsx",
        "export function real(){return 1;}",
    );
    if let Some(source) = declaration {
        write(d.path(), "node_modules/local/client.d.ts", source);
        write(
            d.path(),
            "node_modules/local/package.json",
            r#"{"types":"./client.d.ts"}"#,
        );
    }
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
fn inputs(arm: &str) -> Vec<&str> {
    match arm {
        "typeRoots" => vec!["./node_modules/local"],
        "path" => vec!["./node_modules/local/client.d.ts"],
        _ => vec!["local", "local/client"],
    }
}
#[test]
fn absent_type_inputs_bind() {
    for ext in ["jsx", "tsx"] {
        for arm in ["types", "typeRoots", "path", "reference-types"] {
            for input in inputs(arm) {
                expect(&case(ext, arm, input, None, "@lib"), ext, true);
            }
        }
    }
}
#[test]
fn installed_matching_ambient_keeps_base() {
    for ext in ["jsx", "tsx"] {
        for arm in ["types", "typeRoots", "path", "reference-types"] {
            for input in inputs(arm) {
                expect(
                    &case(
                        ext,
                        arm,
                        input,
                        Some("declare /* tolerant */ module '@lib' {}"),
                        "@lib",
                    ),
                    ext,
                    false,
                );
                expect(
                    &case(
                        ext,
                        arm,
                        input,
                        Some("declare module '*.svg' {}"),
                        "@icon.svg",
                    ),
                    ext,
                    false,
                );
            }
        }
    }
}
#[test]
fn installed_nonmatching_wildcard_binds() {
    for ext in ["jsx", "tsx"] {
        for arm in ["types", "typeRoots", "path", "reference-types"] {
            for input in inputs(arm) {
                expect(
                    &case(ext, arm, input, Some("declare module '*.svg' {}"), "@lib"),
                    ext,
                    true,
                );
            }
        }
    }
}
#[test]
fn outside_type_inputs_decline() {
    for ext in ["jsx", "tsx"] {
        for arm in ["types", "typeRoots", "path", "reference-types"] {
            for input in ["../outside.d.ts", "/outside.d.ts"] {
                expect(&case(ext, arm, input, None, "@lib"), ext, false);
            }
            let external = TempDir::new().unwrap();
            write(external.path(), "outside.d.ts", "declare module '@lib' {}");
            let outside = external.path().join("outside.d.ts");
            expect(
                &case(ext, arm, outside.to_str().unwrap(), None, "@lib"),
                ext,
                false,
            );
            let d = case(ext, arm, "./linked.d.ts", None, "@lib");
            std::os::unix::fs::symlink(&outside, d.path().join("linked.d.ts")).unwrap();
            expect(&d, ext, false);
        }
    }
}
#[test]
fn transitive_absent_and_redirect_inputs() {
    for ext in ["jsx", "tsx"] {
        for arm in ["types", "typeRoots", "path", "reference-types"] {
            let input = inputs(arm)[0];
            let d = case(ext, arm, input, Some("/// <reference path='./missing.d.ts' />\n/// <reference types='not-installed' />"), "@lib");
            expect(&d, ext, true);
            write(
                d.path(),
                "node_modules/local/client.d.ts",
                "/// <reference path='../../../outside.d.ts' />",
            );
            expect(&d, ext, false);
            write(
                d.path(),
                "node_modules/local/client.d.ts",
                "export interface Empty {}",
            );
            write(
                d.path(),
                "node_modules/local/package.json",
                r#"{"typings":"./missing.d.ts"}"#,
            );
            expect(&d, ext, true);
            write(
                d.path(),
                "node_modules/local/unread.txt",
                "declare module '@lib' {}",
            );
            write(
                d.path(),
                "node_modules/local/client.d.ts",
                "/// <reference path='./unread.txt' />",
            );
            expect(&d, ext, true); // non-source type input is a disclosed scan skip
        }
    }
}

#[test]
fn custom_roots_miss_uses_secondary_package_lookup() {
    for ext in ["jsx", "tsx"] {
        for arm in ["types", "reference-types"] {
            for roots in [serde_json::json!([]), serde_json::json!(["./missing"])] {
                let d = case(ext, arm, "local", Some("export interface Empty {}"), "@lib");
                let mut cfg: serde_json::Value = serde_json::from_str(
                    &std::fs::read_to_string(d.path().join("tsconfig.json")).unwrap(),
                )
                .unwrap();
                cfg["compilerOptions"]["typeRoots"] = roots;
                write(d.path(), "tsconfig.json", &cfg.to_string());
                expect(&d, ext, true);
                let external = TempDir::new().unwrap();
                write(external.path(), "outside.d.ts", "declare module '@lib' {}");
                write(
                    d.path(),
                    "node_modules/local/package.json",
                    &serde_json::json!({"types":external.path().join("outside.d.ts")}).to_string(),
                );
                expect(&d, ext, false);
                // A malformed sibling field must not erase a valid selected
                // type redirect. This keeps r2c's explicit outside-input fence.
                for malformed in [serde_json::Value::Null, serde_json::json!(123)] {
                    write(
                        d.path(),
                        "node_modules/local/package.json",
                        &serde_json::json!({
                            "types":external.path().join("outside.d.ts"),
                            "typesVersions":malformed,
                        })
                        .to_string(),
                    );
                    expect(&d, ext, false);
                }
            }
        }
    }
}
