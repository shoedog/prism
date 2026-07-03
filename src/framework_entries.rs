//! P9 S1: framework-entry navigation edges (Flask/FastAPI/Express route
//! registrations as nav-only `framework_entry` caller edges).
//!
//! Mirrors the P5 (`go_registrations`)/P7 (`property_accesses`) dedicated-
//! table pattern: candidate detection lives here as pure, owned-data
//! extraction (no `tree_sitter::Node` leaks into `call_graph.rs`, mirroring
//! `ast.rs`'s `GoRegistrationCandidate`/`PythonAttributeLoadCandidate`), and
//! `CallGraph::apply_framework_entries` (call_graph.rs) owns the whole-
//! program table + telemetry fields and orchestrates this module's
//! extraction functions plus (for Express) the functions-index/shadow-guard
//! resolution that needs the CallGraph's own state.
//!
//! Scope this slice: Flask + FastAPI + Express only (fastify/koa/nestjs/
//! django/drf/gin/gorilla/nethttp are untouched).

use crate::ast::ParsedFile;
use crate::languages::Language;

/// Enclosing-function facts needed to build a `FunctionId`, extracted here so
/// `call_graph.rs` never needs to touch a `tree_sitter::Node` for
/// framework-entry enclosing lookup. `None` (absent) means "no enclosing
/// function" — i.e. a module-level registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnclosingFacts {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
}

/// One recognized Python route-decorator registration site. The handler IS
/// the decorated function (trivial FunctionId, no further resolution needed —
/// unlike Express, where a bare identifier still needs to be resolved against
/// the whole-program function index).
#[derive(Debug, Clone)]
pub struct PythonRouteCandidate {
    pub handler_name: String,
    pub handler_start_line: usize,
    pub handler_end_line: usize,
    pub enclosing: Option<EnclosingFacts>,
    pub site_line: usize,
    pub site_start_byte: usize,
    pub site_end_byte: usize,
    pub framework: &'static str,
}

/// Extract every recognized Flask/FastAPI route-decorator registration in
/// `parsed`. Walks `all_functions()` (mirroring `apply_go_registrations`'s /
/// `apply_python_property_accesses`'s per-function iteration convention) so
/// the handler's FunctionId falls out naturally, then asks each framework's
/// site-returning decorator helper for every matching decorator on that
/// function — a handler with two route decorators yields two candidates.
pub fn python_route_candidates(parsed: &ParsedFile) -> Vec<PythonRouteCandidate> {
    let mut out = Vec::new();
    if parsed.language != Language::Python {
        return out;
    }

    let flask_receivers = crate::frameworks::python::flask::route_receivers(parsed);
    let fastapi_receivers = crate::frameworks::python::fastapi::route_receivers(parsed);
    if flask_receivers.is_empty() && fastapi_receivers.is_empty() {
        return out;
    }

    for func in parsed.all_functions() {
        let Some(name_node) = parsed.language.function_name(&func) else {
            continue;
        };
        let handler_name = parsed.node_text(&name_node).to_string();
        let (handler_start_line, handler_end_line) = parsed.node_line_range(&func);
        let enclosing = python_enclosing_facts(parsed, &func);

        let flask_sites = crate::frameworks::python::flask::route_decorator_sites(
            parsed,
            &func,
            &flask_receivers,
        );
        for site in flask_sites {
            out.push(PythonRouteCandidate {
                handler_name: handler_name.clone(),
                handler_start_line,
                handler_end_line,
                enclosing: enclosing.clone(),
                site_line: site.line,
                site_start_byte: site.start_byte,
                site_end_byte: site.end_byte,
                framework: "flask",
            });
        }

        let fastapi_sites = crate::frameworks::python::fastapi::route_decorator_sites(
            parsed,
            &func,
            &fastapi_receivers,
        );
        for site in fastapi_sites {
            out.push(PythonRouteCandidate {
                handler_name: handler_name.clone(),
                handler_start_line,
                handler_end_line,
                enclosing: enclosing.clone(),
                site_line: site.line,
                site_start_byte: site.start_byte,
                site_end_byte: site.end_byte,
                framework: "fastapi",
            });
        }
    }
    out
}

