use super::js_paths_common::*;
use prism::call_graph::CallGraph;
use prism::repo_loader::load_repo;
use std::path::Path;

fn assert_base(root: &Path, app: &str) {
    let loaded = load_repo(root).unwrap();
    assert_eq!(
        outcome(&graph(root), app, "picked"),
        outcome(&CallGraph::build(&loaded.files), app, "picked")
    );
}

#[test]
fn js_paths_repair_config_swap_cache_key() {
    for ext in ["jsx", "tsx"] {
        let d = fixture(
            ext,
            "@first",
            &config(serde_json::json!({"@first":["lib/real"],"@second":["decoy/real"]})),
        );
        let app = format!("app.{ext}");
        write(d.path(), &app, "import { real as picked } from '@first';\nimport { real as other } from '@second';\nexport function run() { picked(); other(); }\n");
        let before = load_repo(d.path()).unwrap();
        assert_alias(
            d.path(),
            &graph(d.path()),
            &app,
            "picked",
            &format!("lib/real.{ext}"),
        );
        write(
            d.path(),
            "tsconfig.json",
            &config(serde_json::json!({"@first":["decoy/real"],"@second":["lib/real"]})),
        );
        let after = load_repo(d.path()).unwrap();
        assert_ne!(before.manifest_hashes, after.manifest_hashes);
        assert_alias(
            d.path(),
            &graph(d.path()),
            &app,
            "picked",
            &format!("decoy/real.{ext}"),
        );
    }
}

#[test]
fn js_paths_repair_barrel_hops() {
    for ext in ["jsx", "tsx"] {
        for form in [
            "export { real } from './leaf';",
            "export * from './leaf';",
            "import { real } from './leaf'; export { real };",
        ] {
            for blocker in [
                "competition",
                "declaration",
                "package",
                "substitution",
                "opaque",
            ] {
                let d = fixture(
                    ext,
                    "@lib",
                    &config(serde_json::json!({"@lib":["lib/barrel"]})),
                );
                let app = format!("app.{ext}");
                write(d.path(), &format!("lib/barrel.{ext}"), form);
                write(
                    d.path(),
                    &format!("lib/leaf.{ext}"),
                    "export function real() { return 1; }\n",
                );
                assert_hop_alias(
                    d.path(),
                    &graph(d.path()),
                    &app,
                    "picked",
                    &format!("lib/leaf.{ext}"),
                );
                write(d.path(), &app, "import { real as picked } from '@lib';\nimport { real as relative } from './lib/barrel';\nexport function run() { picked(); relative(); }\n");
                match blocker {
                    "competition" => write(
                        d.path(),
                        if ext == "tsx" {
                            "lib/leaf.js"
                        } else {
                            "lib/leaf.ts"
                        },
                        "export function real() { return 2; }\n",
                    ),
                    "declaration" => write(
                        d.path(),
                        "lib/leaf.d.ts",
                        "export declare function real(): number;",
                    ),
                    "package" => {
                        std::fs::remove_file(d.path().join(format!("lib/leaf.{ext}"))).unwrap();
                        write(
                            d.path(),
                            &format!("lib/leaf/index.{ext}"),
                            "export function real() { return 1; }\n",
                        );
                        write(
                            d.path(),
                            "lib/leaf/package.json",
                            "{\"types\":\"../../lib/real.tsx\"}",
                        );
                    }
                    "substitution" => write(
                        d.path(),
                        &format!("lib/barrel.{ext}"),
                        &form.replace("./leaf", &format!("./leaf.{ext}")),
                    ),
                    "opaque" => {
                        std::fs::create_dir_all(d.path().join("outside")).unwrap();
                        write(
                            d.path(),
                            "outside/leaf.d.ts",
                            "export declare function real(): number;",
                        );
                        #[cfg(unix)]
                        std::os::unix::fs::symlink(
                            "../outside/leaf.d.ts",
                            d.path().join("lib/leaf.d.ts"),
                        )
                        .unwrap();
                    }
                    _ => unreachable!(),
                }
                // P2 supports an unopposed explicit relative .jsx as well as .tsx.
                if blocker != "substitution" {
                    assert_base(d.path(), &app);
                } else {
                    assert_hit(&graph(d.path()), &app, "picked", &format!("lib/leaf.{ext}"));
                }
                let loaded = load_repo(d.path()).unwrap();
                assert_eq!(
                    outcome(&graph(d.path()), &app, "relative"),
                    outcome(&CallGraph::build(&loaded.files), &app, "relative")
                );
            }
        }
    }
}

