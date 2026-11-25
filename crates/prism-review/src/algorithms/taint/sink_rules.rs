//! Operand evidence for rule selection after the existing sink admission pass.

use super::cleansing::{flow_path_cleansed_for_sink_call, node_in_ranges};
use super::js_ts::source_ranges::js_ts_request_source_access_ranges_for_alias_on_line;
use super::matches_sink;
use super::sink_match::{call_passes_sink_semantics, sink_call_path_matches};
use prism::access_path::AccessPath;
use prism::ast::tree_helpers::{call_arg_node, call_path_text};
use prism::ast::ParsedFile;
use prism::cpg::CodePropertyGraph;
use prism::data_flow::{FlowPath, VarAccessKind, VarLocation};
use prism::frameworks::SinkPattern;
use prism::languages::Language;
use prism::rule_registry::{RuleInfo, CONFIGURED_SINK, FORMAT_WRAPPER};
use std::collections::BTreeSet;
use std::iter::successors;
use tree_sitter::Node;

type Selection = (&'static RuleInfo, bool);

pub(super) struct NameSinks<'a> {
    configured: &'a [String],
    wrappers: &'a [String],
}

impl<'a> NameSinks<'a> {
    pub(super) fn new(configured: &'a [String], wrappers: &'a [String]) -> Self {
        Self {
            configured,
            wrappers,
        }
    }

    pub(super) fn admission_patterns(&self) -> impl Iterator<Item = &'a str> {
        self.configured
            .iter()
            .chain(self.wrappers)
            .map(String::as_str)
    }

    /// Select name rules for a reached argument whose callee passed the flat filters.
    pub(super) fn select(&self, callee: &str, rows: &mut Vec<Selection>) {
        if self
            .configured
            .iter()
            .any(|name| matches_sink(callee, name))
        {
            rows.push((&CONFIGURED_SINK, false));
        }
        if self
            .wrappers
            .iter()
            .any(|name| name.strip_prefix('=') == Some(callee))
        {
            rows.push((&FORMAT_WRAPPER, false));
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Occurrence<'a> {
    pub(super) call: Node<'a>,
    pub(super) callee: Node<'a>,
    pub(super) operand: Node<'a>,
    pub(super) slot: usize,
}

impl Occurrence<'_> {
    /// The ordinary indexed matcher keeps its original pattern and sanitizer category.
    /// Signature-specific role checks can select their own operands before using a guard.
    pub(super) fn matches_path_pattern(
        self,
        parsed: &ParsedFile,
        cpg: &CodePropertyGraph,
        line: usize,
        path: &FlowPath,
        pattern: &'static SinkPattern,
    ) -> bool {
        (parsed.language != Language::Go || self.call.start_position().row + 1 == line)
            && pattern.tainted_arg_indices.contains(&self.slot)
            && call_path_text(parsed, &self.call)
                .is_some_and(|actual| sink_call_path_matches(parsed, &self.call, &actual, pattern))
            && call_passes_sink_semantics(parsed, &self.call, pattern)
            && !flow_path_cleansed_for_sink_call(parsed, cpg, path, line, pattern, &self.call)
    }
}

#[derive(Clone, Copy)]
pub(super) struct InlineOccurrence<'a> {
    pub(super) occurrence: Occurrence<'a>,
    pub(super) source: Node<'a>,
}

