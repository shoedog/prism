use super::{writes::string_literal, FieldRows, PathMatch, PathUse, Resolver};
use crate::targets::inventory::{
    python::{
        source::{chain, children, has_comma, name, scalar_subscript, ungroup},
        walk::{ScopeId, ScopeKind},
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

    pub(super) fn field_method_declaration(
        &self,
        node: Node<'tree>,
        scope: ScopeId,
        method: &str,
    ) -> Option<(Family, Result<Declaration<'tree>, UnprovenCause>)> {
        self.method_declaration(node, scope, false, method)
            .or_else(|| {
                self.field_annotation_refusal(self.parsed, node, node, scope, method, false)
                    .map(|(family, cause)| (family, Err(cause)))
            })
    }

    // A union can identify a listed class for diagnostics without supplying
    // the single-class declaration required by R3. Resolve its arms with
    // the same header and quoted-annotation rules as a simple annotation.
    fn field_annotation_refusal(
        &self,
        parsed: &ParsedFile,
        node: Node<'_>,
        site: Node<'tree>,
        scope: ScopeId,
        method: &str,
        quoted: bool,
    ) -> Option<(Family, UnprovenCause)> {
        let node = type_expression(node)?;
        let class = self.walk.class(scope);
        let static_only = quoted || self.walk.postponed;
        if let Some(path) = type_path(parsed, node, class) {
            let path_use = PathUse::Declaration {
                method: Some(method),
                static_only,
            };
            let (PathMatch::Class(Some(family)), declaration) = self
                .resolve_parts(&path, site, scope, path_use)
                .or_else(|| self.field_shadowed_annotation(&path, site, scope, path_use))?
            else {
                return None;
            };
            return Some((
                family,
                declaration
                    .err()
                    .unwrap_or(UnprovenCause::ReceiverValueUnproven),
            ));
        }
        if !quoted && matches!(node.kind(), "string" | "concatenated_string") {
            let expression = parse_quoted(parsed, node)?;
            return self.field_annotation_refusal(
                &expression,
                quoted_value(&expression)?,
                site,
                scope,
                method,
                true,
            );
        }
        let (members, wrapper) = match node.kind() {
            "union_type" => (children(node).collect(), None),
            "binary_operator"
                if node
                    .child_by_field_name("operator")
                    .is_some_and(|op| op.kind() == "|") =>
            {
                (children(node).collect(), None)
            }
            "generic_type" | "subscript" => {
                let (wrapper, members) = if node.kind() == "generic_type" {
                    let [wrapper, args] = children(node).collect::<Vec<_>>().try_into().ok()?;
                    (wrapper, children(args).collect())
                } else {
                    let wrapper = node.child_by_field_name("value")?;
                    let mut cursor = node.walk();
                    let members: Vec<_> = node
                        .children_by_field_name("subscript", &mut cursor)
                        .collect();
                    (wrapper, members)
                };
                let path = type_path(parsed, wrapper, class)?;
                let (_, identity) = self.resolve_parts(
                    &path,
                    site,
                    scope,
                    PathUse::Typing {
                        path: "typing.Union",
                        static_only,
                    },
                )?;
                (members, identity.err())
            }
            _ => return None,
        };
        members
            .into_iter()
            .filter_map(|member| {
                let (family, cause) =
                    self.field_annotation_refusal(parsed, member, site, scope, method, quoted)?;
                Some((family, wrapper.map_or(cause, |wrapper| wrapper.min(cause))))
            })
            .min_by_key(|(_, cause)| *cause)
    }

    fn field_shadowed_annotation(
        &self,
        path: &[String],
        site: Node<'tree>,
        scope: ScopeId,
        path_use: PathUse<'_>,
    ) -> Option<(PathMatch, Result<String, UnprovenCause>)> {
        let [root] = path else {
            return None;
        };
        let class = &self.walk.scopes[scope];
        if class.kind != ScopeKind::Class
            || !self.lookup(scope, root, site).iter().any(|alt| {
                alt.owner == scope
                    && alt
                        .bindings
                        .iter()
                        .any(|binding| !matches!(binding.kind, super::Kind::Import(_)))
            })
        {
            return None;
        }
        // The canonical bare spelling still ties a class-body shadow to the
        // explicit enclosing import. An unrelated spelling remains silent.
        let candidate = self
            .lookup(class.parent, root, site)
            .iter()
            .flat_map(|alt| &alt.bindings)
            .filter_map(|binding| match &binding.kind {
                super::Kind::Import(import)
                    if import
                        .path
                        .rsplit_once('.')
                        .is_some_and(|(_, exported)| exported == root) =>
                {
                    path_use.matches(&import.path)
                }
                _ => None,
            })
            .min()?;
        let cause = if self.census.wildcard {
            UnprovenCause::StarImport
        } else {
            UnprovenCause::ShadowedRoot
        };
        Some((candidate, Err(cause)))
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
                PathUse::Typing {
                    path: "typing.Optional",
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
        // A file whose type syntax the grammar may misread has an unreadable
        // binding census: no declaration in it proves a receiver.
        let class = if self.field_rows == FieldRows::Refused {
            class.and(Err(UnprovenCause::ReceiverNotUnique))
        } else {
            class
        };
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
        let quoted = parse_quoted(parsed, node)?;
        return annotation(
            &quoted,
            quoted_value(&quoted)?,
            class,
            nullable_allowed,
            true,
        );
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

fn parse_quoted(parsed: &ParsedFile, node: Node<'_>) -> Option<ParsedFile> {
    let value = string_literal(parsed, node)?;
    ParsedFile::parse("annotation.py", &format!("({value})"), Language::Python).ok()
}

fn quoted_value(parsed: &ParsedFile) -> Option<Node<'_>> {
    let root = parsed.tree.root_node();
    if root.has_error() {
        return None;
    }
    let [statement] = children(root).collect::<Vec<_>>().try_into().ok()?;
    let [value] = children(statement).collect::<Vec<_>>().try_into().ok()?;
    Some(value)
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
