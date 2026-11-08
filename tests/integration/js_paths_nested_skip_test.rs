//! #739 review fold: the tsconfig-paths snapshot walk applies the loader's rule,
//! so a nested `build/` or `dist/` is an ordinary directory (its nearer
//! tsconfig is seen, its files are probeable) and only the root one is opaque.
use super::js_paths_common::*;
use std::path::Path;
use tempfile::TempDir;

fn cfg(base: &str, target: &str) -> String {
    serde_json::json!({"compilerOptions":{"moduleResolution":"node","allowJs":true,
        "baseUrl":base,"paths":{"@lib":[target]}},"include":["**/*"]})
    .to_string()
}

fn two_libs(root: &Path) {
    write(root, "lib/right.ts", "export function real(){return 1;}");
    write(root, "lib/wrong.ts", "export function real(){return 2;}");
}

fn main_ts(root: &Path, at: &str) {
    write(
        root,
        at,
        "import {real as picked} from '@lib'; export function run(){picked();}",
    );
}

#[test]
fn nearest_config_inside_nested_build_or_dist_wins_over_the_root_config() {
    for name in ["build", "dist"] {
        let d = TempDir::new().unwrap();
        let root = d.path();
        two_libs(root);
        write(root, "tsconfig.json", &cfg(".", "lib/wrong.ts"));
        write(
            root,
            &format!("src/{name}/tsconfig.json"),
            &cfg("../..", "lib/right.ts"),
        );
        let main = format!("src/{name}/main.ts");
        main_ts(root, &main);
        assert_hit(&graph(root), &main, "picked", "lib/right.ts");
    }
}

#[test]
fn root_build_or_dist_config_is_still_opaque() {
    // A config under the root `build/` is not a candidate for files there: the
    // root-level rule keeps the directory unindexed, so nothing is resolved from it.
    for name in ["build", "dist"] {
        let d = TempDir::new().unwrap();
        let root = d.path();
        two_libs(root);
        write(root, "tsconfig.json", &cfg(".", "lib/wrong.ts"));
        write(
            root,
            &format!("{name}/tsconfig.json"),
            &cfg("..", "lib/right.ts"),
        );
        main_ts(root, &format!("{name}/main.ts"));
        let r = prism::repo_loader::load_repo(root).unwrap();
        assert!(!r.files.contains_key(&format!("{name}/main.ts")));
    }
}

#[test]
fn alias_target_inside_a_nested_build_or_dist_resolves() {
    for name in ["build", "dist"] {
        let d = TempDir::new().unwrap();
        let root = d.path();
        let target = format!("src/{name}/real.ts");
        write(root, "tsconfig.json", &cfg(".", &target));
        write(root, &target, "export function real(){return 1;}");
        main_ts(root, "app.ts");
        assert_hit(&graph(root), "app.ts", "picked", &target);
    }
}

#[test]
fn malformed_nearer_config_in_nested_build_or_dist_refuses_not_the_root_config() {
    for name in ["build", "dist"] {
        let d = TempDir::new().unwrap();
        let root = d.path();
        two_libs(root);
        write(root, "tsconfig.json", &cfg(".", "lib/wrong.ts"));
        write(root, &format!("src/{name}/tsconfig.json"), "{ not json");
        let main = format!("src/{name}/main.ts");
        main_ts(root, &main);
        let r = outcome(&graph(root), &main, "picked");
        let targets = r["resolved_targets"].as_array().unwrap();
        assert!(
            targets
                .iter()
                .all(|t| t["function_id"]["file"] != "lib/wrong.ts"),
            "{r}"
        );
    }
}
