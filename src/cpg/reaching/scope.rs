//! Lexical binding ownership for reaching definitions.

use super::binding_table::{
    rows, select_binding_row, BindingScopeRule, DeclarationKind, Role, Ruling, Visibility,
};
#[cfg(test)]
use super::grammar_lint::introduction_is_classified;
use super::grammar_lint::{collect_unclassified_binding_lines, grammar_introducing_fields};
use super::scope_python::collect_python_comprehension_declarations;
use super::transfer::{self, HeaderSpan};
use super::{CfgView, DefSite, Line};
use crate::ast::ParsedFile;
use crate::data_flow::FlowEdge;
use crate::languages::Language;
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
    #[allow(dead_code)]
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
    ) -> Self {
        #[cfg(test)]
        super::work_tests::bump(|counts| counts.binding_facts += 1);
        let function_scope = function_scope(parsed, func_node);
        let binders = transfer::discover(parsed, func_node, defs);
        let (header_spans, no_entry_binders, no_entry_spans) =
            transfer::binding_metadata(parsed, &binders, cfg);
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
                .reuses_binding_in_scope()
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
        let value = use_byte(parsed, edge);
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

fn collect_classified_masks(
    parsed: &ParsedFile,
    node: Node<'_>,
    root_function: Node<'_>,
    defs: &[DefSite],
    function_scope: ScopeSpan,
    declarations: &mut Vec<Declaration>,
) {
    if node.id() != root_function.id()
        && parsed
            .language
            .callable_boundary_node_types()
            .contains(&node.kind())
    {
        return;
    }
    if let Some(row) = select_binding_row(parsed, node).filter(|row| {
        row.roles.has(Role::Binding)
            && matches!(
                row.declaration,
                Some(DeclarationKind::Other | DeclarationKind::Pattern)
            )
            && row.ruling == Ruling::Classified
    }) {
        let scope = if row.roles.has(Role::Scope) {
            scope_span(node)
        } else {
            declaration_scope(
                parsed,
                &root_function,
                node,
                row.declaration.expect("filtered declaration kind"),
                function_scope,
            )
        };
        for field in row.fields {
            if let Some(target) = node.child_by_field_name(field) {
                for identifier in transfer::introduced_identifiers(parsed, target) {
                    if defs
                        .iter()
                        .any(|def| def.start_byte == identifier.start_byte())
                    {
                        continue;
                    }
                    let index = declarations.len();
                    declarations.push(Declaration {
                        binding: Binding {
                            id: BindingId::Declaration(index),
                            scope,
                            declaration_line: Some(node.start_position().row + 1),
                            classified_mask: true,
                        },
                        name: parsed.node_text(&identifier).to_string(),
                        visible_from: match row.visibility {
                            Visibility::WholeScope => scope.start_byte,
                            Visibility::Header => target.end_byte(),
                            _ => node.end_byte(),
                        },
                        additional_visibility: Vec::new(),
                    });
                }
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_classified_masks(
            parsed,
            child,
            root_function,
            defs,
            function_scope,
            declarations,
        );
    }
}

pub(super) fn classified_masked_reads(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    spans: &[crate::ast::PathSpan],
) -> Vec<crate::ast::PathSpan> {
    let function_scope = function_scope(parsed, func_node);
    let mut declarations = Vec::new();
    collect_classified_masks(
        parsed,
        *func_node,
        *func_node,
        &[],
        function_scope,
        &mut declarations,
    );
    spans
        .iter()
        .filter(|span| {
            declarations.iter().any(|declaration| {
                declaration.binding.classified_mask
                    && declaration.name == span.path.base
                    && declaration.binding.scope.contains(span.start_byte)
                    && declaration.is_visible_at(span.start_byte)
            })
        })
        .cloned()
        .collect()
}

fn simple_catch_parameter_owns_def(parsed: &ParsedFile, function_id: usize, def: &DefSite) -> bool {
    if parsed.language != Language::JavaScript {
        return false;
    }
    parsed
        .tree
        .root_node()
        .descendant_for_byte_range(def.start_byte, def.start_byte.saturating_add(1))
        .into_iter()
        .flat_map(|node| std::iter::successors(Some(node), |node| node.parent()))
        .take_while(|node| node.id() != function_id)
        .filter(|node| node.kind() == "catch_clause")
        .filter_map(|node| node.child_by_field_name("parameter"))
        .any(|parameter| {
            parameter.kind() == "identifier" && parsed.node_text(&parameter) == def.path.base
        })
}

fn declaration_seed(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    def: &DefSite,
    function_scope: ScopeSpan,
) -> Option<(ScopeSpan, DeclarationKind, usize)> {
    // js-param-defs PR-A: the bare arrow formal (`x => …`) has no
    // `parameters` list; without its region the Def is seeded as a non-
    // parameter binding and its labels diverge from the `(x) => …` control.
    if parsed
        .parameter_binding_region(func_node)
        .is_some_and(|parameters| {
            parameters.start_byte() <= def.start_byte && def.start_byte < parameters.end_byte()
        })
    {
        return Some((
            function_scope,
            DeclarationKind::Parameter,
            function_scope.start_byte,
        ));
    }
    let end_byte = def.start_byte.checked_add(1)?;
    let mut node = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(def.start_byte, end_byte)?;
    loop {
        let selected_row = select_binding_row(parsed, node);
        if selected_row.is_some_and(|row| row.visibility == Visibility::Header) {
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
                    selected_row
                        .and_then(|row| row.declaration)
                        .unwrap_or(DeclarationKind::Other),
                    left.expect("checked above").end_byte(),
                ));
            }
        }
        let rule = binding_scope_rule(parsed.language, node.kind());
        if let Some(kind) = rule.declaration {
            if selected_row.is_some_and(|row| {
                row.ruling == Ruling::Classified
                    && row.roles.has(Role::Binding)
                    && !row.fields.is_empty()
                    && !row.fields.iter().any(|field| {
                        node.child_by_field_name(field).is_some_and(|target| {
                            transfer::introduced_identifiers(parsed, target)
                                .iter()
                                .any(|identifier| identifier.start_byte() == def.start_byte)
                        })
                    })
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
            let visible_from = match kind {
                DeclarationKind::Parameter
                | DeclarationKind::JavaScriptVar
                | DeclarationKind::PythonAssignment
                | DeclarationKind::Reuse => scope.start_byte,
                DeclarationKind::GoShort
                | DeclarationKind::Other
                | DeclarationKind::Pattern
                | DeclarationKind::CaptureCopy => node.end_byte(),
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
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.id() == func_node.id() {
            break;
        }
        if binding_scope_rule(parsed.language, parent.kind()).creates_scope {
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
            creates_scope: r.roles.has(Role::Scope),
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
    ScopeSpan {
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        end_line: node.end_position().row + 1,
    }
}

fn use_byte(parsed: &ParsedFile, edge: &FlowEdge) -> Option<usize> {
    if edge.to.start_byte < edge.to.end_byte {
        return Some(edge.to.start_byte);
    }
    let lines = std::collections::BTreeSet::from([edge.to.line]);
    let func_node = parsed
        .all_functions()
        .into_iter()
        .find(|node| parsed.node_line_range(node).0 == edge.to.function_start_line)?;
    let mut spans = parsed
        .rvalue_identifier_spans_on_lines(&func_node, &lines)
        .into_iter()
        .filter(|span| span.line == edge.to.line && span.path == edge.to.path);
    if let Some(first) = spans.next() {
        return spans.next().is_none().then_some(first.start_byte);
    }
    let mut identifiers = parsed
        .identifiers_on_line(edge.to.line)
        .into_iter()
        .filter(|node| parsed.node_text(node) == edge.to.path.base);
    let first = identifiers.next()?;
    identifiers.next().is_none().then_some(first.start_byte())
}

#[cfg(test)]
#[path = "scope_reuse_tests.rs"]
mod reuse_tests;
