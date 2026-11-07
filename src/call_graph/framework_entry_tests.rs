use super::*;
use crate::framework_entries::MODULE_PSEUDO_CALLER_NAME;
use crate::languages::Language::{Go, JavaScript, Python};
use std::collections::BTreeMap;

fn build_py(files: &[(&str, &str)]) -> CallGraph {
    let mut map = BTreeMap::new();
    for (path, src) in files {
        map.insert(
            path.to_string(),
            ParsedFile::parse(path, src, Python).unwrap(),
        );
    }
    CallGraph::build(&map)
}

fn build_js(files: &[(&str, &str)]) -> CallGraph {
    let mut map = BTreeMap::new();
    for (path, src) in files {
        map.insert(
            path.to_string(),
            ParsedFile::parse(path, src, JavaScript).unwrap(),
        );
    }
    CallGraph::build(&map)
}

fn fid<'a>(cg: &'a CallGraph, name: &str) -> &'a FunctionId {
    cg.functions
        .get(name)
        .unwrap_or_else(|| panic!("no function named {name}"))
        .first()
        .unwrap()
}

// ---- Python: Flask / FastAPI ------------------------------------------

#[test]
fn flask_route_recorded_with_decorator_line() {
    let cg = build_py(&[(
            "app.py",
            "from flask import Flask\napp = Flask(__name__)\n\n@app.route(\"/x\")\ndef handler():\n    return \"ok\"\n",
        )]);
    let handler = fid(&cg, "handler").clone();
    let rec = cg
        .framework_entries
        .iter()
        .find(|r| r.handler == handler)
        .expect("flask route recorded");
    assert_eq!(rec.site.line, 4);
    assert_eq!(rec.framework, "flask");
    assert_eq!(rec.caller.name, MODULE_PSEUDO_CALLER_NAME);
    assert_eq!(rec.caller.start_line, 1);
}

#[test]
fn fastapi_route_recorded() {
    let cg = build_py(&[(
            "app.py",
            "from fastapi import FastAPI\napp = FastAPI()\n\n@app.get(\"/x\")\ndef handler():\n    return \"ok\"\n",
        )]);
    let handler = fid(&cg, "handler").clone();
    let rec = cg
        .framework_entries
        .iter()
        .find(|r| r.handler == handler)
        .expect("fastapi route recorded");
    assert_eq!(rec.framework, "fastapi");
    assert_eq!(rec.site.line, 4);
}

#[test]
fn two_route_decorators_yield_two_records() {
    let cg = build_py(&[(
            "app.py",
            "from flask import Flask\napp = Flask(__name__)\n\n@app.route(\"/a\")\n@app.route(\"/b\")\ndef handler():\n    return \"ok\"\n",
        )]);
    let handler = fid(&cg, "handler").clone();
    let recs: Vec<_> = cg
        .framework_entries
        .iter()
        .filter(|r| r.handler == handler)
        .collect();
    assert_eq!(recs.len(), 2, "two route decorators -> two records");
}

#[test]
fn non_route_decorator_not_recorded() {
    let cg = build_py(&[(
            "app.py",
            "from flask import Flask\napp = Flask(__name__)\n\nclass Foo:\n    @property\n    def value(self):\n        return 1\n",
        )]);
    assert!(cg.framework_entries.is_empty());
}

#[test]
fn python_nested_registration_enclosing_is_factory_function() {
    let cg = build_py(&[(
            "app.py",
            "from flask import Flask\n\n\ndef create_app():\n    app = Flask(__name__)\n\n    @app.route(\"/x\")\n    def handler():\n        return \"ok\"\n\n    return app\n",
        )]);
    let handler = fid(&cg, "handler").clone();
    let rec = cg
        .framework_entries
        .iter()
        .find(|r| r.handler == handler)
        .expect("nested route recorded");
    assert_eq!(rec.caller.name, "create_app");
}

