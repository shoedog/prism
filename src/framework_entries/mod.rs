//! P9 S1: framework-entry navigation edges (Flask/FastAPI/Express route
//! registrations as nav-only `framework_entry` caller edges).
//!
//! Mirrors the P5 (`go_registrations`)/P7 (`property_accesses`) dedicated-
//! table pattern: candidate detection lives in the `python`/`express`
//! submodules as pure, owned-data extraction (no `tree_sitter::Node` leaks
//! into `call_graph.rs`, mirroring `ast.rs`'s `GoRegistrationCandidate`/
//! `PythonAttributeLoadCandidate`), and `CallGraph::apply_framework_entries`
//! (call_graph.rs) owns the whole-program table + telemetry fields and
//! orchestrates `apply` below against the CallGraph's own state.
//!
//! Scope this slice: Flask + FastAPI + Express only (fastify/koa/nestjs/
//! django/drf/gin/gorilla/nethttp are untouched).
//!
//! Split into submodules to keep each file under the codebase's 600-line
//! cap: this file holds the shared types + orchestration, `python.rs` holds
//! Flask/FastAPI extraction, `express.rs` holds Express extraction.

pub mod express;
pub mod python;

use crate::ast::ParsedFile;
use crate::call_graph::FunctionId;
use crate::languages::Language;
use express::ExpressHandlerArg;
use std::collections::{BTreeMap, BTreeSet};

/// The synthetic module-level pseudo-caller name for an incoming-only
/// registration recorded at module/top level (the common case for both
/// Python route decorators and Express `app.get(...)` calls). Not a valid
/// identifier in ANY supported language (`<`/`>` are not identifier
/// characters), so it can never collide with a real function name — see
/// `module_pseudo_caller` and the S3 incoming-only merge rule in
/// `navigation::mod::build_resolved_call_edges`.
pub const MODULE_PSEUDO_CALLER_NAME: &str = "<module>";

/// Enclosing-function facts needed to build a `FunctionId`, extracted here so
/// `call_graph.rs` never needs to touch a `tree_sitter::Node` for
/// framework-entry enclosing lookup. `None` (absent) means "no enclosing
/// function" — i.e. a module-level registration. Shared by the `python` and
/// `express` submodules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnclosingFacts {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
}

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

/// Orchestrates `python::python_route_candidates`/
/// `express::express_route_candidates` across every file into the
/// whole-program `framework_entries` table, doing the Express identifier-arg
/// resolution (same-file function-index lookup + local-shadow guard) that
/// needs `functions`/`js_ts_function_locals` — state this module doesn't
/// own, passed in by `CallGraph::apply_framework_entries`. Returns
/// `(entries, unresolved_handler_count)`.
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
                for cand in python::python_route_candidates(parsed) {
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
                // F1: restrict identifier-arg resolution to BARE BINDINGS —
                // `functions` (passed in from `CallGraph`) indexes every
                // JS/TS function-like node including class/object methods
                // (see `express::is_bare_binding_function`'s doc comment),
                // so an unfiltered name-only lookup can mint a false edge to
                // an unrelated same-named method. Computed once per file
                // (not per candidate) since it only depends on `parsed`.
                let bare_binding_ids: BTreeSet<FunctionId> = parsed
                    .all_functions()
                    .into_iter()
                    .filter(express::is_bare_binding_function)
                    .filter_map(|node| {
                        let name_node = parsed.language.function_name(&node)?;
                        let name = parsed.node_text(&name_node).to_string();
                        let (start_line, end_line) = parsed.node_line_range(&node);
                        Some(FunctionId {
                            file: file_path.clone(),
                            name,
                            start_line,
                            end_line,
                        })
                    })
                    .collect();

                for cand in express::express_route_candidates(parsed) {
                    // F2+M2+M3: reject the WHOLE candidate when the
                    // RECEIVER identifier — or, for a direct-constructor
                    // receiver (M2), its constructor-grounding identifier —
                    // is locally shadowed in ANY enclosing scope, named or
                    // anonymous (M3). `receivers`/import-map grounding in
                    // `express::express_route_candidates` is collected
                    // file-wide with no scope awareness, so a same-named
                    // parameter in an enclosing function — including an
                    // anonymous one (`(app) => { app.get(...) }`) — would
                    // otherwise mint an edge against the non-grounded
                    // parameter instead of the real express instance.
                    // `cand.shadowed` is computed by `express.rs` itself via
                    // an AST-ancestor walk from the actual call node (see
                    // `express::express_receiver_is_shadowed`), not this
                    // module's FunctionId-keyed `js_ts_function_locals` —
                    // that index only covers NAMED functions, which is
                    // exactly the M3 gap. Conservative shadow-bail — the
                    // same house pattern P6-lite's receiver typing uses —
                    // rejects rather than guesses.
                    if cand.shadowed {
                        unresolved += cand.args.len();
                        continue;
                    }

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
                                    .filter(|f| {
                                        f.file == *file_path && bare_binding_ids.contains(f)
                                    })
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
