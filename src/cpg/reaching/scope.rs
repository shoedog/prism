//! Lexical binding ownership for reaching definitions.

mod cpp;
mod masks;
pub(super) mod rust_scope;
mod typescript;
mod use_lookup;
use super::binding_table::{
    rows, select_binding_row, BindingScopeRule, DeclarationKind, Role, Ruling, Visibility,
};
#[cfg(test)]
use super::grammar_lint::introduction_is_classified;
use super::grammar_lint::{collect_unclassified_binding_lines, grammar_introducing_fields};
use super::scope_python::collect_python_comprehension_declarations;
use super::transfer::{self, HeaderSpan, ReuseBinder};
use super::{CfgView, DefSite, Line};
use crate::ast::ParsedFile;
use crate::data_flow::FlowEdge;
use crate::languages::Language;
pub(super) use masks::classified_masked_reads;
use masks::{collect_classified_masks, simple_catch_parameter_owns_def};
use std::cell::OnceCell;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ScopeSpan {
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
pub(super) enum BindingId {
    Declaration(usize),
    ImplicitFunction(String),
    FlatFallback(String),
}

#[derive(Clone, Debug)]
pub(super) struct Binding {
    pub(super) id: BindingId,
    pub(super) scope: ScopeSpan,
    pub(super) declaration_line: Option<Line>,
    pub(super) classified_mask: bool,
}

#[derive(Clone, Debug)]
pub(super) struct Declaration {
    pub(super) binding: Binding,
    pub(super) name: String,
    pub(super) visible_from: usize,
    pub(super) additional_visibility: Vec<ScopeSpan>,
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

pub(super) struct RootFunction<'a> {
    pub(super) function: Node<'a>,
    parameters: OnceCell<Option<Node<'a>>>,
}

impl<'a> RootFunction<'a> {
    pub(super) fn new(parsed: &ParsedFile, function: Node<'a>) -> Self {
        #[cfg(all(test, debug_assertions))]
        super::work_tests::bump(|counts| counts.debug_root_inventories += 1);
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

    pub(super) fn parameters(&self, parsed: &ParsedFile) -> Option<Node<'a>> {
        *self.parameters.get_or_init(|| {
            #[cfg(test)]
            super::work_tests::bump(|counts| counts.parameter_queries += 1);
            // Cache absence too; preserve bare-arrow semantics.
            parsed.find_parameters_node(&self.function)
        })
    }
}

pub(super) struct BindingFacts {
    pub(super) def_bindings: Vec<Binding>,
    def_sites: BTreeMap<(String, Line, usize), usize>,
    declarations: Vec<Declaration>,
    unclassified_binding_lines: BTreeMap<String, BTreeSet<Line>>,
    pub(super) header_spans: Vec<HeaderSpan>,
    pub(super) no_entry_binders: BTreeMap<String, BTreeSet<Line>>,
    pub(super) no_entry_spans: Vec<(String, Line, super::grammar_lint::ByteSpan)>,
    function_scope: ScopeSpan,
    language: Language,
    /// PR-B perf: zero-width Use byte resolution is a pure function of the
    /// pass and `(line, path)`; memoize it per pass (it re-runs whole-function
    /// rvalue queries per edge otherwise).
    use_byte_cache:
        std::cell::RefCell<BTreeMap<(Line, crate::access_path::AccessPath), Option<usize>>>,
}

impl BindingFacts {
    pub(super) fn new(
        parsed: &ParsedFile,
        func_node: &Node<'_>,
        defs: &[DefSite],
        cfg: &CfgView<'_>,
        binders: &[ReuseBinder],
    ) -> Self {
        #[cfg(test)]
        super::work_tests::bump(|counts| counts.binding_facts += 1);
        let function_scope = function_scope(parsed, func_node);
        let (header_spans, no_entry_binders, no_entry_spans) =
            transfer::binding_metadata(parsed, binders, cfg);
        let seeds: Vec<Option<(ScopeSpan, DeclarationKind, usize)>> = defs
            .iter()
            .map(|def| declaration_seed(parsed, func_node, def, function_scope))
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
        collect_classified_masks(
            parsed,
            *func_node,
            *func_node,
            defs,
            function_scope,
            &mut declarations,
        );
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
                .reuses_binding_in_scope(parsed.language)
                .then(|| {
                    declarations
                        .iter()
                        .enumerate()
                        .filter(|(_, declaration)| {
                            declaration.name == name
                                && (declaration.binding.scope == scope
                                    || (kind == DeclarationKind::Reuse
                                        && declaration
                                            .binding
                                            .scope
                                            .contains(defs[index].start_byte)))
                                && declaration.is_visible_at(defs[index].start_byte)
                        })
                        .min_by_key(|(declaration_index, declaration)| {
                            (
                                declaration.binding.scope.width(),
                                Reverse(declaration.visible_from),
                                Reverse(if kind == DeclarationKind::Reuse {
                                    0
                                } else {
                                    *declaration_index
                                }),
                            )
                        })
                        .map(|(declaration_index, _)| declaration_index)
                })
                .flatten();
            let declaration_index = if let Some(reused) = reused {
                reused
            } else if kind == DeclarationKind::Reuse {
                continue;
            } else {
                let declaration_index = declarations.len();
                declarations.push(Declaration {
                    binding: Binding {
                        id: BindingId::Declaration(declaration_index),
                        scope,
                        declaration_line: Some(defs[index].line),
                        classified_mask: false,
                    },
                    name,
                    visible_from,
                    additional_visibility: Vec::new(),
                });
                declaration_index
            };
            declaration_for_def[index] = Some(declaration_index);
        }

        let mut facts = Self {
            def_bindings: Vec::with_capacity(defs.len()),
            def_sites: defs
                .iter()
                .enumerate()
                .map(|(index, def)| ((def.path.base.clone(), def.line, def.start_byte), index))
                .collect(),
            declarations,
            unclassified_binding_lines,
            header_spans,
            no_entry_binders,
            no_entry_spans,
            function_scope,
            language: parsed.language,
            use_byte_cache: std::cell::RefCell::new(BTreeMap::new()),
        };
        for (index, def) in defs.iter().enumerate() {
            let catch_var = seeds[index]
                .is_some_and(|(_, kind, _)| kind == DeclarationKind::JavaScriptVar)
                && simple_catch_parameter_owns_def(parsed, func_node.id(), def);
            let binding = catch_var
                .then(|| facts.visible_declaration(&def.path.base, def.start_byte))
                .flatten()
                .filter(|binding| binding.classified_mask)
                .or_else(|| {
                    declaration_for_def[index]
                        .map(|declaration| facts.declarations[declaration].binding.clone())
                })
                .or_else(|| facts.visible_declaration(&def.path.base, def.start_byte))
                .unwrap_or_else(|| facts.implicit_binding(&def.path.base));
            facts.def_bindings.push(binding);
        }
        facts
    }

    fn use_byte(&self, parsed: &ParsedFile, edge: &FlowEdge) -> Option<usize> {
        if edge.to.start_byte < edge.to.end_byte {
            return Some(edge.to.start_byte);
        }
        let key = (edge.to.line, edge.to.path.clone());
        if let Some(cached) = self.use_byte_cache.borrow().get(&key) {
            return *cached;
        }
        let value = use_lookup::use_byte(parsed, edge);
        #[cfg(test)]
        super::work_tests::bump(|counts| counts.use_byte_misses += 1);
        self.use_byte_cache.borrow_mut().insert(key, value);
        value
    }

    pub(super) fn mask_on_route(&self, edge: &FlowEdge, use_byte: usize) -> Option<Line> {
        let def_index = self.def_sites.get(&(
            edge.from.path.base.clone(),
            edge.from.line,
            edge.from.start_byte,
        ))?;
        let mask = self.visible_declaration(&edge.to.path.base, use_byte)?;
        (mask.classified_mask && self.def_bindings[*def_index].id != mask.id)
            .then_some(mask.declaration_line)
            .flatten()
    }

    pub(super) fn same_def_binding(&self, left: usize, right: usize) -> bool {
        let left = &self.def_bindings[left].id;
        let right = &self.def_bindings[right].id;
        left == right
            || matches!(left, BindingId::FlatFallback(_))
            || matches!(right, BindingId::FlatFallback(_))
    }

    pub(super) fn same_def_binding_strict(&self, left: usize, right: usize) -> bool {
        self.def_bindings[left].id == self.def_bindings[right].id
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
        self.visible_declaration(&edge.to.path.base, use_byte)
            .map(|binding| matches!(binding.id, BindingId::FlatFallback(_)))
            .unwrap_or_else(|| {
                matches!(
                    self.implicit_binding(&edge.to.path.base).id,
                    BindingId::FlatFallback(_)
                )
            })
    }

    pub(super) fn unclassified_binding_lines(&self, name: &str) -> Vec<Line> {
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
        let use_binding = self
            .visible_declaration(&edge.to.path.base, use_byte)
            .unwrap_or_else(|| self.implicit_binding(&edge.to.path.base));
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
            classified_mask: false,
        }
    }
}

