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
    assert!(r["resolved_targets"].as_array().unwrap().is_empty(), "{r}");
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

// R1 witnesses use the existing production graph path; each fixed behavior
// also has a native differential fixture and an edge/negative control.
fn r1_module(d: &TempDir, writer: &str, spec: &str) -> Option<String> {
    graph(d.path())
        .js_ts_path_modules
        .get(&(writer.into(), spec.into()))
        .map(|v| v.0.clone())
}
fn r1_writer(d: &TempDir, writer: &str, spec: &str) {
    if writer != "app.tsx" {
        std::fs::remove_file(d.path().join("app.tsx")).unwrap();
    }
    write(
        d.path(),
        writer,
        &format!("import {{real as picked}} from '{spec}'; export function run(){{picked();}}\n"),
    );
}
#[test]
fn pkg_r1_exact_ts_exports() {
    let d = package(
        "bundler",
        r#"{"name":"@ws/lib","exports":{"types":"./a.ts","default":"./other.ts"}}"#,
        true,
    );
    write(
        d.path(),
        "packages/lib/a.tsx",
        "export function real(){return 3;}\n",
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
        r#"{"name":"@ws/lib","exports":"./a.ts"}"#,
    );
    missing(&d);
    write(
        d.path(),
        "packages/lib/a.ts",
        "export function real(){return 4;}\n",
    );
    assert_hit(&graph(d.path()), "app.tsx", "picked", "packages/lib/a.ts");
    write(
        d.path(),
        "packages/lib/package.json",
        r#"{"name":"@ws/lib","exports":"./index.tsx"}"#,
    );
    missing(&d); // explicit .tsx must not substitute the existing index.ts.
    write(
        d.path(),
        "packages/lib/package.json",
        r#"{"name":"@ws/lib","exports":"./index.js"}"#,
    );
    assert_hit(
        &graph(d.path()),
        "app.tsx",
        "picked",
        "packages/lib/index.ts",
    );
}
#[test]
fn pkg_r1_literal_star_and_pattern() {
    let d = package(
        "bundler",
        r#"{"name":"@ws/lib","exports":"./index*.ts"}"#,
        true,
    );
    missing(&d);
    write(
        d.path(),
        "packages/lib/package.json",
        r#"{"name":"@ws/lib","exports":{"./feature":"./index*.ts"}}"#,
    );
    r1_writer(&d, "app.tsx", "@ws/lib/feature");
    missing(&d);
    write(
        d.path(),
        "packages/lib/package.json",
        r#"{"name":"@ws/lib","exports":{"./*":"./*.ts"}}"#,
    );
    r1_writer(&d, "app.tsx", "@ws/lib/index");
    assert_hit(
        &graph(d.path()),
        "app.tsx",
        "picked",
        "packages/lib/index.ts",
    );
}
#[test]
fn pkg_r1_missing_legacy_declaration_substitutes_source() {
    let d = package(
        "node10",
        r#"{"name":"@ws/lib","types":"src/index.d.ts"}"#,
        true,
    );
    write(
        d.path(),
        "packages/lib/src/index.ts",
        "export function real(){return 3;}\n",
    );
    assert_hit(
        &graph(d.path()),
        "app.tsx",
        "picked",
        "packages/lib/src/index.ts",
    );
    write(
        d.path(),
        "packages/lib/src/index.d.ts",
        "export declare function real():void;\n",
    );
    missing(&d);
}
#[test]
fn pkg_r1_esm_legacy_field_and_index() {
    for mode in ["node16", "nodenext"] {
        let d = package(
            mode,
            r#"{"name":"@ws/lib","type":"module","main":"feature"}"#,
            true,
        );
        write(d.path(), "package.json", r#"{"type":"module"}"#);
        write(
            d.path(),
            "packages/lib/feature.ts",
            "export function real(){return 3;}\n",
        );
        assert_hit(
            &graph(d.path()),
            "app.tsx",
            "picked",
            "packages/lib/index.ts",
        );
        std::fs::remove_file(d.path().join("packages/lib/index.ts")).unwrap();
        missing(&d);
        write(
            d.path(),
            "packages/lib/package.json",
            r#"{"name":"@ws/lib","type":"module","main":"feature.js"}"#,
        );
        assert_hit(
            &graph(d.path()),
            "app.tsx",
            "picked",
            "packages/lib/feature.ts",
        );
        write(d.path(), "package.json", r#"{"type":"commonjs"}"#);
        write(
            d.path(),
            "packages/lib/package.json",
            r#"{"name":"@ws/lib","type":"module","main":"feature"}"#,
        );
        assert_hit(
            &graph(d.path()),
            "app.tsx",
            "picked",
            "packages/lib/feature.ts",
        );
    }
}
#[test]
fn pkg_r1_writer_extension_mode() {
    for (mode, ext, kind, target) in [
        ("nodenext", "mjs", "commonjs", "other"),
        ("nodenext", "cjs", "module", "index"),
        ("bundler", "cjs", "module", "index"),
        ("bundler", "mjs", "commonjs", "other"),
    ] {
        let d = package(
            mode,
            r#"{"name":"@ws/lib","exports":{"import":"./other.ts","require":"./index.ts"}}"#,
            true,
        );
        write(d.path(), "package.json", &json!({"type":kind}).to_string());
        let writer = format!("app.{ext}");
        r1_writer(&d, &writer, "@ws/lib");
        assert_hit(
            &graph(d.path()),
            &writer,
            "picked",
            &format!("packages/lib/{target}.ts"),
        );
    }
}
#[test]
fn pkg_r1_outer_package_scope() {
    let outer = TempDir::new().unwrap();
    let d = package(
        "nodenext",
        r#"{"name":"@ws/lib","exports":{"import":"./other.ts","require":"./index.ts"}}"#,
        true,
    );
    // Move the entire fixture under a controlled parent; retain relative links.
    let root = outer.path().join("repo");
    std::fs::create_dir(&root).unwrap();
    for entry in std::fs::read_dir(d.path()).unwrap() {
        let entry = entry.unwrap();
        std::fs::rename(entry.path(), root.join(entry.file_name())).unwrap();
    }
    std::fs::remove_file(root.join("package.json")).unwrap();
    write(outer.path(), "package.json", r#"{"type":"module"}"#);
    assert_hit(&graph(&root), "app.tsx", "picked", "packages/lib/other.ts");
    write(outer.path(), "package.json", r#"{"type":"commonjs"}"#);
    assert_hit(&graph(&root), "app.tsx", "picked", "packages/lib/index.ts");
    write(outer.path(), "package.json", "{");
    assert!(r1_module_at(&root, "app.tsx", "@ws/lib").is_none());
}
fn r1_module_at(root: &std::path::Path, writer: &str, spec: &str) -> Option<String> {
    graph(root)
        .js_ts_path_modules
        .get(&(writer.into(), spec.into()))
        .map(|v| v.0.clone())
}
#[test]
fn pkg_r1_self_name_with_exports_disabled() {
    let d = package(
        "bundler",
        r#"{"name":"@ws/lib","exports":"./other.ts","types":"index.ts"}"#,
        true,
    );
    write(
        d.path(),
        "tsconfig.json",
        r#"{"compilerOptions":{"moduleResolution":"bundler","module":"esnext","resolvePackageJsonExports":false},"include":["**/*"]}"#,
    );
    r1_writer(&d, "packages/lib/src/app.ts", "@ws/lib");
    assert_hit(
        &graph(d.path()),
        "packages/lib/src/app.ts",
        "picked",
        "packages/lib/other.ts",
    );
    write(
        d.path(),
        "packages/lib/package.json",
        r#"{"name":"@ws/lib","types":"index.ts"}"#,
    );
    assert_hit(
        &graph(d.path()),
        "packages/lib/src/app.ts",
        "picked",
        "packages/lib/index.ts",
    );
}
#[test]
fn pkg_r1_esm_export_hop() {
    for mode in ["node16", "nodenext", "bundler"] {
        let d = package(
            mode,
            r#"{"name":"@ws/lib","type":"module","exports":"./index.ts"}"#,
            true,
        );
        write(d.path(), "package.json", r#"{"type":"module"}"#);
        write(
            d.path(),
            "packages/lib/index.ts",
            "export {real} from './impl';\n",
        );
        write(
            d.path(),
            "packages/lib/impl.ts",
            "export function real(){return 3;}\n",
        );
        assert_eq!(
            r1_module(&d, "app.tsx", "@ws/lib").as_deref(),
            Some("packages/lib/index.ts")
        );
        if mode == "bundler" {
            assert_hit(
                &graph(d.path()),
                "app.tsx",
                "picked",
                "packages/lib/impl.ts",
            );
        } else {
            missing(&d);
        }
        write(
            d.path(),
            "packages/lib/index.ts",
            "export {real} from './impl.js';\n",
        );
        if mode != "bundler" {
            assert_hit(
                &graph(d.path()),
                "app.tsx",
                "picked",
                "packages/lib/impl.ts",
            );
        }
    }
}
#[test]
fn pkg_r1_imports_shadow_is_unsupported() {
    use prism::js_packages::Resolution;
    let d = package(
        "bundler",
        r#"{"name":"@ws/lib","exports":"./index.ts"}"#,
        true,
    );
    write(
        d.path(),
        "package.json",
        r##"{"imports":{"#alias":"./packages/lib/other.ts"}}"##,
    );
    std::os::unix::fs::symlink("../packages/lib", d.path().join("node_modules/#alias")).unwrap();
    r1_writer(&d, "app.tsx", "#alias");
    missing(&d);
    assert!(
        matches!(r1_resolution(&d,"app.tsx","#alias"),Resolution::Unsupported(ref s) if s=="package imports")
    );
}
fn r1_resolution(d: &TempDir, file: &str, spec: &str) -> prism::js_packages::Resolution {
    let repo = prism::repo_loader::load_repo(d.path()).unwrap();
    let indexed = repo.files.keys().cloned().collect();
    prism::js_packages::resolve_import(
        &repo.scope_graph_inputs.as_ref().unwrap().js_paths_snapshot,
        file,
        spec,
        &indexed,
    )
    .0
}
#[test]
fn pkg_r1_colon_paths_precede_scheme() {
    use prism::js_packages::Resolution;
    for mode in ["node10", "bundler", "nodenext"] {
        let d = package(mode, r#"{"name":"@ws/lib","exports":"./index.ts"}"#, true);
        write(d.path(),"tsconfig.json",&json!({"compilerOptions":{"moduleResolution":mode,"module":if mode=="nodenext"{mode}else{"esnext"},"paths":{"node:url":["packages/lib/index.ts"],"virtual:*":["packages/lib/*.ts"]}},"include":["**/*"]}).to_string());
        for (spec, target) in [("node:url", "index"), ("virtual:other", "other")] {
            r1_writer(&d, "app.tsx", spec);
            assert_hit(
                &graph(d.path()),
                "app.tsx",
                "picked",
                &format!("packages/lib/{target}.ts"),
            );
        }
        r1_writer(&d, "app.tsx", "virtual:missing");
        assert!(matches!(
            r1_resolution(&d, "app.tsx", "virtual:missing"),
            Resolution::Unsupported(_)
        ));
        r1_writer(&d, "app.tsx", "node:unmatched");
        assert_eq!(
            r1_resolution(&d, "app.tsx", "node:unmatched"),
            Resolution::ProvenUnresolved
        );
    }
}
#[test]
fn pkg_r1_null_continuation_and_outer_exports() {
    for mode in ["bundler", "node16"] {
        let d = package(
            mode,
            r#"{"name":"@ws/lib","exports":{"types":null,"default":"./index.ts"}}"#,
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
            r#"{"name":"@ws/lib","exports":null,"types":"index.ts"}"#,
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
            r#"{"name":"@ws/lib","exports":{"./feature":"./other.ts"}}"#,
        );
        write(
            d.path(),
            "packages/old/package.json",
            r#"{"name":"@ws/lib","exports":{"./feature":null,"./*":"./index.ts"}}"#,
        );
        write(
            d.path(),
            "packages/old/index.ts",
            "export function real(){return 4;}\n",
        );
        std::fs::create_dir_all(d.path().join("apps/web/node_modules/@ws")).unwrap();
        std::os::unix::fs::symlink(
            "../../../../packages/old",
            d.path().join("apps/web/node_modules/@ws/lib"),
        )
        .unwrap();
        r1_writer(&d, "apps/web/app.tsx", "@ws/lib/feature");
        assert_hit(
            &graph(d.path()),
            "apps/web/app.tsx",
            "picked",
            "packages/lib/other.ts",
        );
        write(
            d.path(),
            "packages/lib/package.json",
            r#"{"name":"@ws/lib","exports":{"./feature":null,"./*":"./index.ts"}}"#,
        );
        assert!(r1_module(&d, "apps/web/app.tsx", "@ws/lib/feature").is_none());
    }
}
#[test]
fn pkg_r1_legacy_link_subpath_and_bundler_index() {
    for mode in ["node10", "bundler", "node16"] {
        let d = package(mode, r#"{"name":"@ws/lib"}"#, true);
        assert_hit(
            &graph(d.path()),
            "app.tsx",
            "picked",
            "packages/lib/index.ts",
        );
        if mode == "bundler" {
            std::fs::remove_file(d.path().join("packages/lib/package.json")).unwrap();
            assert_hit(
                &graph(d.path()),
                "app.tsx",
                "picked",
                "packages/lib/index.ts",
            );
            write(
                d.path(),
                "packages/lib/package.json",
                r#"{"name":"@ws/lib"}"#,
            );
        }
        write(
            d.path(),
            "packages/lib/feature.ts",
            "export function real(){return 3;}\n",
        );
        r1_writer(&d, "app.tsx", "@ws/lib/feature");
        assert_hit(
            &graph(d.path()),
            "app.tsx",
            "picked",
            "packages/lib/feature.ts",
        );
        if mode == "node16" {
            write(d.path(), "package.json", r#"{"type":"module"}"#);
            assert!(r1_module(&d, "app.tsx", "@ws/lib/feature").is_none());
            r1_writer(&d, "app.tsx", "@ws/lib/feature.js");
            assert_hit(
                &graph(d.path()),
                "app.tsx",
                "picked",
                "packages/lib/feature.ts",
            );
        }
    }
}
#[test]
fn pkg_r1_three_way_no_false_absence() {
    use prism::js_packages::Resolution::*;
    let d = package(
        "bundler",
        r#"{"name":"@ws/lib","exports":"./dist/missing.ts"}"#,
        true,
    );
    assert_eq!(r1_resolution(&d, "app.tsx", "@ws/lib"), ProvenUnresolved);
    for (meta, reason) in [
        (
            r#"{"name":"@ws/lib","exports":["./index.ts"]}"#,
            "export arrays",
        ),
        (
            r#"{"name":"@ws/lib","typesVersions":{"*":{"*": ["other.ts"]}}}"#,
            "typesVersions",
        ),
    ] {
        write(d.path(), "packages/lib/package.json", meta);
        assert!(matches!(r1_resolution(&d,"app.tsx","@ws/lib"),Unsupported(ref s) if s==reason));
    }
    write(
        d.path(),
        "packages/lib/package.json",
        r#"{"name":"@ws/lib","exports":"./index.ts"}"#,
    );
    assert_eq!(
        r1_resolution(&d, "app.tsx", "@ws/lib"),
        Bound("packages/lib/index.ts".into())
    );
}
#[test]
fn pkg_r1_scheme_self_reference_precedes_uri_skip() {
    use prism::js_packages::Resolution;
    let d = package(
        "bundler",
        r#"{"name":"@ws/lib","exports":"./index.ts"}"#,
        true,
    );
    write(
        d.path(),
        "package.json",
        r#"{"name":"node:local","exports":"./packages/lib/index.ts"}"#,
    );
    r1_writer(&d, "app.tsx", "node:local");
    assert_hit(
        &graph(d.path()),
        "app.tsx",
        "picked",
        "packages/lib/index.ts",
    );
    r1_writer(&d, "app.tsx", "node:absent");
    assert_eq!(
        r1_resolution(&d, "app.tsx", "node:absent"),
        Resolution::ProvenUnresolved
    );
}
#[test]
fn pkg_r1_tsx_entry_requires_jsx() {
    use prism::js_packages::Resolution;
    let d = package(
        "bundler",
        r#"{"name":"@ws/lib","exports":"./entry.tsx"}"#,
        true,
    );
    write(
        d.path(),
        "packages/lib/entry.tsx",
        "export function real(){return 3;}\n",
    );
    missing(&d);
    assert!(
        matches!(r1_resolution(&d,"app.tsx","@ws/lib"),Resolution::Unsupported(ref s) if s=="JSX compiler option")
    );
    write(
        d.path(),
        "tsconfig.json",
        r#"{"compilerOptions":{"moduleResolution":"bundler","module":"esnext","jsx":"react-jsx"},"include":["**/*"]}"#,
    );
    assert_hit(
        &graph(d.path()),
        "app.tsx",
        "picked",
        "packages/lib/entry.tsx",
    );
}
#[test]
fn pkg_r1_typesversions_subpath_is_unsupported() {
    use prism::js_packages::Resolution;
    let d = package(
        "node10",
        r#"{"name":"@ws/lib","typesVersions":{"*":{"feature":["other.ts"]}}}"#,
        true,
    );
    write(
        d.path(),
        "packages/lib/feature.ts",
        "export function real(){return 3;}\n",
    );
    r1_writer(&d, "app.tsx", "@ws/lib/feature");
    missing(&d);
    assert!(
        matches!(r1_resolution(&d, "app.tsx", "@ws/lib/feature"), Resolution::Unsupported(ref r) if r=="typesVersions")
    );
    write(
        d.path(),
        "packages/lib/package.json",
        r#"{"name":"@ws/lib"}"#,
    );
    assert_hit(
        &graph(d.path()),
        "app.tsx",
        "picked",
        "packages/lib/feature.ts",
    );
}