#[test]
fn python_decorated_factory_enclosing_caller_matches_canonical_function_id() {
    // The factory `make_app` is ITSELF decorated -- `CallGraph::functions`
    // keys decorated Python functions by the `decorated_definition`
    // WRAPPER range (the Functions query captures `decorated_definition`;
    // `all_functions_via_tree` filters out the inner `function_definition`
    // for a decorated function), so the nested registration's `caller`
    // FunctionId must match that SAME wrapper range exactly, or
    // `nav callees(make_app)` can never find the outgoing edge (F4).
    let cg = build_py(&[(
            "app.py",
            "from flask import Flask\n\napp = Flask(__name__)\n\n\n@some_decorator\ndef make_app():\n    @app.route(\"/x\")\n    def handler():\n        return \"ok\"\n    return app\n",
        )]);
    let canonical_make_app = cg
        .functions
        .get("make_app")
        .and_then(|v| v.first())
        .expect("make_app indexed")
        .clone();
    let handler = fid(&cg, "handler").clone();
    let rec = cg
        .framework_entries
        .iter()
        .find(|r| r.handler == handler)
        .expect("nested route recorded");
    assert_eq!(
        rec.caller, canonical_make_app,
        "caller FunctionId must match the canonical (decorated-wrapper) range"
    );
    assert_eq!(rec.caller.start_line, 6);
}

// ---- Express -----------------------------------------------------------

#[test]
fn express_named_handler_recorded_at_module_level() {
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\napp.get(\"/x\", handler);\n",
        )]);
    let handler = fid(&cg, "handler").clone();
    let rec = cg
        .framework_entries
        .iter()
        .find(|r| r.handler == handler)
        .expect("express handler recorded");
    assert_eq!(rec.framework, "express");
    assert_eq!(rec.caller.name, MODULE_PSEUDO_CALLER_NAME);
    assert_eq!(rec.caller.start_line, 1);
    assert_eq!(rec.site.line, 6);
}

#[test]
fn express_multi_arg_middleware_and_handler_both_recorded() {
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction mw(req, res, next) {}\nfunction handler(req, res) {}\n\napp.get(\"/x\", mw, handler);\n",
        )]);
    let mw = fid(&cg, "mw").clone();
    let handler = fid(&cg, "handler").clone();
    assert!(
        cg.framework_entries.iter().any(|r| r.handler == mw),
        "middleware arg must be recorded"
    );
    assert!(
        cg.framework_entries.iter().any(|r| r.handler == handler),
        "handler arg must be recorded"
    );
}

#[test]
fn express_shadowed_identifier_is_skipped_and_counted() {
    // `handler` is a PARAMETER of `setup`, locally shadowing the
    // top-level `handler` function -- the reference inside `setup` names
    // the local, not the free function, so it must be skipped even
    // though exactly one same-file function named `handler` exists.
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nfunction setup(handler) {\n    app.get(\"/x\", handler);\n}\n",
        )]);
    assert!(
        cg.framework_entries.is_empty(),
        "shadowed identifier must not be recorded"
    );
    assert_eq!(cg.framework_entry_unresolved_handlers, 1);
}

#[test]
fn express_inline_arrow_arg_is_skipped_and_counted() {
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\napp.get(\"/x\", (req, res) => {});\n",
        )]);
    assert!(cg.framework_entries.is_empty());
    assert_eq!(cg.framework_entry_unresolved_handlers, 1);
}

#[test]
fn express_zero_match_identifier_is_skipped_and_counted() {
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\napp.get(\"/x\", missingHandler);\n",
        )]);
    assert!(cg.framework_entries.is_empty());
    assert_eq!(cg.framework_entry_unresolved_handlers, 1);
}

#[test]
fn express_multi_match_identifier_is_skipped_and_counted() {
    // Two same-named `handler` function declarations in the same file
    // (annex-B block-scoped `function` inside `if`) -- ambiguous, must
    // not record, must count.
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nif (true) {\n    function handler() {}\n}\n\napp.get(\"/x\", handler);\n",
        )]);
    assert_eq!(cg.framework_entry_unresolved_handlers, 1);
    assert!(!cg
        .framework_entries
        .iter()
        .any(|r| r.framework == "express"));
}