#[test]
fn js_paths_repair_dotted_replacement() {
    for ext in ["jsx", "tsx"] {
        for stem in ["user", "user.multi"] {
            for body in [
                "export function real() { return 2; }",
                "export declare function real(): number;",
            ] {
                let d = fixture(
                    ext,
                    "@lib",
                    &config(serde_json::json!({"@lib":[format!("lib/{stem}.service")]})),
                );
                let app = format!("app.{ext}");
                write(
                    d.path(),
                    &format!("lib/{stem}.service.{ext}"),
                    "export function real() { return 1; }\n",
                );
                assert_alias(
                    d.path(),
                    &graph(d.path()),
                    &app,
                    "picked",
                    &format!("lib/{stem}.service.{ext}"),
                );
                write(d.path(), &format!("lib/{stem}.d.service.ts"), body);
                assert_base(d.path(), &app);
            }
        }
    }
}

#[test]
fn js_paths_repair_trailing_separator() {
    for ext in ["jsx", "tsx"] {
        for (spec, target) in [
            ("@lib/", "lib/real"),
            ("@lib", "lib/real/"),
            ("@lib", "lib/real/."),
        ] {
            let d = fixture(ext, spec, &config(serde_json::json!({spec:[target]})));
            assert_base(d.path(), &format!("app.{ext}"));
        }
    }
}

#[test]
fn js_paths_repair_package_js_pass() {
    for ext in ["jsx", "tsx"] {
        for package in [
            "node_modules/utils",
            "node_modules/@types/utils",
            "node_modules/@scope/utils",
            "node_modules/@types/scope__utils",
        ] {
            let spec = if package.contains("scope") {
                "@scope/utils/format"
            } else {
                "utils/format"
            };
            let d = fixture(ext, spec, &config(serde_json::json!({spec:["lib/js"]})));
            write(
                d.path(),
                "lib/js.jsx",
                "export function real() { return 1; }\n",
            );
            assert_alias(
                d.path(),
                &graph(d.path()),
                &format!("app.{ext}"),
                "picked",
                "lib/js.jsx",
            );
            write(
                d.path(),
                &format!("{package}/format.d.ts"),
                "export declare function real(): number;",
            );
            assert_base(d.path(), &format!("app.{ext}"));
        }
    }
}

#[test]
fn js_paths_repair_ambient() {
    for ext in ["jsx", "tsx"] {
        for pattern in ["@lib", "@*", "*"] {
            for declaration_ext in ["ts", "d.ts"] {
                let d = fixture(
                    ext,
                    "@lib",
                    &config(serde_json::json!({"@lib":["lib/real"]})),
                );
                write(d.path(), &format!("types/ambient.{declaration_ext}"), &format!("declare /* comment */ module '{pattern}' {{ export function real(): number; }}"));
                assert_base(d.path(), &format!("app.{ext}"));
            }
        }
    }
}

#[test]
fn js_paths_repair_raw_membership() {
    for ext in ["jsx", "tsx"] {
        for key in ["include", "exclude", "files"] {
            for pattern in [
                "**",
                "./**",
                "src/**/../*",
                "../app.tsx",
                "a..b",
                "?pp.*",
                "[ab]/*",
            ] {
                for inherited in [false, true] {
                    let mut cfg: serde_json::Value =
                        serde_json::from_str(&config(serde_json::json!({"@lib":["lib/real"]})))
                            .unwrap();
                    cfg[key] = serde_json::json!([pattern]);
                    let d = fixture(ext, "@lib", &cfg.to_string());
                    if inherited {
                        write(d.path(), "tsconfig.parent.json", &cfg.to_string());
                        write(
                            d.path(),
                            "tsconfig.json",
                            "{\"extends\":\"./tsconfig.parent.json\"}",
                        );
                    }
                    assert_base(d.path(), &format!("app.{ext}"));
                }
            }
        }
        let d = fixture(
            ext,
            "@lib",
            &config(serde_json::json!({"@lib":["lib/real"]})),
        );
        write(d.path(), "pkg/tsconfig.json", "{\"include\":[\"**/../*\"]}");
        write(
            d.path(),
            &format!("pkg/app.{ext}"),
            "import { real as picked } from '@lib'; export function run() { picked(); }",
        );
        assert_base(d.path(), &format!("pkg/app.{ext}"));
    }
}

#[test]
fn js_paths_repair_output_options() {
    for ext in ["jsx", "tsx"] {
        for key in ["outDir", "declarationDir", "rootDir"] {
            let mut cfg: serde_json::Value =
                serde_json::from_str(&config(serde_json::json!({"@lib":["lib/real"]}))).unwrap();
            cfg["compilerOptions"][key] = serde_json::json!("gen");
            let d = fixture(ext, "@lib", &cfg.to_string());
            assert_base(d.path(), &format!("app.{ext}"));
            cfg["exclude"] = serde_json::json!([]);
            write(d.path(), "tsconfig.json", &cfg.to_string());
            assert_alias(
                d.path(),
                &graph(d.path()),
                &format!("app.{ext}"),
                "picked",
                &format!("lib/real.{ext}"),
            );
        }
    }
}

