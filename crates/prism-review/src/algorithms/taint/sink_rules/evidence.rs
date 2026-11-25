use super::FindingKey;
use crate::algorithms::taint::cleansing::node_contains_line;
use prism::ast::ParsedFile;
use prism::data_flow::FlowPath;
use prism::languages::Language;
use std::iter::successors;
use tree_sitter::Node;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Role<'a> {
    Argument(usize),
    KeywordArgument(&'a str),
    ReceiverValue,
    AssignmentValue,
    AttributeValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Detector<'a> {
    Flat(&'static str),
    Structured { table: &'static str, entry: usize },
    Configured(&'a str),
    Wrapper(&'a str),
}

#[derive(Clone, Copy)]
pub(super) struct Site<'a> {
    pub(super) operation: Node<'a>,
    pub(super) target: Node<'a>,
    pub(super) operand: Node<'a>,
    pub(super) role: Role<'a>,
}

#[derive(Clone, Copy)]
pub(super) enum PathRoute {
    Destination,
    GoOriginating,
}

#[derive(Clone, Copy)]
pub(super) enum PathProof {
    StableLocal,
    ExactPoint,
}

#[derive(Clone, Copy)]
pub(super) enum InlineRoute {
    JsYaml,
    Flask,
    Go,
}

#[derive(Clone, Copy)]
pub(super) enum Provenance<'a, 'p> {
    Path {
        path: &'p FlowPath,
        edge_index: usize,
        route: PathRoute,
        proof: PathProof,
    },
    Inline {
        key: FindingKey<'a>,
        route: InlineRoute,
    },
}

/// The adapter certifies the value, target spelling and semantic operand.
/// Receipt checks their structural relationship and the admission route.
pub(super) struct Occurrence<'a, 'p> {
    parsed: &'a ParsedFile,
    key: FindingKey<'a>,
    owner: Node<'a>,
    site: Site<'a>,
    reached: Node<'a>,
    detector: Detector<'a>,
    provenance: Provenance<'a, 'p>,
}

impl<'a, 'p> Occurrence<'a, 'p> {
    pub(super) fn from_adapter(
        parsed: &'a ParsedFile,
        key: FindingKey<'a>,
        owner: Node<'a>,
        site: Site<'a>,
        reached: Node<'a>,
        detector: Detector<'a>,
        provenance: Provenance<'a, 'p>,
    ) -> Self {
        Self {
            parsed,
            key,
            owner,
            site,
            reached,
            detector,
            provenance,
        }
    }

    pub(super) fn parsed(&self) -> &'a ParsedFile {
        self.parsed
    }

    pub(super) fn site(&self) -> Site<'a> {
        self.site
    }

    pub(super) fn reached(&self) -> Node<'a> {
        self.reached
    }

    pub(super) fn detector(&self) -> Detector<'a> {
        self.detector
    }

    pub(super) fn provenance(&self) -> Provenance<'a, 'p> {
        self.provenance
    }

    pub(super) fn verifies(&self, parsed: &ParsedFile, key: FindingKey<'_>) -> bool {
        if self.key != key || key.0 != parsed.path || !std::ptr::eq(self.parsed, parsed) {
            return false;
        }

        let Site {
            operation,
            target,
            operand,
            ..
        } = self.site;
        let root = parsed.tree.root_node();
        let boundaries = parsed.language.callable_boundary_node_types();
        let owner = |node| {
            ancestors(node)
                .find(|node| boundaries.contains(&node.kind()))
                .unwrap_or(root)
        };
        if ![self.owner, operation, target, operand, self.reached]
            .into_iter()
            .all(|node| {
                let range = node.byte_range();
                is_current_node(root, node)
                    && !range.is_empty()
                    && parsed.source.get(range).is_some()
                    && owner(node) == self.owner
            })
        {
            return false;
        }
        if !ancestors(target).any(|node| node == operation)
            || !ancestors(operand).any(|node| node == operation)
            || !ancestors(self.reached).any(|node| node == operand)
            || (target.start_byte() < operand.end_byte()
                && operand.start_byte() < target.end_byte())
        {
            return false;
        }

        let opening_line = operation.start_position().row + 1;
        let structured = matches!(self.detector, Detector::Structured { .. });
        match self.provenance {
            Provenance::Path {
                path,
                edge_index,
                route,
                ..
            } => {
                // The destination nominates a line. The adapter supplies the actual read.
                let read_line = self.reached.start_position().row + 1;
                if !path
                    .edges
                    .get(edge_index)
                    .is_some_and(|edge| edge.to.file == key.0 && edge.to.line == read_line)
                {
                    return false;
                }
                match route {
                    PathRoute::Destination => {
                        key.1 == read_line
                            && (parsed.language != Language::Go
                                || !structured
                                || opening_line == key.1)
                    }
                    PathRoute::GoOriginating => {
                        parsed.language == Language::Go
                            && structured
                            && opening_line == key.1
                            && path.edges.first().is_some_and(|edge| {
                                edge.from.file == key.0 && edge.from.line == key.1
                            })
                    }
                }
            }
            Provenance::Inline {
                key: source_key,
                route,
            } => {
                structured
                    && source_key == key
                    && match route {
                        InlineRoute::JsYaml => {
                            matches!(
                                parsed.language,
                                Language::JavaScript | Language::TypeScript | Language::Tsx
                            ) && opening_line == key.1
                        }
                        InlineRoute::Flask => {
                            parsed.language == Language::Python
                                && node_contains_line(&operation, key.1)
                        }
                        InlineRoute::Go => parsed.language == Language::Go && opening_line == key.1,
                    }
            }
        }
    }
}

fn ancestors(node: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    successors(Some(node), Node::parent)
}

fn is_current_node(root: Node<'_>, node: Node<'_>) -> bool {
    // Node equality can share ids across trees and ignores offset-root coordinates.
    if node.into_raw().tree != root.into_raw().tree {
        return false;
    }
    if node == root {
        return node.range() == root.range();
    }
    node.parent().is_some_and(|parent| {
        let mut cursor = parent.walk();
        let mut children = parent.children(&mut cursor);
        children.any(|child| child == node && child.range() == node.range())
    })
}
