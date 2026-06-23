use super::error::query_error_result;
use super::evidence_view::{shape_navigation_result, NavigationViewKind};
use super::input::{
    parse_callees, parse_callers, parse_ego, parse_module_deps, parse_nodes_at, parse_repo_map,
    SeedInput, Verbosity as InputVerbosity,
};
use super::output::{McpToolResult, Verbosity};
use super::registry::{ToolAnnotations, ToolContext, ToolDescriptor, ToolRegistry};
use crate::navigation::types::Evidence;
use crate::navigation::{module_graph, queries};
use serde_json::json;

const SNAPSHOT_NOTICE: &str = "Results reflect the repository snapshot loaded when prism-mcp started. If indexed files change during the server session, Prism marks tool results with stale-index metadata and warnings; restart/re-add the MCP server or use CLI nav for a fresh snapshot.";

pub fn register_all(r: &mut ToolRegistry) {
    r.register(tool_with_handler(
        "nav_nodes_at",
        "Nodes At",
        "Finds first-class CPG evidence at one repository file and 1-indexed line. Use when you need the symbol or enclosing function for a precise source location; do NOT use for name-based caller/callee expansion. Inputs are file string, line integer >= 1, and optional verbosity concise or detailed. Returns Evidence items with warnings for skipped or unknown files. Example: {\"file\":\"src/main.rs\",\"line\":42,\"verbosity\":\"detailed\"}.",
        nodes_at_schema(),
        Box::new(nav_nodes_at),
    ));
    r.register(tool_with_handler(
        "nav_callers",
        "Callers",
        "Finds functions that call a seed function, expanding backward through call edges. Use when you need incoming call evidence; do NOT use when you want outgoing callees or a module-level dependency map. Seed grammar is {\"kind\":\"symbol\",\"name\":\"run\",\"file\":\"src/main.rs\"} or {\"kind\":\"loc\",\"file\":\"src/main.rs\",\"line\":42}; depth is 0..5, max_results is 1..1000, verbosity is concise or detailed. Returns Evidence items for caller functions. Example: {\"seed\":{\"kind\":\"symbol\",\"name\":\"run\",\"file\":\"src/main.rs\"},\"depth\":2,\"max_results\":25,\"verbosity\":\"concise\"}.",
        callers_schema(),
        Box::new(nav_callers),
    ));
    r.register(tool_with_handler(
        "nav_callees",
        "Callees",
        "Finds functions called by a seed function, expanding forward through call edges. Use when you need outgoing call evidence; do NOT use for incoming callers or whole-repo module edges. Seed grammar is {\"kind\":\"symbol\",\"name\":\"handler\",\"file\":\"src/server.rs\"} or {\"kind\":\"loc\",\"file\":\"src/server.rs\",\"line\":18}; depth is 0..5, max_results is 1..1000, verbosity is concise or detailed. Returns Evidence items for callee functions and unresolved call sites. Example: {\"seed\":{\"kind\":\"loc\",\"file\":\"src/server.rs\",\"line\":18},\"depth\":1,\"max_results\":50,\"verbosity\":\"detailed\"}.",
        callees_schema(),
        Box::new(nav_callees),
    ));
    r.register(tool_with_handler(
        "nav_ego_graph",
        "Ego Graph",
        "Builds a local CPG graph around a seed symbol or location. Use when you need nearby structural, call, return, data-flow, control-flow, contains, or field ownership edges; do NOT use for a flat caller/callee list. Seed grammar is {\"kind\":\"symbol\",\"name\":\"parse\",\"file\":\"src/parser.rs\"} or {\"kind\":\"loc\",\"file\":\"src/parser.rs\",\"line\":77}; hops is 0..5, max_results is 1..1000 nodes, edges contains Call, Return, DataFlow, Contains, ControlFlow, or FieldOf. Returns Evidence with graph nodes and edges. Example: {\"seed\":{\"kind\":\"symbol\",\"name\":\"parse\",\"file\":\"src/parser.rs\"},\"hops\":2,\"edges\":[\"Call\",\"DataFlow\"],\"max_results\":100}.",
        ego_graph_schema(),
        Box::new(nav_ego_graph),
    ));
    r.register(tool_with_handler(
        "nav_module_deps",
        "Module Dependencies",
        "Lists outbound module dependencies for one repository file. Use when you need target files reached by resolved cross-file calls or unresolved import labels; do NOT use for symbol-level caller/callee exploration. Inputs are file string, optional max_results 1..1000, and optional verbosity concise or detailed. Returns Evidence items whose locations identify dependency target files plus warnings for unresolved modules or skipped paths. Example: {\"file\":\"src/main.rs\",\"max_results\":20,\"verbosity\":\"concise\"}.",
        module_deps_schema(),
        Box::new(nav_module_deps),
    ));
    r.register(tool_with_handler(
        "nav_repo_map",
        "Repo Map",
        "Builds the whole-repository module graph from indexed files and resolved cross-file call dependencies. Use when you need a file-level overview; do NOT use when you need exact symbols at a line or caller/callee chains for one function. Input is optional max_results 1..1000 bounding graph nodes. Returns Evidence with graph nodes for files and ModuleDep edges. Example: {\"max_results\":200}.",
        repo_map_schema(),
        Box::new(nav_repo_map),
    ));
}

