//! Shared owner-name helper for anonymous JS/TS/TSX callables (lane A2, S1).
//!
//! One module owns the name format and the predicate that recognises it, so
//! every guard (position lookups, symbol seeds, navigation visibility, MCP
//! suggestions, Mermaid labels) agrees on what an anonymous identity is.
//!
//! The js-param-defs PR-B DFG pass mints these names as `<anon@L:C>`.
//! Minting and recognition share [`ANON_OWNER_PREFIX`].
//!
//! `<` cannot start an ECMAScript identifier, and every name `function_name`
//! can return for JS/TS is an identifier, a property name, a string literal
//! (with its quotes) or a computed key (with its bracket), so no real owner can
//! spell this prefix.

/// Prefix of an anonymous callable's owner name (lane A2 key format).
pub const ANON_OWNER_PREFIX: &str = "<anon@";

/// Prefix [`anonymous_owner_name`] emits.
const MINT_OWNER_PREFIX: &str = ANON_OWNER_PREFIX;

// S2a-prep: the only emission switch. S2a-enable changes this after its gates.
pub(crate) const EMIT_ANONYMOUS_OWNERS: bool = false;

use crate::ast::ParsedFile;
use crate::call_graph::FunctionId;
use crate::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

/// Identity independent of emission and DFG parameter admissibility.
pub(crate) fn identity(parsed: &ParsedFile, node: &Node<'_>, file: &str) -> Option<FunctionId> {
    let name = if let Some(name) = parsed.language.function_name(node) {
        parsed.node_text(&name).to_string()
    } else if matches!(
        parsed.language,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    ) && matches!(
        node.kind(),
        "arrow_function" | "function_expression" | "generator_function"
    ) {
        let p = node.start_position();
        anonymous_owner_name(p.row + 1, p.column + 1)
    } else {
        return None;
    };
    let (start_line, end_line) = parsed.node_line_range(node);
    Some(FunctionId {
        file: file.into(),
        name,
        start_line,
        end_line,
    })
}

pub(crate) fn emitted_identity(
    parsed: &ParsedFile,
    node: &Node<'_>,
    file: &str,
) -> Option<FunctionId> {
    identity(parsed, node, file)
        .filter(|id| EMIT_ANONYMOUS_OWNERS || !is_anonymous_owner_name(&id.name))
}

/// Existing eager inventory plus unnamed generators when emission is enabled.
/// Keep the extra AST walk out of the disabled production path.
pub(crate) fn owner_nodes(parsed: &ParsedFile) -> Vec<Node<'_>> {
    candidate_nodes(parsed, EMIT_ANONYMOUS_OWNERS)
}

pub(crate) fn candidate_nodes(parsed: &ParsedFile, include_generators: bool) -> Vec<Node<'_>> {
    let mut nodes = parsed.all_functions();
    if include_generators
        && matches!(
            parsed.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        )
    {
        let mut cursor = parsed.tree.walk();
        loop {
            let node = cursor.node();
            if node.kind() == "generator_function"
                && parsed.language.function_name(&node).is_none()
                && !nodes.iter().any(|n| n.id() == node.id())
            {
                nodes.push(node);
            }
            if cursor.goto_first_child() {
                continue;
            }
            while !cursor.goto_next_sibling() {
                if !cursor.goto_parent() {
                    return nodes;
                }
            }
        }
    }
    nodes
}

/// Actual named inventories, including LegacyInventory and ambiguity refusal.
/// Exact spans are the compatibility contract; syntax arrangements are not.
pub(crate) fn named_claims(parsed: &ParsedFile, enabled: bool) -> BTreeSet<(usize, usize)> {
    if !enabled {
        return BTreeSet::new();
    }
    let functions = parsed.all_functions();
    let mut counts = std::collections::BTreeMap::new();
    for node in &functions {
        if let Some(name) = parsed.language.function_name(node) {
            *counts
                .entry((
                    parsed.node_text(&name).to_string(),
                    parsed.node_line_range(node),
                ))
                .or_insert(0) += 1;
        }
    }
    let mut claims = BTreeSet::new();
    for node in functions {
        let Some(name) = parsed.language.function_name(&node) else {
            continue;
        };
        let range = parsed.node_line_range(&node);
        if counts[&(parsed.node_text(&name).to_string(), range)] != 1 {
            continue;
        }
        let (sites, _) = parsed.function_calls_with_qualifier_and_spans_on_lines(
            &node,
            &(range.0..=range.1).collect(),
            &BTreeSet::new(),
        );
        claims.extend(sites.into_iter().map(|s| (s.start_byte, s.end_byte)));
    }
    claims
}

/// Every builder applies this same fallback-A and strict-ownership predicate.
pub(crate) fn may_mint_call(
    parsed: &ParsedFile,
    owner: &Node<'_>,
    start: usize,
    end: usize,
    claims: &BTreeSet<(usize, usize)>,
) -> bool {
    if parsed.language.function_name(owner).is_some() {
        return true;
    }
    !claims.contains(&(start, end)) && parsed.js_ts_span_in_own_scope(owner, start, end)
}

pub(crate) fn own_locals(parsed: &ParsedFile, node: &Node<'_>) -> BTreeSet<String> {
    parsed.js_ts_function_local_bindings(node)
}

