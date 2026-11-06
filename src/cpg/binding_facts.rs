//! Lexical binding ownership: which declaration a name occurrence binds to.

use super::reaching::{DefSite, Line};
use crate::ast::ParsedFile;
use crate::data_flow::FlowEdge;
use crate::languages::Language;
use std::cell::OnceCell;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;
use tree_sitter::Node;

#[path = "binding_facts/go_bindings.rs"]
mod go_bindings;

#[derive(serde::Deserialize)]
struct NodeTypeSchema {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    subtypes: Vec<NodeTypeRef>,
    #[serde(default)]
    fields: BTreeMap<String, NodeTypeMembers>,
    #[serde(default)]
    children: Option<NodeTypeMembers>,
}

#[derive(serde::Deserialize)]
struct NodeTypeMembers {
    #[serde(default)]
    types: Vec<NodeTypeRef>,
}

#[derive(serde::Deserialize)]
struct NodeTypeRef {
    #[serde(rename = "type")]
    kind: String,
}

type IntroducingFields = BTreeMap<String, BTreeSet<String>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ScopeSpan {
    start_byte: usize,
    end_byte: usize,
    end_line: Line,
}

impl ScopeSpan {
    fn contains(self, byte: usize) -> bool {
        self.start_byte <= byte && byte < self.end_byte
    }