fn tool_with_handler(
    name: &'static str,
    title: &str,
    description: &str,
    input_schema: serde_json::Value,
    handler: Box<super::registry::ToolHandler>,
) -> ToolDescriptor {
    ToolDescriptor {
        name,
        description: format!("{description} {SNAPSHOT_NOTICE}"),
        input_schema,
        annotations: ToolAnnotations::read_only(title),
        handler,
    }
}

fn nav_nodes_at(ctx: &ToolContext<'_>, args: &serde_json::Value) -> McpToolResult {
    let input = match parse_nodes_at(args) {
        Ok(input) => input,
        Err(error) => return error.into_result(),
    };
    let evidence = queries::nodes_at(ctx.session, &input.file, input.line);
    let total = evidence.items.len();
    shape_navigation_result(
        ctx.session,
        &evidence,
        evidence.clone(),
        total,
        false,
        output_verbosity(input.verbosity),
        ctx.cap,
        input.view,
        NavigationViewKind::NodesAt,
    )
}

fn nav_callers(ctx: &ToolContext<'_>, args: &serde_json::Value) -> McpToolResult {
    let input = match parse_callers(args) {
        Ok(input) => input,
        Err(error) => return error.into_result(),
    };
    let (symbol, file, location) = input.seed.to_triple();
    let evidence =
        match queries::callers(ctx.session, symbol, file, location.as_deref(), input.depth) {
            Ok(evidence) => evidence,
            Err(error) => return query_error_result(error),
        };
    let (canonical, total, max_results_clipped) = clip_flat(evidence.clone(), input.max_results);
    shape_navigation_result(
        ctx.session,
        &evidence,
        canonical,
        total,
        max_results_clipped,
        output_verbosity(input.verbosity),
        ctx.cap,
        input.view,
        NavigationViewKind::Callers,
    )
}

fn nav_callees(ctx: &ToolContext<'_>, args: &serde_json::Value) -> McpToolResult {
    let input = match parse_callees(args) {
        Ok(input) => input,
        Err(error) => return error.into_result(),
    };
    let view_kind = NavigationViewKind::Callees {
        depth: input.depth,
        seed_file: seed_file(&input.seed),
    };
    let (symbol, file, location) = input.seed.to_triple();
    let evidence =
        match queries::callees(ctx.session, symbol, file, location.as_deref(), input.depth) {
            Ok(evidence) => evidence,
            Err(error) => return query_error_result(error),
        };
    let (canonical, total, max_results_clipped) = clip_flat(evidence.clone(), input.max_results);
    shape_navigation_result(
        ctx.session,
        &evidence,
        canonical,
        total,
        max_results_clipped,
        output_verbosity(input.verbosity),
        ctx.cap,
        input.view,
        view_kind,
    )
}