fn declaration_seed(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    def: &DefSite,
    function_scope: ScopeSpan,
) -> Option<(ScopeSpan, DeclarationKind, usize)> {
    // A bare arrow formal has no `parameters` list; without this region its Def
    // is seeded as a non-parameter binding and diverges from the `(x) => …` control.
    if let Some(binding) = typescript::parameter_binding_in_region(parsed, func_node, def) {
        match binding {
            typescript::ParameterBinding::Owned => {
                return Some((
                    function_scope,
                    DeclarationKind::Parameter,
                    function_scope.start_byte,
                ));
            }
            typescript::ParameterBinding::FailClosed => return None,
            typescript::ParameterBinding::Legacy => {}
        }
    }
    let end_byte = def.start_byte.checked_add(1)?;
    let mut node = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(def.start_byte, end_byte)?;
    loop {
        let selected_row = select_binding_row(parsed, node);
        if let Some(row) = selected_row.filter(|row| row.visibility == Visibility::Header) {
            let left = node.child_by_field_name("left");
            if left.is_some_and(|left| {
                left.start_byte() <= def.start_byte && def.start_byte < left.end_byte()
            }) {
                let owner = if node.kind() == "range_clause" {
                    node.parent()
                        .filter(|parent| parent.kind() == "for_statement")
                        .unwrap_or(node)
                } else {
                    node
                };
                return Some((
                    scope_span(owner),
                    row.declaration.unwrap_or(DeclarationKind::Other),
                    if node.kind() == "range_clause" {
                        node.child_by_field_name("right")?.end_byte()
                    } else {
                        left.expect("checked above").end_byte()
                    },
                ));
            }
            if row.roles.has(Role::Scope) {
                if let Some(identifier) =
                    transfer::row_introduced_identifiers(parsed, node, row.fields)
                        .into_iter()
                        .find(|identifier| identifier.start_byte() == def.start_byte)
                {
                    return Some((
                        scope_span(node),
                        row.declaration.unwrap_or(DeclarationKind::Other),
                        identifier.end_byte(),
                    ));
                }
            }
        }
        let selected = selected_row.and_then(|row| row.declaration);
        let rule = binding_scope_rule(parsed.language, node.kind());
        if let Some(kind) = selected.or(rule.declaration) {
            if rust_scope::pattern_excludes_def(parsed, node, &def.path.base) {
                if node.id() == func_node.id() {
                    return None;
                }
                node = node.parent()?;
                continue;
            }
            if selected_row.is_some_and(|row| {
                row.declaration.is_some()
                    && transfer::row_excludes_identifier(parsed, node, row.fields, def.start_byte)
            }) {
                if node.id() == func_node.id() {
                    return None;
                }
                node = node.parent()?;
                continue;
            }
            let value = if kind == DeclarationKind::PythonAssignment {
                parsed.language.assignment_value(&node)
            } else {
                parsed.language.declaration_value(&node)
            };
            if value.is_some_and(|value| {
                value.start_byte() <= def.start_byte && def.start_byte < value.end_byte()
            }) {
                return None;
            }
            let scope = declaration_scope(parsed, func_node, node, kind, function_scope);
            let rust_item_is_hoisted = parsed.language == Language::Rust
                && matches!(node.kind(), "const_item" | "static_item");
            let visible_from = if rust_item_is_hoisted {
                scope.start_byte
            } else {
                match kind {
                    DeclarationKind::Parameter
                    | DeclarationKind::JavaScriptVar
                    | DeclarationKind::PythonAssignment
                    | DeclarationKind::Reuse => scope.start_byte,
                    DeclarationKind::GoShort
                    | DeclarationKind::Other
                    | DeclarationKind::CaptureCopy => node.end_byte(),
                    DeclarationKind::Pattern => rust_scope::pattern_visible_from(node),
                }
            };
            return Some((scope, kind, visible_from));
        }
        if node.id() == func_node.id() {
            return None;
        }
        node = node.parent()?;
    }
}

