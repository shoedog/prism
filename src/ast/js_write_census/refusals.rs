//! The refusal classes of #660 S1d-1 (DESIGN.md 6.1, 6.1b-6.1e, G6a, G6b): writes and bindings that are
//! not spelled as an assignment target, so neither the producer nor the census records them.
//!
//! `classes` reads one owner once and returns plain data the solver applies as labels:
//! - `eval` (6.1b): a direct `eval` anywhere in the owner, nested callables included;
//! - `names` (6.1 sloppy mapped `arguments`, 6.1e Annex B): bases refused by name, with the mechanism;
//! - `defs` (G6a, G6b): start bytes of recorded defs whose own write targets another binding;
//! - `with_bodies` (6.1c): the byte and line ranges of `with` bodies;
//! - `dead` (6.1d): formals a hoisted function declaration, or a write in the owner's own parameter
//!   list, replaces before the first statement.
//!
//! Nothing here changes an edge set, availability or a counter: every class only moves a label toward
//! `NameOnly`. Detection reuses the existing strictness, eval and arguments helpers and the binding
//! helpers `js_ts_function_scope_binds` / `js_ts_block_statement_binds`; no new strictness detection.
use super::tables::Tables;
use super::walk::{is_callable, Form, Site};
use crate::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

/// Trace mechanisms (DESIGN.md 10).
pub(crate) const EVAL: &str = "eval";
pub(crate) const ARGUMENTS: &str = "arguments";
pub(crate) const ANNEXB: &str = "annexb";
pub(crate) const CATCH_VAR: &str = "catch-var";
pub(crate) const IMMUTABLE_NAME: &str = "immutable-name";
pub(crate) const WITH: &str = "with";
pub(crate) const DEAD_FORMAL: &str = "dead-formal";

/// A formal that never reaches the first statement: the defs of `name` that start inside `range` are
/// killed at `line` (the declaration, or the default that writes the formal).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Dead {
    pub name: String,
    pub range: (usize, usize),
    pub line: usize,
}

/// A `with` body: its bytes, and its lines for a use whose bytes are unknown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WithBody {
    pub bytes: (usize, usize),
    pub lines: (usize, usize),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Classes {
    pub eval: bool,
    /// Bases refused by name (`arguments`, `annexb`), the first mechanism naming a base wins.
    pub names: BTreeMap<String, &'static str>,
    /// Start bytes of recorded defs refused (`catch-var`, `immutable-name`).
    pub defs: BTreeMap<usize, &'static str>,
    pub with_bodies: Vec<WithBody>,
    pub dead: Vec<Dead>,
}

impl Classes {
    pub(crate) fn is_empty(&self) -> bool {
        !self.eval
            && self.names.is_empty()
            && self.defs.is_empty()
            && self.with_bodies.is_empty()
            && self.dead.is_empty()
    }
}

/// What the single walk of the owner found.
#[derive(Default)]
struct Found {
    /// Names of function and class expressions inside the owner or enclosing it.
    inner_names: BTreeSet<String>,
    /// Declarations (name, line) reachable from the body statement list through labels only.
    hoisted: Vec<(String, usize)>,
    /// Block-level function declaration names (outside nested callables).
    annexb: BTreeSet<String>,
}

pub(crate) fn classes(parsed: &ParsedFile, owner: Node<'_>, sites: &[Site]) -> Classes {
    let t = Tables::shared();
    let mut out = Classes {
        eval: parsed.js_ts_direct_eval_anywhere(&owner),
        ..Classes::default()
    };
    let params = parsed.parameter_binding_region(&owner);
    let mut formals = Vec::new();
    if let Some(p) = params {
        parsed.js_ts_pattern_bound_names(p, &mut formals);
    }
    if parsed.js_ts_mapped_arguments_possible(&owner) {
        for n in &formals {
            out.names.entry(n.clone()).or_insert(ARGUMENTS);
        }
    }
    let sloppy = !parsed.js_ts_callable_is_strict(&owner);
    let found = walk(parsed, t, owner, &mut out);
    if sloppy {
        // B.3.3.1 skips a name that is a formal parameter: its block function stays block-scoped and
        // the outer binding is untouched, so a formal is not refused here (a hoisted declaration of a
        // formal's name is the dead-formal rule's).
        for n in found.annexb.into_iter().filter(|n| !formals.contains(n)) {
            out.names.entry(n).or_insert(ANNEXB);
        }
    }
    dead_formals(&found.hoisted, params, sites, &mut out);
    if !found.inner_names.is_empty() {
        for site in sites {
            if site.form != Form::Declare
                && site.path.comps.is_empty()
                && found.inner_names.contains(&site.path.base)
            {
                let at = parsed
                    .tree
                    .root_node()
                    .descendant_for_byte_range(site.target_start, site.target_end);
                if at.is_some_and(|n| writes_inner_name(parsed, t, n, &site.path.base)) {
                    out.defs.entry(site.target_start).or_insert(IMMUTABLE_NAME);
                }
            }
        }
    }
    out
}