#[test]
fn express_registration_inside_setup_function_has_enclosing_caller() {
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nfunction setup() {\n    app.get(\"/x\", handler);\n}\n",
        )]);
    let handler = fid(&cg, "handler").clone();
    let rec = cg
        .framework_entries
        .iter()
        .find(|r| r.handler == handler)
        .expect("recorded");
    assert_eq!(rec.caller.name, "setup");
    assert_ne!(rec.caller.name, MODULE_PSEUDO_CALLER_NAME);
}

// ---- F1: bare-binding-only identifier resolution ------------------------

#[test]
fn express_identifier_matching_method_only_is_unresolved() {
    // `handler` exists ONLY as a class METHOD in this file -- a bare
    // identifier reference (`app.get("/x", handler)`) can never mean a
    // method (methods need a receiver), so this must be unresolved, not
    // a false edge to `C.handler`.
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nclass C {\n    handler() {}\n}\n\napp.get(\"/x\", handler);\n",
        )]);
    assert!(
        !cg.framework_entries
            .iter()
            .any(|r| r.framework == "express"),
        "method-only same-name match must not mint a framework_entry edge"
    );
    assert_eq!(cg.framework_entry_unresolved_handlers, 1);
}

#[test]
fn express_identifier_matching_prefers_bare_binding_over_same_name_method() {
    // A top-level function AND a same-name class method both named
    // `handler` -- the bare-binding filter must exclude the method,
    // leaving exactly one match (the top-level function) instead of
    // dropping the identifier for false ambiguity.
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nclass C {\n    handler() {}\n}\n\napp.get(\"/x\", handler);\n",
        )]);
    let free_handler = cg
        .functions
        .get("handler")
        .expect("handler name indexed")
        .iter()
        .find(|f| !cg.method_owners.contains_key(*f))
        .expect("free function handler must exist")
        .clone();
    let rec = cg
        .framework_entries
        .iter()
        .find(|r| r.framework == "express")
        .expect("express route recorded against the free function");
    assert_eq!(rec.handler, free_handler);
    assert_eq!(cg.framework_entry_unresolved_handlers, 0);
}

#[test]
fn express_identifier_matching_object_property_arrow_only_is_unresolved() {
    // M1 (a): `handler` exists ONLY as an object-literal property arrow
    // (`{ handler: () => {} }`) -- `languages::function_name` Pattern 2
    // still infers the name `handler` for it (so it enters
    // `CallGraph::functions`), but a bare `handler` identifier can never
    // reach a `pair` value (only `api.handler` can), so this must be
    // unresolved, not a false edge into the object property.
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nconst api = { handler: () => {} };\n\napp.get(\"/x\", handler);\n",
        )]);
    assert!(
        !cg.framework_entries
            .iter()
            .any(|r| r.framework == "express"),
        "object-property arrow must not mint a framework_entry edge"
    );
    assert_eq!(cg.framework_entry_unresolved_handlers, 1);
}

#[test]
fn express_identifier_matching_member_expression_assignment_only_is_unresolved() {
    // M1 (b): `handler` exists ONLY as `exports.handler = () => {}` --
    // Pattern 5 name inference (`languages::function_name`) names it
    // `handler` via the assignment's member-expression LHS property, but
    // a bare `handler` identifier can never reach `exports.handler`
    // (only the qualified reference can), so this must be unresolved.
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nexports.handler = () => {};\n\napp.get(\"/x\", handler);\n",
        )]);
    assert!(
        !cg.framework_entries
            .iter()
            .any(|r| r.framework == "express"),
        "member-expression-LHS assignment must not mint a framework_entry edge"
    );
    assert_eq!(cg.framework_entry_unresolved_handlers, 1);
}

