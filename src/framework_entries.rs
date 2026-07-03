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
use crate::call_graph::FunctionId;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

/// The synthetic module-level pseudo-caller name for an incoming-only
/// registration recorded at module/top level (the common case for both
/// Python route decorators and Express `app.get(...)` calls). Not a valid
/// identifier in ANY supported language (`<`/`>` are not identifier
/// characters), so it can never collide with a real function name — see
/// `module_pseudo_caller` and the S3 incoming-only merge rule in
/// `navigation::mod::build_resolved_call_edges`.
pub const MODULE_PSEUDO_CALLER_NAME: &str = "<module>";

/// The source location of one recognized framework route registration (the
/// decorator for Python, the registration call for Express). Mirrors
/// `RegistrationSite`/`PropertyAccessSite`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct FrameworkEntrySite {
    pub file: String,
    pub line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

/// P9 S1/S2: one recognized Flask/FastAPI/Express route registration binding
/// a handler function to its registering scope.
///
/// Deliberately NOT a `CallSite` — mirrors the architecture note on
/// `CallGraph::go_registrations`/`CallGraph::property_accesses`: a route
/// registration (`app.get("/x", handler)`, `@app.route("/x")`) is an
/// entrypoint/discoverability fact, not an executable call at that line —
/// minting a synthetic CallSite would assert dataflow that doesn't exist
/// there. Surfaced as NameOnly `framework_entry` nav edges at query time
/// (`NavigationIndex::build_resolved_call_edges`). Nav-only per the
/// consumer-visibility doctrine — never consulted by CPG Call/Return edges,
/// Step-5b DataFlow, taint, or any other non-nav consumer.
///
/// `caller` is either the real enclosing function (registration nested
/// inside a setup/factory function) or the `MODULE_PSEUDO_CALLER_NAME`
/// sentinel for a module-level registration (the common case). The S3 merge
/// (`navigation::mod::build_resolved_call_edges`) inserts every record into
/// `incoming_by_target[handler]`, but ONLY inserts into
/// `outgoing_by_caller[caller]` when `caller` is a REAL enclosing function —
/// the module pseudo-caller is not navigable (no CPG node, no Module symbol
/// kind) and must never appear as an outgoing/callees entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct FrameworkEntryRecord {
    pub caller: FunctionId,
    pub handler: FunctionId,
    pub site: FrameworkEntrySite,
    pub framework: String,
}

/// Orchestrates `python_route_candidates`/`express_route_candidates` across
/// every file into the whole-program `framework_entries` table, doing the
/// Express identifier-arg resolution (same-file function-index lookup +
/// local-shadow guard) that needs `functions`/`js_ts_function_locals` — state
/// this module doesn't own, passed in by `CallGraph::apply_framework_entries`.
/// Returns `(entries, unresolved_handler_count)`.
pub fn apply(
    files: &BTreeMap<String, ParsedFile>,
    functions: &BTreeMap<String, Vec<FunctionId>>,
    js_ts_function_locals: &BTreeMap<FunctionId, BTreeSet<String>>,
) -> (BTreeSet<FrameworkEntryRecord>, usize) {
    let mut entries = BTreeSet::new();
    let mut unresolved = 0usize;

    for (file_path, parsed) in files {
        match parsed.language {
            Language::Python => {
                for cand in python_route_candidates(parsed) {
                    let handler = FunctionId {
                        file: file_path.clone(),
                        name: cand.handler_name,
                        start_line: cand.handler_start_line,
                        end_line: cand.handler_end_line,
                    };
                    let caller = build_caller(file_path, parsed, cand.enclosing);
                    entries.insert(FrameworkEntryRecord {
                        caller,
                        handler,
                        site: FrameworkEntrySite {
                            file: file_path.clone(),
                            line: cand.site_line,
                            start_byte: cand.site_start_byte,
                            end_byte: cand.site_end_byte,
                        },
                        framework: cand.framework.to_string(),
                    });
                }
            }
            Language::JavaScript | Language::TypeScript | Language::Tsx => {
                for cand in express_route_candidates(parsed) {
                    let caller = build_caller(file_path, parsed, cand.enclosing.clone());
                    // The shadow guard reads `js_ts_function_locals` keyed by
                    // the REAL enclosing function's FunctionId — the module
                    // pseudo-caller never has a locals entry (it's not a
                    // function `all_functions()` ever visits), so a
                    // module-level registration simply has no shadow set to
                    // check, matching the Go registration precedent's
                    // "no binding -> not shadowed" default.
                    let locals_key = cand.enclosing.as_ref().map(|e| FunctionId {
                        file: file_path.clone(),
                        name: e.name.clone(),
                        start_line: e.start_line,
                        end_line: e.end_line,
                    });
                    for arg in cand.args {
                        match arg {
                            ExpressHandlerArg::InlineAnonymous => {
                                unresolved += 1;
                            }
                            ExpressHandlerArg::Identifier(name) => {
                                let matches: Vec<&FunctionId> = functions
                                    .get(&name)
                                    .into_iter()
                                    .flatten()
                                    .filter(|f| f.file == *file_path)
                                    .collect();
                                if matches.len() != 1 {
                                    unresolved += 1;
                                    continue;
                                }
                                let shadowed = locals_key
                                    .as_ref()
                                    .and_then(|k| js_ts_function_locals.get(k))
                                    .is_some_and(|locals| locals.contains(&name));
                                if shadowed {
                                    unresolved += 1;
                                    continue;
                                }
                                entries.insert(FrameworkEntryRecord {
                                    caller: caller.clone(),
                                    handler: matches[0].clone(),
                                    site: FrameworkEntrySite {
                                        file: file_path.clone(),
                                        line: cand.site_line,
                                        start_byte: cand.site_start_byte,
                                        end_byte: cand.site_end_byte,
                                    },
                                    framework: "express".to_string(),
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    (entries, unresolved)
}

fn build_caller(
    file_path: &str,
    parsed: &ParsedFile,
    enclosing: Option<EnclosingFacts>,
) -> FunctionId {
    match enclosing {
        Some(f) => FunctionId {
            file: file_path.to_string(),
            name: f.name,
            start_line: f.start_line,
            end_line: f.end_line,
        },
        None => module_pseudo_caller(file_path, parsed),
    }
}

/// Build the `<module>` synthetic pseudo-caller for a module-level
/// registration. `start_line` is fixed at 1; `end_line` is the file's last
/// line (per `ParsedFile::node_line_range` on the root node).
fn module_pseudo_caller(file_path: &str, parsed: &ParsedFile) -> FunctionId {
    let (_, last_line) = parsed.node_line_range(&parsed.tree.root_node());
    FunctionId {
        file: file_path.to_string(),
        name: MODULE_PSEUDO_CALLER_NAME.to_string(),
        start_line: 1,
        end_line: last_line,
    }
}

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
