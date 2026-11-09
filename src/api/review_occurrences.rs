//! Source facts for consumers that need exact call and value occurrences.

use crate::ast::{ParsedFile, PathSpan};
use crate::call_graph::FunctionId;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use tree_sitter::Node;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReviewOccurrenceOwner {
    /// Unindexed callable boundaries remain present with no claimed identity.
    pub function: Option<FunctionId>,
    pub span: Range<usize>,
    /// Unique by name and start line, the identity available on value locations.
    pub unique: bool,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ReviewCallOccurrence {
    pub span: Range<usize>,
    pub line: usize,
    pub callee: String,
    pub arguments: Vec<Range<usize>>,
    pub owner: Option<ReviewOccurrenceOwner>,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ReviewValueOccurrence {
    pub occurrence: PathSpan,
    pub owner: Option<ReviewOccurrenceOwner>,
    pub semantic_value: bool,
}

/// A source-bound inventory, reusable for every call and trace in this file.
/// Values are existing rvalue facts, NOT a complete syntax census for recovery
/// of missing bytes. No filesystem reads, reparsing or global cache is used.
#[non_exhaustive]
pub struct ReviewOccurrenceFacts<'a> {
    parsed: &'a ParsedFile,
    owners: Vec<ReviewOccurrenceOwner>,
    calls: BTreeMap<(usize, usize), ReviewCallOccurrence>,
    values: BTreeMap<(usize, usize), Vec<ReviewValueOccurrence>>,
}

impl<'a> ReviewOccurrenceFacts<'a> {
    pub fn new(parsed: &'a ParsedFile) -> Self {
        let mut facts = Self {
            parsed,
            owners: Vec::new(),
            calls: BTreeMap::new(),
            values: BTreeMap::new(),
        };
        if !matches!(
            parsed.language,
            Language::JavaScript
                | Language::TypeScript
                | Language::Tsx
                | Language::Python
                | Language::Go
        ) {
            return facts;
        }
        let boundaries = parsed.language.callable_boundary_node_types();
        let root = parsed.tree.root_node();
        let mut pending = vec![root];
        let mut owner_nodes = Vec::new();
        let mut owner_ids = BTreeMap::new();
        let mut call_nodes = Vec::new();
        let mut counts = BTreeMap::new();
        while let Some(node) = pending.pop() {
            // A decorated definition and its function child describe one callable.
            let decorated_child = parsed.language == Language::Python
                && node.kind() == "function_definition"
                && node
                    .parent()
                    .is_some_and(|parent| parent.kind() == "decorated_definition");
            if node == root || (boundaries.contains(&node.kind()) && !decorated_child) {
                let function = crate::anon_owner::identity(parsed, &node, &parsed.path);
                if let Some(id) = &function {
                    *counts.entry((id.name.clone(), id.start_line)).or_insert(0) += 1;
                }
                owner_ids.insert(node.id(), facts.owners.len());
                owner_nodes.push(node);
                facts.owners.push(ReviewOccurrenceOwner {
                    function,
                    span: node.byte_range(),
                    unique: false,
                });
            }
            if parsed.language.is_call_node(node.kind()) && !node.has_error() {
                call_nodes.push(node);
            }
            let mut cursor = node.walk();
            pending.extend(node.named_children(&mut cursor));
        }
        for owner in &mut facts.owners {
            owner.unique = owner
                .function
                .as_ref()
                .is_some_and(|id| counts[&(id.name.clone(), id.start_line)] == 1);
        }
        let mut lines: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        for node in call_nodes {
            let Some(name) = parsed.call_function_name(&node) else {
                continue;
            };
            let callee = parsed.node_text(&name).to_string();
            let line = node.start_position().row + 1;
            let owner = owner_index(node, &owner_ids, &owner_nodes);
            if let Some(index) = owner {
                lines.entry(index).or_default().insert(line);
            }
            let arguments = parsed
                .call_argument_texts_and_spans_at(node.start_byte(), &callee)
                .into_iter()
                .map(|(_, span)| span)
                .collect();
            facts.calls.insert(
                (node.start_byte(), node.end_byte()),
                ReviewCallOccurrence {
                    span: node.byte_range(),
                    line,
                    callee,
                    arguments,
                    owner: owner.map(|index| facts.owners[index].clone()),
                },
            );
        }
        // One query per owner, including call-start lines for multiline arguments.
        // Exact ownership is independently recovered from the full boundary census.
        for (index, lines) in lines {
            if facts.owners[index].function.is_none() {
                continue;
            }
            for occurrence in parsed.rvalue_identifier_spans_on_lines(&owner_nodes[index], &lines) {
                let (start, end) = (occurrence.start_byte, occurrence.end_byte);
                let owner = root
                    .descendant_for_byte_range(start, end)
                    .and_then(|node| owner_index(node, &owner_ids, &owner_nodes))
                    .map(|index| facts.owners[index].clone());
                let values = facts.values.entry((start, end)).or_default();
                if !values
                    .iter()
                    .any(|value| value.occurrence == occurrence && value.owner == owner)
                {
                    values.push(ReviewValueOccurrence {
                        occurrence,
                        owner,
                        semantic_value: parsed.is_semantic_value_use(start, end),
                    });
                }
            }
        }
        facts
    }

    pub fn file(&self) -> &str {
        &self.parsed.path
    }
    pub fn owners(&self) -> &[ReviewOccurrenceOwner] {
        &self.owners
    }
    pub fn call(&self, span: Range<usize>) -> Option<&ReviewCallOccurrence> {
        self.calls.get(&(span.start, span.end))
    }
    pub fn values(&self, span: Range<usize>) -> &[ReviewValueOccurrence] {
        self.values
            .get(&(span.start, span.end))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

/// Unrecognized boundaries fence off their values too. Header/decorator and
/// class-evaluation regions stay unsupported rather than acquiring a body owner.
fn owner_index(
    mut node: Node<'_>,
    ids: &BTreeMap<usize, usize>,
    owners: &[Node<'_>],
) -> Option<usize> {
    let span = node.byte_range();
    loop {
        if let Some(&index) = ids.get(&node.id()) {
            let owner = owners[index];
            if owner.parent().is_none() {
                return Some(index);
            }
            let body = owner.child_by_field_name("body").or_else(|| {
                (owner.kind() == "decorated_definition")
                    .then(|| {
                        owner
                            .child_by_field_name("definition")?
                            .child_by_field_name("body")
                    })
                    .flatten()
            })?;
            return (body.start_byte() <= span.start && span.end <= body.end_byte())
                .then_some(index);
        }
        if matches!(node.kind(), "class_body" | "computed_property_name") {
            return None;
        }
        node = node.parent()?;
    }
}

#[cfg(test)]
#[path = "review_occurrences_tests.rs"]
mod tests;
