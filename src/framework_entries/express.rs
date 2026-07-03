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
    /// M2+M3 (review-fix wave 2): whether this call's grounding is
    /// invalidated by a local shadow of either the bare receiver identifier
    /// (`app` in `app.get(...)`) or, for a direct-constructor receiver, its
    /// M2 constructor-grounding identifier (`express` in `express()` /
    /// `express.Router()` / `new express.Router()`; `require` in
    /// `require("express")()`). Computed HERE — not deferred to
    /// `super::apply` via a name + enclosing-chain pair, as F2 originally
    /// did — because the M3 fix requires walking the call node's ACTUAL AST
    /// ancestor chain (to see anonymous enclosing scopes, which have no
    /// `FunctionId` and so can't be represented in `enclosing`/an
    /// `EnclosingFacts` chain at all), and this module is the one that still
    /// has the real `tree_sitter::Node` in hand.
    pub shadowed: bool,
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
    // F6 (opus nit): hoisted out of the per-arg loop below — it's a `Vec`
    // allocated fresh per call, and doesn't depend on anything but
    // `parsed.language`, which is fixed for the whole file.
    let function_kinds = parsed.language.function_node_types();

    for call in calls {
        let Some(is_route_builder) = express_call_match_kind(parsed, &call, &receivers, &imports)
        else {
            continue;
        };
        let Some(args_node) = parsed.language.call_arguments(&call) else {
            continue;
        };
        let mut cursor = args_node.walk();
        let arg_nodes: Vec<tree_sitter::Node<'_>> = args_node.named_children(&mut cursor).collect();
        // F3: method-aware arg positioning. Direct-instance `.use(...)`
        // uniquely allows OMITTING the mount path (`app.use(middlewareFn)`
        // mounts globally) -- the uniform "always skip arg 0" rule used to
        // drop that single-arg form entirely. Only treat arg 0 as a handler
        // candidate for `use` when it ISN'T a string/template path (a real
        // mount-path arg 0 keeps the pre-existing skip-arg-0 behavior); every
        // other route method (`get`/`post`/`all`/...) keeps the path-at-0
        // rule unchanged, and the `.route(path).get(...)` builder form
        // (`is_route_builder`) already scans from 0 regardless of method.
        let start_idx: usize = if is_route_builder {
            0
        } else if express_call_method_name(parsed, &call).as_deref() == Some("use") {
            match arg_nodes.first() {
                Some(first) if is_js_ts_string_like_node(first.kind()) => 1,
                Some(_) => 0,
                None => 1,
            }
        } else {
            1
        };
        if arg_nodes.len() <= start_idx {
            continue;
        }

        let mut args = Vec::new();
        for arg in &arg_nodes[start_idx..] {
            if parsed.language.is_identifier_node(arg.kind()) {
                args.push(ExpressHandlerArg::Identifier(
                    parsed.node_text(arg).to_string(),
                ));
            } else if function_kinds.contains(&arg.kind()) {
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
        let shadowed = express_receiver_is_shadowed(parsed, &call);

        out.push(ExpressRouteCandidate {
            enclosing,
            shadowed,
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

/// F3: recover the matched call's own method name (`get`/`use`/`post`/...)
/// for the method-aware arg-positioning decision — recomputed independently
/// from `express_call_match_kind`'s internal `method` local (which isn't
/// returned) rather than threading it through that function's signature;
/// this is the same minimal-duplication choice as
/// `express_receiver_identifier_name` (F2).
fn express_call_method_name(parsed: &ParsedFile, call: &tree_sitter::Node<'_>) -> Option<String> {
    let function = call.child_by_field_name("function")?;
    let function = crate::algorithms::taint::unwrap_parenthesized(function);
    if function.kind() != "member_expression" {
        return None;
    }
    let property = function.child_by_field_name("property")?;
    Some(
        parsed
            .node_text(&property)
            .trim_matches(|c| c == '\'' || c == '"' || c == '`')
            .to_string(),
    )
}

/// F3: a JS/TS string or template-literal node — used to decide whether
/// `app.use(arg0, ...)`'s arg 0 is a mount path (keep the skip-arg-0 rule)
/// vs. a handler candidate (function-valued or identifier).
fn is_js_ts_string_like_node(kind: &str) -> bool {
    matches!(kind, "string" | "template_string")
}

/// M2+M3 (review-fix wave 2): whether this matched Express call's grounding
/// is invalidated by a local shadow — the single entry point both the
/// receiver-identifier guard (F2) and the constructor-grounding-identifier
/// guard (M2) now share. `express_shadow_check_identifier` returning `None`
/// (nothing to check — shouldn't happen for an already-matched call) is
/// treated as "not shadowed" defensively.
fn express_receiver_is_shadowed(parsed: &ParsedFile, call: &tree_sitter::Node<'_>) -> bool {
    express_shadow_check_identifier(parsed, call)
        .is_some_and(|name| is_identifier_shadowed_in_enclosing_scopes(parsed, call, &name))
}

/// F2+M2: recover the ONE identifier whose local shadowing would invalidate
/// this matched Express call's receiver grounding — either the bare receiver
/// identifier (`app` in `app.get(...)`) or, when the receiver is itself a
/// direct grounded constructor expression (M2: `express()`,
/// `express.Router()`, `new express.Router()`, `require("express")()`), its
/// constructor-grounding identifier. Duplicates the minimal structural peel
/// `js_ts_receiver_expr_is_framework_instance`/
/// `js_ts_receiver_expr_is_route_builder` already do (those return `bool`
/// only, never the receiver node) rather than changing those functions'
/// signatures.
fn express_shadow_check_identifier(
    parsed: &ParsedFile,
    call: &tree_sitter::Node<'_>,
) -> Option<String> {
    let function = call.child_by_field_name("function")?;
    let function = crate::algorithms::taint::unwrap_parenthesized(function);
    if function.kind() != "member_expression" {
        return None;
    }
    let receiver = function.child_by_field_name("object")?;
    express_receiver_shadow_identifier(parsed, receiver)
}

/// F2+M2: peel a receiver expression down to its shadow-check identifier.
/// Recurses through the `.route(path).get(...)` builder form to whichever of
/// the two shapes backs `.route()`'s own object, so a builder receiver whose
/// object is ITSELF a direct constructor (`express().route('/x').get(...)`)
/// is handled uniformly with the direct-instance case.
fn express_receiver_shadow_identifier(
    parsed: &ParsedFile,
    receiver: tree_sitter::Node<'_>,
) -> Option<String> {
    let receiver = crate::algorithms::taint::unwrap_parenthesized(receiver);
    if receiver.kind() == "identifier" {
        return Some(parsed.node_text(&receiver).to_string());
    }
    if matches!(receiver.kind(), "call_expression" | "new_expression") {
        if let Some(name) = express_constructor_grounding_identifier(parsed, receiver) {
            return Some(name);
        }
    }
    // `.route(path).get(...)` builder form: peel one level to the object the
    // `.route()` call itself was invoked on, and recurse.
    if parsed.language.is_call_node(receiver.kind()) {
        let inner_function = receiver.child_by_field_name("function")?;
        let inner_function = crate::algorithms::taint::unwrap_parenthesized(inner_function);
        if inner_function.kind() == "member_expression" {
            let property = inner_function.child_by_field_name("property")?;
            if parsed.node_text(&property) == "route" {
                let object = inner_function.child_by_field_name("object")?;
                return express_receiver_shadow_identifier(parsed, object);
            }
        }
    }
    None
}

/// M2: peel a direct grounded constructor receiver expression (`express()`,
/// `express.Router()`, `new express.Router()`, `require("express")()`) down
/// to its SHADOWABLE grounding identifier — mirrors the same peel
/// `js_ts_callee_constructs_framework_receiver`/
/// `js_ts_expr_resolves_to_framework_module` (taint.rs) use to decide the
/// receiver constructs a framework instance, but returns the identifier
/// itself (rather than a bool) so the caller can shadow-check it. Those
/// taint.rs functions ground the identifier PURELY against the whole-file
/// import map, with no local-shadow awareness at all — this is exactly the
/// M2 bug (`function setup(express) { express().get(...) }` still minted a
/// false edge because the direct-constructor exception in
/// `express_receiver_shadow_identifier` short-circuited BEFORE any shadow
/// check ran).
fn express_constructor_grounding_identifier(
    parsed: &ParsedFile,
    expr: tree_sitter::Node<'_>,
) -> Option<String> {
    let expr = crate::algorithms::taint::unwrap_parenthesized(expr);
    if !matches!(expr.kind(), "call_expression" | "new_expression") {
        return None;
    }
    let callee = expr
        .child_by_field_name("function")
        .or_else(|| expr.child_by_field_name("constructor"))
        .or_else(|| expr.child_by_field_name("name"))
        .or_else(|| expr.named_child(0))?;
    let callee = crate::algorithms::taint::unwrap_parenthesized(callee);
    match callee.kind() {
        // `express()`, or the callee of `new express.Router()`/
        // `express.Router()` once already peeled to `express` below.
        "identifier" => Some(parsed.node_text(&callee).to_string()),
        // `express.Router()` / `new express.Router()` — the SHADOWABLE
        // grounding identifier is the member expression's OBJECT
        // (`express`), not the property (`Router`).
        "member_expression" => {
            let object = callee.child_by_field_name("object")?;
            let object = crate::algorithms::taint::unwrap_parenthesized(object);
            (object.kind() == "identifier").then(|| parsed.node_text(&object).to_string())
        }
        // `require("express")()` — the grounding identifier is `require`
        // itself, the inner call's own callee.
        kind if parsed.language.is_call_node(kind) => {
            let inner_function = callee.child_by_field_name("function")?;
            let inner_function = crate::algorithms::taint::unwrap_parenthesized(inner_function);
            (inner_function.kind() == "identifier")
                .then(|| parsed.node_text(&inner_function).to_string())
        }
        _ => None,
    }
}

/// M3: whether `name` is bound as a parameter or local variable in any
/// function-like scope enclosing `call` — walks the call node's ACTUAL AST
/// ancestor chain (repeated `.parent()`) and collects bindings DIRECTLY from
/// every enclosing function-like node, named or anonymous, via
/// `ParsedFile::js_ts_function_local_bindings` (which operates on any
/// function-like node regardless of whether it can be named). This
/// deliberately does NOT go through the FunctionId-keyed
/// `CallGraph::js_ts_function_locals` index (`call_graph.rs`), which only
/// ever has entries for NAMED functions — an anonymous arrow/IIFE scope has
/// no `FunctionId` and so is invisible to that index, which is exactly why
/// `(app) => { app.get(...) }`'s parameter shadow was missed before this
/// fix. Serves BOTH the receiver-identifier (F2) and (M2) the
/// constructor-grounding-identifier shadow checks via
/// `express_receiver_is_shadowed`.
fn is_identifier_shadowed_in_enclosing_scopes(
    parsed: &ParsedFile,
    call: &tree_sitter::Node<'_>,
    name: &str,
) -> bool {
    let function_kinds = parsed.language.function_node_types();
    let mut current = call.parent();
    while let Some(node) = current {
        if function_kinds.contains(&node.kind())
            && parsed.js_ts_function_local_bindings(&node).contains(name)
        {
            return true;
        }
        current = node.parent();
    }
    false
}

/// F1: discriminate a "bare binding" function-like definition (a hoisted
/// `function` declaration, or a variable/assignment-bound function
/// expression/arrow) from a class or object-literal METHOD. Needed because
/// `CallGraph::functions` (the whole-program name index consulted for
/// identifier-arg resolution in `super::apply`) is built from
/// `ParsedFile::all_functions()`, which folds in every JS/TS function-like
/// node regardless of context: per the tree-sitter-javascript grammar, a
/// class method (`class C { handler() {} }`) and an object-literal shorthand
/// method (`{ handler() {} }`) both use the SAME `method_definition` node
/// kind (distinguished only by parent: `class_body` vs `object`), so
/// excluding that one kind excludes both shapes with a single check. A bare
/// identifier reference (`app.get(path, handler)`) can only ever mean a free
/// variable/function binding reachable by that exact name — never a method,
/// which is reachable only via a receiver (`this.handler`/`obj.handler`).
///
/// Also excludes a class-field arrow/function-expression binding (`class C {
/// handler = () => {} }`, Pattern 4 in `languages::mod::function_name`) —
/// same false-edge shape as a `method_definition`, just spelled as a field
/// instead of shorthand-method syntax, so it's just as unreachable by a bare
/// `handler` reference.
///
/// M1 (review-fix wave 2): the same problem also applies to two OTHER
/// name-inference patterns in `languages::mod::function_name` that this
/// function previously let through by defaulting to `true` for anything
/// that wasn't a `method_definition`/class-field: Pattern 2 (`{ handler: ()
/// => {} }`, an object-literal property arrow — the function's parent is a
/// `pair`) and Pattern 5's member-expression-LHS case (`exports.handler = ()
/// => {}` — the function's parent is an `assignment_expression` whose LHS is
/// a `member_expression`, and the inferred name is the property, not a
/// binding). Neither is reachable by a bare `handler` identifier (only via a
/// receiver: `api.handler`/`exports.handler`), so both must be excluded too.
///
/// Rewritten as a definition-site AST-shape ALLOW-list rather than a
/// deny-list, since the old deny-list's "anything else defaults to true" is
/// exactly what let Patterns 2 and 5's member form leak through. The only
/// shapes a bare identifier can actually reach: a hoisted `function`
/// declaration, a `variable_declarator`-bound function/arrow
/// (`const handler = () => {}`, Pattern 1), and a plain identifier
/// assignment (`handler = () => {}`, Pattern 5's identifier-LHS case).
pub(crate) fn is_bare_binding_function(node: &tree_sitter::Node<'_>) -> bool {
    match node.kind() {
        "method_definition" => false,
        "function_declaration" | "generator_function_declaration" => true,
        "arrow_function" | "function_expression" => match node.parent() {
            Some(parent) => match parent.kind() {
                "variable_declarator" => true,
                "assignment_expression" => parent
                    .child_by_field_name("left")
                    .is_some_and(|left| left.kind() == "identifier"),
                // Excludes: `pair` (object-literal property, Pattern 2),
                // `field_definition`/`public_field_definition` (class field,
                // Pattern 4), `arguments` (Pattern 3's
                // `React.memo(() => {})` wrapper — the function's OWN
                // immediate parent isn't a `variable_declarator`, only its
                // grandparent's call is), and anything else.
                _ => false,
            },
            None => false,
        },
        _ => true,
    }
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
