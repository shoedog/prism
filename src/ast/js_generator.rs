//! Admissibility of newly inventoried anonymous generator formal lists.
use super::ParsedFile;
use tree_sitter::Node;

impl ParsedFile {
    pub(super) fn js_ts_generator_formals_refused(&self, owner: &Node<'_>) -> bool {
        if owner.kind() != "generator_function" {
            return false;
        }
        let Some(params) = self.parameter_binding_region(owner) else {
            return true;
        };
        // Recovery is not proof of executable generator parameters. This also
        // refuses the grammar's recovered, script-valid bare `await` formal.
        if params.has_error() || params.is_missing() {
            return true;
        }
        let mut names = Vec::new();
        self.js_ts_pattern_bound_names_with_decoder(
            params,
            &mut names,
            Self::js_ts_decode_identifier_text,
        );
        let mut cursor = owner.walk();
        let async_ = owner.children(&mut cursor).any(|n| n.kind() == "async");
        if names
            .iter()
            .any(|name| name == "yield" || (async_ && name == "await"))
        {
            return true;
        }
        let boundaries = self.language.callable_boundary_node_types();
        let mut stack = vec![params];
        while let Some(node) = stack.pop() {
            if node.id() != params.id() && boundaries.contains(&node.kind()) {
                // A computed method key executes while its object/class is
                // constructed. Nested callable bodies and defaults do not.
                if node.kind() == "method_definition" {
                    stack.extend(node.child_by_field_name("name"));
                }
                continue;
            }
            if matches!(node.kind(), "yield_expression" | "await_expression") {
                return true;
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
        false
    }
}