#[test]
fn express_identifier_matching_variable_declarator_arrow_resolves() {
    // M1 (c) positive: `const handler = () => {}` -- Pattern 1 name
    // inference (bound via a `variable_declarator`) IS a genuine bare
    // binding (a bare `handler` reference really does reach it), so it
    // must keep resolving under the rewritten allow-list-based
    // `is_bare_binding_function`.
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nconst handler = (req, res) => {};\n\napp.get(\"/x\", handler);\n",
        )]);
    let handler = fid(&cg, "handler").clone();
    assert!(
        cg.framework_entries.iter().any(|r| r.handler == handler),
        "variable-declarator-bound arrow must still resolve"
    );
    assert_eq!(cg.framework_entry_unresolved_handlers, 0);
}

// ---- F2: Express receiver local-shadow guard ----------------------------

#[test]
fn express_receiver_shadowed_by_enclosing_parameter_is_skipped_and_counted() {
    // `app` is REBOUND as a parameter of `setup`, shadowing the
    // module-level `const app = express()` -- the receiver in
    // `app.get("/x", handler)` inside `setup` names the non-grounded
    // PARAMETER, not the express instance, so this must not mint an
    // edge even though `app` is a recognized express receiver name
    // file-wide.
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nfunction setup(app) {\n    app.get(\"/x\", handler);\n}\n",
        )]);
    assert!(
        cg.framework_entries.is_empty(),
        "shadowed receiver must not be recorded"
    );
    assert_eq!(cg.framework_entry_unresolved_handlers, 1);
}

#[test]
fn express_receiver_not_shadowed_when_setup_does_not_rebind_app() {
    // Non-shadowed control: `setup` does NOT take an `app` parameter, so
    // the receiver inside it is still the module-level express
    // instance -- must record normally. (Distinct from the pre-existing
    // `express_registration_inside_setup_function_has_enclosing_caller`
    // in that it asserts the unresolved counter stays at 0.)
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nfunction setup() {\n    app.get(\"/x\", handler);\n}\n",
        )]);
    let handler = fid(&cg, "handler").clone();
    assert!(
        cg.framework_entries.iter().any(|r| r.handler == handler),
        "non-shadowed receiver must still record"
    );
    assert_eq!(cg.framework_entry_unresolved_handlers, 0);
}

// ---- M2: direct-constructor receiver local-shadow guard -----------------

#[test]
fn express_direct_constructor_receiver_shadowed_by_enclosing_parameter_is_skipped_and_counted() {
    // `express` is REBOUND as a parameter of `setup`, shadowing the
    // module-level `express` import -- `express()` inside `setup` calls
    // the non-grounded PARAMETER, not the express factory, so this must
    // not mint an edge even though the receiver is a direct-constructor
    // form (`express().get(...)`), which the old
    // `express_receiver_identifier_name` returned `None` for and so
    // never shadow-checked at all.
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nfunction setup(express) {\n    express().get(\"/x\", handler);\n}\n",
        )]);
    assert!(
        cg.framework_entries.is_empty(),
        "direct constructor receiver shadowed by enclosing param must not record"
    );
    assert_eq!(cg.framework_entry_unresolved_handlers, 1);
}

#[test]
fn express_direct_constructor_receiver_not_shadowed_at_module_level_still_records() {
    // Non-shadowed control: a module-level direct-constructor receiver
    // (no enclosing function to shadow `express` at all) must still
    // record normally.
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nexpress().get(\"/x\", handler);\n",
        )]);
    let handler = fid(&cg, "handler").clone();
    assert!(
        cg.framework_entries.iter().any(|r| r.handler == handler),
        "non-shadowed module-level direct constructor receiver must still record"
    );
    assert_eq!(cg.framework_entry_unresolved_handlers, 0);
}

// ---- M3: shadow guard must see anonymous enclosing scopes ---------------