/// Whether a node of the owner's subtree is in a lower position than the body statement list.
/// The owner's body, and a label chain directly under it, hold the top-level statements.
fn top_level_parent(owner: Node<'_>, parent: Node<'_>, parent_lower: bool) -> bool {
    !parent_lower
        && (parent.id() == owner.id()
            || owner
                .child_by_field_name("body")
                .is_some_and(|b| b.id() == parent.id())
            || parent.kind() == "labeled_statement")
}

/// One pass over the owner's subtree (an explicit stack: generated code nests deeply).
fn walk(parsed: &ParsedFile, t: &Tables, owner: Node<'_>, out: &mut Classes) -> Found {
    let mut found = Found::default();
    let mut up = owner.parent();
    while let Some(n) = up {
        note_inner_name(parsed, n, &mut found.inner_names);
        up = n.parent();
    }
    // (node, in a lower position, inside a nested callable)
    let mut stack = vec![(owner, false, false)];
    while let Some((node, lower, nested)) = stack.pop() {
        let is_owner = node.id() == owner.id();
        match node.kind() {
            "function_declaration" | "generator_function_declaration" if !nested && !is_owner => {
                if let Some(name) = node.child_by_field_name("name") {
                    let name = parsed.node_text(&name).to_string();
                    if lower {
                        found.annexb.insert(name);
                    } else {
                        let line = parsed.line_for_byte(node.start_byte());
                        found.hoisted.push((name, line));
                    }
                }
            }
            "with_statement" => {
                if let Some(body) = node.child_by_field_name("body") {
                    out.with_bodies.push(WithBody {
                        bytes: (body.start_byte(), body.end_byte()),
                        lines: (
                            parsed.line_for_byte(body.start_byte()),
                            parsed.line_for_byte(body.end_byte().saturating_sub(1)),
                        ),
                    });
                }
            }
            "catch_clause" => catch_var_defs(parsed, t, node, out),
            _ => note_inner_name(parsed, node, &mut found.inner_names),
        }
        let child_nested = nested
            || (!is_owner && is_callable(t, node.kind()))
            || node.kind() == "class_static_block";
        let child_lower = lower || !top_level_parent(owner, node, lower);
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            stack.push((child, child_lower, child_nested));
        }
    }
    found
}

/// A named function expression or class expression: its name is bound inside it only.
fn note_inner_name(parsed: &ParsedFile, node: Node<'_>, names: &mut BTreeSet<String>) {
    if is_named_expression(node) {
        if let Some(n) = node.child_by_field_name("name") {
            names.insert(parsed.node_text(&n).to_string());
        }
    }
}

fn is_named_expression(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "function_expression" | "function" | "generator_function" | "class"
    ) && node.child_by_field_name("name").is_some()
}

/// G6a (Annex B.3.5): a `var` of the catch parameter's name inside `catch (a)` writes the parameter.
/// The clause's body is searched outside nested callables, whose `var` is their own.
fn catch_var_defs(parsed: &ParsedFile, t: &Tables, clause: Node<'_>, out: &mut Classes) {
    let (Some(param), Some(body)) = (
        clause.child_by_field_name("parameter"),
        clause.child_by_field_name("body"),
    ) else {
        return;
    };
    if param.kind() != "identifier" {
        return; // a destructured parameter with a same-named `var` is an early error
    }
    let name = parsed.node_text(&param);
    let mut stack = vec![body];
    while let Some(node) = stack.pop() {
        let pattern = match node.kind() {
            "variable_declarator" => node.child_by_field_name("name"),
            "for_in_statement"
                if node
                    .child_by_field_name("kind")
                    .is_some_and(|k| parsed.node_text(&k) == "var") =>
            {
                node.child_by_field_name("left")
            }
            _ => None,
        };
        if let Some(p) = pattern {
            let mut inner = vec![p];
            while let Some(n) = inner.pop() {
                if matches!(
                    n.kind(),
                    "identifier" | "shorthand_property_identifier_pattern"
                ) && parsed.node_text(&n) == name
                {
                    out.defs.entry(n.start_byte()).or_insert(CATCH_VAR);
                }
                let mut c = n.walk();
                inner.extend(n.named_children(&mut c));
            }
        }
        let mut c = node.walk();
        for child in node.named_children(&mut c) {
            if !is_callable(t, child.kind()) && child.kind() != "class_static_block" {
                stack.push(child);
            }
        }
    }
}

