use prism::call_graph::CallGraph;
use prism::repo_loader::load_repo;
use prism::resolution::ResolutionConfidence;
use std::path::Path;
use tempfile::TempDir;
fn write(root: &Path, p: &str, text: &str) {
    let p = root.join(p);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}
fn graph(root: &Path) -> CallGraph {
    let r = load_repo(root).unwrap();
    CallGraph::build_with_scope_graph_inputs(&r.files, r.scope_graph_inputs.as_ref())
}
fn outcome(g: &CallGraph, file: &str, name: &str) -> serde_json::Value {
    let rows = prism::navigation::queries::call_site_dump(g);
    let r = rows
        .into_iter()
        .find(|r| r["caller"]["file"] == file && r["callee_text"] == name)
        .unwrap();
    serde_json::from_str(&r.to_string()).unwrap()
}
fn assert_hit(g: &CallGraph, file: &str, callee: &str, target: &str) {
    let r = outcome(g, file, callee);
    let t = r["resolved_targets"].as_array().unwrap();
    assert_eq!(t.len(), 1, "{r}");
    assert_eq!(t[0]["function_id"]["file"], target, "{r}");
    assert_eq!(t[0]["function_id"]["name"], "real", "{r}");
    assert_eq!(t[0]["function_id"]["start_line"], 1, "{r}");
    assert_eq!(t[0]["function_id"]["end_line"], 1, "{r}");
    assert_eq!(t[0]["confidence"], "exact", "{r}");
    assert_eq!(t[0]["kind"], "import_member", "{r}");
}
fn config(paths: serde_json::Value) -> String {
    serde_json::json!({"compilerOptions":{"moduleResolution":"node","allowJs":true,"baseUrl":".","paths":paths},"include":["**/*"]}).to_string()
}
fn fixture(ext: &str, spec: &str, cfg: &str) -> TempDir {
    let d = TempDir::new().unwrap();
    write(d.path(), "tsconfig.json", cfg);
    write(
        d.path(),
        &format!("app.{ext}"),
        &format!(
            "import {{ real as picked }} from '{spec}';\nexport function run() {{ picked(); }}\n"
        ),
    );
    write(
        d.path(),
        &format!("lib/real.{ext}"),
        "export function real() { return 1; }\n",
    );
    write(
        d.path(),
        &format!("decoy/real.{ext}"),
        "export function real() { return 2; }\n",
    );
    d
}
#[test]
fn js_paths_same_directory_tsconfig_precedes_jsconfig() {
    for ext in ["jsx", "tsx"] {
        for jsconfig in [
            config(serde_json::json!({"@lib":["decoy/real"]})),
            "{ invalid ignored jsconfig".into(),
        ] {
            let d = fixture(
                ext,
                "@lib",
                &config(serde_json::json!({"@lib":["lib/real"]})),
            );
            write(d.path(), "jsconfig.json", &jsconfig);
            assert_hit(
                &graph(d.path()),
                &format!("app.{ext}"),
                "picked",
                &format!("lib/real.{ext}"),
            );
        }
    }
}

#[test]
fn js_paths_strictly_nearer_or_only_jsconfig_preserves_base() {
    for ext in ["jsx", "tsx"] {
        let d = fixture(
            ext,
            "@lib",
            &config(serde_json::json!({"@lib":["lib/real"]})),
        );
        write(
            d.path(),
            "jsconfig.json",
            &config(serde_json::json!({"@lib":["decoy/real"]})),
        );
        write(
            d.path(),
            "pkg/jsconfig.json",
            &config(serde_json::json!({"@lib":["../decoy/real"]})),
        );
        let app = format!("pkg/app.{ext}");
        write(
            d.path(),
            &app,
            "import { real as picked } from '@lib';\nexport function run() { picked(); }\n",
        );
        for tsconfig_present in [true, false] {
            if !tsconfig_present {
                std::fs::remove_file(d.path().join("tsconfig.json")).unwrap();
            }
            let loaded = load_repo(d.path()).unwrap();
            assert_eq!(
                outcome(&graph(d.path()), &app, "picked"),
                outcome(&CallGraph::build(&loaded.files), &app, "picked")
            );
        }
    }
}

