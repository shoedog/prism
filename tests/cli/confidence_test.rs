use assert_cmd::Command;
use serde_json::Value;

fn bin() -> Command {
    Command::cargo_bin("prism").unwrap()
}

fn same_name_collision_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.rs"), "pub fn helper() -> i32 { 1 }\n").unwrap();
    std::fs::write(dir.path().join("b.rs"), "pub fn helper() -> i32 { 2 }\n").unwrap();
    std::fs::write(
        dir.path().join("main.rs"),
        r#"
mod a;
mod b;

fn exact_caller() -> i32 {
    a::helper()
}

fn name_only_caller() -> i32 {
    helper()
}
"#,
    )
    .unwrap();
    dir
}

fn run_json(args: &[&str]) -> (Vec<u8>, Value) {
    let out = bin().args(args).output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value = serde_json::from_slice(&out.stdout).unwrap();
    (out.stdout, value)
}

#[test]
fn nav_callers_confidence_exact_filters_nameonly_edges() {
    let repo = same_name_collision_repo();
    let repo = repo.path().to_str().unwrap();

    let (_, all) = run_json(&[
        "nav",
        "--no-cache",
        "callers",
        "--repo",
        repo,
        "--symbol",
        "helper",
        "--file",
        "a.rs",
        "--format",
        "json",
    ]);
    let (_, exact) = run_json(&[
        "nav",
        "--no-cache",
        "callers",
        "--repo",
        repo,
        "--symbol",
        "helper",
        "--file",
        "a.rs",
        "--confidence",
        "exact",
        "--format",
        "json",
    ]);

    let all_items = all["items"].as_array().unwrap();
    let exact_items = exact["items"].as_array().unwrap();
    assert!(
        exact_items.len() < all_items.len(),
        "exact should drop at least one NameOnly edge; all={all_items:#?} exact={exact_items:#?}"
    );
    assert!(
        exact_items
            .iter()
            .all(|item| item["score"].as_f64() == Some(1.0)),
        "exact caller evidence must contain only Exact score-1.0 edges: {exact_items:#?}"
    );
    assert!(
        all_items
            .iter()
            .any(|item| item["score"].as_f64().map_or(false, |score| score < 1.0)),
        "fixture must include a demoted all-mode edge: {all_items:#?}"
    );
}

#[test]
fn nav_confidence_absent_matches_all_byte_for_byte() {
    let repo = same_name_collision_repo();
    let repo = repo.path().to_str().unwrap();

    let (implicit_callers, _) = run_json(&[
        "nav",
        "--no-cache",
        "callers",
        "--repo",
        repo,
        "--symbol",
        "helper",
        "--file",
        "a.rs",
        "--format",
        "json",
    ]);
    let (explicit_callers, _) = run_json(&[
        "nav",
        "--no-cache",
        "callers",
        "--repo",
        repo,
        "--symbol",
        "helper",
        "--file",
        "a.rs",
        "--confidence",
        "all",
        "--format",
        "json",
    ]);
    assert_eq!(implicit_callers, explicit_callers);

    let (implicit_callees, _) = run_json(&[
        "nav",
        "--no-cache",
        "callees",
        "--repo",
        repo,
        "--symbol",
        "exact_caller",
        "--format",
        "json",
    ]);
    let (explicit_callees, _) = run_json(&[
        "nav",
        "--no-cache",
        "callees",
        "--repo",
        repo,
        "--symbol",
        "exact_caller",
        "--confidence",
        "all",
        "--format",
        "json",
    ]);
    assert_eq!(implicit_callees, explicit_callees);
}

#[test]
fn finding_confidence_flags_do_not_extend_the_nav_wire_shape() {
    for flag in ["--min-confidence", "--resolution"] {
        let value = if flag == "--min-confidence" {
            "exact"
        } else {
            "scoped"
        };
        let output = bin()
            .args(["nav", "callers", flag, value])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{flag}: {output:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("unexpected argument"),
            "{flag}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