fn nav_ego_graph(ctx: &ToolContext<'_>, args: &serde_json::Value) -> McpToolResult {
    let input = match parse_ego(args) {
        Ok(input) => input,
        Err(error) => return error.into_result(),
    };
    let (symbol, file, location) = input.seed.to_triple();
    let edges = input.edges.iter().map(String::as_str).collect::<Vec<_>>();
    let evidence = match queries::ego_graph(
        ctx.session,
        symbol,
        file,
        location.as_deref(),
        input.hops,
        &edges,
    ) {
        Ok(evidence) => evidence,
        Err(error) => return query_error_result(error),
    };
    let (canonical, total, max_results_clipped) = clip_graph(evidence.clone(), input.max_results);
    shape_navigation_result(
        ctx.session,
        &evidence,
        canonical,
        total,
        max_results_clipped,
        Verbosity::Concise,
        ctx.cap,
        input.view,
        NavigationViewKind::EgoGraph,
    )
}

fn nav_module_deps(ctx: &ToolContext<'_>, args: &serde_json::Value) -> McpToolResult {
    let input = match parse_module_deps(args) {
        Ok(input) => input,
        Err(error) => return error.into_result(),
    };
    let evidence = module_graph::module_deps(ctx.session, &input.file);
    let (canonical, total, max_results_clipped) = clip_flat(evidence.clone(), input.max_results);
    shape_navigation_result(
        ctx.session,
        &evidence,
        canonical,
        total,
        max_results_clipped,
        output_verbosity(input.verbosity),
        ctx.cap,
        input.view,
        NavigationViewKind::ModuleDeps {
            file: input.file.clone(),
        },
    )
}

fn nav_repo_map(ctx: &ToolContext<'_>, args: &serde_json::Value) -> McpToolResult {
    let input = match parse_repo_map(args) {
        Ok(input) => input,
        Err(error) => return error.into_result(),
    };
    let evidence = module_graph::repo_map(ctx.session);
    let (canonical, total, max_results_clipped) = clip_graph(evidence.clone(), input.max_results);
    shape_navigation_result(
        ctx.session,
        &evidence,
        canonical,
        total,
        max_results_clipped,
        Verbosity::Concise,
        ctx.cap,
        input.view,
        NavigationViewKind::RepoMap,
    )
}

fn seed_file(seed: &SeedInput) -> Option<String> {
    match seed {
        SeedInput::Symbol {
            file: Some(file), ..
        }
        | SeedInput::Loc { file, .. } => Some(file.clone()),
        SeedInput::Symbol { file: None, .. } => None,
    }
}

fn clip_flat(mut evidence: Evidence, max_results: usize) -> (Evidence, usize, bool) {
    let total = evidence.items.len();
    let n = max_results.min(total);
    evidence.items.truncate(n);
    (evidence, total, n < total)
}

fn clip_graph(mut evidence: Evidence, max_results: usize) -> (Evidence, usize, bool) {
    let total = evidence
        .graph
        .as_ref()
        .map(|graph| graph.nodes.len())
        .unwrap_or(0);
    let n = max_results.min(total);
    if let Some(graph) = &mut evidence.graph {
        graph.nodes.truncate(n);
        graph.edges.retain(|edge| edge.from < n && edge.to < n);
    }
    (evidence, total, n < total)
}

fn output_verbosity(verbosity: InputVerbosity) -> Verbosity {
    match verbosity {
        InputVerbosity::Concise => Verbosity::Concise,
        InputVerbosity::Detailed => Verbosity::Detailed,
    }
}

fn seed_schema() -> serde_json::Value {
    json!({
        "oneOf": [
            {
                "type": "object",
            "additionalProperties": false,
                "properties": {
                    "kind": { "const": "symbol" },
                    "name": { "type": "string" },
                    "file": { "type": "string" }
                },
                "required": ["kind", "name"]
            },
            {
                "type": "object",
            "additionalProperties": false,
                "properties": {
                    "kind": { "const": "loc" },
                    "file": { "type": "string" },
                    "line": { "type": "integer", "minimum": 1 }
                },
                "required": ["kind", "file", "line"]
            }
        ]
    })
}