/// Flatten the callable chain once at fact construction, never per call.
/// Named owners keep the existing own-locals contract, including class fallback.
pub(crate) fn shadow_locals(parsed: &ParsedFile, node: &Node<'_>) -> BTreeSet<String> {
    let mut locals = own_locals(parsed, node);
    if parsed.language.function_name(node).is_some() {
        return locals;
    }
    let mut parent = node.parent();
    while let Some(scope) = parent {
        if parsed
            .language
            .callable_boundary_node_types()
            .contains(&scope.kind())
        {
            locals.extend(own_locals(parsed, &scope));
        }
        parent = scope.parent();
    }
    locals
}

pub(crate) fn shadows(
    graph: &crate::call_graph::CallGraph,
    caller: &FunctionId,
    name: &str,
) -> bool {
    graph
        .js_ts_function_locals
        .get(caller)
        .is_some_and(|locals| locals.contains(name))
}

/// After proven local/alias routes, anonymous bare names need import proof.
/// Refused, uncertain and unchecked facts cannot authorize same-name Exacts.
/// Named owners retain their existing resolution contract.
pub(crate) fn permits_unqualified_fallback(site: &crate::call_graph::CallSite) -> bool {
    !is_anonymous_owner_name(&site.caller.name)
        || matches!(&site.local_binding, crate::call_graph::JsLocalBinding::Unproven(r) if r == "import")
}

/// Positional anonymous identity; retain historical first-match semantics for named IDs.
pub(crate) fn node_for_id<'a>(parsed: &'a ParsedFile, id: &FunctionId) -> Option<Node<'a>> {
    candidate_nodes(parsed, is_anonymous_owner_name(&id.name))
        .into_iter()
        .find(|node| identity(parsed, node, &id.file).as_ref() == Some(id))
}

pub(crate) fn source_bytes(parsed: &ParsedFile, id: &FunctionId) -> Option<(usize, usize)> {
    if is_anonymous_owner_name(&id.name) {
        node_for_id(parsed, id).map(|n| (n.start_byte(), n.end_byte()))
    } else {
        parsed
            .functions()
            .iter()
            .find(|f| f.name.as_deref() == Some(id.name.as_str()) && f.start_line == id.start_line)
            .map(|f| (f.start_byte, f.end_byte))
    }
}

pub(crate) fn owner_key(file: &str, name: &str, line: usize) -> (String, String, usize) {
    (file.into(), name.into(), line)
}

/// Exact owner-source prefix. Assignment recovery never inspects later writes.
pub(crate) fn source_before<'a>(
    parsed: &'a ParsedFile,
    id: &FunctionId,
    before: usize,
) -> Option<&'a str> {
    let function = node_for_id(parsed, id)?;
    if before < function.start_byte() || before > function.end_byte() {
        return None;
    }
    parsed.source.get(function.start_byte()..before)
}

/// Legacy indirect synthesis has line scans and Unchecked binding facts. New
/// owners decline it until a byte/scope-proven replacement is implemented.
pub(crate) fn permits_legacy_indirect(id: &FunctionId) -> bool {
    !is_anonymous_owner_name(&id.name)
}

/// D12's existing new-source population, shared across named/anonymous owners.
pub(crate) fn d12_source_spans(parsed: &ParsedFile, function: &Node<'_>) -> Vec<(usize, usize)> {
    parsed
        .function_parameter_occurrences(function)
        .into_iter()
        .filter_map(|(_, start, end)| {
            let is_new = parsed
                .js_ts_bare_arrow_parameter(function)
                .is_some_and(|p| (p.start_byte(), p.end_byte()) == (start, end))
                || parsed.find_parameters_node(function).is_some_and(|params| {
                    let mut cursor = params.walk();
                    let found = params.named_children(&mut cursor).any(|p| {
                        let pattern = p.child_by_field_name("pattern").unwrap_or(p);
                        parsed
                            .js_ts_rest_identifier(pattern)
                            .is_some_and(|n| (n.start_byte(), n.end_byte()) == (start, end))
                    });
                    found
                });
            is_new.then_some((start, end))
        })
        .collect()
}

#[cfg(test)]
#[path = "anon_owner_tests.rs"]
mod node_tests;

/// Whether an owner name denotes an anonymous callable identity.
pub fn is_anonymous_owner_name(name: &str) -> bool {
    name.starts_with(ANON_OWNER_PREFIX)
}

/// Owner name of the anonymous callable whose first byte is at 1-based `line`
/// and 1-based byte column `col`.
pub fn anonymous_owner_name(line: usize, col: usize) -> String {
    format!("{MINT_OWNER_PREFIX}{line}:{col}>")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_only_anon_spelling() {
        assert!(!is_anonymous_owner_name("<cb@1:10>"));
        assert!(is_anonymous_owner_name("<anon@12:3>"));
    }

    #[test]
    fn rejects_real_owner_names() {
        for name in [
            "",
            "f",
            "anon@1:1",
            "anon",
            "\"<anon@1:3>\"",
            "[<anon@1:1>]",
            "<module>",
            "<anonymous>",
            "<c",
        ] {
            assert!(!is_anonymous_owner_name(name), "{name:?}");
        }
    }

    #[test]
    fn mint_round_trips_through_the_predicate() {
        let minted = anonymous_owner_name(4, 19);
        assert!(minted.ends_with("4:19>"), "{minted}");
        // The minted spelling is the lane A2 owner key.
        assert!(minted.starts_with("<anon@"), "{minted}");
        assert!(is_anonymous_owner_name(&minted));
    }
}