/// 6.1d: a formal named like a hoisted declaration, or written by a later default in the own list.
fn dead_formals(
    hoisted: &[(String, usize)],
    params: Option<Node<'_>>,
    sites: &[Site],
    out: &mut Classes,
) {
    let own = |s: &&Site| s.region.own_params && !s.region.nested && !s.region.instance_field;
    let formals: Vec<&Site> = sites
        .iter()
        .filter(own)
        .filter(|s| s.form == Form::Declare && s.decl == "param")
        .collect();
    if formals.is_empty() {
        return;
    }
    for w in sites.iter().filter(own) {
        if w.form == Form::Declare || !w.path.comps.is_empty() {
            continue;
        }
        for f in formals.iter().filter(|f| f.path.base == w.path.base) {
            if f.target_start < w.target_start {
                out.dead.push(Dead {
                    name: f.path.base.clone(),
                    range: (f.target_start, f.target_end),
                    line: w.line,
                });
            }
        }
    }
    let Some(params) = params else { return };
    let range = (params.start_byte(), params.end_byte());
    for (name, line) in hoisted {
        if formals.iter().any(|f| &f.path.base == name) {
            out.dead.push(Dead {
                name: name.clone(),
                range,
                line: *line,
            });
        }
    }
}

/// G6b: the write at `node` names the own name of a function or class expression enclosing it, and no
/// declaration of that name scopes the write between them (a declaration in the expression's own
/// parameters or body included): it writes nothing in sloppy code and throws in strict code.
fn writes_inner_name(parsed: &ParsedFile, t: &Tables, node: Node<'_>, name: &str) -> bool {
    let named = |n: Node<'_>| {
        n.child_by_field_name("name")
            .is_some_and(|id| parsed.node_text(&id) == name)
    };
    let mut cur = node;
    while let Some(p) = cur.parent() {
        match p.kind() {
            "function_expression" | "function" | "generator_function" if named(p) => {
                return !callable_declares(parsed, p, name);
            }
            "class" if named(p) => return true,
            _ => {
                if is_callable(t, p.kind()) {
                    if callable_declares(parsed, p, name) {
                        return false;
                    }
                } else if block_declares(parsed, p, name) {
                    return false;
                }
            }
        }
        cur = p;
    }
    false
}

/// The callable's parameters bind `name`, or its function scope does (`var` anywhere outside nested
/// callables, body-level lexical, function and class declarations).
fn callable_declares(parsed: &ParsedFile, callable: Node<'_>, name: &str) -> bool {
    let mut names = Vec::new();
    if let Some(p) = parsed.parameter_binding_region(&callable) {
        parsed.js_ts_pattern_bound_names(p, &mut names);
    }
    names.iter().any(|n| n == name)
        || callable
            .child_by_field_name("body")
            .filter(|b| b.kind() == "statement_block")
            .is_some_and(|b| {
                parsed.js_ts_function_scope_binds(
                    b,
                    &parsed.language.callable_boundary_node_types(),
                    name,
                )
            })
}

/// A block-like node declares `name` lexically (`let`, `const`, `class`, a block function), a `for`
/// head does, or a `catch` parameter does.
fn block_declares(parsed: &ParsedFile, node: Node<'_>, name: &str) -> bool {
    match node.kind() {
        "statement_block" | "switch_body" => {
            let mut c = node.walk();
            let declares = node
                .named_children(&mut c)
                .any(|s| statement_binds(parsed, s, name));
            declares
        }
        "for_statement" => node
            .child_by_field_name("initializer")
            .is_some_and(|i| statement_binds(parsed, i, name)),
        "for_in_statement" => {
            node.child_by_field_name("kind")
                .is_some_and(|k| parsed.node_text(&k) != "var")
                && node.child_by_field_name("left").is_some_and(|l| {
                    let mut names = Vec::new();
                    parsed.js_ts_pattern_bound_names(l, &mut names);
                    names.iter().any(|n| n == name)
                })
        }
        "catch_clause" => node.child_by_field_name("parameter").is_some_and(|p| {
            let mut names = Vec::new();
            parsed.js_ts_pattern_bound_names(p, &mut names);
            names.iter().any(|n| n == name)
        }),
        _ => false,
    }
}

/// A statement declares `name` in its enclosing block: `let`/`const` (not `var`, which the function
/// scope owns), a function, class or enum declaration, through labels and `switch` cases.
fn statement_binds(parsed: &ParsedFile, stmt: Node<'_>, name: &str) -> bool {
    match stmt.kind() {
        "labeled_statement" => stmt
            .child_by_field_name("body")
            .is_some_and(|b| statement_binds(parsed, b, name)),
        "lexical_declaration" => {
            let mut c = stmt.walk();
            let binds = stmt.named_children(&mut c).any(|d| {
                d.kind() == "variable_declarator"
                    && d.child_by_field_name("name").is_some_and(|n| {
                        let mut names = Vec::new();
                        parsed.js_ts_pattern_bound_names(n, &mut names);
                        names.iter().any(|x| x == name)
                    })
            });
            binds
        }
        "function_declaration"
        | "generator_function_declaration"
        | "class_declaration"
        | "abstract_class_declaration"
        | "enum_declaration" => stmt
            .child_by_field_name("name")
            .is_some_and(|n| parsed.node_text(&n) == name),
        "switch_case" | "switch_default" => {
            let mut c = stmt.walk();
            let binds = stmt
                .named_children(&mut c)
                .any(|s| statement_binds(parsed, s, name));
            binds
        }
        _ => false,
    }
}
