use super::source::{children, identifier, ungroup};
use prism::ast::ParsedFile;
use tree_sitter::Node;

pub(in crate::targets::inventory) type ScopeId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::targets::inventory) enum ScopeKind {
    Module,
    Function,
    Class,
    Lambda,
    Comprehension,
}

impl ScopeKind {
    pub(in crate::targets::inventory) fn is_function(self) -> bool {
        matches!(self, Self::Function | Self::Lambda)
    }
}

pub(in crate::targets::inventory) struct Scope<'tree> {
    pub kind: ScopeKind,
    pub parent: ScopeId,
    pub node: Node<'tree>,
    pub body: Node<'tree>,
    pub class: Option<String>,
}

#[derive(Clone, Copy)]
pub(in crate::targets::inventory) struct Context {
    pub scope: ScopeId,
    pub owner: Option<ScopeId>,
    pub frame: Option<ScopeKind>,
    pub evaluated: bool,
    pub local_annotation: bool,
}

pub(super) struct Call<'tree> {
    pub node: Node<'tree>,
    pub callee: Node<'tree>,
    pub context: Context,
    pub order: (usize, Vec<usize>),
    pub local_annotation: bool,
}

pub(in crate::targets::inventory) struct Walk<'tree> {
    pub scopes: Vec<Scope<'tree>>,
    pub nodes: Vec<(Node<'tree>, Context)>,
    pub(super) calls: Vec<Call<'tree>>,
    pub(in crate::targets::inventory) postponed: bool,
}

impl<'tree> Walk<'tree> {
    pub(super) fn new(parsed: &'tree ParsedFile) -> Self {
        let root = parsed.tree.root_node();
        let postponed = children(root).any(|node| {
            node.kind() == "future_import_statement"
                && children(node).any(|item| {
                    let target = item.child_by_field_name("name").unwrap_or(item);
                    parsed.node_text(&target) == "annotations"
                })
        });
        let mut walk = Self {
            scopes: vec![Scope {
                kind: ScopeKind::Module,
                parent: 0,
                node: root,
                body: root,
                class: None,
            }],
            nodes: Vec::new(),
            calls: Vec::new(),
            postponed,
        };
        walk.visit(
            parsed,
            root,
            Context {
                scope: 0,
                owner: None,
                frame: None,
                evaluated: true,
                local_annotation: false,
            },
            &mut Vec::new(),
            0,
        );
        walk
    }

    pub(in crate::targets::inventory) fn class(&self, scope: ScopeId) -> Option<&str> {
        self.scopes[scope].class.as_deref()
    }

    fn add_scope(
        &mut self,
        parsed: &ParsedFile,
        node: Node<'tree>,
        body: Node<'tree>,
        context: Context,
        kind: ScopeKind,
    ) -> Context {
        let scope = self.scopes.len();
        let class = if kind == ScopeKind::Class {
            node.child_by_field_name("name")
                .map(|name| identifier(parsed.node_text(&name)))
        } else {
            self.scopes[context.scope].class.clone()
        };
        self.scopes.push(Scope {
            kind,
            parent: context.scope,
            node,
            body,
            class,
        });
        let mut inner = Context { scope, ..context };
        match kind {
            ScopeKind::Function => {
                inner.owner = Some(scope);
                inner.frame = None;
            }
            ScopeKind::Class | ScopeKind::Lambda => inner.frame = Some(kind),
            _ => {}
        }
        inner
    }

