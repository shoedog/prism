//! P9 S1: Express registration-first route candidate extraction.

use super::EnclosingFacts;
use crate::ast::ParsedFile;
use crate::languages::Language;

/// One function-valued argument to a matched Express route-registration call.
/// Middleware and the final handler are both middleware/handler args — every
/// one that resolves gets its own `FrameworkEntryRecord` (all execute).
#[derive(Debug, Clone)]
pub enum ExpressHandlerArg {
    /// A bare identifier — resolution (same-file function-index lookup +
    /// local-shadow guard) needs the whole-program `functions`/
    /// `js_ts_function_locals` index this module doesn't own, so it is done
    /// by `CallGraph::apply_framework_entries` in call_graph.rs.
    Identifier(String),
    /// An inline arrow/function-expression argument. Grounding-verified
    /// (`languages/mod.rs` function-name-inference Pattern 3): a function
    /// passed directly as a call argument to a call whose own parent is NOT
    /// a `variable_declarator` never receives an inferred name, so it can
    /// never have a `FunctionId` — always unresolved, never a synthetic
    /// identity.
    InlineAnonymous,
}

/// One recognized Express route-registration call, with each function-valued
/// argument AFTER the path/mount argument of the matched outer method call
/// (arg positioning is new: for a direct instance call like
/// `app.get(path, mw, handler)`, arg 0 is the path and the rest are
/// candidates; for a `.route(path).get(handler)` builder, the path was
/// already consumed by `.route()`, so ALL of the outer `.get(...)` call's
/// args are candidates).
#[derive(Debug, Clone)]
pub struct ExpressRouteCandidate {
    pub enclosing: Option<EnclosingFacts>,
    pub site_line: usize,
    pub site_start_byte: usize,
    pub site_end_byte: usize,
    pub args: Vec<ExpressHandlerArg>,
}

/// Extract every recognized Express route-registration call in `parsed`.
/// Registration-first (unlike taint.rs's handler-first
/// `js_ts_is_framework_route_handler`, which is a boolean per-handler check
/// and never extracts an argument or a FunctionId) — reuses taint.rs's shape
/// constants/logic (`js_ts_framework_receiver_names`,
/// `js_ts_framework_route_method`, `js_ts_receiver_expr_is_framework_instance`,
/// `js_ts_receiver_expr_is_route_builder`, `unwrap_parenthesized`,
/// `collect_js_ts_call_nodes`) via `pub(crate)` visibility rather than
/// duplicating or refactoring that code this slice.
pub fn express_route_candidates(parsed: &ParsedFile) -> Vec<ExpressRouteCandidate> {
    let mut out = Vec::new();
    if !matches!(
        parsed.language,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    ) {
        return out;
    }

    let receivers = crate::algorithms::taint::js_ts_framework_receiver_names(parsed, "express");
    if receivers.is_empty() {
        return out;
    }
    let imports = parsed.extract_imports();

    let mut calls = Vec::new();
    crate::algorithms::taint::collect_js_ts_call_nodes(parsed.tree.root_node(), parsed, &mut calls);

    for call in calls {
        let Some(is_route_builder) = express_call_match_kind(parsed, &call, &receivers, &imports)
        else {
            continue;
        };
        let Some(args_node) = parsed.language.call_arguments(&call) else {
            continue;
        };
        let start_idx: usize = if is_route_builder { 0 } else { 1 };
        let mut cursor = args_node.walk();
        let arg_nodes: Vec<tree_sitter::Node<'_>> = args_node.named_children(&mut cursor).collect();
        if arg_nodes.len() <= start_idx {
            continue;
        }

        let mut args = Vec::new();
        for arg in &arg_nodes[start_idx..] {
            if parsed.language.is_identifier_node(arg.kind()) {
                args.push(ExpressHandlerArg::Identifier(
                    parsed.node_text(arg).to_string(),
                ));
            } else if parsed.language.function_node_types().contains(&arg.kind()) {
                args.push(ExpressHandlerArg::InlineAnonymous);
            }
            // Any other argument shape (member expression, call, literal,
            // object/array literal, ...) is not a function-valued argument by
            // our narrow definition — not counted, not recorded.
        }
        if args.is_empty() {
            continue;
        }

        let site_line = call.start_position().row + 1;
        let enclosing = parsed
            .enclosing_function(site_line)
            .and_then(|node| js_ts_enclosing_facts(parsed, node));

        out.push(ExpressRouteCandidate {
            enclosing,
            site_line,
            site_start_byte: call.start_byte(),
            site_end_byte: call.end_byte(),
            args,
        });
    }
    out
}