#[test]
fn express_receiver_shadowed_by_anonymous_iife_parameter_is_skipped_and_counted() {
    // `app` is REBOUND as a parameter of an ANONYMOUS IIFE (no
    // inferable name, so it can never be a `FunctionId` and is
    // invisible to the FunctionId-keyed `js_ts_function_locals` index)
    // -- the old shadow guard walked `enclosing_chain`, which only ever
    // contains NAMED enclosing functions, so this shadow was missed
    // entirely and a false edge was recorded.
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\n(function (app) {\n    app.get(\"/x\", handler);\n})(express());\n",
        )]);
    assert!(
        cg.framework_entries.is_empty(),
        "anonymous-scope receiver shadow must not be recorded"
    );
    assert_eq!(cg.framework_entry_unresolved_handlers, 1);
}

#[test]
fn express_receiver_not_shadowed_by_anonymous_iife_without_param_records_at_module_caller() {
    // Non-shadowed control: the anonymous IIFE does NOT rebind `app`, so
    // the receiver inside it is still the module-level express
    // instance -- must record normally. Because the wrapper is
    // anonymous, `enclosing` is still `None` for it (name inference
    // doesn't apply to an un-bound IIFE), so the caller stays the
    // `<module>` pseudo-caller -- verifies the M3 AST-walk rewrite left
    // the enclosing-CALLER determination for anonymous wrappers
    // unchanged, only the shadow/binding collection moved.
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\n(function () {\n    app.get(\"/x\", handler);\n})();\n",
        )]);
    let handler = fid(&cg, "handler").clone();
    let rec = cg
        .framework_entries
        .iter()
        .find(|r| r.handler == handler)
        .expect("non-shadowed anonymous-wrapper registration must still record");
    assert_eq!(rec.caller.name, MODULE_PSEUDO_CALLER_NAME);
    assert_eq!(cg.framework_entry_unresolved_handlers, 0);
}

// ---- M-A (codex re-review of fix wave 2): caller attribution walks past
// anonymous scopes to the nearest NAMED enclosing function ---------------

#[test]
fn express_registration_inside_anonymous_iife_nested_in_named_function_attributes_outer_function() {
    // The registration's DEEPEST enclosing function/method node is an
    // anonymous IIFE with no `FunctionId` -- the pre-fix
    // `enclosing_function(site_line)` (a top-down smallest-containing-
    // node search) stopped at that deepest node and returned `None`,
    // misattributing the caller to `<module>` even though `setup` truly
    // encloses the registration. The fix walks the call node's ACTUAL
    // AST ancestor chain and skips past anonymous function-like
    // ancestors to the nearest one that can be named.
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nfunction setup() {\n    (() => {\n        app.get(\"/x\", handler);\n    })();\n}\n",
        )]);
    let handler = fid(&cg, "handler").clone();
    let rec = cg
        .framework_entries
        .iter()
        .find(|r| r.handler == handler)
        .expect("registration inside the IIFE nested in setup() must still record");
    assert_eq!(rec.caller.name, "setup");
    assert_ne!(rec.caller.name, MODULE_PSEUDO_CALLER_NAME);
}

#[test]
fn express_registration_inside_top_level_iife_with_no_named_ancestor_attributes_module() {
    // Control: a top-level IIFE with NO named function anywhere in its
    // ancestor chain must still attribute to the `<module>` pseudo-
    // caller, unchanged by the M-A ancestor-walk rewrite.
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\n(() => {\n    app.get(\"/x\", handler);\n})();\n",
        )]);
    let handler = fid(&cg, "handler").clone();
    let rec = cg
        .framework_entries
        .iter()
        .find(|r| r.handler == handler)
        .expect("top-level IIFE registration must still record");
    assert_eq!(rec.caller.name, MODULE_PSEUDO_CALLER_NAME);
}

// ---- M-B (codex re-review of fix wave 2): `require("express").Router()`
// constructor grounding-id peel ------------------------------------------

#[test]
fn express_require_dot_router_constructor_receiver_shadowed_by_enclosing_parameter_is_skipped_and_counted(
) {
    // `require` is REBOUND as a parameter of `setup`, shadowing the
    // global Node.js `require` -- `require("express").Router()` inside
    // `setup` calls the non-grounded PARAMETER, not the real module
    // loader, so this must not mint an edge. The grounding-identifier
    // peel previously only handled a bare-identifier member-expression
    // object (`express.Router()`); a require-CALL object
    // (`require("express").Router()`) fell through and returned `None`,
    // silently skipping the shadow check entirely (M-B).
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nfunction setup(require) {\n    require(\"express\").Router().get(\"/x\", handler);\n}\n",
        )]);
    assert!(
        cg.framework_entries.is_empty(),
        "shadowed require-constructor receiver must not be recorded"
    );
    assert_eq!(cg.framework_entry_unresolved_handlers, 1);
}

