use super::certificate::{self, Refusals};
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

/// The binder's whole answer for one operation: the five fields originate
/// together and are never recombined from separate sources.
#[derive(Clone, Copy)]
pub(super) struct Bound<'a> {
    operation: Node<'a>,
    target: Node<'a>,
    operand: Node<'a>,
    role: Role<'a>,
    detector: Detector<'a>,
}

impl<'a> Bound<'a> {
    /// Callable only by a language's operand binder, for the detector entry it matched.
    pub(super) fn from_binder(
        operation: Node<'a>,
        target: Node<'a>,
        operand: Node<'a>,
        role: Role<'a>,
        detector: Detector<'a>,
    ) -> Self {
        Self {
            operation,
            target,
            operand,
            role,
            detector,
        }
    }

    pub(super) fn operation(&self) -> Node<'a> {
        self.operation
    }

    pub(super) fn target(&self) -> Node<'a> {
        self.target
    }

    pub(super) fn operand(&self) -> Node<'a> {
        self.operand
    }

    pub(super) fn role(&self) -> Role<'a> {
        self.role
    }

    pub(super) fn detector(&self) -> Detector<'a> {
        self.detector
    }
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
        visit: InlineVisit<'a>,
    },
}

/// What the admission visit loop actually did: its route, the operation it
/// visited, the source key it iterated and the key the finding was inserted at.
#[derive(Clone, Copy)]
pub(super) struct InlineVisit<'a> {
    route: InlineRoute,
    operation: (usize, usize),
    source: FindingKey<'a>,
    insertion: FindingKey<'a>,
}

impl<'a> InlineVisit<'a> {
    /// Callable only by the admission visit loop, with the values of that visit.
    pub(super) fn from_visit(
        route: InlineRoute,
        operation: Node<'_>,
        source: FindingKey<'a>,
        insertion: FindingKey<'a>,
    ) -> Self {
        Self {
            route,
            operation: (operation.start_byte(), operation.end_byte()),
            source,
            insertion,
        }
    }
}

/// The adapter certifies the value, target spelling and semantic operand.
/// Receipt checks their structural relationship and the admission route.
pub(super) struct Occurrence<'a, 'p> {
    parsed: &'a ParsedFile,
    key: FindingKey<'a>,
    owner: Node<'a>,
    bound: Bound<'a>,
    reached: Node<'a>,
    provenance: Provenance<'a, 'p>,
}

impl<'a, 'p> Occurrence<'a, 'p> {
    pub(super) fn from_adapter(
        parsed: &'a ParsedFile,
        key: FindingKey<'a>,
        owner: Node<'a>,
        bound: Bound<'a>,
        reached: Node<'a>,
        provenance: Provenance<'a, 'p>,
    ) -> Self {
        Self {
            parsed,
            key,
            owner,
            bound,
            reached,
            provenance,
        }
    }

    pub(super) fn parsed(&self) -> &'a ParsedFile {
        self.parsed
    }

    pub(super) fn site(&self) -> Bound<'a> {
        self.bound
    }

    pub(super) fn reached(&self) -> Node<'a> {
        self.reached
    }

    pub(super) fn detector(&self) -> Detector<'a> {
        self.bound.detector
    }

    pub(super) fn provenance(&self) -> Provenance<'a, 'p> {
        self.provenance
    }

    pub(super) fn verifies(
        &self,
        parsed: &ParsedFile,
        key: FindingKey<'_>,
        refusals: &mut Refusals,
    ) -> bool {
        if self.key != key || key.0 != parsed.path || !std::ptr::eq(self.parsed, parsed) {
            return false;
        }
        if !parsed.language.is_js_family()
            && !matches!(parsed.language, Language::Python | Language::Go)
        {
            return false;
        }

        let Bound {
            operation,
            target,
            operand,
            role,
            detector,
        } = self.bound;
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
        if !ancestors(self.reached).any(|node| node == operand) {
            return false;
        }
        if let Err((_, reason)) = certificate::certify(parsed, operation, target, operand, role) {
            refusals.record(reason);
            return false;
        }

        let opening_line = operation.start_position().row + 1;
        let structured = matches!(detector, Detector::Structured { .. });
        match self.provenance {
            Provenance::Path {
                path,
                edge_index,
                route,
                ..
            } => {
                // The destination nominates a line. The adapter supplies the actual read.
                let read_line = self.reached.start_position().row + 1;
                let owner_line = self.owner.start_position().row + 1;
                let read = self.reached.byte_range();
                if !path.edges.get(edge_index).is_some_and(|edge| {
                    let to = &edge.to;
                    let (start, end) = (to.start_byte, to.end_byte);
                    // A zero-width location is a point that must lie inside the read.
                    let touches = if start >= end {
                        read.start <= start && start < read.end
                    } else {
                        start < read.end && read.start < end
                    };
                    to.file == key.0
                        && to.line == read_line
                        && (self.owner == root || to.function_start_line == owner_line)
                        && touches
                }) {
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
            Provenance::Inline { visit } => {
                let (source, route) = (visit.source, visit.route);
                structured
                    && visit.operation == (operation.start_byte(), operation.end_byte())
                    && visit.insertion == key
                    && source.0 == key.0
                    && match route {
                        InlineRoute::JsYaml => {
                            matches!(
                                parsed.language,
                                Language::JavaScript | Language::TypeScript | Language::Tsx
                            ) && opening_line == key.1
                                && source.1 == opening_line
                        }
                        InlineRoute::Flask => {
                            parsed.language == Language::Python
                                && node_contains_line(&operation, key.1)
                                && node_contains_line(&operation, source.1)
                        }
                        InlineRoute::Go => {
                            parsed.language == Language::Go
                                && opening_line == key.1
                                && source.1 == opening_line
                        }
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