fn declaration_scope(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    node: Node<'_>,
    kind: DeclarationKind,
    function_scope: ScopeSpan,
) -> ScopeSpan {
    if kind == DeclarationKind::JavaScriptVar {
        return function_scope;
    }
    if kind == DeclarationKind::Pattern {
        if let Some(scope) = rust_scope::pattern_declaration_scope(parsed, node) {
            return scope;
        }
    }
    if let Some(scope) = rust_scope::closure_parameter_scope(parsed, node) {
        return scope;
    }
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.id() == func_node.id() {
            break;
        }
        if binding_scope_rule(parsed.language, parent.kind()).creates_scope {
            if let Some(owner) = cpp::condition_scope_owner(parsed.language, parent) {
                return scope_span(owner);
            }
            return scope_span(parent);
        }
        current = parent.parent();
    }
    function_scope
}

pub(super) fn binding_scope_rule(language: Language, kind: &str) -> BindingScopeRule {
    if let Some(r) = rows(language)
        .iter()
        .find(|r| r.kind == kind && r.variant.is_none())
    {
        return BindingScopeRule {
            creates_scope: r.roles.has(Role::Scope)
                && !(language == Language::Python
                    && matches!(kind, "match_statement" | "case_clause")),
            declaration: r.declaration,
        };
    }
    BindingScopeRule {
        creates_scope: false,
        declaration: None,
    }
}

fn function_scope(_parsed: &ParsedFile, func_node: &Node<'_>) -> ScopeSpan {
    func_node
        .child_by_field_name("body")
        .map(scope_span)
        .unwrap_or_else(|| scope_span(*func_node))
}

pub(super) fn scope_span(node: Node<'_>) -> ScopeSpan {
    if let Some(span) = rust_scope::let_header_scope(node) {
        return span;
    }
    ScopeSpan {
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        end_line: node.end_position().row + 1,
    }
}

#[cfg(test)]
#[path = "scope_reuse_tests.rs"]
mod reuse_tests;