/// Find the function TRULY enclosing `func` (i.e. the outer scope `func` is
/// nested inside — e.g. an app-factory), or `None` for a module-level
/// definition.
///
/// Deliberately walks the PARENT chain from `func` itself rather than doing a
/// line-based `ParsedFile::enclosing_function` lookup on the decorator's own
/// line: Python's `function_node_types()` includes BOTH
/// `"function_definition"` AND `"decorated_definition"` (languages/mod.rs),
/// and `decorated_definition`'s span starts at the FIRST decorator — so a
/// line-based search rooted at the decorator line would match the handler's
/// OWN `decorated_definition` wrapper as the "smallest enclosing function"
/// and incorrectly report the handler as its own enclosing scope. Walking
/// from `func`'s parent skips past the handler's own wrapper entirely.
fn python_enclosing_facts(
    parsed: &ParsedFile,
    func: &tree_sitter::Node<'_>,
) -> Option<EnclosingFacts> {
    // Skip past the handler's own `decorated_definition` wrapper (if present)
    // before starting the upward walk, so we never re-match `func` itself.
    let self_wrapper = func
        .parent()
        .filter(|p| p.kind() == "decorated_definition")
        .unwrap_or(*func);

    let mut current = self_wrapper.parent();
    while let Some(node) = current {
        let fd_node = match node.kind() {
            "function_definition" => Some(node),
            "decorated_definition" => first_function_definition_child(node),
            _ => None,
        };
        if let Some(fd) = fd_node {
            let name_node = parsed.language.function_name(&fd)?;
            let name = parsed.node_text(&name_node).to_string();
            let (start_line, end_line) = parsed.node_line_range(&fd);
            return Some(EnclosingFacts {
                name,
                start_line,
                end_line,
            });
        }
        current = node.parent();
    }
    None
}

fn first_function_definition_child<'a>(
    node: tree_sitter::Node<'a>,
) -> Option<tree_sitter::Node<'a>> {
    let mut cursor = node.walk();
    let found = node
        .children(&mut cursor)
        .find(|c| c.kind() == "function_definition");
    found
}

// ---------------------------------------------------------------------------
// Express (JS/TS/Tsx): registration-first extraction
// ---------------------------------------------------------------------------

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

#[cfg(test)]
mod python_candidate_tests {
    use super::*;
    use crate::languages::Language::Python;

    fn parse(src: &str) -> ParsedFile {
        ParsedFile::parse("app.py", src, Python).unwrap()
    }

    #[test]
    fn flask_route_recorded_with_decorator_line() {
        let parsed = parse(
            "from flask import Flask\napp = Flask(__name__)\n\n@app.route(\"/x\")\ndef handler():\n    return \"ok\"\n",
        );
        let cands = python_route_candidates(&parsed);
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].handler_name, "handler");
        assert_eq!(cands[0].framework, "flask");
        assert_eq!(cands[0].site_line, 4);
        assert!(cands[0].enclosing.is_none(), "module-level registration");
    }

    #[test]
    fn fastapi_route_recorded_with_decorator_line() {
        let parsed = parse(
            "from fastapi import FastAPI\napp = FastAPI()\n\n@app.get(\"/x\")\ndef handler():\n    return \"ok\"\n",
        );
        let cands = python_route_candidates(&parsed);
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].handler_name, "handler");
        assert_eq!(cands[0].framework, "fastapi");
        assert_eq!(cands[0].site_line, 4);
    }

    #[test]
    fn two_route_decorators_on_one_handler_yield_two_candidates() {
        let parsed = parse(
            "from flask import Flask\napp = Flask(__name__)\n\n@app.route(\"/a\")\n@app.route(\"/b\")\ndef handler():\n    return \"ok\"\n",
        );
        let cands = python_route_candidates(&parsed);
        assert_eq!(cands.len(), 2);
        let mut lines: Vec<usize> = cands.iter().map(|c| c.site_line).collect();
        lines.sort();
        assert_eq!(lines, vec![4, 5]);
        assert!(cands.iter().all(|c| c.handler_name == "handler"));
    }

    #[test]
    fn non_route_decorator_is_not_recorded() {
        let parsed = parse(
            "from flask import Flask\napp = Flask(__name__)\n\nclass Foo:\n    @property\n    def value(self):\n        return 1\n",
        );
        let cands = python_route_candidates(&parsed);
        assert!(cands.is_empty());
    }

    #[test]
    fn nested_registration_enclosing_is_the_factory_function() {
        let parsed = parse(
            "from flask import Flask\n\n\ndef create_app():\n    app = Flask(__name__)\n\n    @app.route(\"/x\")\n    def handler():\n        return \"ok\"\n\n    return app\n",
        );
        let cands = python_route_candidates(&parsed);
        assert_eq!(cands.len(), 1);
        let enclosing = cands[0]
            .enclosing
            .as_ref()
            .expect("registration nested inside create_app must have an enclosing function");
        assert_eq!(enclosing.name, "create_app");
    }
}