fn verbosity_schema() -> serde_json::Value {
    json!({ "enum": ["concise", "detailed"] })
}

fn format_schema() -> serde_json::Value {
    json!({ "enum": ["canonical_json", "agent_markdown", "agent_json"] })
}

fn profile_schema(values: &[&str]) -> serde_json::Value {
    json!({ "enum": values })
}

fn snippets_schema() -> serde_json::Value {
    json!({ "enum": ["none", "line", "symbol_header"] })
}

fn group_by_schema() -> serde_json::Value {
    json!({ "enum": ["none", "file", "symbol"] })
}

fn max_view_bytes_schema() -> serde_json::Value {
    json!({ "type": "integer", "minimum": 1, "maximum": 80000 })
}

fn max_results_schema() -> serde_json::Value {
    json!({ "type": "integer", "minimum": 1, "maximum": 1000 })
}

fn depth_schema() -> serde_json::Value {
    json!({ "type": "integer", "minimum": 0, "maximum": 5 })
}

fn hops_schema() -> serde_json::Value {
    json!({ "type": "integer", "minimum": 0, "maximum": 5 })
}

fn edges_schema() -> serde_json::Value {
    json!({
        "type": "array",
        "items": {
            "enum": ["Call", "Return", "DataFlow", "Contains", "ControlFlow", "FieldOf"]
        }
    })
}

fn nodes_at_schema() -> serde_json::Value {
    json!({
        "type": "object",
            "additionalProperties": false,
        "properties": {
            "file": { "type": "string" },
            "line": { "type": "integer", "minimum": 1 },
            "verbosity": verbosity_schema(),
            "format": format_schema(),
            "profile": profile_schema(&["seed", "edit_context", "audit"]),
            "snippets": snippets_schema(),
            "group_by": group_by_schema(),
            "max_view_bytes": max_view_bytes_schema()
        },
        "required": ["file", "line"]
    })
}

fn callers_schema() -> serde_json::Value {
    json!({
        "type": "object",
            "additionalProperties": false,
        "properties": {
            "seed": seed_schema(),
            "depth": depth_schema(),
            "max_results": max_results_schema(),
            "verbosity": verbosity_schema(),
            "format": format_schema(),
            "profile": profile_schema(&["impact", "edit_context", "audit"]),
            "snippets": snippets_schema(),
            "group_by": group_by_schema(),
            "max_view_bytes": max_view_bytes_schema()
        },
        "required": ["seed"]
    })
}

fn callees_schema() -> serde_json::Value {
    json!({
        "type": "object",
            "additionalProperties": false,
        "properties": {
            "seed": seed_schema(),
            "depth": depth_schema(),
            "max_results": max_results_schema(),
            "verbosity": verbosity_schema(),
            "format": format_schema(),
            "profile": profile_schema(&["dependencies", "edit_context", "audit"]),
            "snippets": snippets_schema(),
            "group_by": group_by_schema(),
            "max_view_bytes": max_view_bytes_schema()
        },
        "required": ["seed"]
    })
}

fn ego_graph_schema() -> serde_json::Value {
    json!({
        "type": "object",
            "additionalProperties": false,
        "properties": {
            "seed": seed_schema(),
            "hops": hops_schema(),
            "edges": edges_schema(),
            "max_results": max_results_schema(),
            "format": format_schema(),
            "profile": profile_schema(&["graph", "audit"]),
            "snippets": snippets_schema(),
            "group_by": group_by_schema(),
            "max_view_bytes": max_view_bytes_schema()
        },
        "required": ["seed"]
    })
}

fn module_deps_schema() -> serde_json::Value {
    json!({
        "type": "object",
            "additionalProperties": false,
        "properties": {
            "file": { "type": "string" },
            "max_results": max_results_schema(),
            "verbosity": verbosity_schema(),
            "format": format_schema(),
            "profile": profile_schema(&["dependencies", "orientation", "audit"]),
            "snippets": snippets_schema(),
            "group_by": group_by_schema(),
            "max_view_bytes": max_view_bytes_schema()
        },
        "required": ["file"]
    })
}

