//! Slice V: a site resolved through a returned value is one record for its target in every
//! consumer, whatever the binding is spelled: the navigation callers index, `--format callers`,
//! the CPG call edges, `resolved_caller_edges`, and `nav call-stats`.
use prism::ast::ParsedFile;
use prism::languages::Language;
use prism::navigation::types::SymbolRef;
use prism::navigation::{queries, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use std::collections::BTreeMap;
use std::sync::Arc;

fn session(files: &[(&str, &str)]) -> NavigationSession {
    let dir = tempfile::tempdir().unwrap();
    for (name, src) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, src).unwrap();
    }
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    NavigationSession { repo, index }
}

fn caller_names(s: &NavigationSession, target: &str, file: &str) -> Vec<String> {
    let ev = queries::callers(s, Some(target), Some(file), None, 1).unwrap();
    ev.items
        .iter()
        .filter_map(|i| match &i.symbol {
            Some(SymbolRef::Function { name, .. }) => Some(name.clone()),
            _ => None,
        })
        .collect()
}

/// `(label, hook source, caller source, name of the caller function, target name)`. Hook lines:
/// the target function is on lines 2-4 of `hooks.ts`.
fn spellings() -> Vec<(&'static str, String, String, &'static str, &'static str)> {
    let hook = |ret: &str, name: &str| {
        format!("export function useThing() {{\n  function {name}() {{\n    return 1;\n  }}\n  {ret}\n}}\n")
    };
    let head = "import { useThing } from './hooks';\n";
    vec![
        (
            "key spelled like the function",
            hook("return { save };", "save"),
            format!("{head}export function viaShorthand() {{\n  const {{ save }} = useThing();\n  return save();\n}}\n"),
            "viaShorthand",
            "save",
        ),
        (
            "renamed key",
            hook("return { run: doSave };", "doSave"),
            format!("{head}export function viaRenamedKey() {{\n  const {{ run }} = useThing();\n  return run();\n}}\n"),
            "viaRenamedKey",
            "doSave",
        ),
        (
            "renamed binding",
            hook("return { doSave };", "doSave"),
            format!("{head}export function viaRenamedBinding() {{\n  const {{ doSave: go }} = useThing();\n  return go();\n}}\n"),
            "viaRenamedBinding",
            "doSave",
        ),
        (
            "array slot",
            hook("return [doSave];", "doSave"),
            format!("{head}export function viaSlot() {{\n  const [go] = useThing();\n  return go();\n}}\n"),
            "viaSlot",
            "doSave",
        ),
        (
            "whole value",
            hook("return doSave;", "doSave"),
            format!("{head}export function viaWhole() {{\n  const go = useThing();\n  return go();\n}}\n"),
            "viaWhole",
            "doSave",
        ),
    ]
}

#[test]
fn the_callers_index_lists_a_returned_value_caller_once() {
    for (label, hook, app, who, target) in spellings() {
        let s = session(&[("hooks.ts", &hook), ("app.ts", &app)]);
        assert_eq!(caller_names(&s, target, "hooks.ts"), [who], "{label}");
    }
}

#[test]
fn format_callers_lists_a_returned_value_caller_once() {
    use prism::cpg::CpgContext;
    use prism::diff::DiffInput;
    for (label, hook, app, who, _) in spellings() {
        let mut files = BTreeMap::new();
        for (path, text) in [("hooks.ts", &hook), ("app.ts", &app)] {
            files.insert(
                path.to_string(),
                ParsedFile::parse(path, text, Language::TypeScript).unwrap(),
            );
        }
        let ctx = CpgContext::build(&files, None);
        // Line 3 is `return 1;` inside the target function in every hook spelling.
        let diff = DiffInput::parse_unified_diff(
            "--- a/hooks.ts\n+++ b/hooks.ts\n@@ -3,1 +3,1 @@\n-    return 0;\n+    return 1;\n",
        );
        let out = prism::output::to_callers_output(&ctx, &diff, 1);
        let names: Vec<_> = out
            .callers
            .iter()
            .flat_map(|e| e.callers.iter().map(|c| c.name.clone()))
            .collect();
        assert_eq!(names, [who], "{label}: {out:?}");
    }
}

#[test]
fn a_returned_value_call_is_one_cpg_edge_and_one_resolved_caller_row() {
    use prism::cpg::{CpgContext, CpgEdge, CpgNode};
    for (label, hook, app, who, target) in spellings() {
        let mut files = BTreeMap::new();
        for (path, text) in [("hooks.ts", &hook), ("app.ts", &app)] {
            files.insert(
                path.to_string(),
                ParsedFile::parse(path, text, Language::TypeScript).unwrap(),
            );
        }
        let ctx = CpgContext::build(&files, None);
        let g = &ctx.cpg.graph;
        let edges = g
            .edge_indices()
            .filter(|&e| matches!(g[e], CpgEdge::Call(_)))
            .filter(|&e| {
                let (from, to) = g.edge_endpoints(e).unwrap();
                matches!(g.node_weight(from), Some(CpgNode::Function { name, .. }) if name == who)
                    && matches!(g.node_weight(to), Some(CpgNode::Function { name, .. }) if name == target)
            })
            .count();
        let f = ctx.cpg.call_graph.functions[target]
            .iter()
            .find(|f| f.file == "hooks.ts")
            .unwrap()
            .clone();
        let rows = ctx.cpg.call_graph.resolved_caller_edges(&f).len();
        assert_eq!(
            (edges, rows),
            (1, 1),
            "{label}: (CPG Call edges {who} -> {target}, resolved_caller_edges rows)"
        );
    }
}

#[test]
fn call_stats_counts_returned_values_by_outcome_and_keeps_the_base_reason_when_unproven() {
    use super::js_alias_follow_test::graph_n;
    let hooks =
        "function a() {\n  return 1;\n}\nexport function useGood() {\n  return { go: a };\n}\n\
        export function useBad(p) {\n  return { go: p };\n}\n";
    let app = "import { useGood, useBad } from './hooks';\nexport function run() {\n  \
        const { go } = useGood();\n  const { stop } = useBad();\n  const [x] = useBad(() => 1);\n  \
        return [go(), stop(), x()];\n}\n";
    let cg = graph_n(&[("hooks.ts", hooks), ("app.ts", app)]);
    let stats = queries::call_stats(&cg);
    assert_eq!(
        stats["js_alias_follow"],
        serde_json::json!({"returned_proven": 1, "returned_kept_main": 2})
    );
    // The two unproven sites keep the base's reasons: `alias` and `may_call` (an inline
    // function argument).
    assert_eq!(
        stats["local_binding_may_call"],
        serde_json::json!({"alias": 1, "may_call": 1})
    );
}
