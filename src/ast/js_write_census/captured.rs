//! Writes in deferred regions, and the declarations that scope them (#660 S1c, DESIGN.md 6.2 item 1).
//!
//! A write in a nested callable or an instance field initializer (a region) is ignored by D2-c only when
//! a declaration of its base provably scopes it, so that it writes a binding no outer edge names. Which
//! declarations count is the design's closed list, read from the `DECL:*` slots the walk already
//! classified (`Site::decl`):
//! - `param`: the parameter list that holds it and the callable's whole body. A write in a parameter list
//!   is scoped only by a parameter of that same list (a body `var` or function never binds it: a10, a11);
//! - `declarator`: a `var` scopes the body of its nearest callable; `let`/`const`/`using` the nearest
//!   block, `for` statement, `switch` body or static block;
//! - `fn-hoist` and `class`: the block (or `switch` body) they are declared in, in both modes (a14);
//! - `fn-inner` (a named function expression's own name): that function's body;
//! - `catch`: the clause.
//!
//! Every other declaration (`class-inner`, imports, `enum`, `namespace`) and any shape not listed
//! proves nothing, and the write counts as outer. The scope is a byte range; a write is scoped by every
//! declaration whose range holds it, and the caller decides which of them lie inside a region.
use super::tables::Tables;
use super::walk::{is_callable, Form, Site};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

/// A declaration inside a region: where it is and the byte range it scopes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Shadow {
    pub start: usize,
    pub scope: (usize, usize),
}

/// A write in a region: its path, where it is, and the declarations scoping it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Captured {
    pub path: AccessPath,
    pub start: usize,
    pub shadows: Vec<Shadow>,
}

/// Whether a site is a write D2-c judges: a write (not a declaration) in a nested callable or an
/// instance field initializer.
pub(crate) fn is_region_write(site: &Site) -> bool {
    site.form != Form::Declare && (site.region.nested || site.region.instance_field)
}

/// The scoping declarations of an owner's regions, by base.
pub(crate) struct Shadows {
    by_base: BTreeMap<String, Vec<Shadow>>,
}

impl Shadows {
    /// The region declarations among `sites` for the given bases.
    pub(crate) fn new(parsed: &ParsedFile, sites: &[Site], bases: &BTreeSet<&str>) -> Self {
        let t = Tables::shared();
        let root = parsed.tree.root_node();
        let mut by_base: BTreeMap<String, Vec<Shadow>> = BTreeMap::new();
        for site in sites {
            let in_region = site.region.nested || site.region.instance_field;
            if site.form != Form::Declare || !in_region || !bases.contains(site.path.base.as_str())
            {
                continue;
            }
            let Some(node) = root.descendant_for_byte_range(site.target_start, site.target_end)
            else {
                continue;
            };
            if let Some(scope) = scope_of(t, &parsed.source, &site.decl, node) {
                by_base
                    .entry(site.path.base.clone())
                    .or_default()
                    .push(Shadow {
                        start: site.target_start,
                        scope,
                    });
            }
        }
        Self { by_base }
    }

    /// The declarations of `base` whose scope holds `at`.
    pub(crate) fn scoping(&self, base: &str, at: usize) -> Vec<Shadow> {
        self.by_base.get(base).map_or_else(Vec::new, |all| {
            all.iter()
                .filter(|d| d.scope.0 <= at && at < d.scope.1)
                .copied()
                .collect()
        })
    }
}

fn range(n: Node<'_>) -> (usize, usize) {
    (n.start_byte(), n.end_byte())
}

/// The byte range a declaration of kind `decl` at `node` scopes, `None` when the design proves nothing.
fn scope_of(t: &Tables, src: &str, decl: &str, node: Node<'_>) -> Option<(usize, usize)> {
    match decl {
        "param" => param_scope(t, node),
        "declarator" => declarator_scope(t, src, node),
        "fn-hoist" | "class" => block_scope(node.parent()?),
        "fn-inner" => {
            let function = node.parent()?;
            is_callable(t, function.kind()).then_some(())?;
            function.child_by_field_name("body").map(range)
        }
        "catch" => ancestor(node, |n| n.kind() == "catch_clause").map(range),
        _ => None,
    }
}

/// The nearest ancestor of `node` (itself excluded) that satisfies `pred`.
fn ancestor<'t>(node: Node<'t>, pred: impl Fn(Node<'t>) -> bool) -> Option<Node<'t>> {
    let mut cur = node.parent();
    while let Some(n) = cur {
        if pred(n) {
            return Some(n);
        }
        cur = n.parent();
    }
    None
}

/// A parameter: its own list and the callable's body (the callable's end), nothing before the list.
fn param_scope(t: &Tables, node: Node<'_>) -> Option<(usize, usize)> {
    let (mut prev, mut cur) = (node, node.parent());
    while let Some(n) = cur {
        if n.kind() == "formal_parameters" {
            let callable = n.parent().filter(|c| is_callable(t, c.kind()))?;
            return Some((n.start_byte(), callable.end_byte()));
        }
        if is_callable(t, n.kind()) {
            // A lone arrow parameter is the callable's own `parameter` child.
            let lone = n
                .child_by_field_name("parameter")
                .is_some_and(|p| p.id() == prev.id());
            return lone.then(|| (prev.start_byte(), n.end_byte()));
        }
        prev = n;
        cur = n.parent();
    }
    None
}

/// The scope of a block-like container holding the declaration statement `decl` (`let`, `class`,
/// a function declaration): a block, a `switch` body, a `for` statement or a static block.
fn block_scope(decl: Node<'_>) -> Option<(usize, usize)> {
    let mut at = decl;
    // A labelled declaration is scoped like the statement it labels.
    while let Some(p) = at.parent() {
        if p.kind() == "labeled_statement" {
            at = p;
        } else {
            break;
        }
    }
    let parent = at.parent()?;
    match parent.kind() {
        "statement_block" | "class_static_block" | "for_statement" | "for_in_statement" => {
            Some(range(parent))
        }
        "switch_case" | "switch_default" => parent.parent().map(range),
        _ => None,
    }
}

fn declarator_scope(t: &Tables, src: &str, node: Node<'_>) -> Option<(usize, usize)> {
    let mut cur = Some(node);
    while let Some(n) = cur {
        match n.kind() {
            "variable_declarator" => {
                let declaration = n.parent()?;
                return if declaration.kind() == "variable_declaration" {
                    var_scope(t, declaration)
                } else {
                    lexical_scope(declaration)
                };
            }
            // `for (var a of ..)` / `for (let a of ..)`: the head binds like a declaration.
            "for_in_statement" => {
                let kind = n.child_by_field_name("kind")?;
                return if &src[kind.start_byte()..kind.end_byte()] == "var" {
                    var_scope(t, n)
                } else {
                    Some(range(n))
                };
            }
            _ => cur = n.parent(),
        }
    }
    None
}

fn var_scope(t: &Tables, at: Node<'_>) -> Option<(usize, usize)> {
    let owner = ancestor(at, |n| {
        is_callable(t, n.kind()) || n.kind() == "class_static_block"
    })?;
    owner.child_by_field_name("body").map(range)
}

fn lexical_scope(declaration: Node<'_>) -> Option<(usize, usize)> {
    let container = ancestor(declaration, |n| {
        matches!(
            n.kind(),
            "statement_block" | "switch_body" | "for_statement" | "for_in_statement"
        ) || n.kind() == "class_static_block"
    })?;
    Some(range(container))
}
