//! #664, round 1 fold: a consumer that reaches a producer through a tsconfig `paths` alias. The matrix helper builds
//! without the path-resolution snapshot, so this fixture loads a real directory. The paths route admits ESM
//! producers only: a CommonJS producer (pure or mixed) is unresolved on base f4c6297d and on head, so there is no
//! Exact edge for the #664 rule to remove here; the test pins that and that the ESM side of a mixed file still binds.
use super::js_paths_common::*;

fn run_case(ext: &str, producer: &str) -> (tempfile::TempDir, serde_json::Value) {
    let d = fixture(
        ext,
        "@lib",
        &config(serde_json::json!({"@lib":["lib/real"]})),
    );
    write(d.path(), &format!("lib/real.{ext}"), producer);
    let r = outcome(&graph(d.path()), &format!("app.{ext}"), "picked");
    (d, r)
}

fn exact_import_member(r: &serde_json::Value) -> bool {
    r["resolved_targets"].as_array().is_some_and(|t| {
        t.iter()
            .any(|x| x["confidence"] == "exact" && x["kind"] == "import_member")
    })
}

#[test]
fn paths_alias_never_binds_a_commonjs_claim_and_keeps_the_esm_side() {
    for ext in ["jsx", "tsx"] {
        let cjs = "function real() { return 1; }\nexports.real = real;\n";
        let mixed = "function real() { return 1; }\nexports.real = real;\nexport const z = 1;\n";
        for (label, src) in [("pure-cjs", cjs), ("mixed", mixed)] {
            let (_d, r) = run_case(ext, src);
            assert!(!exact_import_member(&r), "{ext} {label}: {r}");
        }
        // The ESM export of a mixed file is unaffected by the refusal of its CommonJS writes.
        let esm_side = "export function real() { return 1; }\nexports.other = real;\n";
        let (d, _) = run_case(ext, esm_side);
        assert_hit(
            &graph(d.path()),
            &format!("app.{ext}"),
            "picked",
            &format!("lib/real.{ext}"),
        );
    }
}
