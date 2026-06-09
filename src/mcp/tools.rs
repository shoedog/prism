use super::error::query_error_result;
use super::input::{
    parse_callees, parse_callers, parse_ego, parse_module_deps, parse_nodes_at, parse_repo_map,
    Verbosity as InputVerbosity,
};
use super::output::{resolve_cap, shape_result, McpToolResult, Verbosity};
use super::registry::{ToolAnnotations, ToolDescriptor, ToolRegistry};
use crate::navigation::types::Evidence;
use crate::navigation::{module_graph, queries, NavigationSession};
use serde_json::json;

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
        description: description.into(),
        input_schema,
        annotations: ToolAnnotations::read_only(title),
        handler,
    }
}

fn nav_nodes_at(session: &NavigationSession, args: &serde_json::Value) -> McpToolResult {
    let input = match parse_nodes_at(args) {
        Ok(input) => input,
        Err(error) => return error.into_result(),
    };
    let evidence = queries::nodes_at(session, &input.file, input.line);
    let total = evidence.items.len();
    shape_result(
        evidence,
        total,
        false,
        output_verbosity(input.verbosity),
        resolve_cap(),
    )
}

fn nav_callers(session: &NavigationSession, args: &serde_json::Value) -> McpToolResult {
    let input = match parse_callers(args) {
        Ok(input) => input,
        Err(error) => return error.into_result(),
    };
    let (symbol, file, location) = input.seed.to_triple();
    let evidence = match queries::callers(session, symbol, file, location.as_deref(), input.depth) {
        Ok(evidence) => evidence,
        Err(error) => return query_error_result(error),
    };
    let (evidence, total, max_results_clipped) = clip_flat(evidence, input.max_results);
    shape_result(
        evidence,
        total,
        max_results_clipped,
        output_verbosity(input.verbosity),
        resolve_cap(),
    )
}

fn nav_callees(session: &NavigationSession, args: &serde_json::Value) -> McpToolResult {
    let input = match parse_callees(args) {
        Ok(input) => input,
        Err(error) => return error.into_result(),
    };
    let (symbol, file, location) = input.seed.to_triple();
    let evidence = match queries::callees(session, symbol, file, location.as_deref(), input.depth) {
        Ok(evidence) => evidence,
        Err(error) => return query_error_result(error),
    };
    let (evidence, total, max_results_clipped) = clip_flat(evidence, input.max_results);
    shape_result(
        evidence,
        total,
        max_results_clipped,
        output_verbosity(input.verbosity),
        resolve_cap(),
    )
}

fn nav_ego_graph(session: &NavigationSession, args: &serde_json::Value) -> McpToolResult {
    let input = match parse_ego(args) {
        Ok(input) => input,
        Err(error) => return error.into_result(),
    };
    let (symbol, file, location) = input.seed.to_triple();
    let edges = input.edges.iter().map(String::as_str).collect::<Vec<_>>();
    let evidence = match queries::ego_graph(
        session,
        symbol,
        file,
        location.as_deref(),
        input.hops,
        &edges,
    ) {
        Ok(evidence) => evidence,
        Err(error) => return query_error_result(error),
    };
    let (evidence, total, max_results_clipped) = clip_graph(evidence, input.max_results);
    shape_result(
        evidence,
        total,
        max_results_clipped,
        Verbosity::Concise,
        resolve_cap(),
    )
}

fn nav_module_deps(session: &NavigationSession, args: &serde_json::Value) -> McpToolResult {
    let input = match parse_module_deps(args) {
        Ok(input) => input,
        Err(error) => return error.into_result(),
    };
    let evidence = module_graph::module_deps(session, &input.file);
    let (evidence, total, max_results_clipped) = clip_flat(evidence, input.max_results);
    shape_result(
        evidence,
        total,
        max_results_clipped,
        output_verbosity(input.verbosity),
        resolve_cap(),
    )
}

fn nav_repo_map(session: &NavigationSession, args: &serde_json::Value) -> McpToolResult {
    let input = match parse_repo_map(args) {
        Ok(input) => input,
        Err(error) => return error.into_result(),
    };
    let evidence = module_graph::repo_map(session);
    let (evidence, total, max_results_clipped) = clip_graph(evidence, input.max_results);
    shape_result(
        evidence,
        total,
        max_results_clipped,
        Verbosity::Concise,
        resolve_cap(),
    )
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
            "verbosity": verbosity_schema()
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
            "verbosity": verbosity_schema()
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
            "verbosity": verbosity_schema()
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
            "max_results": max_results_schema()
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
            "verbosity": verbosity_schema()
        },
        "required": ["file"]
    })
}

fn repo_map_schema() -> serde_json::Value {
    json!({
        "type": "object",
            "additionalProperties": false,
        "properties": {
            "max_results": max_results_schema()
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
            &s,
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
            &s,
            &json!({"file":"a.py","line":0}),
        );
        assert!(out.is_error);
    }

    #[test]
    fn nodes_at_escaping_file_is_empty_skippedpath() {
        let s = test_support::session(&[("a.py", "def f():\n    return 1\n")]);
        let out = (ToolRegistry::nav_v1().get("nav_nodes_at").unwrap().handler)(
            &s,
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
            &s,
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
            &s,
            &json!({"seed":{"kind":"symbol","name":"run"}}),
        );
        assert!(out.is_error);
    }

    #[test]
    fn ego_escaping_seed_iserror() {
        // M9 seed divergence
        let s = test_support::session(&[("a.py", "def f():\n    return 1\n")]);
        let out = (ToolRegistry::nav_v1().get("nav_ego_graph").unwrap().handler)(
            &s,
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
            &s,
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
            .handler)(&s, &json!({"file":"main.py"}));
        let v: serde_json::Value = serde_json::from_str(&out.content_text).unwrap();
        assert!(v["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["location"]["file"] == "util.py"));
    }
}