fn repo_map_schema() -> serde_json::Value {
    json!({
        "type": "object",
            "additionalProperties": false,
        "properties": {
            "max_results": max_results_schema(),
            "format": format_schema(),
            "profile": profile_schema(&["orientation", "graph", "audit"]),
            "snippets": snippets_schema(),
            "group_by": group_by_schema(),
            "max_view_bytes": max_view_bytes_schema()
        }
    })
}

#[cfg(test)]
pub(crate) mod test_support {
    use crate::mcp::session::{CacheMode, ServerConfig, SessionProvider};
    use crate::navigation::NavigationSession;

    pub(crate) fn session(files: &[(&str, &str)]) -> NavigationSession {
        let dir = tempfile::tempdir().unwrap();
        for (name, source) in files {
            let path = dir.path().join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(path, source).unwrap();
        }
        let cfg = ServerConfig {
            repo_root: dir.path().to_path_buf(),
            cache: CacheMode::NoCache,
        };
        let provider = SessionProvider::bootstrap(&cfg).expect("bootstrap");
        NavigationSession {
            repo: provider.session().repo.clone(),
            index: provider.session().index.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn nodes_at_ok() {
        let s = test_support::session(&[("a.py", "def f():\n    return 1\n")]);
        let out = (ToolRegistry::nav_v1().get("nav_nodes_at").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({"file":"a.py","line":1}),
        );
        assert!(!out.is_error);
        let v: serde_json::Value = serde_json::from_str(&out.content_text).unwrap();
        assert_eq!(v["query"], "nodes-at:a.py:1");
    }

    #[test]
    fn nodes_at_bad_line_iserror() {
        let s = test_support::session(&[("a.py", "x=1\n")]);
        let out = (ToolRegistry::nav_v1().get("nav_nodes_at").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({"file":"a.py","line":0}),
        );
        assert!(out.is_error);
    }

    #[test]
    fn nodes_at_escaping_file_is_empty_skippedpath() {
        let s = test_support::session(&[("a.py", "def f():\n    return 1\n")]);
        let out = (ToolRegistry::nav_v1().get("nav_nodes_at").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({"file":"/etc/passwd","line":1}),
        );
        assert!(!out.is_error);
        let v: serde_json::Value = serde_json::from_str(&out.content_text).unwrap();
        assert!(v["items"].as_array().unwrap().is_empty());
        assert!(v["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["kind"] == "SkippedPath"));
    }

    #[test]
    fn callees_scoped_seed() {
        let s = test_support::session(&[
            ("util.py", "def helper():\n    return 1\n"),
            (
                "main.py",
                "from util import helper\n\ndef run():\n    return helper()\n",
            ),
        ]);
        let out = (ToolRegistry::nav_v1().get("nav_callees").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({"seed":{"kind":"symbol","name":"run","file":"main.py"}}),
        );
        let v: serde_json::Value = serde_json::from_str(&out.content_text).unwrap();
        assert!(v["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["symbol"]["Function"]["name"] == "helper"));
    }

    #[test]
    fn callers_ambiguous_seed_iserror() {
        let s = test_support::session(&[
            ("a.py", "def run():\n    return 1\n"),
            ("b.py", "def run():\n    return 2\n"),
        ]);
        let out = (ToolRegistry::nav_v1().get("nav_callers").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({"seed":{"kind":"symbol","name":"run"}}),
        );
        assert!(out.is_error);
    }

    #[test]
    fn ego_escaping_seed_iserror() {
        // M9 seed divergence
        let s = test_support::session(&[("a.py", "def f():\n    return 1\n")]);
        let out = (ToolRegistry::nav_v1().get("nav_ego_graph").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({"seed":{"kind":"loc","file":"/etc/passwd","line":1}}),
        );
        assert!(out.is_error);
    }

    #[test]
    fn repo_map_graph_in_bounds() {
        let s = test_support::session(&[
            ("util.py", "def helper():\n    return 1\n"),
            (
                "main.py",
                "from util import helper\n\ndef run():\n    return helper()\n",
            ),
        ]);
        let out = (ToolRegistry::nav_v1().get("nav_repo_map").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({"max_results":1}),
        );
        let v: serde_json::Value = serde_json::from_str(&out.content_text).unwrap();
        let n = v["graph"]["nodes"].as_array().unwrap().len();
        assert!(v["graph"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| (e["from"].as_u64().unwrap() as usize) < n
                && (e["to"].as_u64().unwrap() as usize) < n));
    }

    #[test]
    fn module_deps_lists_targets() {
        let s = test_support::session(&[
            ("util.py", "def helper():\n    return 1\n"),
            (
                "main.py",
                "from util import helper\n\ndef run():\n    return helper()\n",
            ),
        ]);
        let out = (ToolRegistry::nav_v1()
            .get("nav_module_deps")
            .unwrap()
            .handler)(&ToolContext::for_test(&s), &json!({"file":"main.py"}));
        let v: serde_json::Value = serde_json::from_str(&out.content_text).unwrap();
        assert!(v["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["location"]["file"] == "util.py"));
    }

    #[test]
    fn agent_markdown_keeps_canonical_structured_content() {
        let s = test_support::session(&[(
            "a.py",
            "def target():\n    return 1\n\ndef one():\n    return target()\n\ndef two():\n    return target()\n",
        )]);
        let default = (ToolRegistry::nav_v1().get("nav_callers").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({
                "seed":{"kind":"symbol","name":"target","file":"a.py"},
                "max_results":1
            }),
        );
        let agent = (ToolRegistry::nav_v1().get("nav_callers").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({
                "seed":{"kind":"symbol","name":"target","file":"a.py"},
                "max_results":1,
                "format":"agent_markdown",
                "group_by":"file"
            }),
        );

        assert_eq!(agent.structured, default.structured);
        assert!(agent.content_text.starts_with("# Prism Evidence"));
        assert_eq!(
            agent.meta["prism/content_text_format"],
            json!("agent_markdown")
        );
        assert_eq!(agent.meta["prism/view_schema_version"], json!("0.2"));
    }

    #[test]
    fn agent_json_view_is_json_content_text() {
        let s = test_support::session(&[(
            "a.py",
            "def target():\n    return 1\n\ndef caller():\n    return target()\n",
        )]);
        let out = (ToolRegistry::nav_v1().get("nav_callers").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({
                "seed":{"kind":"symbol","name":"target","file":"a.py"},
                "format":"agent_json",
                "profile":"impact"
            }),
        );
        let view: serde_json::Value = serde_json::from_str(&out.content_text).unwrap();
        assert_eq!(view["profile"], "impact");
        assert_eq!(out.meta["prism/content_text_format"], json!("agent_json"));
    }

    #[test]
    fn impact_profile_groups_by_symbol_unless_group_by_none_is_explicit() {
        let s = test_support::session(&[(
            "a.py",
            "def target():\n    return 1\n\ndef one():\n    return target()\n\ndef two():\n    return target()\n",
        )]);
        let grouped = (ToolRegistry::nav_v1().get("nav_callers").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({
                "seed":{"kind":"symbol","name":"target","file":"a.py"},
                "format":"agent_json",
                "profile":"impact"
            }),
        );
        let grouped_view: serde_json::Value = serde_json::from_str(&grouped.content_text).unwrap();
        assert!(grouped_view["groups"].as_array().unwrap().len() >= 2);
        assert_eq!(grouped_view["meta"]["group_by"], "symbol");

        let ungrouped = (ToolRegistry::nav_v1().get("nav_callers").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({
                "seed":{"kind":"symbol","name":"target","file":"a.py"},
                "format":"agent_json",
                "profile":"impact",
                "group_by":"none"
            }),
        );
        let ungrouped_view: serde_json::Value =
            serde_json::from_str(&ungrouped.content_text).unwrap();
        assert!(ungrouped_view.get("groups").is_none());
        assert!(ungrouped_view["items"].as_array().unwrap().len() >= 2);
        assert_eq!(ungrouped_view["meta"]["group_by"], "none");
    }

    #[test]
    fn module_deps_symbolless_resolved_target_is_exact_not_unresolved() {
        let s = test_support::session(&[
            ("util.py", "def helper():\n    return 1\n"),
            (
                "main.py",
                "from util import helper\n\ndef run():\n    return helper()\n",
            ),
        ]);
        let out = (ToolRegistry::nav_v1()
            .get("nav_module_deps")
            .unwrap()
            .handler)(
            &ToolContext::for_test(&s),
            &json!({
                "file":"main.py",
                "format":"agent_json",
                "profile":"dependencies",
                "group_by":"none"
            }),
        );
        let view: serde_json::Value = serde_json::from_str(&out.content_text).unwrap();
        let util_item = view["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["loc"].as_str().unwrap().starts_with("util.py:"))
            .expect("resolved util.py dependency item");
        assert_eq!(util_item["trust"], "exact");
        assert!(view["summary"]["exact"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn module_deps_next_queries_keep_call_site_on_source_file() {
        let s = test_support::session(&[
            ("util.py", "def helper():\n    return 1\n"),
            (
                "main.py",
                "from util import helper\n\ndef run():\n    return helper()\n",
            ),
        ]);
        let out = (ToolRegistry::nav_v1()
            .get("nav_module_deps")
            .unwrap()
            .handler)(
            &ToolContext::for_test(&s),
            &json!({
                "file":"main.py",
                "format":"agent_json",
                "profile":"dependencies",
                "group_by":"none"
            }),
        );
        let view: serde_json::Value = serde_json::from_str(&out.content_text).unwrap();
        let queries = view["next_queries"].as_array().unwrap();
        assert!(queries.iter().any(|query| {
            query["tool"] == "nav_nodes_at"
                && query["reason"] == "call_site"
                && query["arguments"]["file"] == "main.py"
                && query["arguments"]["line"] == 4
        }));
        assert!(queries.iter().any(|query| {
            query["tool"] == "nav_nodes_at"
                && query["reason"] == "dependency_target"
                && query["arguments"]["file"] == "util.py"
                && query["arguments"]["line"] == 1
        }));
        assert!(!queries.iter().any(|query| {
            query["reason"] == "call_site"
                && query["arguments"]["file"] == "util.py"
                && query["arguments"]["line"] == 4
        }));
    }

    #[test]
    fn transitive_exact_score_discount_is_not_fallback() {
        let s = test_support::session(&[(
            "a.py",
            "def a():\n    return b()\n\ndef b():\n    return c()\n\ndef c():\n    return 1\n",
        )]);
        let out = (ToolRegistry::nav_v1().get("nav_callees").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({
                "seed":{"kind":"symbol","name":"a","file":"a.py"},
                "depth":2,
                "format":"agent_json",
                "profile":"dependencies",
                "group_by":"none"
            }),
        );
        let view: serde_json::Value = serde_json::from_str(&out.content_text).unwrap();
        let discounted = view["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["score"].as_f64().unwrap() < 1.0)
            .expect("transitive discounted item");
        assert_eq!(discounted["trust"], "exact");
        assert_eq!(view["summary"]["fallback"], 0);
    }

    #[test]
    fn callees_next_queries_keep_call_site_on_seed_file() {
        let s = test_support::session(&[
            ("util.py", "def helper():\n    return 1\n"),
            (
                "main.py",
                "from util import helper\n\ndef run():\n    return helper()\n",
            ),
        ]);
        let out = (ToolRegistry::nav_v1().get("nav_callees").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({
                "seed":{"kind":"symbol","name":"run","file":"main.py"},
                "format":"agent_json",
                "profile":"dependencies"
            }),
        );
        let view: serde_json::Value = serde_json::from_str(&out.content_text).unwrap();
        let queries = view["next_queries"].as_array().unwrap();
        assert!(queries.iter().any(|query| {
            query["tool"] == "nav_nodes_at"
                && query["reason"] == "call_site"
                && query["arguments"]["file"] == "main.py"
                && query["arguments"]["line"] == 4
        }));
        assert!(queries.iter().any(|query| {
            query["tool"] == "nav_nodes_at"
                && query["reason"] == "callee_definition"
                && query["arguments"]["file"] == "util.py"
                && query["arguments"]["line"] == 1
        }));
        assert!(!queries.iter().any(|query| {
            query["reason"] == "call_site"
                && query["arguments"]["file"] == "util.py"
                && query["arguments"]["line"] == 4
        }));
    }

    #[test]
    fn repo_map_orientation_hints_modules_without_node_trust() {
        let s = test_support::session(&[
            ("util.py", "def helper():\n    return 1\n"),
            (
                "main.py",
                "from util import helper\n\ndef run():\n    return helper()\n",
            ),
        ]);
        let out = (ToolRegistry::nav_v1().get("nav_repo_map").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({
                "format":"agent_json",
                "profile":"orientation"
            }),
        );
        let view: serde_json::Value = serde_json::from_str(&out.content_text).unwrap();
        assert!(view["next_queries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|query| {
                query["tool"] == "nav_module_deps" && query["reason"] == "inspect_module"
            }));
        assert!(view["graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|node| { node.get("trust").is_none() }));
        assert_eq!(view["summary"]["exact"], 0);
    }

    #[test]
    fn callers_line_snippet_uses_call_site_line() {
        let s = test_support::session(&[(
            "a.py",
            "def target():\n    return 1\n\ndef caller():\n    return target()\n",
        )]);
        let out = (ToolRegistry::nav_v1().get("nav_callers").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({
                "seed":{"kind":"symbol","name":"target","file":"a.py"},
                "format":"agent_markdown",
                "snippets":"line"
            }),
        );
        assert!(out.content_text.contains("5:     return target()"));
    }

    #[test]
    fn callees_line_snippet_is_omitted_for_transitive_depth() {
        let s = test_support::session(&[(
            "a.py",
            "def a():\n    return b()\n\ndef b():\n    return c()\n\ndef c():\n    return 1\n",
        )]);
        let out = (ToolRegistry::nav_v1().get("nav_callees").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({
                "seed":{"kind":"symbol","name":"a","file":"a.py"},
                "depth":2,
                "format":"agent_markdown",
                "snippets":"line"
            }),
        );
        assert!(!out.content_text.contains("return b()"));
        assert!(!out.content_text.contains("return c()"));
    }

    #[test]
    fn max_view_bytes_clips_only_agent_view() {
        let s = test_support::session(&[(
            "a.py",
            "def target():\n    return 1\n\ndef one():\n    return target()\n\ndef two():\n    return target()\n\ndef three():\n    return target()\n",
        )]);
        let default = (ToolRegistry::nav_v1().get("nav_callers").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({
                "seed":{"kind":"symbol","name":"target","file":"a.py"}
            }),
        );
        let out = (ToolRegistry::nav_v1().get("nav_callers").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({
                "seed":{"kind":"symbol","name":"target","file":"a.py"},
                "format":"agent_markdown",
                "max_view_bytes":64
            }),
        );
        assert!(out.content_text.len() <= 64);
        assert_eq!(out.structured, default.structured);
        assert_eq!(out.meta["prism/view_clipped"], json!(true));
    }

    #[test]
    fn view_controls_require_agent_format() {
        let s = test_support::session(&[("a.py", "def f():\n    return 1\n")]);
        let out = (ToolRegistry::nav_v1().get("nav_nodes_at").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({"file":"a.py","line":1,"snippets":"line"}),
        );
        assert!(out.is_error);
        assert!(out.content_text.contains("agent_markdown"));
    }

    #[test]
    fn schemas_expose_view_controls() {
        let registry = ToolRegistry::nav_v1();
        let schema = &registry.get("nav_callers").unwrap().input_schema;
        assert_eq!(
            schema["properties"]["format"]["enum"],
            json!(["canonical_json", "agent_markdown", "agent_json"])
        );
        assert_eq!(
            schema["properties"]["profile"]["enum"],
            json!(["impact", "edit_context", "audit"])
        );
    }
}
