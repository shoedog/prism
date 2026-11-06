//! The census as reaching-definitions input (#660 S1b-1, DESIGN.md 5.2 A7, 5.3 E15/E16/E18, 8).
//!
//! `completion` runs the S0 walk once for an owner and keeps only what the solver needs: the owner's
//! refusal, the kill-only defs for sites no recorded def covers, and the bases whose site maps to no
//! statement node. The result is small (most owners have none of the three), so the two
//! deduplication seams share it through a per-file memo instead of walking twice.

use super::tables::Tables;
use super::walk::{census_with, Census, Form, Refusal, Site};
use crate::access_path::{AccessPath, ARRAY_FIELD};
use crate::ast::ParsedFile;
use crate::js_write_census::identity::{Comp, Path};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

/// A write the producer did not record: it gens and kills at `line`'s node, never emits an edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KillOnly {
    pub path: AccessPath,
    /// First line of the mapped statement span (E18).
    pub line: usize,
    pub start_byte: usize,
    /// The census form (trace mechanism `kill:<form>`).
    pub form: &'static str,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Completion {
    /// The owner is refused as a whole (escape, parse, a refused target kind).
    pub refusal: Option<Refusal>,
    pub kill_only: Vec<KillOnly>,
    /// Bases with an eager site that maps to no statement node (`site-unmapped`).
    pub refused_bases: BTreeSet<String>,
}

/// `ident`'s path in the producer's representation (`[]` for a subscript).
pub(crate) fn access_path(p: &Path) -> AccessPath {
    AccessPath::with_fields(
        p.base.clone(),
        p.comps
            .iter()
            .map(|c| match c {
                Comp::Name(n) => n.clone(),
                Comp::Index => ARRAY_FIELD.to_string(),
            })
            .collect(),
    )
}

/// A recorded def as the completion sees it: stored path, start byte, and whether it is an alias twin
/// (a twin never kills, so it never covers a census site).
pub(crate) type Recorded<'a> = (&'a AccessPath, usize, bool);

