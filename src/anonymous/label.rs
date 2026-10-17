use super::*;
use crate::ast::ParsedFile;
use tree_sitter::Node;

#[derive(Clone)]
pub(super) struct Context {
    pub root: AnchorRoot,
    pub ordinals: Vec<usize>,
    pub segments: Vec<String>,
    display: String,
}

impl Context {
    pub fn named(parsed: &ParsedFile, node: Node<'_>, id: &FunctionId) -> Self {
        let display = parsed
            .language
            .method_owner(&node)
            .or_else(|| {
                // Display-only lookup for abstract classes. Do not broaden
                // Language::method_owner, which also authorizes resolution.
                let mut body = node.parent()?;
                if matches!(body.kind(), "field_definition" | "public_field_definition") {
                    body = body.parent()?;
                }
                if body.kind() != "class_body" {
                    return None;
                }
                let class = body.parent()?;
                (class.kind() == "abstract_class_declaration")
                    .then(|| class.child_by_field_name("name"))
                    .flatten()
            })
            .map(|owner| format!("{}.{}", parsed.node_text(&owner), id.name))
            .unwrap_or_else(|| id.name.clone());
        Self {
            root: AnchorRoot::Named {
                function: id.clone(),
            },
            ordinals: vec![],
            segments: vec![],
            display,
        }
    }

    pub fn scope(file: &str, scope: &str, display: String) -> Self {
        Self {
            root: AnchorRoot::Scope {
                file: file.into(),
                scope: scope.into(),
            },
            ordinals: vec![],
            segments: vec![],
            display,
        }
    }

    pub fn render(&self, line: usize, col: usize) -> String {
        format!(
            "{} › {} ({line}:{col})",
            self.display,
            self.segments.join(" › ")
        )
    }
}

pub(super) fn segment(meta: &AnonMeta, handoff_display: Option<&str>) -> String {
    // Explicit precedence: prop, IIFE, handoff, then remaining syntax roles.
    match &meta.bound_as {
        Some(BoundAs::JsxProp { prop, .. }) => return format!("{prop} handler"),
        Some(BoundAs::Iife) => return "IIFE".into(),
        _ => {}
    }
    if let Some(passed) = &meta.passed_to {
        // Display only: retain the complete source spelling in PassedTo.
        let short = display_text(handoff_display.unwrap_or(&passed.callee_text));
        return format!("{short} callback");
    }
    match &meta.bound_as {
        Some(BoundAs::DefaultParam { param, .. }) => format!("default {param}"),
        Some(BoundAs::Returned) => "returned callable".into(),
        Some(BoundAs::DefaultExport) => "default export".into(),
        Some(BoundAs::VariableParenthesized { name }) => format!("variable {name}"),
        Some(BoundAs::ObjectKey { key }) => format!("object {key}"),
        Some(BoundAs::ClassField { field, .. }) => format!("field {field}"),
        _ => "anonymous callable".into(),
    }
}

fn display_text(raw: &str) -> String {
    let normalized = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() > 48 {
        format!("{}…", normalized.chars().take(47).collect::<String>())
    } else {
        normalized
    }
}
