//! Operand evidence for rule selection after the existing sink admission pass.
//!
//! The path walk currently projects call arguments, including Bash `command` arguments
//! with a static command name. The language slices still need to wire that Bash route
//! and extend the walk to assignment RHS and JSX `__html` values in S2-JS.

use super::cleansing::{flow_path_cleansed_for_sink_call, node_contains_line, node_in_ranges};
use super::matches_sink;
use super::sink_match::{call_passes_sink_semantics, sink_call_path_matches};
use prism::ast::tree_helpers::call_path_text;
use prism::ast::ParsedFile;
use prism::cpg::CodePropertyGraph;
use prism::data_flow::FlowPath;
use prism::frameworks::SinkPattern;
use prism::languages::Language;
use prism::rule_registry::{RuleInfo, CONFIGURED_SINK, FORMAT_WRAPPER};
use std::collections::BTreeSet;
use std::iter::successors;
use tree_sitter::Node;

mod layout;
mod uses;

use layout::call_operand;
use uses::located_uses;

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

    #[cfg(test)]
    pub(super) fn old_first_match(&self, text: &str) -> Option<(&'static str, &'a str)> {
        self.configured
            .iter()
            .map(|entry| ("configured", entry.as_str()))
            .chain(
                self.wrappers
                    .iter()
                    .map(|entry| ("wrapper", entry.as_str())),
            )
            .find(|(_, entry)| matches_sink(text, entry))
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Occurrence<'a> {
    pub(super) call: Node<'a>,
    pub(super) callee: Node<'a>,
    pub(super) operand: Node<'a>,
    pub(super) slot: usize,
}

impl Occurrence<'_> {
    /// Flat cleansing examines the verified callee, including callee-only safe ranges.
    pub(super) fn passes_flat_safe_ranges(self, ranges: &[(usize, usize)]) -> bool {
        !node_in_ranges(&self.callee, ranges)
    }

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
    pub(super) admission_key: (&'a str, usize),
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
    let key = (parsed.path.as_str(), line);
    if !admitted.contains(&(parsed.path.clone(), line)) {
        return rows;
    }
    let boundaries = parsed.language.callable_boundary_node_types();
    for path in paths {
        for edge in &path.edges {
            for reached in located_uses(parsed, line, edge) {
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
    for InlineOccurrence {
        admission_key,
        occurrence,
        source,
    } in inline
    {
        if admission_key != key || !node_contains_line(&occurrence.call, line) {
            continue;
        }
        let mut ancestors = successors(Some(source), Node::parent);
        if ancestors
            .by_ref()
            .take_while(|node| !boundaries.contains(&node.kind()))
            .any(|node| node == occurrence.call)
            && ancestors.last() == Some(parsed.tree.root_node())
            && call_operand(parsed, occurrence.call, source) == Some(occurrence)
        {
            project(occurrence, Witness::Inline(source), &mut rows);
        }
    }
    rows
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(super) mod old_visits;