    fn width(self) -> usize {
        self.end_byte.saturating_sub(self.start_byte)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BindingId {
    Declaration(usize),
    ImplicitFunction(String),
    FlatFallback(String),
}

#[derive(Clone, Debug)]
struct Binding {
    id: BindingId,
    scope: ScopeSpan,
    declaration_line: Option<Line>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeclarationKind {
    Parameter,
    GoShort,
    JavaScriptVar,
    PythonAssignment,
    Other,
}

impl DeclarationKind {
    fn reuses_binding_in_scope(self) -> bool {
        matches!(
            self,
            Self::GoShort | Self::JavaScriptVar | Self::PythonAssignment
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BindingScopeRule {
    creates_scope: bool,
    declaration: Option<DeclarationKind>,
}

#[derive(Clone, Debug)]
struct Declaration {
    binding: Binding,
    name: String,
    visible_from: usize,
    additional_visibility: Vec<ScopeSpan>,
}

impl Declaration {
    fn is_visible_at(&self, byte: usize) -> bool {
        self.visible_from <= byte
            || self
                .additional_visibility
                .iter()
                .any(|span| span.contains(byte))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BindingRelation {
    Same,
    KilledAt(Line),
    Unresolved,
}

struct RootFunction<'a> {
    function: Node<'a>,
    parameters: OnceCell<Option<Node<'a>>>,
}

impl<'a> RootFunction<'a> {
    fn new(parsed: &ParsedFile, function: Node<'a>) -> Self {
        #[cfg(all(test, debug_assertions))]
        super::reaching::work_tests::bump(|counts| counts.debug_root_inventories += 1);
        debug_assert!(
            parsed
                .all_functions()
                .iter()
                .any(|f| f.id() == function.id()),
            "root function must remain indexed"
        );
        Self {
            function,
            parameters: OnceCell::new(),
        }
    }

    fn parameters(&self, parsed: &ParsedFile) -> Option<Node<'a>> {
        *self.parameters.get_or_init(|| {
            #[cfg(test)]
            super::reaching::work_tests::bump(|counts| counts.parameter_queries += 1);
            // Cache absence too; preserve bare-arrow semantics.
            parsed.find_parameters_node(&self.function)
        })
    }
}

pub(crate) struct BindingFacts {
    def_bindings: Vec<Binding>,
    declarations: Vec<Declaration>,
    unclassified_binding_lines: BTreeMap<String, BTreeSet<Line>>,
    function_scope: ScopeSpan,
    language: Language,
    use_bytes: super::reaching::use_bytes::UseBytes,
}

impl BindingFacts {
    pub(crate) fn new<'a>(
        parsed: &ParsedFile,
        func_node: &Node<'_>,
        defs: &[DefSite],
        use_edges: impl Iterator<Item = &'a FlowEdge>,
    ) -> Self {
        #[cfg(test)]
        super::reaching::work_tests::bump(|counts| counts.binding_facts += 1);
        let function_scope = function_scope(parsed, func_node);
        let seeds: Vec<Option<(ScopeSpan, DeclarationKind, usize)>> = defs
            .iter()
            .map(|def| {
                (parsed.language != Language::Go)
                    .then(|| declaration_seed(parsed, func_node, def, function_scope))
                    .flatten()
            })
            .collect();

        let mut declarations = Vec::<Declaration>::new();
        if parsed.language == Language::Python {
            collect_python_comprehension_declarations(
                parsed,
                *func_node,
                func_node.id(),
                &mut declarations,
            );
        }
        let mut unclassified_binding_lines = BTreeMap::new();
        let callable_boundary_kinds = parsed.language.callable_boundary_node_types();
        collect_unclassified_binding_lines(
            parsed,
            *func_node,
            &RootFunction::new(parsed, *func_node),
            &callable_boundary_kinds,
            grammar_introducing_fields(parsed.language),
            &mut unclassified_binding_lines,
        );
        let mut declaration_for_def = vec![None; defs.len()];
        let mut order: Vec<usize> = (0..defs.len()).collect();
        order.sort_by_key(|index| defs[*index].start_byte);
        for index in order {
            let Some((scope, kind, visible_from)) = seeds[index] else {
                continue;
            };
            let name = defs[index].path.base.clone();
            let reused = kind
                .reuses_binding_in_scope()
                .then(|| {
                    declarations
                        .iter()
                        .enumerate()
                        .filter(|(_, declaration)| {
                            declaration.name == name
                                && declaration.binding.scope == scope
                                && declaration.is_visible_at(defs[index].start_byte)
                        })
                        .max_by_key(|(_, declaration)| declaration.visible_from)
                        .map(|(declaration_index, _)| declaration_index)
                })
                .flatten();
            let declaration_index = if let Some(reused) = reused {
                reused
            } else {
                let declaration_index = declarations.len();
                declarations.push(Declaration {
                    binding: Binding {
                        id: BindingId::Declaration(declaration_index),
                        scope,
                        declaration_line: Some(defs[index].line),
                    },
                    name,
                    visible_from,
                    additional_visibility: Vec::new(),
                });
                declaration_index
            };
            declaration_for_def[index] = Some(declaration_index);
        }

        if parsed.language == Language::Go {
            (declarations, declaration_for_def) = go_bindings::collect(parsed, *func_node, defs);
        }

        let mut facts = Self {
            def_bindings: Vec::with_capacity(defs.len()),
            declarations,
            unclassified_binding_lines,
            function_scope,
            language: parsed.language,
            use_bytes: super::reaching::use_bytes::UseBytes::new(use_edges),
        };
        for (index, def) in defs.iter().enumerate() {
            let binding = declaration_for_def[index]
                .map(|declaration| facts.declarations[declaration].binding.clone())
                .or_else(|| facts.visible_declaration(&def.path.base, def.start_byte))
                .unwrap_or_else(|| facts.implicit_binding(&def.path.base));
            facts.def_bindings.push(binding);
        }
        facts
    }

    /// Append the bindings of census kill-only defs (#660 S1b-1), which follow the recorded defs in
    /// the RD def list. They are writes, never declarations: each resolves like a recorded non-
    /// declaring def, through the declaration visible at its byte, else the flat fallback, so they
    /// add no declaration and change no other def's binding.
    pub(super) fn extend_kill_only(&mut self, kill_only: &[DefSite]) {
        for def in kill_only {
            let binding = self
                .visible_declaration(&def.path.base, def.start_byte)
                .unwrap_or_else(|| self.implicit_binding(&def.path.base));
            self.def_bindings.push(binding);
        }
    }

    fn use_byte(&self, parsed: &ParsedFile, edge: &FlowEdge) -> Option<usize> {
        #[cfg(test)]
        if super::reaching::scope::use_byte_tests::legacy_enabled() {
            return super::reaching::scope::use_byte(parsed, edge);
        }
        self.use_bytes.get(parsed, edge)
    }

    pub(super) fn same_def_binding(&self, left: usize, right: usize) -> bool {
        let left = &self.def_bindings[left].id;
        let right = &self.def_bindings[right].id;
        left == right
            || matches!(left, BindingId::FlatFallback(_))
            || matches!(right, BindingId::FlatFallback(_))
    }

    pub(super) fn lookup_requires_flat_fallback(
        &self,
        parsed: &ParsedFile,
        edge: &FlowEdge,
        def_index: usize,
    ) -> bool {
        if matches!(self.def_bindings[def_index].id, BindingId::FlatFallback(_)) {
            return true;
        }
        let Some(use_byte) = self.use_byte(parsed, edge) else {
            return true;
        };
        self.lookup_at_requires_flat_fallback(&edge.to.path.base, use_byte, def_index)
    }

    pub(super) fn proven_same_def_binding(&self, left: usize, right: usize) -> bool {
        self.def_bindings[left].id == self.def_bindings[right].id
            && !matches!(self.def_bindings[left].id, BindingId::FlatFallback(_))
    }

    /// Two defs that both resolve to the same unresolved (package-level or
    /// otherwise undeclared) spelling. The contribution lane treats a write
    /// through one as killing the other: fewer edges, never more.
    pub(super) fn same_fallback_binding(&self, left: usize, right: usize) -> bool {
        matches!(
            (&self.def_bindings[left].id, &self.def_bindings[right].id),
            (BindingId::FlatFallback(a), BindingId::FlatFallback(b)) if a == b
        )
    }

    pub(super) fn lookup_at_requires_flat_fallback(
        &self,
        base: &str,
        use_byte: usize,
        def_index: usize,
    ) -> bool {
        if matches!(self.def_bindings[def_index].id, BindingId::FlatFallback(_)) {
            return true;
        }
        self.visible_declaration(base, use_byte)
            .map(|binding| matches!(binding.id, BindingId::FlatFallback(_)))
            .unwrap_or_else(|| matches!(self.implicit_binding(base).id, BindingId::FlatFallback(_)))
    }

    pub(crate) fn unclassified_binding_lines(&self, name: &str) -> Vec<Line> {
        self.unclassified_binding_lines
            .get(name)
            .map(|lines| lines.iter().copied().collect())
            .unwrap_or_default()
    }

    pub(super) fn relation(
        &self,
        parsed: &ParsedFile,
        edge: &FlowEdge,
        def_index: usize,
    ) -> BindingRelation {
        let definition = &self.def_bindings[def_index];
        let Some(use_byte) = self.use_byte(parsed, edge) else {
            return if definition
                .scope
                .contains(parsed.line_start_byte(edge.to.line))
            {
                BindingRelation::Unresolved
            } else {
                BindingRelation::KilledAt(definition.scope.end_line)
            };
        };
        self.relation_at(&edge.to.path.base, use_byte, def_index)
    }

    /// Explicit-byte admission for the contribution lane, without allocating
    /// prospective FlowEdges just to ask about the root binding.
    pub(super) fn relation_at(
        &self,
        base: &str,
        use_byte: usize,
        def_index: usize,
    ) -> BindingRelation {
        let definition = &self.def_bindings[def_index];
        let use_binding = self
            .visible_declaration(base, use_byte)
            .unwrap_or_else(|| self.implicit_binding(base));
        if definition.id == use_binding.id {
            return BindingRelation::Same;
        }
        if !definition.scope.contains(use_byte) {
            return BindingRelation::KilledAt(definition.scope.end_line);
        }
        if use_binding.scope.contains(use_byte) {
            if let Some(line) = use_binding.declaration_line {
                return BindingRelation::KilledAt(line);
            }
        }
        BindingRelation::Unresolved
    }

    /// The binding the definition at `defs[index]` resolves to.
    pub(crate) fn binding_of_def(&self, index: usize) -> &BindingId {
        &self.def_bindings[index].id
    }

    /// Declarations per name; lets a test compare a syntactic pre-scan to the inventory.
    #[cfg(test)]
    pub(crate) fn declaration_counts(&self) -> std::collections::BTreeMap<String, usize> {
        let mut counts = std::collections::BTreeMap::new();
        for declaration in &self.declarations {
            *counts.entry(declaration.name.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// The binding a name occurrence at `byte` resolves to: the innermost
    /// visible explicit declaration, else the language's implicit binding.
    pub(crate) fn binding_at(&self, name: &str, byte: usize) -> BindingId {
        self.visible_declaration(name, byte)
            .unwrap_or_else(|| self.implicit_binding(name))
            .id
    }

    fn visible_declaration(&self, name: &str, byte: usize) -> Option<Binding> {
        self.declarations
            .iter()
            .filter(|declaration| {
                declaration.name == name
                    && declaration.binding.scope.contains(byte)
                    && declaration.is_visible_at(byte)
            })
            .min_by_key(|declaration| {
                (
                    declaration.binding.scope.width(),
                    Reverse(declaration.visible_from),
                )
            })
            .map(|declaration| declaration.binding.clone())
    }

    fn implicit_binding(&self, name: &str) -> Binding {
        let id = if self.language == Language::Python {
            BindingId::ImplicitFunction(name.to_string())
        } else {
            BindingId::FlatFallback(name.to_string())
        };
        Binding {
            id,
            scope: self.function_scope,
            declaration_line: None,
        }
    }
}

mod lexical;
use lexical::*;

#[cfg(test)]
#[path = "binding_facts/scope_reuse_tests.rs"]
mod reuse_tests;

#[cfg(test)]
#[path = "binding_facts/go_binding_tests.rs"]
mod go_binding_tests;
