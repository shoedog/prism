use prism::call_graph::CallGraph;
use prism::repo_loader::load_repo;
use std::path::Path;
use tempfile::TempDir;
pub(super) fn write(root: &Path, p: &str, text: &str) {
    let p = root.join(p);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}
pub(super) fn graph(root: &Path) -> CallGraph {
    let r = load_repo(root).unwrap();
    CallGraph::build_with_scope_graph_inputs(&r.files, r.scope_graph_inputs.as_ref())
}
pub(super) fn outcome(g: &CallGraph, file: &str, name: &str) -> serde_json::Value {
    let rows = prism::navigation::queries::call_site_dump(g);
    let r = rows
        .into_iter()
        .find(|r| r["caller"]["file"] == file && r["callee_text"] == name)
        .unwrap();
    serde_json::from_str(&r.to_string()).unwrap()
}
pub(super) fn assert_hit(g: &CallGraph, file: &str, callee: &str, target: &str) {
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
pub(super) fn assert_alias(_root: &Path, g: &CallGraph, file: &str, callee: &str, target: &str) {
    assert_hit(g, file, callee, target);
}
// Cut 2 still refuses JS-family relative export hops.
pub(super) fn assert_hop_alias(root: &Path, g: &CallGraph, file: &str, callee: &str, target: &str) {
    if [".js", ".jsx", ".mjs", ".cjs"]
        .iter()
        .any(|ext| target.ends_with(ext))
    {
        let loaded = load_repo(root).unwrap();
        assert_eq!(
            outcome(g, file, callee),
            outcome(&CallGraph::build(&loaded.files), file, callee)
        );
    } else {
        assert_hit(g, file, callee, target);
    }
}
pub(super) fn config(paths: serde_json::Value) -> String {
    serde_json::json!({"compilerOptions":{"moduleResolution":"node","allowJs":true,"baseUrl":".","paths":paths},"include":["**/*"]}).to_string()
}
pub(super) fn fixture(ext: &str, spec: &str, cfg: &str) -> TempDir {
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
