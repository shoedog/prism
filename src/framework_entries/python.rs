//! P9 S1: Flask/FastAPI route-decorator candidate extraction.

use super::EnclosingFacts;
use crate::ast::ParsedFile;
use crate::languages::Language;

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
            // F4: if the enclosing function is ITSELF decorated, key the
            // range to the `decorated_definition` WRAPPER (keeping the name
            // from `fd`) rather than the inner `function_definition` — this
            // is how Python functions are keyed everywhere else (the
            // Functions query captures `decorated_definition`;
            // `ParsedFile::all_functions` filters out the inner
            // `function_definition` for a decorated function), so using the
            // inner range here would produce an `outgoing_by_caller` key
            // that `nav callees` can never match against the canonical
            // FunctionId.
            let range_node = fd
                .parent()
                .filter(|p| p.kind() == "decorated_definition")
                .unwrap_or(fd);
            let (start_line, end_line) = parsed.node_line_range(&range_node);
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
    // F6 (opus nit) asked to return directly instead of let-binding, but
    // that doesn't compile: `children()`'s returned iterator borrows
    // `cursor` for `'cursor`, and in tail position the compiler extends that
    // temporary's lifetime past `cursor`'s own scope ("temporary value
    // dropped while still borrowed") -- the `let` binding here forces the
    // iterator to drop before the function returns, which is why the
    // original code has it. Kept as-is; not a nit worth papering over with
    // an `#[allow]`.
    let mut cursor = node.walk();
    let found = node
        .children(&mut cursor)
        .find(|c| c.kind() == "function_definition");
    found
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
    fn flask_route_with_kwargs_is_still_recorded() {
        // F6 pin: `decorator_receiver_and_method` matches on the decorator's
        // `function` field only (`app.route`), never its call arguments, so
        // kwargs like `methods=["GET", "POST"]` must not break the match.
        let parsed = parse(
            "from flask import Flask\napp = Flask(__name__)\n\n@app.route(\"/x\", methods=[\"GET\", \"POST\"])\ndef handler():\n    return \"ok\"\n",
        );
        let cands = python_route_candidates(&parsed);
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].handler_name, "handler");
        assert_eq!(cands[0].framework, "flask");
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

    #[test]
    fn decorated_factory_enclosing_uses_the_decorated_definition_wrapper_range() {
        // `make_app` is ITSELF decorated -- the enclosing FunctionId must use
        // the `decorated_definition` WRAPPER's range (starting at the
        // decorator line), not the inner `function_definition`'s range,
        // because that's how Python functions are keyed everywhere else
        // (queries.rs's Functions query captures `decorated_definition`;
        // `ParsedFile::all_functions` filters out the inner
        // `function_definition` for a decorated function) -- see F4.
        let parsed = parse(
            "from flask import Flask\n\napp = Flask(__name__)\n\n\n@some_decorator\ndef make_app():\n    @app.route(\"/x\")\n    def handler():\n        return \"ok\"\n    return app\n",
        );
        let cands = python_route_candidates(&parsed);
        assert_eq!(cands.len(), 1);
        let enclosing = cands[0]
            .enclosing
            .as_ref()
            .expect("registration nested inside make_app must have an enclosing function");
        assert_eq!(enclosing.name, "make_app");
        assert_eq!(
            enclosing.start_line, 6,
            "must start at the decorator line (the decorated_definition wrapper), not the `def` line"
        );
        assert_eq!(enclosing.end_line, 11);
    }
}
