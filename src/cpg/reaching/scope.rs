//! Lexical binding ownership for reaching definitions.

use super::{DefSite, Line};
use crate::ast::ParsedFile;
use crate::data_flow::FlowEdge;
use crate::languages::Language;
use std::cmp::Reverse;
use tree_sitter::Node;

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
enum BindingId {
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BindingRelation {
    Same,
    KilledAt(Line),
    Unresolved,
}

pub(super) struct BindingFacts {
    def_bindings: Vec<Binding>,
    declarations: Vec<Declaration>,
    function_scope: ScopeSpan,
    language: Language,
}

impl BindingFacts {
    pub(super) fn new(parsed: &ParsedFile, func_node: &Node<'_>, defs: &[DefSite]) -> Self {
        let function_scope = function_scope(parsed, func_node);
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
                                && declaration.visible_from <= defs[index].start_byte
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
                });
                declaration_index
            };
            declaration_for_def[index] = Some(declaration_index);
        }

        let mut facts = Self {
            def_bindings: Vec::with_capacity(defs.len()),
            declarations,
            function_scope,
            language: parsed.language,
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

    pub(super) fn same_def_binding(&self, left: usize, right: usize) -> bool {
        let left = &self.def_bindings[left].id;
        let right = &self.def_bindings[right].id;
        left == right
            || matches!(left, BindingId::FlatFallback(_))
            || matches!(right, BindingId::FlatFallback(_))
    }

    pub(super) fn relation(
        &self,
        parsed: &ParsedFile,
        edge: &FlowEdge,
        def_index: usize,
    ) -> BindingRelation {
        let definition = &self.def_bindings[def_index];
        let Some(use_byte) = use_byte(parsed, edge) else {
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
                    && declaration.visible_from <= byte
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

fn declaration_seed(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    def: &DefSite,
    function_scope: ScopeSpan,
) -> Option<(ScopeSpan, DeclarationKind, usize)> {
    if parsed
        .find_parameters_node(func_node)
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
        if matches!(
            parsed.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) && node.kind() == "for_in_statement"
        {
            let left = node.child_by_field_name("left");
            let lexical_kind = node
                .child_by_field_name("kind")
                .map(|kind| parsed.node_text(&kind));
            if left.is_some_and(|left| {
                left.start_byte() <= def.start_byte && def.start_byte < left.end_byte()
            }) && matches!(lexical_kind, Some("let" | "const"))
            {
                return Some((
                    scope_span(node),
                    DeclarationKind::Other,
                    left.expect("checked above").end_byte(),
                ));
            }
        }
        let rule = binding_scope_rule(parsed.language, node.kind());
        if let Some(kind) = rule.declaration {
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
                | DeclarationKind::PythonAssignment => scope.start_byte,
                DeclarationKind::GoShort | DeclarationKind::Other => node.end_byte(),
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

fn binding_scope_rule(language: Language, kind: &str) -> BindingScopeRule {
    let (creates_scope, declaration) = match language {
        Language::Python => (
            matches!(
                kind,
                "function_definition"
                    | "lambda"
                    | "class_definition"
                    | "list_comprehension"
                    | "set_comprehension"
                    | "dictionary_comprehension"
                    | "generator_expression"
            ),
            matches!(
                kind,
                "assignment" | "augmented_assignment" | "named_expression"
            )
            .then_some(DeclarationKind::PythonAssignment),
        ),
        Language::JavaScript | Language::TypeScript | Language::Tsx => (
            matches!(
                kind,
                "statement_block"
                    | "class_body"
                    | "function_declaration"
                    | "function_expression"
                    | "arrow_function"
                    | "for_statement"
                    | "for_in_statement"
            ),
            match kind {
                "variable_declaration" => Some(DeclarationKind::JavaScriptVar),
                "lexical_declaration" | "class_declaration" => Some(DeclarationKind::Other),
                _ => None,
            },
        ),
        Language::Go => (
            matches!(
                kind,
                "block"
                    | "if_statement"
                    | "for_statement"
                    | "expression_switch_statement"
                    | "type_switch_statement"
                    | "switch_statement"
                    | "select_statement"
            ),
            match kind {
                "short_var_declaration" => Some(DeclarationKind::GoShort),
                "var_declaration" | "const_declaration" => Some(DeclarationKind::Other),
                _ => None,
            },
        ),
        Language::Rust => (
            kind == "block",
            matches!(kind, "let_declaration" | "const_item" | "static_item")
                .then_some(DeclarationKind::Other),
        ),
        _ => (false, None),
    };
    BindingScopeRule {
        creates_scope,
        declaration,
    }
}

fn collect_python_comprehension_declarations(
    parsed: &ParsedFile,
    node: Node<'_>,
    root_function_id: usize,
    declarations: &mut Vec<Declaration>,
) {
    if node.id() != root_function_id
        && matches!(
            node.kind(),
            "function_definition" | "lambda" | "class_definition"
        )
    {
        return;
    }

    if node.kind() == "for_in_clause" {
        let mut parent = node.parent();
        while let Some(candidate) = parent {
            if matches!(
                candidate.kind(),
                "list_comprehension"
                    | "set_comprehension"
                    | "dictionary_comprehension"
                    | "generator_expression"
            ) {
                if let Some(target) = node.child_by_field_name("left") {
                    collect_python_target_identifiers(
                        parsed,
                        target,
                        scope_span(candidate),
                        declarations,
                    );
                }
                break;
            }
            parent = candidate.parent();
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_python_comprehension_declarations(parsed, child, root_function_id, declarations);
    }
}

fn collect_python_target_identifiers(
    parsed: &ParsedFile,
    node: Node<'_>,
    scope: ScopeSpan,
    declarations: &mut Vec<Declaration>,
) {
    if node.kind() == "identifier" {
        let declaration_index = declarations.len();
        declarations.push(Declaration {
            binding: Binding {
                id: BindingId::Declaration(declaration_index),
                scope,
                declaration_line: Some(node.start_position().row + 1),
            },
            name: parsed.node_text(&node).to_string(),
            visible_from: scope.start_byte,
        });
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_python_target_identifiers(parsed, child, scope, declarations);
    }
}

fn function_scope(_parsed: &ParsedFile, func_node: &Node<'_>) -> ScopeSpan {
    func_node
        .child_by_field_name("body")
        .map(scope_span)
        .unwrap_or_else(|| scope_span(*func_node))
}

fn scope_span(node: Node<'_>) -> ScopeSpan {
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