#[test]
fn js_paths_exact_and_wildcard_red() {
    for ext in ["jsx", "tsx"] {
        for paths in [
            serde_json::json!({"@lib/real":["lib/real"]}),
            serde_json::json!({"@lib/*":["lib/*"]}),
        ] {
            let d = fixture(ext, "@lib/real", &config(paths));
            assert_hit(
                &graph(d.path()),
                &format!("app.{ext}"),
                "picked",
                &format!("lib/real.{ext}"),
            );
        }
    }
}
#[test]
fn js_paths_exact_precedes_longest_prefix_and_suffix() {
    for ext in ["jsx", "tsx"] {
        for paths in [
            serde_json::json!({"@lib/real":["lib/real"],"@lib/*":["decoy/*"]}),
            serde_json::json!({"@lib/*end":["lib/*"],"@*end":["decoy/real"]}),
        ] {
            let spec = if paths.get("@lib/real").is_some() {
                "@lib/real"
            } else {
                "@lib/realend"
            };
            let d = fixture(ext, spec, &config(paths));
            assert_hit(
                &graph(d.path()),
                &format!("app.{ext}"),
                "picked",
                &format!("lib/real.{ext}"),
            );
        }
    }
}
#[test]
fn js_paths_parent_origin_without_baseurl_and_child_override() {
    for ext in ["jsx", "tsx"] {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "packages/tsconfig.base.json",
            r#"{"compilerOptions":{"moduleResolution":"node","allowJs":true,"paths":{"@lib":["lib/real"]}}}"#,
        );
        write(
            d.path(),
            "packages/app/tsconfig.json",
            r#"{"extends":"../tsconfig.base.json","include":["**/*"],"exclude":["dist"]}"#,
        );
        write(
            d.path(),
            &format!("packages/app/app.{ext}"),
            "import { real as picked } from '@lib';\nexport function run() { picked(); }\n",
        );
        write(
            d.path(),
            &format!("packages/lib/real.{ext}"),
            "export function real() { return 1; }\n",
        );
        write(
            d.path(),
            &format!("packages/app/lib/real.{ext}"),
            "export function real() { return 2; }\n",
        );
        assert_hit(
            &graph(d.path()),
            &format!("packages/app/app.{ext}"),
            "picked",
            &format!("packages/lib/real.{ext}"),
        );
        write(
            d.path(),
            "packages/app/tsconfig.json",
            r#"{"extends":"../tsconfig.base.json","compilerOptions":{"paths":{"@lib":["app/lib/real"]}},"include":["**/*"],"exclude":["dist"]}"#,
        );
        // Child paths origin is the child config, not the parent's origin.
        write(
            d.path(),
            &format!("packages/app/app/lib/real.{ext}"),
            "export function real() { return 3; }\n",
        );
        assert_hit(
            &graph(d.path()),
            &format!("packages/app/app.{ext}"),
            "picked",
            &format!("packages/app/app/lib/real.{ext}"),
        );
    }
}
#[test]
fn js_paths_baseurl_declaration_origin() {
    for ext in ["jsx", "tsx"] {
        let d = fixture(
            ext,
            "@lib",
            r#"{"compilerOptions":{"moduleResolution":"node","allowJs":true,"baseUrl":"lib"}}"#,
        );
        write(
            d.path(),
            "pkg/tsconfig.json",
            r#"{"extends":"../tsconfig.json","compilerOptions":{"paths":{"@lib":["real"]}},"include":["**/*"]}"#,
        );
        write(
            d.path(),
            &format!("pkg/app.{ext}"),
            "import { real as picked } from '@lib';\nexport function run() { picked(); }\n",
        );
        assert_hit(
            &graph(d.path()),
            &format!("pkg/app.{ext}"),
            "picked",
            &format!("lib/real.{ext}"),
        );
    }
}
#[test]
fn js_paths_nearest_including_config_and_files_override_exclude() {
    for ext in ["jsx", "tsx"] {
        let d = fixture(
            ext,
            "@lib",
            &config(serde_json::json!({"@lib":["lib/real"]})),
        );
        write(
            d.path(),
            "pkg/tsconfig.json",
            r#"{"compilerOptions":{"moduleResolution":"node","allowJs":true,"baseUrl":"..","paths":{"@lib":["decoy/real"]}},"include":["other/**/*"]}"#,
        );
        write(
            d.path(),
            &format!("pkg/app.{ext}"),
            "import { real as picked } from '@lib';\nexport function run() { picked(); }\n",
        );
        assert_hit(
            &graph(d.path()),
            &format!("pkg/app.{ext}"),
            "picked",
            &format!("lib/real.{ext}"),
        );
        write(d.path(),"pkg/tsconfig.json",&serde_json::json!({"compilerOptions":{"moduleResolution":"node","allowJs":true,"baseUrl":"..","paths":{"@lib":["decoy/real"]}},"files":[format!("app.{ext}")],"exclude":["**/*"]}).to_string());
        assert_hit(
            &graph(d.path()),
            &format!("pkg/app.{ext}"),
            "picked",
            &format!("decoy/real.{ext}"),
        );
    }
}
#[test]
fn js_paths_index_and_relative_barrel() {
    for ext in ["jsx", "tsx"] {
        let d = fixture(ext, "@lib", &config(serde_json::json!({"@lib":["lib"]})));
        write(
            d.path(),
            &format!("lib/index.{ext}"),
            "export { real } from './real';\n",
        );
        assert_hit(
            &graph(d.path()),
            &format!("app.{ext}"),
            "picked",
            &format!("lib/real.{ext}"),
        );
    }
}
#[test]
fn js_paths_jsonc_and_unicode_escaped_key() {
    for ext in ["jsx", "tsx"] {
        let d = fixture(
            ext,
            "@lib",
            r#"{// comment
"compilerOptions":{"moduleResolution":"node","allowJs":true,"paths":{"@\u006cib":["./lib/real"],},},"include":["**/*"], /* end */ }"#,
        );
        assert_hit(
            &graph(d.path()),
            &format!("app.{ext}"),
            "picked",
            &format!("lib/real.{ext}"),
        );
    }
}
#[test]
fn js_paths_unproven_configs_preserve_complete_base_row() {
    for ext in ["jsx", "tsx"] {
        let cfgs=[r#"{"extends":"./tsconfig.json"}"#.to_string(),r#"{"extends":"missing-package"}"#.to_string(),r#"{"compilerOptions":{"moduleResolution":"node","allowJs":true,"paths":{"@lib":["lib/real"],"@lib":["decoy/real"]}}}"#.to_string(),config(serde_json::json!({"@lib":["missing","lib/real"]})),config(serde_json::json!({"@*ib":["lib/real"],"@*lib":["decoy/real"]})),config(serde_json::json!({"@lib":["../lib/real"]})),config(serde_json::json!({"@lib":["lib/real.js"]})),r#"{"compilerOptions":{"moduleResolution":"NodeNext","allowJs":true,"paths":{"@lib":["lib/real"]}},"include":["**/*"]}"#.to_string(),r#"{"compilerOptions":{"moduleResolution":"node","allowJs":true,"baseUrl":"lib"},"include":["**/*"]}"#.to_string()];
        for cfg in cfgs {
            let d = fixture(ext, "@lib", &cfg);
            let loaded = load_repo(d.path()).unwrap();
            let base = CallGraph::build(&loaded.files);
            let head = graph(d.path());
            assert_eq!(
                outcome(&head, &format!("app.{ext}"), "picked"),
                outcome(&base, &format!("app.{ext}"), "picked"),
                "{cfg}"
            );
        }
    }
}
#[test]
fn js_paths_competing_extensions_declarations_and_package_json_preserve_base() {
    for ext in ["jsx", "tsx"] {
        for blocker in ["lib/real.d.ts", "lib/real.js", "lib/real/package.json"] {
            let d = fixture(
                ext,
                "@lib",
                &config(serde_json::json!({"@lib":["lib/real"]})),
            );
            write(
                d.path(),
                blocker,
                if blocker.ends_with("json") {
                    r#"{"main":"other.js"}"#
                } else {
                    "export function real() {}\n"
                },
            );
            let loaded = load_repo(d.path()).unwrap();
            let base = CallGraph::build(&loaded.files);
            assert_eq!(
                outcome(&graph(d.path()), &format!("app.{ext}"), "picked"),
                outcome(&base, &format!("app.{ext}"), "picked")
            );
        }
    }
}
#[cfg(unix)]
#[test]
fn js_paths_symlink_candidate_preserves_base() {
    for ext in ["jsx", "tsx"] {
        let d = fixture(
            ext,
            "@lib",
            &config(serde_json::json!({"@lib":["lib/link"]})),
        );
        std::os::unix::fs::symlink(
            format!("real.{ext}"),
            d.path().join(format!("lib/link.{ext}")),
        )
        .unwrap();
        let loaded = load_repo(d.path()).unwrap();
        let base = CallGraph::build(&loaded.files);
        assert_eq!(
            outcome(&graph(d.path()), &format!("app.{ext}"), "picked"),
            outcome(&base, &format!("app.{ext}"), "picked")
        );
    }
}
#[test]
fn js_paths_config_change_and_incremental_rebuild() {
    for ext in ["jsx", "tsx"] {
        let d = fixture(
            ext,
            "@lib",
            &config(serde_json::json!({"@lib":["lib/real"]})),
        );
        let old = load_repo(d.path()).unwrap();
        let old_graph = graph(d.path());
        assert_hit(
            &old_graph,
            &format!("app.{ext}"),
            "picked",
            &format!("lib/real.{ext}"),
        );
        write(
            d.path(),
            "tsconfig.json",
            &config(serde_json::json!({"@lib":["decoy/real"]})),
        );
        let new = load_repo(d.path()).unwrap();
        assert_ne!(old.manifest_hashes, new.manifest_hashes);
        let cached = prism::cpg::CodePropertyGraph::build_enriched_with_scope_graph_inputs(
            &old.files,
            None,
            old.scope_graph_inputs.as_ref(),
        );
        let rebuilt = prism::cpg::CodePropertyGraph::build_incremental_with_scope_graph_inputs(
            cached.call_graph,
            cached.dfg,
            &Default::default(),
            &new.files,
            None,
            new.scope_graph_inputs.as_ref(),
        );
        let full = graph(d.path());
        assert_hit(
            &full,
            &format!("app.{ext}"),
            "picked",
            &format!("decoy/real.{ext}"),
        );
        assert_eq!(
            outcome(&rebuilt.call_graph, &format!("app.{ext}"), "picked"),
            outcome(&full, &format!("app.{ext}"), "picked")
        );
    }
}
#[test]
fn js_paths_s1b_namespace_star_proof_is_reused() {
    for ext in ["jsx", "tsx"] {
        for (barrel, proven) in [
            ("export * from './lib/real';\n", true),
            (
                "export * from './lib/real';\nexport * from './missing';\n",
                false,
            ),
            (
                "export { real } from './lib/real';\nexport * from './missing';\n",
                true,
            ),
        ] {
            let d = fixture(ext, "@lib", &config(serde_json::json!({"@lib":["barrel"]})));
            write(d.path(), &format!("barrel.{ext}"), barrel);
            write(d.path(), &format!("app.{ext}"), "import { real as picked } from '@lib';\nimport * as Lib from './barrel';\nexport function run() { picked(); Lib.real(); }\n");
            let loaded = load_repo(d.path()).unwrap();
            let base = CallGraph::build(&loaded.files);
            let head = graph(d.path());
            let namespace = head
                .js_ts_namespace_exports
                .get(&format!("barrel.{ext}"))
                .and_then(|exports| exports.get("real"));
            assert_eq!(namespace.is_some(), proven);
            if let Some(terminal) = namespace {
                assert!(!terminal.via_unresolved_star);
                assert_hit(
                    &head,
                    &format!("app.{ext}"),
                    "picked",
                    &format!("lib/real.{ext}"),
                );
            } else {
                assert!(
                    head.js_ts_resolved_exports[&format!("barrel.{ext}")]["real"]
                        .via_unresolved_star
                );
                assert_eq!(
                    outcome(&head, &format!("app.{ext}"), "picked"),
                    outcome(&base, &format!("app.{ext}"), "picked")
                );
            }
            // P1 only adds named-import aliases; the landed relative namespace
            // route keeps precisely its original complete row in every case.
            assert_eq!(
                outcome(&head, &format!("app.{ext}"), "real"),
                outcome(&base, &format!("app.{ext}"), "real")
            );
        }
    }
}

#[test]
fn js_paths_namespace_route_and_parameter_shadow_remain_base() {
    for ext in ["jsx", "tsx"] {
        let d = fixture(
            ext,
            "@lib",
            &config(serde_json::json!({"@lib":["lib/real"]})),
        );
        write(d.path(),&format!("app.{ext}"),"import { real as picked } from '@lib';\nimport * as Lib from '@lib';\nexport function run(picked) { picked(); Lib.real(); }\n");
        let loaded = load_repo(d.path()).unwrap();
        let base = CallGraph::build(&loaded.files);
        let head = graph(d.path());
        assert_eq!(
            outcome(&head, &format!("app.{ext}"), "picked"),
            outcome(&base, &format!("app.{ext}"), "picked")
        );
        assert_eq!(
            outcome(&head, &format!("app.{ext}"), "real"),
            outcome(&base, &format!("app.{ext}"), "real")
        );
        assert!(head
            .resolve_call_site(&head.calls.values().flat_map(|s| s).next().unwrap())
            .iter()
            .all(|t| t.confidence != ResolutionConfidence::Exact));
    }
}

#[test]
fn js_paths_explicit_exclude_preserves_base() {
    for ext in ["jsx", "tsx"] {
        let cfg = serde_json::json!({"compilerOptions":{"moduleResolution":"node","allowJs":true,"paths":{"@lib":["lib/real"]}},"include":["**/*"],"exclude":[format!("app.{ext}")]});
        let d = fixture(ext, "@lib", &cfg.to_string());
        let loaded = load_repo(d.path()).unwrap();
        assert_eq!(
            outcome(&graph(d.path()), &format!("app.{ext}"), "picked"),
            outcome(
                &CallGraph::build(&loaded.files),
                &format!("app.{ext}"),
                "picked"
            )
        );
    }
}

#[test]
fn js_paths_cjs_terminal_without_span_preserves_base() {
    for ext in ["jsx", "tsx"] {
        let d = fixture(
            ext,
            "@lib",
            &config(serde_json::json!({"@lib":["lib/real"]})),
        );
        write(
            d.path(),
            &format!("lib/real.{ext}"),
            "function real() { return 1; }\nmodule.exports = { real };\n",
        );
        let loaded = load_repo(d.path()).unwrap();
        assert_eq!(
            outcome(&graph(d.path()), &format!("app.{ext}"), "picked"),
            outcome(
                &CallGraph::build(&loaded.files),
                &format!("app.{ext}"),
                "picked"
            )
        );
    }
}

#[test]
fn js_paths_require_alias_preserves_base_even_with_esm_same_module() {
    for ext in ["jsx", "tsx"] {
        for prefix in ["", "import { real as other } from '@lib';\n"] {
            let d = fixture(
                ext,
                "@lib",
                &config(serde_json::json!({"@lib":["lib/real"]})),
            );
            write(d.path(), &format!("app.{ext}"), &format!("{prefix}const {{ real: picked }} = require('@lib');\nexport function run() {{ picked(); }}\n"));
            let loaded = load_repo(d.path()).unwrap();
            assert_eq!(
                outcome(&graph(d.path()), &format!("app.{ext}"), "picked"),
                outcome(
                    &CallGraph::build(&loaded.files),
                    &format!("app.{ext}"),
                    "picked"
                )
            );
        }
    }
}

#[test]
fn js_paths_unmatched_target_star_preserves_base() {
    for ext in ["jsx", "tsx"] {
        let d = fixture(ext, "@lib", &config(serde_json::json!({"@lib":["lib/*"]})));
        write(
            d.path(),
            &format!("lib/index.{ext}"),
            "export function real() { return 1; }\n",
        );
        let loaded = load_repo(d.path()).unwrap();
        assert_eq!(
            outcome(&graph(d.path()), &format!("app.{ext}"), "picked"),
            outcome(
                &CallGraph::build(&loaded.files),
                &format!("app.{ext}"),
                "picked"
            )
        );
    }
}

#[test]
fn js_paths_unproven_positions_preserve_base() {
    use prism::call_graph::JsLocalBinding;
    for ext in ["jsx", "tsx"] {
        let d = fixture(
            ext,
            "@lib",
            &config(serde_json::json!({"@lib":["lib/real"]})),
        );
        let loaded = load_repo(d.path()).unwrap();
        for state in [
            JsLocalBinding::Position("dynamic_scope".into()),
            JsLocalBinding::Unchecked,
            JsLocalBinding::MayCall("may_call".into()),
            JsLocalBinding::Unproven("unbound".into()),
        ] {
            let mut head = graph(d.path());
            let mut base = CallGraph::build(&loaded.files);
            for g in [&mut head, &mut base] {
                for sites in g.calls.values_mut() {
                    *sites = std::mem::take(sites)
                        .into_iter()
                        .map(|mut s| {
                            if s.callee_name == "picked" {
                                s.local_binding = state.clone();
                            }
                            s
                        })
                        .collect();
                }
            }
            assert_eq!(
                outcome(&head, &format!("app.{ext}"), "picked"),
                outcome(&base, &format!("app.{ext}"), "picked"),
                "{state:?}"
            );
        }
    }
}

fn r1_controls(stars: bool) {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/js_paths_r1.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let name = case["case"].as_str().unwrap();
        if (name.starts_with("C44")
            || name.starts_with("C55")
            || name.starts_with("C59")
            || name.starts_with("C60")
            || name.starts_with("C61"))
            != stars
        {
            continue;
        }
        let d = TempDir::new().unwrap();
        for (p, text) in case["payload"].as_object().unwrap() {
            write(d.path(), p, text.as_str().unwrap());
        }
        let loaded = load_repo(d.path()).unwrap();
        let head = graph(d.path());
        let base = CallGraph::build(&loaded.files);
        let app = case["app"].as_str().unwrap();
        if case["expectation"] == "gain" {
            assert_hit(&head, app, "picked", case["target"].as_str().unwrap());
        } else {
            assert_eq!(
                outcome(&head, app, "picked"),
                outcome(&base, app, "picked"),
                "{name}"
            );
        }
        if name.starts_with("C44") || name.starts_with("C59") {
            let ext = name.rsplit('-').next().unwrap();
            let relative = format!("rel.{ext}");
            assert_eq!(
                outcome(&head, &relative, "picked"),
                outcome(&base, &relative, "picked"),
                "relative {name}"
            );
        }
    }
}
#[test]
fn js_paths_r1_membership_barriers_dot_and_relative() {
    r1_controls(false);
}
#[test]
fn js_paths_r1_skipped_star_preserves_base() {
    r1_controls(true);
}
#[test]
fn js_paths_unrelated_text_keeps_topology() {
    let d = fixture(
        "tsx",
        "@lib",
        &config(serde_json::json!({"@lib":["lib/real"]})),
    );
    let before = load_repo(d.path()).unwrap().manifest_hashes;
    write(d.path(), "unrelated.txt", "irrelevant");
    let after = load_repo(d.path()).unwrap().manifest_hashes;
    assert_eq!(before, after);
    write(
        d.path(),
        "lib/real.d.ts",
        "export declare function real(): number;",
    );
    assert_ne!(before, load_repo(d.path()).unwrap().manifest_hashes);
}

