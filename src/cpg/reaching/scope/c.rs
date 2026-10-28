use super::super::binding_table::{select_binding_row, DeclarationKind};
use super::{DefSite, Line, ScopeSpan};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

#[derive(Default)]
pub(super) struct BindingFacts {
    rejected: BTreeSet<(String, Line, usize)>,
    recovered_lines: BTreeMap<String, Line>,
}

impl BindingFacts {
    pub(super) fn new(parsed: &ParsedFile, function: &Node<'_>, defs: &[DefSite]) -> Self {
        if parsed.language != Language::C {
            return Self::default();
        }
        let parameters = parsed.find_parameters_node(function);
        let mut facts = Self::default();
        for def in defs {
            if recovered_parameter_seed(parsed, *function, parameters, def) {
                facts
                    .rejected
                    .insert((def.path.base.clone(), def.line, def.start_byte));
                facts
                    .recovered_lines
                    .entry(def.path.base.clone())
                    .and_modify(|line| *line = (*line).min(def.line))
                    .or_insert(def.line);
            }
        }
        facts
    }

    pub(super) fn allows(&self, def: &DefSite) -> bool {
        !self
            .rejected
            .contains(&(def.path.base.clone(), def.line, def.start_byte))
    }

    pub(super) fn recovered_line(&self, name: &str) -> Option<Line> {
        self.recovered_lines.get(name).copied()
    }
}

fn recovered_parameter_seed(
    parsed: &ParsedFile,
    function: Node<'_>,
    parameters: Option<Node<'_>>,
    def: &DefSite,
) -> bool {
    let Some(end_byte) = def.start_byte.checked_add(1) else {
        return false;
    };
    let Some(node) = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(def.start_byte, end_byte)
    else {
        return false;
    };
    let parameter_row = std::iter::successors(Some(node), |node| node.parent())
        .take_while(|node| node.id() != function.id())
        .any(|node| {
            select_binding_row(parsed, node)
                .and_then(|row| row.declaration)
                .is_some_and(|kind| kind == DeclarationKind::Parameter)
        });
    parameter_row
        && !parameters.is_some_and(|parameters| {
            parameters.start_byte() <= def.start_byte && def.start_byte < parameters.end_byte()
        })
}

pub(super) fn scope_span(parsed: &ParsedFile, node: Node<'_>) -> ScopeSpan {
    let mut span = super::scope_span(node);
    if parsed.language == Language::C
        && node.kind() == "for_statement"
        && node.start_position().row == node.end_position().row
    {
        span.end_line += 1;
    }
    span
}
