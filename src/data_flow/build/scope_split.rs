//! Split declaration headers keep main's behaviour (#694 slice A, fold 2; owner decision
//! 2026-11-07, the real fix is #728).
//!
//! Queries and taint connect a Use to a Def only on one line. A range/receive `:=`, a
//! type-switch alias or a `var`/`const` spec is *split* when any token of its right-hand side
//! (the range expression, the received channel expression, the asserted expression, the spec's
//! values) is on a different row from any of its declared names: a right-hand read on row N
//! never reaches the new def on row M, so no replacement path exists. Such a header keeps
//! main's categorical exemption: rule S and rule O keep every pair of a name it declares
//! (`SplitFacts::exempts`), and rule D never takes a split spec as a dominating write.
//! The exemption covers the defs main had. A def at a header target is new (main emitted none),
//! so it is exempt only where its variable is the one the split header shadows: sibling loops'
//! same-named headers are not a path.
//! Rule U keeps its declare-only anchor handling: the anchor line holds only the declaration,
//! and the kept S pair carries the outer value to the body use.
use crate::ast::ParsedFile;
use crate::cpg::binding_facts::{BindingFacts, BindingId};
use std::collections::BTreeMap;
use tree_sitter::Node;

/// Names declared by split headers of one owner, with the start byte of each header.
pub(super) struct SplitFacts {
    by_name: BTreeMap<String, Vec<usize>>,
}

impl SplitFacts {
    pub(super) fn new(parsed: &ParsedFile, owner: Node<'_>) -> Self {
        let mut by_name: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut stack = vec![owner];
        while let Some(node) = stack.pop() {
            for id in split_declared(node) {
                by_name
                    .entry(parsed.node_text(&id).to_owned())
                    .or_default()
                    .push(node.start_byte());
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
        Self { by_name }
    }

    /// Main's categorical keep for the pair of `name` whose def is `defs[def_index]` at `def_start`.
    pub(super) fn exempts(
        &self,
        parsed: &ParsedFile,
        facts: &BindingFacts,
        name: &str,
        def_index: usize,
        def_start: usize,
    ) -> bool {
        let Some(headers) = self.by_name.get(name) else {
            return false;
        };
        let at_target = parsed
            .tree
            .root_node()
            .descendant_for_byte_range(def_start, def_start)
            .is_some_and(is_header_target);
        !at_target
            || headers.iter().any(|byte| {
                matches!(facts.binding_at(name, *byte), BindingId::Declaration(d)
                    if facts.def_reaches_declaration(def_index, d))
            })
    }
}

/// The identifier is declared by a range/receive `:=`, a type-switch alias or a `var`/`const` spec.
fn is_header_target(node: Node<'_>) -> bool {
    node.kind() == "identifier"
        && super::scope_prune::is_declaring_name(node)
        && !matches!(
            node.parent().map(|p| p.kind()),
            Some("parameter_declaration" | "variadic_parameter_declaration")
        )
        && node
            .parent()
            .and_then(|p| p.parent())
            .is_none_or(|g| g.kind() != "short_var_declaration")
}

/// The declared identifiers of `node` when it is a split header or spec, else empty.
pub(super) fn split_declared(node: Node<'_>) -> Vec<Node<'_>> {
    let (names, rhs) = match node.kind() {
        "range_clause" | "receive_statement" => {
            let mut cursor = node.walk();
            let declares = node.children(&mut cursor).any(|c| c.kind() == ":=");
            (
                node.child_by_field_name("left").filter(|_| declares),
                node.child_by_field_name("right"),
            )
        }
        "type_switch_statement" => (
            node.child_by_field_name("alias"),
            node.child_by_field_name("value"),
        ),
        "var_spec" | "const_spec" => {
            let mut cursor = node.walk();
            let first = node.children_by_field_name("name", &mut cursor).next();
            return match (first, node.child_by_field_name("value")) {
                (Some(_), Some(rhs)) => {
                    let mut cursor = node.walk();
                    let ids: Vec<_> = node
                        .children_by_field_name("name", &mut cursor)
                        .filter(|n| n.kind() == "identifier")
                        .collect();
                    keep_split(ids, rhs)
                }
                _ => Vec::new(),
            };
        }
        _ => (None, None),
    };
    let (Some(names), Some(rhs)) = (names, rhs) else {
        return Vec::new();
    };
    let ids: Vec<_> = if names.kind() == "identifier" {
        vec![names]
    } else {
        let mut cursor = names.walk();
        names
            .named_children(&mut cursor)
            .filter(|n| n.kind() == "identifier")
            .collect()
    };
    keep_split(ids, rhs)
}

/// All of `ids` when some token of `rhs` sits on a row other than some name's row. The right-hand
/// side's first and last tokens bound its rows, so comparing its first and last rows is exact.
fn keep_split<'a>(ids: Vec<Node<'a>>, rhs: Node<'_>) -> Vec<Node<'a>> {
    let (first, last) = (rhs.start_position().row, rhs.end_position().row);
    if ids
        .iter()
        .any(|id| id.start_position().row != first || id.start_position().row != last)
    {
        ids
    } else {
        Vec::new()
    }
}