/// `None` = not an express route-registration call. `Some(false)` = matched
/// via a direct instance receiver (arg 0 is the path). `Some(true)` = matched
/// via the `.route(path).get(...)` builder receiver (no path arg in THIS
/// call's own arguments).
fn express_call_match_kind(
    parsed: &ParsedFile,
    call: &tree_sitter::Node<'_>,
    receivers: &std::collections::BTreeSet<String>,
    imports: &std::collections::BTreeMap<String, String>,
) -> Option<bool> {
    let function = call.child_by_field_name("function")?;
    let function = crate::algorithms::taint::unwrap_parenthesized(function);
    if function.kind() != "member_expression" {
        return None;
    }
    let property = function.child_by_field_name("property")?;
    let method = parsed
        .node_text(&property)
        .trim_matches(|c| c == '\'' || c == '"' || c == '`');
    if !crate::algorithms::taint::js_ts_framework_route_method("express", method) {
        return None;
    }
    let receiver = function.child_by_field_name("object")?;
    if crate::algorithms::taint::js_ts_receiver_expr_is_framework_instance(
        parsed, receiver, "express", receivers, imports,
    ) {
        return Some(false);
    }
    if crate::algorithms::taint::js_ts_receiver_expr_is_route_builder(
        parsed, receiver, "express", receivers, imports,
    ) {
        return Some(true);
    }
    None
}

fn js_ts_enclosing_facts(
    parsed: &ParsedFile,
    node: tree_sitter::Node<'_>,
) -> Option<EnclosingFacts> {
    let name_node = parsed.language.function_name(&node)?;
    let name = parsed.node_text(&name_node).to_string();
    let (start_line, end_line) = parsed.node_line_range(&node);
    Some(EnclosingFacts {
        name,
        start_line,
        end_line,
    })
}

#[cfg(test)]
mod express_candidate_tests {
    use super::*;
    use crate::languages::Language::JavaScript;

    fn parse(src: &str) -> ParsedFile {
        ParsedFile::parse("app.js", src, JavaScript).unwrap()
    }

    #[test]
    fn named_handler_recorded_at_module_level() {
        let parsed = parse(
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\napp.get(\"/x\", handler);\n",
        );
        let cands = express_route_candidates(&parsed);
        assert_eq!(cands.len(), 1);
        assert!(cands[0].enclosing.is_none(), "module-level registration");
        assert_eq!(cands[0].args.len(), 1);
        assert!(matches!(&cands[0].args[0], ExpressHandlerArg::Identifier(n) if n == "handler"));
        assert_eq!(cands[0].site_line, 6);
    }

    #[test]
    fn multi_arg_middleware_and_handler_both_present() {
        let parsed = parse(
            "const express = require(\"express\");\nconst app = express();\n\nfunction mw(req, res, next) {}\nfunction handler(req, res) {}\n\napp.get(\"/x\", mw, handler);\n",
        );
        let cands = express_route_candidates(&parsed);
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].args.len(), 2);
        assert!(matches!(&cands[0].args[0], ExpressHandlerArg::Identifier(n) if n == "mw"));
        assert!(matches!(&cands[0].args[1], ExpressHandlerArg::Identifier(n) if n == "handler"));
    }

    #[test]
    fn inline_arrow_arg_is_anonymous() {
        let parsed = parse(
            "const express = require(\"express\");\nconst app = express();\n\napp.get(\"/x\", (req, res) => {});\n",
        );
        let cands = express_route_candidates(&parsed);
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].args.len(), 1);
        assert!(matches!(
            cands[0].args[0],
            ExpressHandlerArg::InlineAnonymous
        ));
    }

    #[test]
    fn registration_inside_setup_function_has_enclosing() {
        let parsed = parse(
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nfunction setup() {\n    app.get(\"/x\", handler);\n}\n",
        );
        let cands = express_route_candidates(&parsed);
        assert_eq!(cands.len(), 1);
        let enclosing = cands[0]
            .enclosing
            .as_ref()
            .expect("registration inside setup() must have an enclosing function");
        assert_eq!(enclosing.name, "setup");
    }

    #[test]
    fn route_builder_args_start_at_zero() {
        let parsed = parse(
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\napp.route(\"/x\").get(handler);\n",
        );
        let cands = express_route_candidates(&parsed);
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].args.len(), 1);
        assert!(matches!(&cands[0].args[0], ExpressHandlerArg::Identifier(n) if n == "handler"));
    }

    #[test]
    fn non_express_receiver_is_not_recorded() {
        let parsed = parse(
            "function handler(req, res) {}\nconst x = { get(path, cb) {} };\nx.get(\"/y\", handler);\n",
        );
        let cands = express_route_candidates(&parsed);
        assert!(cands.is_empty());
    }
}
