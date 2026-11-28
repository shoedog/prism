use super::{writes::string_literal, PathMatch, PathUse, Resolver};
use crate::targets::inventory::{
    python::{
        source::{chain, children, has_comma, name, scalar_subscript, ungroup},
        walk::ScopeId,
    },
    Family, UnprovenCause,
};
use prism::{ast::ParsedFile, languages::Language};
use tree_sitter::Node;

pub(super) struct Declaration<'tree> {
    pub class: String,
    #[cfg_attr(not(any(test, feature = "inventory-evidence")), allow(dead_code))]
    pub node: Node<'tree>,
    pub nullable: bool,
}

struct Annotation {
    class: Vec<String>,
    quoted: bool,
    nullable: bool,
    optional: Option<(Vec<String>, bool)>,
}

impl<'tree> Resolver<'_, 'tree> {
    pub(super) fn declaration(
        &self,
        node: Node<'tree>,
        scope: ScopeId,
        local: bool,
    ) -> Option<Result<Declaration<'tree>, UnprovenCause>> {
        self.declaration_match(node, scope, local, None)
            .map(|(_, declaration)| declaration)
    }

    pub(super) fn method_declaration(
        &self,
        node: Node<'tree>,
        scope: ScopeId,
        local: bool,
        method: &str,
    ) -> Option<(Family, Result<Declaration<'tree>, UnprovenCause>)> {
        let (family, declaration) = self.declaration_match(node, scope, local, Some(method))?;
        Some((family?, declaration))
    }

    fn declaration_match(
        &self,
        node: Node<'tree>,
        scope: ScopeId,
        local: bool,
        method: Option<&str>,
    ) -> Option<(Option<Family>, Result<Declaration<'tree>, UnprovenCause>)> {
        let annotation = annotation(self.parsed, node, self.walk.class(scope), true, false)?;
        let static_site = local || self.walk.postponed;
        let (kind, class) = self.resolve_parts(
            &annotation.class,
            node,
            scope,
            PathUse::Declaration {
                method,
                static_only: static_site || annotation.quoted,
            },
        )?;
        let PathMatch::Class(family) = kind else {
            return None;
        };
        let class = class.map(|class| Declaration {
            class,
            node,
            nullable: annotation.nullable,
        });
        if let Some((optional, quoted)) = annotation.optional {
            let (_, wrapper) = self.resolve_parts(
                &optional,
                node,
                scope,
                PathUse::Optional {
                    static_only: static_site || quoted,
                },
            )?;
            if let Err(cause) = wrapper {
                return Some((
                    family,
                    Err(class.err().map_or(cause, |class| class.min(cause))),
                ));
            }
        }
        Some((family, class))
    }
}

fn annotation(
    parsed: &ParsedFile,
    node: Node<'_>,
    class: Option<&str>,
    nullable_allowed: bool,
    quoted: bool,
) -> Option<Annotation> {
    let node = type_expression(node)?;
    if let Some(path) = type_path(parsed, node, class) {
        return Some(Annotation {
            class: path,
            quoted,
            nullable: false,
            optional: None,
        });
    }
    if matches!(node.kind(), "string" | "concatenated_string") && !quoted {
        let value = string_literal(parsed, node)?;
        let parsed =
            ParsedFile::parse("annotation.py", &format!("({value})"), Language::Python).ok()?;
        let root = parsed.tree.root_node();
        if root.has_error() {
            return None;
        }
        let [statement] = children(root).collect::<Vec<_>>().try_into().ok()?;
        let [value] = children(statement).collect::<Vec<_>>().try_into().ok()?;
        return annotation(&parsed, value, class, nullable_allowed, true);
    }
    if !nullable_allowed {
        return None;
    }
    match node.kind() {
        "union_type" | "binary_operator" => {
            if node.kind() == "binary_operator"
                && node
                    .child_by_field_name("operator")
                    .is_none_or(|op| op.kind() != "|")
            {
                return None;
            }
            let [left, right] = children(node).collect::<Vec<_>>().try_into().ok()?;
            let inner = match (
                type_expression(left)?.kind(),
                type_expression(right)?.kind(),
            ) {
                ("none", _) => right,
                (_, "none") => left,
                _ => return None,
            };
            let mut annotation = annotation(parsed, inner, class, false, quoted)?;
            annotation.nullable = true;
            Some(annotation)
        }
        "subscript" | "generic_type" => {
            let (wrapper, inner) = if node.kind() == "subscript" {
                (node.child_by_field_name("value")?, scalar_subscript(node)?)
            } else {
                let [wrapper, parameters] = children(node).collect::<Vec<_>>().try_into().ok()?;
                if parameters.kind() != "type_parameter" || has_comma(parameters) {
                    return None;
                }
                let [inner] = children(parameters).collect::<Vec<_>>().try_into().ok()?;
                (wrapper, inner)
            };
            let path = type_path(parsed, wrapper, class)?;
            let mut annotation = annotation(parsed, inner, class, false, quoted)?;
            annotation.nullable = true;
            annotation.optional = Some((path, quoted));
            Some(annotation)
        }
        _ => None,
    }
}

fn type_expression(mut node: Node<'_>) -> Option<Node<'_>> {
    loop {
        node = ungroup(node);
        if node.kind() != "type" {
            return Some(node);
        }
        let [child] = children(node).collect::<Vec<_>>().try_into().ok()?;
        node = child;
    }
}

fn type_path(parsed: &ParsedFile, node: Node<'_>, class: Option<&str>) -> Option<Vec<String>> {
    let node = type_expression(node)?;
    if node.kind() == "member_type" {
        let [base, terminal] = children(node).collect::<Vec<_>>().try_into().ok()?;
        let mut path = type_path(parsed, base, class)?;
        path.push(name(parsed.node_text(&terminal), class));
        Some(path)
    } else {
        chain(parsed, node, class)
    }
}
