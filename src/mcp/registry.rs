use crate::mcp::output::McpToolResult;
use crate::navigation::NavigationSession;

pub type ToolHandler =
    dyn Fn(&NavigationSession, &serde_json::Value) -> McpToolResult + Send + Sync;

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
}

pub struct ToolDescriptor {
    pub name: &'static str,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub annotations: ToolAnnotations,
    pub handler: Box<ToolHandler>,
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
        }
    }
}
