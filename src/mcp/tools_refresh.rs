use super::output::{clamp_user_text, McpToolResult, SCHEMA_VERSION};
use super::registry::{
    ToolAnnotations, ToolContext, ToolDescriptor, ToolRegistry, ToolRuntimeBehavior,
};
use super::session::RefreshSummary;
use serde_json::{json, Map, Value};

pub const REFRESH_INDEX_TOOL: &str = "refresh_index";

const DESCRIPTION: &str = "Rebuilds the prism-mcp repository snapshot, navigation index, and freshness baseline for this server session. Use after stale-index metadata or warnings report changed tracked files; do NOT use for ordinary navigation when no files changed. Takes no arguments and returns a refresh summary with the new generation and whether the previous snapshot was stale. Example: {}.";

pub fn register_all(r: &mut ToolRegistry) {
    r.register(ToolDescriptor {
        name: REFRESH_INDEX_TOOL,
        description: DESCRIPTION.into(),
        input_schema: json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        }),
        annotations: ToolAnnotations::local_state_change("Refresh Index"),
        runtime_behavior: Some(ToolRuntimeBehavior::RefreshIndex),
        handler: Box::new(refresh_unavailable),
    });
}

pub fn refresh_result(summary: &RefreshSummary) -> McpToolResult {
    let structured = serde_json::to_value(summary).expect("refresh summary serializes");
    let content_text = serde_json::to_string_pretty(&structured).expect("refresh summary renders");
    let mut meta = base_meta();
    meta.insert(
        "prism/refresh_generation".into(),
        Value::Number(serde_json::Number::from(summary.generation)),
    );
    meta.insert(
        "prism/refresh_status".into(),
        Value::String(summary.status.into()),
    );
    McpToolResult {
        content_text,
        structured: Some(structured),
        is_error: false,
        meta,
    }
}

pub fn refresh_error_result(error: &anyhow::Error) -> McpToolResult {
    error_result(&format!(
        "refresh_index failed: {}",
        clamp_user_text(&error.to_string())
    ))
}

pub fn invalid_arguments_result() -> McpToolResult {
    error_result("refresh_index takes no arguments")
}

fn refresh_unavailable(_ctx: &ToolContext<'_>, _args: &Value) -> McpToolResult {
    error_result("refresh_index requires provider-backed prism-mcp transport")
}

fn error_result(message: &str) -> McpToolResult {
    McpToolResult {
        content_text: message.to_string(),
        structured: None,
        is_error: true,
        meta: base_meta(),
    }
}

fn base_meta() -> Map<String, Value> {
    let mut meta = Map::new();
    meta.insert(
        "prism/schema_version".into(),
        Value::String(SCHEMA_VERSION.into()),
    );
    meta
}
