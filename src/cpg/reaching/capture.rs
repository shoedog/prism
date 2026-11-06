use super::capture_regions::{self, JsRegions, RegionKind};
use super::census_trace;
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::data_flow::FlowEdge;
use std::collections::BTreeSet;
use tree_sitter::Node;

pub(super) struct CaptureFacts {
    /// Main's rule, every language: bodies of the outermost nested callables.
    ranges: Vec<(usize, usize)>,
    references: BTreeSet<(AccessPath, usize)>,
    /// #660 S1a, JS/TS/TSX owners only: parameter lists, instance field initializers and the
    /// innermost-region belt. Strictly additive to main's rule (`is_capture` ors the two).
    js: Option<JsRegions>,
    /// `PRISM_RD_CENSUS_TRACE` was on when the facts were built.
    trace: bool,
}

pub(super) fn capture_facts(parsed: &ParsedFile, func_node: Node<'_>) -> CaptureFacts {
    capture_facts_with(parsed, func_node, census_trace::enabled())
}

/// `capture_facts` with the trace switch injected (tests do not race on the process environment).
pub(super) fn capture_facts_with(
    parsed: &ParsedFile,
    func_node: Node<'_>,
    trace: bool,
) -> CaptureFacts {
    fn visit(
        parsed: &ParsedFile,
        node: Node<'_>,
        boundary_kinds: &[&str],
        facts: &mut CaptureFacts,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if is_nested_callable_kind(boundary_kinds, child.kind()) {
                let body = child.child_by_field_name("body").unwrap_or(child);
                facts.ranges.push((body.start_byte(), body.end_byte()));
                collect_reference_identities(parsed, body, &mut facts.references);
            } else {
                visit(parsed, child, boundary_kinds, facts);
            }
        }
    }

    let mut facts = CaptureFacts {
        ranges: Vec::new(),
        references: BTreeSet::new(),
        js: None,
        trace,
    };
    let boundary_kinds = parsed.language.callable_boundary_node_types();
    visit(parsed, func_node, &boundary_kinds, &mut facts);
    if parsed.language.is_js_family() {
        facts.js = Some(capture_regions::js_regions(parsed, func_node));
    }
    facts
}

/// Main's rule: a use read in a nested body whose def lies in no nested body.
fn main_capture(edge: &FlowEdge, facts: &CaptureFacts) -> bool {
    facts
        .references
        .contains(&(edge.to.path.clone(), edge.to.line))
        && !facts
            .ranges
            .iter()
            .any(|(start, end)| *start <= edge.from.start_byte && edge.from.start_byte < *end)
}

/// The innermost-region belt (JS family): the use lies in a region and the def does not lie in the
/// use's innermost region (directly, or inside a region nested in it). Returns the use's region
/// (`Some(None)`: it cannot be placed, which is a capture). The reference identity is line-keyed
/// and an owner's use endpoint is usually a zero-width line anchor, so the line names the region;
/// occurrences of one (path, line) in different regions cannot be placed. A use with real bytes
/// outside every region is eager and keeps main's rule.
fn region_belt(edge: &FlowEdge, js: &JsRegions) -> Option<Option<usize>> {
    let by_line = js.line_region(&edge.to.path, edge.to.line)?;
    let use_region = if edge.to.end_byte > edge.to.start_byte {
        Some(js.innermost(edge.to.start_byte)?)
    } else {
        by_line
    };
    match use_region {
        Some(region) if js.within(edge.from.start_byte, region) => None,
        other => Some(other),
    }
}

pub(super) fn is_capture(edge: &FlowEdge, facts: &CaptureFacts) -> bool {
    main_capture(edge, facts)
        || facts
            .js
            .as_ref()
            .is_some_and(|js| region_belt(edge, js).is_some())
}

/// The mechanism of an edge S1a's rules (and not main's) turn into a capture.
pub(super) fn new_mechanism(edge: &FlowEdge, facts: &CaptureFacts) -> Option<&'static str> {
    if main_capture(edge, facts) {
        return None;
    }
    let js = facts.js.as_ref()?;
    let use_region = region_belt(edge, js)?;
    Some(match use_region.map(|region| js.kind(region)) {
        Some(RegionKind::Param) => census_trace::CAPTURE_PARAM,
        Some(RegionKind::Field) => census_trace::CAPTURE_FIELD,
        Some(RegionKind::Body) | None => census_trace::CAPTURE_REGION,
    })
}

/// `is_capture` for the production classifier: also records the trace line when it is on.
pub(super) fn is_capture_traced(edge: &FlowEdge, facts: &CaptureFacts) -> bool {
    if !facts.trace {
        return is_capture(edge, facts);
    }
    is_capture_to(edge, facts, &mut std::io::stderr().lock())
}

/// `is_capture` that writes the trace line for an edge S1a's rules decide to `out`.
pub(super) fn is_capture_to(
    edge: &FlowEdge,
    facts: &CaptureFacts,
    out: &mut impl std::io::Write,
) -> bool {
    let capture = is_capture(edge, facts);
    if capture && facts.trace {
        if let Some(mechanism) = new_mechanism(edge, facts) {
            census_trace::emit(out, mechanism, edge);
        }
    }
    capture
}

fn collect_reference_identities(
    parsed: &ParsedFile,
    node: Node<'_>,
    references: &mut BTreeSet<(AccessPath, usize)>,
) {
    if is_field_access_kind(node.kind()) {
        references.insert((
            AccessPath::from_expr(parsed.node_text(&node)),
            node.start_position().row + 1,
        ));
        if let Some(base) = leftmost_receiver_identifier(parsed, node) {
            references.insert((
                AccessPath::simple(parsed.node_text(&base)),
                base.start_position().row + 1,
            ));
        }
        return;
    }

    if parsed.language.is_identifier_node(node.kind()) {
        references.insert((
            AccessPath::simple(parsed.node_text(&node)),
            node.start_position().row + 1,
        ));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_reference_identities(parsed, child, references);
    }
}

pub(super) fn leftmost_receiver_identifier<'a>(
    parsed: &ParsedFile,
    mut node: Node<'a>,
) -> Option<Node<'a>> {
    loop {
        let receiver = node
            .child_by_field_name("argument")
            .or_else(|| node.child_by_field_name("object"))
            .or_else(|| node.child_by_field_name("operand"))
            .or_else(|| node.named_child(0))?;
        if is_field_access_kind(receiver.kind()) {
            node = receiver;
            continue;
        }
        return parsed
            .language
            .is_identifier_node(receiver.kind())
            .then_some(receiver);
    }
}

pub(super) fn is_field_access_kind(kind: &str) -> bool {
    matches!(
        kind,
        "field_expression"
            | "member_expression"
            | "selector_expression"
            | "attribute"
            | "field_access"
            | "dot_index_expression"
            | "method_index_expression"
    )
}

fn is_nested_callable_kind(boundary_kinds: &[&str], kind: &str) -> bool {
    boundary_kinds.contains(&kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::Language;

    #[test]
    fn capture_boundaries_match_every_language_callable_boundary() {
        for language in Language::all() {
            let boundary_kinds = language.callable_boundary_node_types();
            for kind in &boundary_kinds {
                assert!(
                    is_nested_callable_kind(&boundary_kinds, kind),
                    "capture classifier drifted from {language:?} boundary {kind}"
                );
            }
        }
    }
}

#[cfg(test)]
#[path = "capture_placement_tests.rs"]
mod placement_tests;
