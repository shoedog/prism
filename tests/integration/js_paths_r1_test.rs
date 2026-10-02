// Split from js_paths_test.rs to respect the repo's 600-line file cap (CLAUDE.md).
// Covers require/alias preservation, unmatched target stars, unproven position
// states, the R1/R2 fixture-driven controls and extension-priority controls.
use super::js_paths_common::*;
use prism::call_graph::CallGraph;
use prism::repo_loader::load_repo;
use tempfile::TempDir;
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
            let assert = if name.starts_with("C56") || name.starts_with("C61") {
                assert_hop_alias
            } else {
                assert_alias
            };
            assert(
                d.path(),
                &head,
                app,
                "picked",
                case["target"].as_str().unwrap(),
            );
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
            assert_alias(
                d.path(),
                &head,
                app,
                "picked",
                case["target"].as_str().unwrap(),
            );
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
