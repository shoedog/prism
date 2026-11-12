//! The loop trees of an owner as the ownership inventory (`js_disposition`) reads them: syntax only,
//! for every outermost loop whether or not `tree` rewires it. Observational: nothing here changes an
//! edge.
use super::{is_loop, Comp, Cx};
use crate::ast::ParsedFile;
use crate::cfg::js_ownership::Anchor;
use tree_sitter::Node;

pub(in crate::cfg) struct LoopFacts {
    pub(in crate::cfg) node: Anchor,
    pub(in crate::cfg) start_line: usize,
    pub(in crate::cfg) end_line: usize,
    /// The continue point's line: the header, or an admitted `do`'s condition.
    pub(in crate::cfg) head: usize,
    /// The loop's body node.
    pub(in crate::cfg) body: Option<Anchor>,
    /// A `do` whose condition has no execution point of its own (`do_geometry_line` is `None`: a
    /// one-line `do`, or a condition sharing a line with a body or a following statement).
    pub(in crate::cfg) condition_collapsed: bool,
    /// The body's normal completions: the line each leaves and the construct that completes.
    pub(in crate::cfg) completions: Vec<(usize, Anchor)>,
    /// False when `Cx::completions` has no rule for some form in the body (the list is partial).
    pub(in crate::cfg) completions_known: bool,
}

pub(in crate::cfg) struct TreeFacts {
    pub(in crate::cfg) root: Anchor,
    /// Whether this tree is rewired: the plan admits it (under a plan pass, the assumed admission) and
    /// `Cx::tree` routes it.
    pub(in crate::cfg) rewired: bool,
    /// Every loop of the tree in source order, the root first.
    pub(in crate::cfg) loops: Vec<LoopFacts>,
}

/// The loop trees of `owner` (outermost loops outside nested callables, source order).
pub(in crate::cfg) fn tree_facts(owner: Node<'_>, parsed: &ParsedFile) -> Vec<TreeFacts> {
    let Some(body) = owner.child_by_field_name("body") else {
        return Vec::new();
    };
    let boundaries = parsed.language.callable_boundary_node_types();
    let cx = Cx {
        parsed,
        body,
        boundaries: &boundaries,
    };
    cx.outermost()
        .into_iter()
        .map(|root| TreeFacts {
            root: Anchor::of(root),
            rewired: super::plan::admits(parsed, (root.start_byte(), root.end_byte()))
                && cx.tree(root).is_some(),
            loops: loops_under(&cx, root)
                .into_iter()
                .map(|n| facts(&cx, n))
                .collect(),
        })
        .collect()
}

/// The loops of the tree rooted at `root`, never crossing a nested callable, in source order.
fn loops_under<'t>(cx: &Cx<'_, 't>, root: Node<'t>) -> Vec<Node<'t>> {
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        if n.id() != root.id() && cx.boundaries.contains(&n.kind()) {
            continue;
        }
        if is_loop(n.kind()) {
            found.push(n);
        }
        let mut cursor = n.walk();
        let children: Vec<_> = n.named_children(&mut cursor).collect();
        stack.extend(children);
    }
    found.sort_by_key(|n| n.start_byte());
    found
}

fn facts<'t>(cx: &Cx<'_, 't>, node: Node<'t>) -> LoopFacts {
    let mut comps: Vec<Comp<'t>> = Vec::new();
    let known = match node.child_by_field_name("body") {
        Some(body) => {
            let empty = body.kind() == "empty_statement"
                || (body.kind() == "statement_block" && Cx::last_statement(body).is_none());
            empty || cx.completions(body, &mut comps)
        }
        None => false,
    };
    let start_line = node.start_position().row + 1;
    let geometry = (node.kind() == "do_statement").then(|| cx.parsed.do_geometry_line(&node));
    let head = geometry.flatten().unwrap_or(start_line);
    LoopFacts {
        node: Anchor::of(node),
        start_line,
        end_line: node.end_position().row + 1,
        head,
        body: node.child_by_field_name("body").map(Anchor::of),
        condition_collapsed: geometry == Some(None),
        completions: comps.iter().map(|c| (c.line, Anchor::of(c.node))).collect(),
        completions_known: known,
    }
}