#[test]
fn js_paths_r2_matcher_and_targeted_ownership_barriers() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/js_paths_r2.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let d = TempDir::new().unwrap();
        for (p, text) in case["payload"].as_object().unwrap() {
            write(d.path(), p, text.as_str().unwrap());
        }
        let loaded = load_repo(d.path()).unwrap();
        let base = CallGraph::build(&loaded.files);
        assert_eq!(
            outcome(&graph(d.path()), case["app"].as_str().unwrap(), "picked"),
            outcome(&base, case["app"].as_str().unwrap(), "picked"),
            "{}",
            case["case"]
        );
    }
}

fn extension_priority_controls(prefixes: &[&str]) {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/js_paths_priority.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let name = case["case"].as_str().unwrap();
        if !prefixes.iter().any(|p| name.starts_with(p)) {
            continue;
        }
        let d = TempDir::new().unwrap();
        for (p, text) in case["payload"].as_object().unwrap() {
            write(d.path(), p, text.as_str().unwrap());
        }
        let app = case["app"].as_str().unwrap();
        let head = graph(d.path());
        if case["expectation"] == "gain" {
            assert_hit(&head, app, "picked", case["target"].as_str().unwrap());
        } else {
            let loaded = load_repo(d.path()).unwrap();
            assert_eq!(
                outcome(&head, app, "picked"),
                outcome(&CallGraph::build(&loaded.files), app, "picked"),
                "{name}"
            );
        }
    }
}

#[test]
fn js_paths_tsx_ts_priority_preserves_base_and_literal_exemption() {
    extension_priority_controls(&["C85-", "C88-files-priority-exempt-tsx-"]);
}

#[test]
fn js_paths_jsx_js_priority_preserves_base_and_literal_exemption() {
    extension_priority_controls(&["C86-", "C88-files-priority-exempt-jsx-"]);
}

#[test]
fn js_paths_declaration_js_priority_exception() {
    extension_priority_controls(&["C89-"]);
}