pub(super) enum Witness<'a, 'p> {
    Path {
        path: &'p FlowPath,
        reached: Node<'a>,
    },
    Inline(Node<'a>),
}

/// Collect one finding's complete selection. Keep path visits distinct until projection
/// so an uncleansed witness can win for the same occurrence. Inline candidates come
/// from the language's source-line route and may span the selected physical line.
pub(super) fn collect_rules<'a, 'p>(
    parsed: &'a ParsedFile,
    line: usize,
    admitted: &BTreeSet<(String, usize)>,
    paths: impl IntoIterator<Item = &'p FlowPath>,
    inline: impl IntoIterator<Item = InlineOccurrence<'a>>,
    mut project: impl FnMut(Occurrence<'a>, Witness<'a, 'p>, &mut Vec<Selection>),
) -> Vec<Selection> {
    let mut rows = Vec::new();
    if !admitted.contains(&(parsed.path.clone(), line)) {
        return rows;
    }
    let boundaries = parsed.language.callable_boundary_node_types();
    for path in paths {
        for edge in &path.edges {
            for reached in located_uses(parsed, line, &edge.to) {
                for node in successors(reached.parent(), Node::parent) {
                    if boundaries.contains(&node.kind()) || node.kind().ends_with("_statement") {
                        break;
                    }
                    if parsed.language.is_call_node(node.kind()) {
                        if let Some(occurrence) = call_operand(parsed, node, reached) {
                            project(occurrence, Witness::Path { path, reached }, &mut rows);
                        }
                    }
                }
            }
        }
    }
    for InlineOccurrence { occurrence, source } in inline {
        let mut ancestors = successors(Some(source), Node::parent);
        if ancestors.any(|node| node == occurrence.operand)
            && ancestors.any(|node| node == occurrence.call)
            && ancestors.last() == Some(parsed.tree.root_node())
        {
            project(occurrence, Witness::Inline(source), &mut rows);
        }
    }
    rows
}

fn call_operand<'a>(
    parsed: &ParsedFile,
    call: Node<'a>,
    reached: Node<'a>,
) -> Option<Occurrence<'a>> {
    let callee = parsed.call_function_name(&call)?;
    for (slot, operand) in (0..).map_while(|slot| call_arg_node(&call, slot).map(|arg| (slot, arg)))
    {
        if operand.start_byte() <= reached.start_byte() && reached.end_byte() <= operand.end_byte()
        {
            return Some(Occurrence {
                call,
                callee,
                operand,
                slot,
            });
        }
    }
    None
}

fn located_uses<'a>(parsed: &'a ParsedFile, line: usize, target: &VarLocation) -> Vec<Node<'a>> {
    if target.file != parsed.path
        || target.line != line
        || target.kind != VarAccessKind::Use
        || target.start_byte >= target.end_byte
    {
        return Vec::new();
    }
    let Some(exact) = parsed
        .tree
        .root_node()
        .named_descendant_for_byte_range(target.start_byte, target.end_byte)
        .filter(|node| {
            node.byte_range() == (target.start_byte..target.end_byte)
                && node.start_position().row + 1 == line
                && AccessPath::from_expr(parsed.node_text(node)) == target.path
        })
    else {
        return Vec::new();
    };
    let ranges =
        js_ts_request_source_access_ranges_for_alias_on_line(parsed, line, target.var_name());
    if !ranges.is_empty() {
        // Access ranges are occurrence-specific. Only the exact reached access can qualify.
        return if node_in_ranges(&exact, &ranges) {
            vec![exact]
        } else {
            Vec::new()
        };
    }
    let mut uses = vec![exact];
    let Some(region) = coalescing_region(parsed, exact).filter(|_| target.path.fields.is_empty())
    else {
        return uses;
    };
    let same_name: Vec<_> = parsed
        .identifiers_on_line(line)
        .into_iter()
        .filter(|node| parsed.node_text(node) == target.var_name())
        .collect();
    uses.extend(same_name.iter().copied().filter(|&candidate| {
        let start = exact.start_byte().min(candidate.start_byte());
        let end = exact.start_byte().max(candidate.start_byte());
        candidate != exact
            && same_name
                .iter()
                .filter(|node| (start..=end).contains(&node.start_byte()))
                .all(|&node| coalescing_region(parsed, node) == Some(region))
    }));
    uses
}

/// Reuse a coalesced spelling only for bare argument reads in the same argument
/// list or sibling expression statements. Any other mention between the reads
/// (including a write, declaration, or read in a nested scope) blocks reuse.
fn coalescing_region<'a>(parsed: &ParsedFile, node: Node<'a>) -> Option<Node<'a>> {
    let arguments = node.parent()?;
    let call = arguments.parent()?;
    if node.kind() != "identifier"
        || call.child_by_field_name("arguments") != Some(arguments)
        || !parsed.language.is_call_node(call.kind())
    {
        return None;
    }
    let statement = call.parent()?;
    if statement.kind() == "expression_statement" {
        return statement.parent();
    }
    Some(arguments)
}

#[cfg(test)]
mod tests;
