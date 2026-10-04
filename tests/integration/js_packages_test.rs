use super::js_paths_common::{assert_hit, graph, outcome, write};
use serde_json::json;
use tempfile::TempDir;

fn package(mode: &str, exports: &str, linked: bool) -> TempDir {
    let d = TempDir::new().unwrap();
    write(d.path(),"tsconfig.json",&json!({"compilerOptions":{"moduleResolution":mode,"module":if mode=="node16"||mode=="nodenext"{mode}else{"esnext"},"allowJs":true},"include":["**/*"]}).to_string());
    write(
        d.path(),
        "package.json",
        r#"{"private":true,"workspaces":["packages/*"]}"#,
    );
    write(
        d.path(),
        "app.tsx",
        "import {real as picked} from '@ws/lib'; export function run(){picked();}\n",
    );
    write(d.path(), "packages/lib/package.json", exports);
    write(
        d.path(),
        "packages/lib/index.ts",
        "export function real(){return 1;}\n",
    );
    write(
        d.path(),
        "packages/lib/other.ts",
        "export function real(){return 2;}\n",
    );
    if linked {
        std::fs::create_dir_all(d.path().join("node_modules/@ws")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("../../packages/lib", d.path().join("node_modules/@ws/lib"))
            .unwrap();
    }
    d
}
fn missing(d: &TempDir) {
    let r = outcome(&graph(d.path()), "app.tsx", "picked");
    assert!(r["exact_target"].is_null(), "{r}");
}

#[test]
fn pkg_node10_index_and_missing_declared_entry() {
    for meta in [
        r#"{"name":"@ws/lib"}"#,
        r#"{"name":"@ws/lib","types":"dist/missing.d.ts","main":"other.ts"}"#,
    ] {
        let d = package("node10", meta, true);
        assert_hit(
            &graph(d.path()),
            "app.tsx",
            "picked",
            "packages/lib/index.ts",
        );
        write(
            d.path(),
            "packages/lib/index.d.ts",
            "export declare function real():void;\n",
        );
        // index.ts still wins ahead of the declaration.
        assert_hit(
            &graph(d.path()),
            "app.tsx",
            "picked",
            "packages/lib/index.ts",
        );
        std::fs::remove_file(d.path().join("packages/lib/index.ts")).unwrap();
        missing(&d);
    }
}
#[test]
fn pkg_exports_mode_and_source_order() {
    for mode in ["bundler", "node16", "nodenext"] {
        let d = package(
            mode,
            r#"{"name":"@ws/lib","exports":{"types":"./index.ts","default":"./other.ts"}}"#,
            true,
        );
        assert_hit(
            &graph(d.path()),
            "app.tsx",
            "picked",
            "packages/lib/index.ts",
        );
        write(
            d.path(),
            "packages/lib/package.json",
            r#"{"name":"@ws/lib","exports":{"default":"./other.ts","types":"./index.ts"}}"#,
        );
        assert_hit(
            &graph(d.path()),
            "app.tsx",
            "picked",
            "packages/lib/other.ts",
        );
        write(
            d.path(),
            "packages/lib/package.json",
            r#"{"name":"@ws/lib","exports":{"types":"./missing.d.ts","default":"./index.ts"}}"#,
        );
        assert_hit(
            &graph(d.path()),
            "app.tsx",
            "picked",
            "packages/lib/index.ts",
        );
    }
}
#[test]
fn pkg_import_require_and_subpath_patterns() {
    for mode in ["bundler", "node16", "nodenext"] {
        let d = package(
            mode,
            r#"{"name":"@ws/lib","exports":{"import":"./other.ts","require":"./index.ts"}}"#,
            true,
        );
        assert_hit(
            &graph(d.path()),
            "app.tsx",
            "picked",
            if mode == "bundler" {
                "packages/lib/other.ts"
            } else {
                "packages/lib/index.ts"
            },
        );
        if mode == "bundler" {
            for module in ["commonjs", "preserve", "esnext"] {
                write(d.path(), "tsconfig.json", &json!({"compilerOptions":{"moduleResolution":mode,"module":module,"allowJs":true},"include":["**/*"]}).to_string());
                assert_hit(
                    &graph(d.path()),
                    "app.tsx",
                    "picked",
                    if module == "commonjs" {
                        "packages/lib/index.ts"
                    } else {
                        "packages/lib/other.ts"
                    },
                );
            }
            write(d.path(), "tsconfig.json", &json!({"compilerOptions":{"moduleResolution":mode,"module":"amd"},"include":["**/*"]}).to_string());
            missing(&d);
            write(d.path(), "tsconfig.json", &json!({"compilerOptions":{"moduleResolution":mode,"module":"esnext"},"include":["**/*"]}).to_string());
        }
        write(
            d.path(),
            "packages/lib/package.json",
            r#"{"name":"@ws/lib","exports":{"./*":"./other.ts","./feature/*":"./*.ts"}}"#,
        );
        write(d.path(),"app.tsx","import {real as picked} from '@ws/lib/feature/index'; export function run(){picked();}\n");
        assert_eq!(
            graph(d.path())
                .js_ts_path_modules
                .get(&("app.tsx".into(), "@ws/lib/feature/index".into()))
                .map(|v| v.0.as_str()),
            Some("packages/lib/index.ts")
        );
        assert_hit(
            &graph(d.path()),
            "app.tsx",
            "picked",
            "packages/lib/index.ts",
        );
        write(
            d.path(),
            "packages/lib/package.json",
            r#"{"name":"@ws/lib","exports":{"./feature/index":null,"./feature/*":"./*.ts"}}"#,
        );
        missing(&d);
    }
}
#[test]
fn pkg_discovery_is_not_binding_and_paths_win() {
    let d = package("node10", r#"{"name":"@ws/lib","types":"index.ts"}"#, false);
    missing(&d);
    #[cfg(unix)]
    {
        std::fs::create_dir_all(d.path().join("node_modules/@ws")).unwrap();
        std::os::unix::fs::symlink("../../packages/lib", d.path().join("node_modules/@ws/lib"))
            .unwrap();
    }
    assert_hit(
        &graph(d.path()),
        "app.tsx",
        "picked",
        "packages/lib/index.ts",
    );
    write(d.path(),"tsconfig.json",&json!({"compilerOptions":{"moduleResolution":"node10","paths":{"@ws/lib":["packages/lib/other.ts"]}},"include":["**/*"]}).to_string());
    assert_hit(
        &graph(d.path()),
        "app.tsx",
        "picked",
        "packages/lib/other.ts",
    );
    write(d.path(),"tsconfig.json",&json!({"compilerOptions":{"moduleResolution":"node10","paths":{"@ws/lib":["missing"]}},"include":["**/*"]}).to_string());
    missing(&d);
    assert!(!graph(d.path())
        .js_ts_path_modules
        .contains_key(&("app.tsx".into(), "@ws/lib".into())));
}
#[test]
fn pkg_declarations_external_shadow_and_malformed_exports() {
    let d = package(
        "bundler",
        r#"{"name":"@ws/lib","exports":"./index.ts"}"#,
        true,
    );
    assert_hit(
        &graph(d.path()),
        "app.tsx",
        "picked",
        "packages/lib/index.ts",
    );
    for meta in [
        r#"{"name":"@ws/lib","exports":"./missing.d.ts"}"#,
        r#"{"name":"@ws/lib","exports":"../lib/index.ts"}"#,
        r#"{"name":"@ws/lib","exports":["./index.ts"]}"#,
        r#"{"name":"@ws/lib","exports":{"default":"./index.ts","default":"./other.ts"}}"#,
    ] {
        write(d.path(), "packages/lib/package.json", meta);
        missing(&d);
    }
    write(
        d.path(),
        "packages/lib/package.json",
        r#"{"name":"@ws/lib","exports":"./index.ts"}"#,
    );
    write(
        d.path(),
        "node_modules/@ws/lib.ts",
        "export function real(){return 3;}\n",
    );
    // Modern exports win over a sibling module file. Node10 does not read
    // exports and must preserve the unindexed sibling winner instead.
    assert_hit(
        &graph(d.path()),
        "app.tsx",
        "picked",
        "packages/lib/index.ts",
    );
    write(
        d.path(),
        "tsconfig.json",
        r#"{"compilerOptions":{"moduleResolution":"node10"},"include":["**/*"]}"#,
    );
    missing(&d);
}
#[test]
fn pkg_builtin_and_loader_scheme_classification() {
    use prism::js_packages::{classify, SpecifierClass::*};
    assert_eq!(classify("node:url"), ExternalBuiltin);
    assert_eq!(classify("virtual:pwa-register"), LoaderScheme);
    assert_eq!(classify("@ws/lib"), Package);
    assert_eq!(classify("./node:url"), Relative);
    let d = package("node10", r#"{"name":"@ws/lib","types":"index.ts"}"#, true);
    for spec in ["node:url", "virtual:pwa-register"] {
        write(
            d.path(),
            "app.tsx",
            &format!(
                "import {{real as picked}} from '{spec}'; export function run(){{picked();}}\n"
            ),
        );
        missing(&d);
    }
}

#[test]
fn pkg_nested_missing_condition_falls_through_only_on_absence() {
    let d = package(
        "bundler",
        r#"{"name":"@ws/lib","exports":{"types":{"default":"./missing.d.ts"},"default":"./index.ts"}}"#,
        true,
    );
    assert_hit(
        &graph(d.path()),
        "app.tsx",
        "picked",
        "packages/lib/index.ts",
    );
    write(
        d.path(),
        "packages/lib/missing.d.ts",
        "export declare function real():void;\n",
    );
    missing(&d);
}

#[test]
fn pkg_ambient_fence_is_specifier_scoped() {
    let d = package("node10", r#"{"name":"@ws/lib","types":"index.ts"}"#, true);
    write(
        d.path(),
        "unrelated.d.ts",
        "declare module 'elsewhere' {}\n",
    );
    assert_hit(
        &graph(d.path()),
        "app.tsx",
        "picked",
        "packages/lib/index.ts",
    );
    write(d.path(), "unrelated.d.ts", "declare module '@ws/*' {}\n");
    missing(&d);
}
