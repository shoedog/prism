use super::error::query_error_result;
use super::input::{parse_taint_reaches, SeedInput, Verbosity as InputVerbosity};
use super::output::{shape_result, McpToolResult, Verbosity};
use super::registry::{ToolAnnotations, ToolContext, ToolDescriptor, ToolRegistry};
use crate::navigation::types::{Evidence, Warning, WarningKind};
use crate::reasoning::seeds::SeedSpec;
use serde_json::json;

// S1: see `crate::mcp::tools`'s notice consts — the full snapshot notice now lives once in
// `initialize`'s `instructions`; `taint_reaches` (no `format` argument, so the view notice never
// applied here) keeps only a short hedge.
const SNAPSHOT_HEDGE: &str = "Snapshot details: see server instructions.";

pub fn register_all(r: &mut ToolRegistry) {
    r.register(tool_with_handler(
        "taint_reaches",
        "Taint Reaches",
        "Traces tainted variable flow from source seeds. Use without sinks to inspect the frontier; use with sinks to get per-sink Reached, BoundaryExited, Sanitized, or NotReached verdicts and witness graphs. Sanitized means the witness chain proves a recognized sanitizer call sits ON this specific path (see the sanitized_by site); a sanitizer merely present elsewhere in the source function only produces an advisory Cleansed warning and leaves the verdict Reached. Sink seed resolution is all-or-nothing; unresolved source seeds warn and continue when at least one source resolves. Do NOT use this for call hierarchy or module dependency questions. Seed grammar is {\"kind\":\"symbol\",\"name\":\"handler\",\"file\":\"src/app.py\"} for function parameters or {\"kind\":\"loc\",\"file\":\"src/app.py\",\"line\":42} for variable nodes at a line. Inputs are sources array, optional sinks array, optional max_results 1..1000, and optional verbosity concise or detailed. Example: {\"sources\":[{\"kind\":\"loc\",\"file\":\"app.py\",\"line\":2}],\"sinks\":[{\"kind\":\"loc\",\"file\":\"app.py\",\"line\":4}],\"verbosity\":\"detailed\"}.",
        taint_reaches_schema(),
        Box::new(taint_reaches),
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
        description: format!("{description} {SNAPSHOT_HEDGE}"),
        input_schema,
        annotations: ToolAnnotations::read_only(title),
        runtime_behavior: None,
        handler,
    }
}

fn taint_reaches(ctx: &ToolContext<'_>, args: &serde_json::Value) -> McpToolResult {
    let input = match parse_taint_reaches(args) {
        Ok(input) => input,
        Err(error) => return error.into_result(),
    };
    let sources = input
        .sources
        .iter()
        .map(seed_spec)
        .collect::<Vec<SeedSpec>>();
    let sinks = input
        .sinks
        .as_ref()
        .map(|sinks| sinks.iter().map(seed_spec).collect::<Vec<SeedSpec>>());
    let evidence = match crate::reasoning::taint_reaches::taint_reaches(
        ctx.session,
        &sources,
        sinks.as_deref(),
    ) {
        Ok(evidence) => evidence,
        Err(error) => return query_error_result(error),
    };
    let (evidence, total, max_results_clipped) = clip_taint_reaches(evidence, input.max_results);
    shape_result(
        evidence,
        total,
        max_results_clipped,
        output_verbosity(input.verbosity),
        ctx.cap,
    )
}

fn seed_spec(seed: &SeedInput) -> SeedSpec {
    match seed {
        SeedInput::Symbol { name, file } => SeedSpec::Symbol {
            name: name.clone(),
            file: file.clone(),
        },
        SeedInput::Loc { file, line } => SeedSpec::Loc {
            file: file.clone(),
            line: *line,
        },
    }
}

fn clip_taint_reaches(mut evidence: Evidence, max_results: usize) -> (Evidence, usize, bool) {
    if evidence
        .reasoning
        .as_ref()
        .is_some_and(|reasoning| !reasoning.per_sink.is_empty())
    {
        let total_sinks = evidence
            .reasoning
            .as_ref()
            .map(|reasoning| reasoning.per_sink.len())
            .unwrap_or(0);
        let retained = max_results.min(total_sinks);
        if retained < total_sinks {
            if let Some(reasoning) = &mut evidence.reasoning {
                reasoning.truncate_sinks(retained);
            }
            evidence.truncated = true;
            evidence
                .warnings
                .push(reasoning_truncated_warning(retained, total_sinks));
        }
        return (evidence, total_sinks, false);
    }

    let total = evidence.items.len();
    let retained = max_results.min(total);
    evidence.items.truncate(retained);
    (evidence, total, retained < total)
}

fn reasoning_truncated_warning(retained: usize, total: usize) -> Warning {
    Warning {
        kind: WarningKind::ResultTruncated,
        message: format!(
            "showing {retained} of {total} sinks; raise max_results or narrow the sink seeds"
        ),
        location: None,
    }
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

fn taint_reaches_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "sources": {
                "type": "array",
                "minItems": 1,
                "items": seed_schema()
            },
            "sinks": {
                "type": "array",
                "minItems": 1,
                "items": seed_schema()
            },
            "max_results": { "type": "integer", "minimum": 1, "maximum": 1000 },
            "verbosity": { "enum": ["concise", "detailed"] }
        },
        "required": ["sources"]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn taint_reaches_description_hedges_to_instructions_not_full_notice() {
        // S1: the full snapshot notice moved to `initialize`'s `instructions`; the tool
        // description keeps only a short hedge (taint_reaches has no agent-view notice, unlike
        // the nav tools, since it never accepts a `format` argument).
        let registry = ToolRegistry::all_v1();
        let desc = &registry.get("taint_reaches").unwrap().description;
        assert!(
            desc.contains("see server instructions"),
            "taint_reaches description must hedge to server instructions: {desc}"
        );
        assert!(
            !desc.contains("repository snapshot loaded when prism-mcp started"),
            "taint_reaches description must NOT duplicate the full snapshot notice text: {desc}"
        );
    }

    #[test]
    fn taint_reaches_witness_ok() {
        let s = crate::mcp::tools::test_support::session(&[(
            "app.py",
            "def f():\n    user = input()\n    value = user\n    sink(value)\n",
        )]);
        let out = (ToolRegistry::all_v1().get("taint_reaches").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({
                "sources":[{"kind":"loc","file":"app.py","line":2}],
                "sinks":[{"kind":"loc","file":"app.py","line":4}],
                "verbosity":"detailed"
            }),
        );
        assert!(!out.is_error);
        let value: serde_json::Value = serde_json::from_str(&out.content_text).unwrap();
        assert_eq!(value["reasoning"]["reachability"], "Reached");
        assert!(value["graph"]["nodes"]
            .as_array()
            .is_some_and(|nodes| !nodes.is_empty()));
    }

    #[test]
    fn taint_reaches_witness_sanitized_verdict_carries_site_and_witness_step() {
        let s = crate::mcp::tools::test_support::session(&[(
            "app.py",
            "def f():\n    user = input()\n    safe = html.escape(user)\n    sink(safe)\n",
        )]);
        let out = (ToolRegistry::all_v1().get("taint_reaches").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({
                "sources":[{"kind":"loc","file":"app.py","line":2}],
                "sinks":[{"kind":"loc","file":"app.py","line":4}],
                "verbosity":"detailed"
            }),
        );
        assert!(!out.is_error);
        let value: serde_json::Value = serde_json::from_str(&out.content_text).unwrap();
        assert_eq!(value["reasoning"]["reachability"], "Sanitized");
        let source = &value["reasoning"]["per_sink"][0]["sources"][0];
        assert_eq!(source["reachability"], "Sanitized");
        assert_eq!(source["sanitized_by"][0]["callee_text"], "html.escape");
        assert_eq!(source["sanitized_by"][0]["category"], "xss");
        assert_eq!(source["sanitized_by"][0]["line"], 3);
        assert!(
            value["graph"]["edges"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["kind"] == "SanitizedBy"),
            "{value}"
        );
    }

    #[test]
    fn taint_reaches_frontier_max_results_clips_items() {
        let s = crate::mcp::tools::test_support::session(&[(
            "app.py",
            "def f():\n    user = input()\n    a = user\n    b = a\n",
        )]);
        let out = (ToolRegistry::all_v1().get("taint_reaches").unwrap().handler)(
            &ToolContext::for_test(&s),
            &json!({"sources":[{"kind":"loc","file":"app.py","line":2}],"max_results":1}),
        );
        assert!(!out.is_error);
        let value: serde_json::Value = serde_json::from_str(&out.content_text).unwrap();
        assert_eq!(value["items"].as_array().unwrap().len(), 1);
        assert_eq!(value["truncated"], true);
    }
}