#[test]
fn express_require_dot_router_constructor_receiver_not_shadowed_at_module_level_still_records() {
    // Non-shadowed control: a module-level
    // `require("express").Router().get(...)` (no enclosing function to
    // shadow `require` at all) must still record normally.
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nrequire(\"express\").Router().get(\"/x\", handler);\n",
        )]);
    let handler = fid(&cg, "handler").clone();
    assert!(
        cg.framework_entries.iter().any(|r| r.handler == handler),
        "non-shadowed module-level require-constructor receiver must still record"
    );
    assert_eq!(cg.framework_entry_unresolved_handlers, 0);
}

// ---- F3: method-aware arg positioning for `app.use(...)` ----------------

#[test]
fn express_use_single_identifier_arg_is_recorded() {
    // `app.use(loggerFn)` -- no path argument at all; arg 0 IS the
    // handler. The uniform "always skip arg 0" rule used to drop this
    // entirely.
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction loggerFn(req, res, next) {}\n\napp.use(loggerFn);\n",
        )]);
    let logger_fn = fid(&cg, "loggerFn").clone();
    let rec = cg
        .framework_entries
        .iter()
        .find(|r| r.handler == logger_fn)
        .expect("app.use(loggerFn) must record the single identifier arg");
    assert_eq!(rec.site.line, 6);
}

#[test]
fn express_use_with_path_arg_still_skips_the_path() {
    // `app.use('/api', mw)` -- arg 0 IS a string path, so the existing
    // skip-arg-0 behavior must still apply; only `mw` is recorded.
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction mw(req, res, next) {}\n\napp.use(\"/api\", mw);\n",
        )]);
    let mw = fid(&cg, "mw").clone();
    assert!(
        cg.framework_entries.iter().any(|r| r.handler == mw),
        "mw must be recorded"
    );
    assert_eq!(
        cg.framework_entries
            .iter()
            .filter(|r| r.framework == "express")
            .count(),
        1,
        "the path argument itself must not be recorded as a handler"
    );
}

#[test]
fn express_use_identifier_router_var_fails_bare_binding_resolution() {
    // `app.use(routerVar)` where `routerVar` is bound to
    // `express.Router()` -- arg 0 is method-aware-scanned as a handler
    // candidate, but `routerVar` is never a function/method definition
    // at all, so it fails resolution (0 matches) and must not record.
    let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\nconst routerVar = express.Router();\n\napp.use(routerVar);\n",
        )]);
    assert!(
        !cg.framework_entries
            .iter()
            .any(|r| r.framework == "express"),
        "a router/config identifier must not resolve to a handler"
    );
    assert_eq!(cg.framework_entry_unresolved_handlers, 1);
}

// ---- Non-target-language guard -----------------------------------------

#[test]
fn non_target_language_go_file_never_mints() {
    let mut map = BTreeMap::new();
    map.insert(
        "main.go".to_string(),
        ParsedFile::parse("main.go", "package main\n\nfunc main() {}\n", Go).unwrap(),
    );
    let cg = CallGraph::build(&map);
    assert!(cg.framework_entries.is_empty());
    assert_eq!(cg.framework_entry_unresolved_handlers, 0);
}