#[test]
fn js_paths_repair_allowjs_target() {
    for ext in ["jsx", "tsx"] {
        let mut cfg: serde_json::Value =
            serde_json::from_str(&config(serde_json::json!({"@lib":["lib/js"]}))).unwrap();
        cfg["compilerOptions"]["allowJs"] = serde_json::json!(false);
        cfg["files"] = serde_json::json!([format!("app.{ext}")]);
        let d = fixture(ext, "@lib", &cfg.to_string());
        write(
            d.path(),
            "lib/js.jsx",
            "export function real() { return 1; }\n",
        );
        assert_base(d.path(), &format!("app.{ext}"));
    }
}

#[test]
fn js_paths_repair_cache_occupancy() {
    for ext in ["jsx", "tsx"] {
        for (target, blocker, directory) in [
            ("lib/user.service", "lib/user.service", false),
            ("lib/real", "lib/real.d.ts", true),
            ("lib/user.service", "lib/user.d.service.ts", false),
            ("lib/name.txt", "lib/name.txt", false),
        ] {
            let d = fixture(ext, "@lib", &config(serde_json::json!({"@lib":[target]})));
            write(
                d.path(),
                &format!("{target}.{ext}"),
                "export function real() { return 1; }\n",
            );
            let before = load_repo(d.path()).unwrap().manifest_hashes;
            if directory {
                std::fs::create_dir_all(d.path().join(blocker)).unwrap();
            } else {
                write(d.path(), blocker, "occupied");
            }
            let added = load_repo(d.path()).unwrap().manifest_hashes;
            assert_ne!(before, added, "{blocker}");
            if directory {
                std::fs::remove_dir(d.path().join(blocker)).unwrap();
            } else {
                std::fs::remove_file(d.path().join(blocker)).unwrap();
            }
            assert_eq!(before, load_repo(d.path()).unwrap().manifest_hashes);
            write(d.path(), "unrelated.txt", "irrelevant");
            assert_eq!(before, load_repo(d.path()).unwrap().manifest_hashes);
        }
    }
}

#[test]
fn js_paths_repair_package_above_root_and_ts_control() {
    for ext in ["jsx", "tsx"] {
        let outer = tempfile::TempDir::new().unwrap();
        let root = outer.path().join("repo");
        write(
            &root,
            "tsconfig.json",
            &config(serde_json::json!({"utils/format":["lib/real"]})),
        );
        write(
            &root,
            &format!("app.{ext}"),
            "import { real as picked } from 'utils/format'; export function run() { picked(); }",
        );
        write(
            &root,
            "lib/real.jsx",
            "export function real() { return 1; }\n",
        );
        assert_alias(
            &root,
            &graph(&root),
            &format!("app.{ext}"),
            "picked",
            "lib/real.jsx",
        );
        write(
            outer.path(),
            "node_modules/@types/utils/format.d.ts",
            "export declare function real(): number;",
        );
        assert_base(&root, &format!("app.{ext}"));
        std::fs::remove_file(root.join("lib/real.jsx")).unwrap();
        write(
            &root,
            "lib/real.tsx",
            "export function real() { return 1; }\n",
        );
        assert_alias(
            &root,
            &graph(&root),
            &format!("app.{ext}"),
            "picked",
            "lib/real.tsx",
        );
        // Explicit empty types also retain this TS-family alias.
        let mut cfg: serde_json::Value =
            serde_json::from_str(&config(serde_json::json!({"utils/format":["lib/real"]})))
                .unwrap();
        cfg["compilerOptions"]["types"] = serde_json::json!([]);
        write(&root, "tsconfig.json", &cfg.to_string());
        assert_alias(
            &root,
            &graph(&root),
            &format!("app.{ext}"),
            "picked",
            "lib/real.tsx",
        );
    }
}

#[test]
fn js_paths_repair_allowjs_barrel_terminal() {
    for ext in ["jsx", "tsx"] {
        let mut cfg: serde_json::Value =
            serde_json::from_str(&config(serde_json::json!({"@lib":["lib/barrel"]}))).unwrap();
        cfg["compilerOptions"]["allowJs"] = serde_json::json!(false);
        cfg["files"] = serde_json::json!([format!("app.{ext}")]);
        let d = fixture(ext, "@lib", &cfg.to_string());
        write(d.path(), "lib/barrel.tsx", "export { real } from './leaf';");
        write(
            d.path(),
            "lib/leaf.jsx",
            "export function real() { return 1; }\n",
        );
        assert_base(d.path(), &format!("app.{ext}"));
        cfg["compilerOptions"]["allowJs"] = serde_json::json!(true);
        write(d.path(), "tsconfig.json", &cfg.to_string());
        assert_hop_alias(
            d.path(),
            &graph(d.path()),
            &format!("app.{ext}"),
            "picked",
            "lib/leaf.jsx",
        );
    }
}