/// Run the walk for `owner` against the recorded defs.
pub(crate) fn completion(
    parsed: &ParsedFile,
    owner: Node<'_>,
    recorded: &[Recorded<'_>],
) -> Completion {
    let mut out = Completion::default();
    let c = census_with(parsed, Tables::shared(), owner, false);
    if let Some(r) = c.refusals.first() {
        out.refusal = Some(r.clone());
        return out;
    }
    let bases: BTreeSet<&str> = recorded.iter().map(|(p, _, _)| p.base.as_str()).collect();
    let mut by_start: BTreeMap<usize, Vec<&AccessPath>> = BTreeMap::new();
    for (path, byte, twin) in recorded {
        if !twin {
            by_start.entry(*byte).or_default().push(path);
        }
    }
    for site in &c.sites {
        // Declarations are the refusal classes of S1d (hoisting, Annex B, dead formals), not writes
        // the producer fails to record. Writes in a deferred region or the owner's own parameter list
        // need the scope rules of S1c (inner bindings: a shadowing parameter is not the owner's
        // binding) and S1d (dead formals); the producer's own nested-write handling stands until then.
        let r = site.region;
        if site.form == Form::Declare
            || r.nested
            || r.instance_field
            || r.own_params
            || !bases.contains(site.path.base.as_str())
        {
            continue;
        }
        // `this` inside a class body (static field, static block) or a nested non-arrow callable is
        // another object: the producer's rule for the same question.
        if site.path.base == "this" && this_is_rebound(parsed, owner, site) {
            continue;
        }
        let path = access_path(&site.path);
        let covered = by_start
            .get(&site.target_start)
            .is_some_and(|stored| stored.iter().any(|p| **p == path));
        if covered {
            continue;
        }
        match node_line(parsed, &c, site) {
            Some(line) => out.kill_only.push(KillOnly {
                path,
                line,
                start_byte: site.target_start,
                form: form_of(parsed, site),
            }),
            None => {
                out.refused_bases.insert(site.path.base.clone());
            }
        }
    }
    out
}

/// Whether `this` at the site is another object than the owner's. Follows S0's child positions: a
/// static block, a field value, and the parameters or body of a non-arrow callable rebind it; a
/// computed key and every decorator (a method's, a parameter's, a class-body one) run in the
/// enclosing scope and keep it.
fn this_is_rebound(parsed: &ParsedFile, owner: Node<'_>, site: &Site) -> bool {
    let Some(node) = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(site.target_start, site.target_start)
    else {
        return false;
    };
    let (mut prev, mut cur, mut decorated) = (node, node.parent(), false);
    while let Some(n) = cur {
        if n.id() == owner.id() {
            return false;
        }
        match n.kind() {
            "decorator" => decorated = true,
            "class_body" | "class_declaration" | "class" | "abstract_class_declaration" => {
                decorated = false
            }
            "class_static_block" => return true,
            "field_definition" | "public_field_definition"
                if n.child_by_field_name("value")
                    .is_some_and(|v| v.id() == prev.id()) =>
            {
                return true
            }
            "function_expression"
            | "function"
            | "function_declaration"
            | "generator_function"
            | "generator_function_declaration"
            | "method_definition" => {
                if decorated {
                    // The decorator belongs to this callable's declaration site, outside the callable.
                    decorated = false;
                } else if ["parameters", "parameter", "body"].iter().any(|f| {
                    n.child_by_field_name(f)
                        .is_some_and(|c| c.id() == prev.id())
                }) {
                    return true;
                }
            }
            _ => {}
        }
        prev = n;
        cur = n.parent();
    }
    false
}

/// Compound statements: their body runs after (and apart from) the header that the span names.
const COMPOUND: [&str; 10] = [
    "if_statement",
    "for_statement",
    "for_in_statement",
    "while_statement",
    "do_statement",
    "switch_statement",
    "try_statement",
    "labeled_statement",
    "statement_block",
    "with_statement",
];

/// E18: the CFG node (its line) that executes the site's write.
/// The innermost retained statement span containing the write by bytes names it, with one exception.
/// The statement universe keeps one statement per line, so a statement can be lost behind another on
/// its line (`log(); a++;`), and a class or enum declaration is no statement at all. When the site
/// sits inside such an un-retained statement and the innermost span is a compound statement (an `if`,
/// a loop, a block: its header runs before the body), that span would place the kill earlier than the
/// defs of its own body: the write maps to the retained statement that starts on the site's line, or
/// is unmapped (the caller refuses the base). A header site (case value, `for` head) has no
/// statement between it and the span and keeps the span's node.
fn node_line(parsed: &ParsedFile, c: &Census, site: &Site) -> Option<usize> {
    let start = site.construct_start.unwrap_or(site.target_start);
    let end = site.moment;
    let on_line = || {
        c.spans
            .iter()
            .any(|(a, _)| parsed.line_for_byte(*a) == site.line)
            .then_some(site.line)
    };
    let contained = c
        .spans
        .iter()
        .filter(|(a, b)| *a <= start && end <= *b)
        .min_by_key(|(a, b)| (b - a, std::cmp::Reverse(*a)));
    let Some(&(a, b)) = contained else {
        return on_line();
    };
    let line = parsed.line_for_byte(a);
    if line == site.line || !lost_statement_under_compound(parsed, site, a, b) {
        return Some(line);
    }
    on_line()
}

/// The span `(a, b)` is a compound statement and an un-retained statement or declaration lies between
/// the site and it.
fn lost_statement_under_compound(parsed: &ParsedFile, site: &Site, a: usize, b: usize) -> bool {
    let root = parsed.tree.root_node();
    let Some(mut n) = root.descendant_for_byte_range(site.target_start, site.target_start) else {
        return false;
    };
    let mut lost = false;
    loop {
        if n.start_byte() == a && n.end_byte() == b {
            return lost && COMPOUND.contains(&n.kind());
        }
        let k = n.kind();
        if k.ends_with("_statement") && k != "statement_block" || k.ends_with("_declaration") {
            lost = true;
        }
        match n.parent() {
            Some(p) => n = p,
            None => return false,
        }
    }
}

/// The census form of a site, as the trace names it.
fn form_of(parsed: &ParsedFile, site: &Site) -> &'static str {
    let Some(node) = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(site.target_start, site.target_start)
    else {
        return "other";
    };
    if in_for_increment(node) {
        return "for-increment";
    }
    match site.form {
        Form::Delete => "delete",
        Form::ReadWrite if site.construct_kind == Some("update_expression") => "update",
        Form::ReadWrite => "augmented",
        _ if site.for_head => "for-head",
        _ if in_pattern(node) => "pattern",
        _ => "assign",
    }
}

fn in_for_increment(node: Node<'_>) -> bool {
    let mut cur = node.parent();
    while let Some(n) = cur {
        if n.kind() == "for_statement" {
            return n.child_by_field_name("increment").is_some_and(|i| {
                i.start_byte() <= node.start_byte() && node.end_byte() <= i.end_byte()
            });
        }
        if n.kind().ends_with("statement") || n.kind().ends_with("declaration") {
            return false;
        }
        cur = n.parent();
    }
    false
}

fn in_pattern(node: Node<'_>) -> bool {
    let mut cur = node.parent();
    while let Some(n) = cur {
        if n.kind().ends_with("_pattern") {
            return true;
        }
        if n.kind().ends_with("statement")
            || n.kind().ends_with("expression") && n.kind() != "parenthesized_expression"
        {
            return false;
        }
        cur = n.parent();
    }
    false
}
