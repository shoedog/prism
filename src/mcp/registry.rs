use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::output::{McpToolResult, StructuredContentMode};
use crate::navigation::NavigationSession;

pub struct ToolContext<'a> {
    pub session: &'a NavigationSession,
    pub cap: usize,
    /// S3: resolved ONCE per request in `transport.rs` (mirroring `cap`) and threaded down so
    /// handlers never read `PRISM_MCP_CONCISE_SHAPE` themselves. Defaults to `Legacy` in
    /// `for_test` so the ~150 existing `ToolContext::for_test` call sites keep pinning the
    /// legacy canonical shapes (the live env default is `slim` since 2026-07-03; the wire
    /// default is decided only in `transport.rs`).
    pub concise_shape_mode: ConciseShapeMode,
    /// F1 (controller-adjudicated): resolved ONCE per request in `transport.rs` (mirroring `cap`
    /// and `concise_shape_mode`) and threaded down so cap-fitting sizing (`output::shape_result`,
    /// `evidence_view::shape_navigation_result`) can size against the mode that will ACTUALLY reach
    /// the wire, instead of the frozen `StructuredContentMode::Always` that made the
    /// `omit-default-path` item-retention win never materialize. Defaults to `Always` (via
    /// `StructuredContentMode::default()`) in `for_test`, pinning the always-shape for the
    /// existing call sites (the live env default is `omit-default-path` since 2026-07-03; the
    /// wire default is decided only in `transport.rs`).
    pub structured_content_mode: StructuredContentMode,
}

impl<'a> ToolContext<'a> {
    pub fn new(
        session: &'a NavigationSession,
        cap: usize,
        concise_shape_mode: ConciseShapeMode,
        structured_content_mode: StructuredContentMode,
    ) -> Self {
        Self {
            session,
            cap,
            concise_shape_mode,
            structured_content_mode,
        }
    }

    #[cfg(test)]
    pub fn for_test(session: &'a NavigationSession) -> Self {
        Self {
            session,
            cap: crate::mcp::output::resolve_cap(),
            concise_shape_mode: ConciseShapeMode::default(),
            structured_content_mode: StructuredContentMode::default(),
        }
    }
}

pub type ToolHandler =
    dyn for<'a> Fn(&ToolContext<'a>, &serde_json::Value) -> McpToolResult + Send + Sync;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolAnnotations {
    pub title: String,
    pub read_only_hint: bool,
    pub destructive_hint: bool,
    pub idempotent_hint: bool,
    pub open_world_hint: bool,
}

impl ToolAnnotations {
    pub fn read_only(title: &str) -> Self {
        Self {
            title: title.into(),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: true,
            open_world_hint: false,
        }
    }

    pub fn local_state_change(title: &str) -> Self {
        Self {
            title: title.into(),
            read_only_hint: false,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }
    }
}

pub struct ToolDescriptor {
    pub name: &'static str,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub annotations: ToolAnnotations,
    pub runtime_behavior: Option<ToolRuntimeBehavior>,
    pub handler: Box<ToolHandler>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolRuntimeBehavior {
    RefreshIndex,
}

impl ToolDescriptor {
    pub fn to_listed(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "title": &self.annotations.title,
            "description": &self.description,
            "inputSchema": &self.input_schema,
            "annotations": &self.annotations,
            "_meta": { "prism/x-stability": "experimental" }
        })
    }
}

pub struct ToolRegistry {
    tools: Vec<ToolDescriptor>,
}

impl ToolRegistry {
    pub fn nav_v1() -> Self {
        let mut registry = Self { tools: Vec::new() };
        crate::mcp::tools::register_all(&mut registry);
        registry
    }

    pub fn reason_v1() -> Self {
        let mut registry = Self { tools: Vec::new() };
        crate::mcp::tools_reasoning::register_all(&mut registry);
        registry
    }

    pub fn all_v1() -> Self {
        let mut registry = Self::nav_v1();
        crate::mcp::tools_reasoning::register_all(&mut registry);
        crate::mcp::tools_refresh::register_all(&mut registry);
        registry
    }

    pub fn register(&mut self, descriptor: ToolDescriptor) {
        self.tools.push(descriptor);
    }

    pub fn get(&self, name: &str) -> Option<&ToolDescriptor> {
        self.tools.iter().find(|tool| tool.name == name)
    }

    pub fn list(&self) -> &[ToolDescriptor] {
        &self.tools
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_lists_six_tools_with_annotations() {
        let r = ToolRegistry::nav_v1();
        assert_eq!(
            r.list().iter().map(|d| d.name).collect::<Vec<_>>(),
            [
                "nav_nodes_at",
                "nav_callers",
                "nav_callees",
                "nav_ego_graph",
                "nav_module_deps",
                "nav_repo_map"
            ]
        );
        let listed = r.get("nav_callers").unwrap().to_listed();
        assert_eq!(listed["annotations"]["readOnlyHint"], true);
        assert_eq!(listed["annotations"]["openWorldHint"], false);
        assert_eq!(listed["_meta"]["prism/x-stability"], "experimental");
        assert!(listed["inputSchema"]["properties"]["seed"].is_object());
        for d in r.list() {
            let desc = &d.description;
            assert!(
                desc.contains("Example") && desc.contains("NOT"),
                "tool {} description must front-load when/when-NOT + a worked Example: {desc}",
                d.name
            );
            // S1: the full snapshot/view notice text moved to `initialize`'s `instructions`
            // (stated once for the whole session); each tool description keeps only a short
            // hedge pointing there, so a client that never surfaces `instructions` still learns
            // the notices exist without re-paying ~592 B/tool on every `tools/list`.
            assert!(
                desc.contains("see server instructions"),
                "tool {} description must hedge to server instructions instead of repeating the full notice: {desc}",
                d.name
            );
            assert!(
                !desc.contains("repository snapshot loaded when prism-mcp started"),
                "tool {} description must NOT duplicate the full snapshot notice text (moved to initialize instructions): {desc}",
                d.name
            );
            // F2 (codex MINOR): the S1 move-to-`instructions` applies to BOTH notices — pin the
            // view-notice absence too, not just the snapshot notice above.
            assert!(
                !desc.contains("Optional LLM views are opt-in"),
                "tool {} description must NOT duplicate the full view notice text (moved to initialize instructions): {desc}",
                d.name
            );
        }
    }

    #[test]
    fn all_registry_adds_reasoning_tools_without_changing_nav_v1() {
        let nav = ToolRegistry::nav_v1();
        let reason = ToolRegistry::reason_v1();
        let all = ToolRegistry::all_v1();

        assert_eq!(nav.list().len(), 6);
        assert_eq!(
            reason.list().iter().map(|d| d.name).collect::<Vec<_>>(),
            ["taint_reaches"]
        );
        assert_eq!(all.list().len(), 8);
        assert!(all.get("taint_reaches").is_some());
        assert_eq!(
            all.get("refresh_index").unwrap().runtime_behavior,
            Some(ToolRuntimeBehavior::RefreshIndex)
        );
        let refresh = all.get("refresh_index").unwrap().to_listed();
        assert_eq!(refresh["annotations"]["readOnlyHint"], false);
        assert_eq!(refresh["annotations"]["destructiveHint"], false);
        assert_eq!(refresh["annotations"]["idempotentHint"], false);
        assert_eq!(refresh["annotations"]["openWorldHint"], false);
        assert_eq!(nav.list().len(), 6);
    }
}
