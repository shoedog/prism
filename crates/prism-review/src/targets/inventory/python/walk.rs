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
    postponed: bool,
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

// The path through Python AST fields orders equal-depth calls as ast.walk
// does. Concrete-only nodes are transparent. In particular, the grammar
// interleaves positional and keyword arguments, while the AST groups them.
fn ast_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut nodes: Vec<_> = children(node).collect();
    match node.kind() {
        "argument_list" => {
            nodes.sort_by_key(|node| matches!(node.kind(), "keyword_argument" | "dictionary_splat"))
        }
        "parameters" | "lambda_parameters" => nodes.sort_by_key(|node| {
            matches!(node.kind(), "default_parameter" | "typed_default_parameter")
        }),
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
        _ => {
            matches!(node.kind(), "default_parameter" | "typed_default_parameter")
                && Some(child) == node.child_by_field_name("value")
        }
    }
}