    fn visit(
        &mut self,
        parsed: &'tree ParsedFile,
        node: Node<'tree>,
        context: Context,
        path: &mut Vec<usize>,
        depth: usize,
    ) {
        // In an f-string, a top-level colon starts the format specifier.
        // The grammar can recover a bare `:=` there as a named expression.
        if node.kind() == "named_expression"
            && node.parent().is_some_and(|parent| {
                matches!(parent.kind(), "interpolation" | "format_expression")
            })
        {
            return;
        }
        self.nodes.push((node, context));
        if matches!(node.kind(), "case_pattern" | "type_alias_statement") {
            return;
        }
        if node.kind() == "call" {
            if let Some(callee) = parsed.call_function_node(&node) {
                self.calls.push(Call {
                    node,
                    callee: ungroup(callee),
                    context,
                    order: (depth, path.clone()),
                    local_annotation: context.local_annotation,
                });
            }
        }

        let kind = match node.kind() {
            "function_definition" => Some(ScopeKind::Function),
            "class_definition" => Some(ScopeKind::Class),
            "lambda" => Some(ScopeKind::Lambda),
            "list_comprehension"
            | "set_comprehension"
            | "dictionary_comprehension"
            | "generator_expression" => Some(ScopeKind::Comprehension),
            _ => None,
        };
        let body = node.child_by_field_name("body");
        let inner = kind
            .zip(body)
            .map(|(kind, body)| self.add_scope(parsed, node, body, context, kind));
        let first_for = (kind == Some(ScopeKind::Comprehension))
            .then(|| children(node).find(|n| n.kind() == "for_in_clause"))
            .flatten();
        let local_annotation =
            node.kind() == "assignment" && self.scopes[context.scope].kind.is_function();
        let annotation = node
            .child_by_field_name("type")
            .or_else(|| node.child_by_field_name("return_type"));

        for (index, child) in ast_children(node).into_iter().enumerate() {
            let mut child_context = if kind == Some(ScopeKind::Comprehension) || body == Some(child)
            {
                inner.unwrap_or(context)
            } else {
                context
            };
            if Some(child) == annotation && (self.postponed || local_annotation) {
                child_context.evaluated = false;
                child_context.local_annotation |= local_annotation;
            }
            path.push(index);
            let child_depth = depth + usize::from(!transparent(node, child));
            if Some(child) == first_for {
                self.nodes.push((child, child_context));
                let left = child.child_by_field_name("left");
                for (clause_index, part) in children(child).enumerate() {
                    path.push(clause_index);
                    self.visit(
                        parsed,
                        part,
                        if Some(part) == left {
                            child_context
                        } else {
                            context
                        },
                        path,
                        child_depth + 1,
                    );
                    path.pop();
                }
            } else {
                self.visit(parsed, child, child_context, path, child_depth);
            }
            path.pop();
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TieField {
    DictKey,
    DictValue,
    ConditionalTest,
    ConditionalBody,
    ConditionalElse,
    KeywordOnlyDefault,
    PositionalDefault,
    AssignmentTarget,
}

use TieField::*;

// Sibling field pairs covered by the Python 3.11.11 ast.walk oracle. The
// parser's traversal order alone cannot settle a tie in any other pair.
const ORDERED_TIE_FIELDS: &[(TieField, TieField)] = &[
    (DictKey, DictValue),
    (ConditionalTest, ConditionalBody),
    (ConditionalTest, ConditionalElse),
    (ConditionalBody, ConditionalElse),
    (KeywordOnlyDefault, PositionalDefault),
    (AssignmentTarget, AssignmentTarget),
];

pub(super) fn supports_anchor_tie(left: Node<'_>, right: Node<'_>) -> bool {
    fn ancestry(node: Node<'_>) -> Vec<Node<'_>> {
        std::iter::successors(Some(node), Node::parent).collect()
    }
    let left = ancestry(left);
    let right = ancestry(right);
    let shared = left
        .iter()
        .rev()
        .zip(right.iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let (left_end, right_end) = (left.len() - shared, right.len() - shared);
    if left_end == 0 || right_end == 0 {
        // An ancestor call precedes its descendants in breadth-first order.
        return true;
    }
    let parent = left[left_end];
    let fields = (
        tie_field(parent, &left[..left_end]),
        tie_field(parent, &right[..right_end]),
    );
    ORDERED_TIE_FIELDS
        .iter()
        .any(|&(a, b)| fields == (Some(a), Some(b)) || fields == (Some(b), Some(a)))
        && exact_anchor_branch(&left[..=left_end])
        && exact_anchor_branch(&right[..=right_end])
}

fn exact_anchor_branch(lineage: &[Node<'_>]) -> bool {
    // For these listed splits, the walk visits the field directly through
    // the pair or parameter wrapper that tie_field has already checked.
    let skip_wrapper = matches!(
        lineage.last().map(Node::kind),
        Some("dictionary" | "parameters" | "lambda_parameters")
    );
    lineage
        .windows(2)
        .rev()
        .skip(usize::from(skip_wrapper))
        .all(|edge| {
            let (child, parent) = (edge[0], edge[1]);
            transparent(parent, child)
                || match child.kind() {
                    "call" | "attribute" | "list" | "tuple" | "set" | "unary_operator"
                    | "not_operator" | "binary_operator" | "await" | "keyword_argument" => true,
                    "subscript" => {
                        // A comma, including a trailing comma, adds an AST Tuple index.
                        let mut cursor = child.walk();
                        let has_comma = child.children(&mut cursor).any(|part| part.kind() == ",");
                        !has_comma
                    }
                    _ => false,
                }
        })
}

fn tie_field(parent: Node<'_>, branch: &[Node<'_>]) -> Option<TieField> {
    let (child, inner) = branch.split_last()?;
    match parent.kind() {
        "dictionary" if child.kind() == "pair" => dictionary_field(*child, *inner.last()?),
        "pair"
            if parent
                .parent()
                .is_some_and(|node| node.kind() == "dictionary") =>
        {
            dictionary_field(parent, *child)
        }
        "conditional_expression" => children(parent)
            .zip([ConditionalBody, ConditionalTest, ConditionalElse])
            .find_map(|(node, field)| (node == *child).then_some(field)),
        "parameters" | "lambda_parameters"
            if inner
                .last()
                .is_some_and(|&value| child.child_by_field_name("value") == Some(value)) =>
        {
            let keyword_only = children(parent)
                .take_while(|node| node != child)
                .any(|node| {
                    matches!(
                        parameter_kind(node),
                        "keyword_separator" | "list_splat_pattern"
                    )
                });
            Some(if keyword_only {
                KeywordOnlyDefault
            } else {
                PositionalDefault
            })
        }
        "assignment" => {
            let mut assignment = parent;
            for &node in branch.iter().rev() {
                if assignment.child_by_field_name("left") == Some(node) {
                    return Some(AssignmentTarget);
                }
                if node.kind() != "assignment"
                    || assignment.child_by_field_name("right") != Some(node)
                {
                    break;
                }
                assignment = node;
            }
            None
        }
        _ => None,
    }
}

fn dictionary_field(pair: Node<'_>, child: Node<'_>) -> Option<TieField> {
    if pair.child_by_field_name("key") == Some(child) {
        Some(DictKey)
    } else if pair.child_by_field_name("value") == Some(child) {
        Some(DictValue)
    } else {
        None
    }
}

// Approximate ast.walk depth and field indices. Anchor selection consults
// this order only after verifying that the tie is supported.
fn ast_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut nodes: Vec<_> = children(node).collect();
    match node.kind() {
        "argument_list" => {
            nodes.sort_by_key(|node| matches!(node.kind(), "keyword_argument" | "dictionary_splat"))
        }
        "parameters" | "lambda_parameters" => return parameter_children(nodes),
        "default_parameter" | "typed_default_parameter" => {
            nodes.retain(|child| Some(*child) != node.child_by_field_name("value"));
        }
        "dictionary" => {
            let keys = nodes
                .iter()
                .filter_map(|pair| pair.child_by_field_name("key"));
            let values = nodes.iter().filter_map(|item| {
                if item.kind() == "dictionary_splat" {
                    children(*item).next()
                } else {
                    item.child_by_field_name("value")
                }
            });
            return keys.chain(values).collect();
        }
        "conditional_expression" if nodes.len() == 3 => nodes.swap(0, 1),
        "function_definition" => nodes.sort_by_key(|n| {
            if Some(*n) == node.child_by_field_name("return_type") {
                1
            } else {
                0
            }
        }),
        "decorated_definition" => nodes.sort_by_key(|node| node.kind() == "decorator"),
        _ => {}
    }
    nodes
}

fn parameter_children(parameters: Vec<Node<'_>>) -> Vec<Node<'_>> {
    // ast.arguments visits positional args, vararg, keyword-only args,
    // keyword-only defaults, kwarg, then positional defaults. Default
    // values are siblings of arg nodes, rather than children of those args.
    let mut args = Vec::new();
    let mut keyword_only = false;
    let mut keyword_defaults = Vec::new();
    let mut positional_defaults = Vec::new();
    let mut kwarg = None;
    for parameter in parameters {
        let kind = parameter_kind(parameter);
        match kind {
            "list_splat_pattern" => {
                keyword_only = true;
                args.push(parameter);
            }
            "keyword_separator" => keyword_only = true,
            "positional_separator" => {}
            "dictionary_splat_pattern" => kwarg = Some(parameter),
            _ => {
                args.push(parameter);
                if matches!(kind, "default_parameter" | "typed_default_parameter") {
                    if let Some(value) = parameter.child_by_field_name("value") {
                        if keyword_only {
                            keyword_defaults.push(value);
                        } else {
                            positional_defaults.push(value);
                        }
                    }
                }
            }
        }
    }
    args.extend(keyword_defaults);
    args.extend(kwarg);
    args.extend(positional_defaults);
    args
}

fn parameter_kind(parameter: Node<'_>) -> &'static str {
    if parameter.kind() == "typed_parameter" {
        children(parameter)
            .next()
            .map_or(parameter.kind(), |target| target.kind())
    } else {
        parameter.kind()
    }
}

fn transparent(node: Node<'_>, child: Node<'_>) -> bool {
    match child.kind() {
        "parenthesized_expression"
        | "type"
        | "argument_list"
        | "decorated_definition"
        | "decorator"
        | "block" => true,
        "expression_statement" => children(child)
            .next()
            .is_some_and(|child| matches!(child.kind(), "assignment" | "augmented_assignment")),
        "assignment" => node.kind() == "assignment",
        _ => false,
    }
}
