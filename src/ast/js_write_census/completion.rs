//! The census as reaching-definitions input (#660 S1b-1, DESIGN.md 5.2 A7, 5.3 E15/E16/E18, 8).
//!
//! `completion` runs the S0 walk once for an owner and keeps only what the solver needs: the owner's
//! refusal, the kill-only defs for sites no recorded def covers, and the bases whose site maps to no
//! statement node. The result is small (most owners have none of the three), so the two
//! deduplication seams share it through a per-file memo instead of walking twice.

use super::captured::{self, Captured, Shadows};
use super::counting::{self, Counts};
use super::tables::Tables;
use super::walk::{census_mapped, Census, Form, Mapping, Refusal, Site};
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
    /// The writes D3 counts per node (#660 S1b-2).
    pub counting: Counts,
    /// The writes in nested callables and instance field initializers, for the bases the owner records
    /// (#660 S1c, D2-c).
    pub captured: Vec<Captured>,
    /// (target start byte, path) of every recorded def the census's precise identity for the same site
    /// equals (`covered` below): the recorded paths that are complete storage identities (#674).
    pub covered: BTreeSet<(usize, AccessPath)>,
    /// The refusal classes of #660 S1d-1 (eval, `arguments`, `with`, dead formals, Annex B, G6a, G6b).
    pub classes: super::refusals::Classes,
    /// The single-step body alias candidates and their syntactic verdicts (#783 option A, inert).
    /// `None` when the owner has no bare-identifier declarator; shared (not deep-copied) by clones.
    pub alias: Option<std::rc::Rc<Vec<super::alias_cert::Candidate>>>,
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
    let bases: BTreeSet<&str> = recorded.iter().map(|(p, _, _)| p.base.as_str()).collect();
    // D3 positions the writes of the owner's recorded bases (the only edges it can refuse).
    let mut c = census_mapped(parsed, Tables::shared(), owner, Mapping::Bases(&bases));
    if let Some(r) = c.refusals.first() {
        out.refusal = Some(r.clone());
        return out;
    }
    out.classes = super::refusals::classes(parsed, owner, &c.sites, &bases);
    out.alias = Some(super::alias_cert::candidates(
        parsed,
        owner,
        &c.sites,
        &out.classes,
    ))
    .filter(|v| !v.is_empty())
    .map(std::rc::Rc::new);
    let shadows = Shadows::new(parsed, &c.sites, &bases);
    let mut by_start: BTreeMap<usize, Vec<&AccessPath>> = BTreeMap::new();
    for (path, byte, twin) in recorded {
        if !twin {
            by_start.entry(*byte).or_default().push(path);
        }
    }
    for site in &c.sites {
        // E18 placement, once per site: the kill-only def and D3's counting write share it.
        let placed = bases
            .contains(site.path.base.as_str())
            .then(|| node_line(parsed, &c, site));
        // `this` inside a class body (static field, static block) or a nested non-arrow callable is
        // another object: not a write of the owner's `this`.
        let rebound = site.path.base == "this" && this_is_rebound(parsed, owner, site);
        // Declarations are the refusal classes of S1d (hoisting, Annex B, dead formals), not writes
        // the producer fails to record. A write in a deferred region is no kill-only def either: its
        // time is unknown, so D2-c (S1c, `captured`) refuses the edges it may reach instead, and the
        // producer's own nested-write handling stands. The owner's own parameter list is S1d's (dead formals).
        let r = site.region;
        let candidate = !(site.form == Form::Declare
            || r.nested
            || r.instance_field
            || r.own_params
            || !bases.contains(site.path.base.as_str())
            || rebound);
        let path = access_path(&site.path);
        if captured::is_region_write(site) && !rebound && bases.contains(site.path.base.as_str()) {
            out.captured.push(Captured {
                path: path.clone(),
                start: site.target_start,
                shadow: shadows.innermost(&site.path.base, site.target_start),
            });
        }
        let covered = candidate
            && by_start
                .get(&site.target_start)
                .is_some_and(|stored| stored.iter().any(|p| **p == path));
        // #674: a declaration of a bare name (`let`/`const`/`var`, a destructured name) is complete by
        // construction: it cannot be truncated, and `this` cannot be declared. Formals (`own_params`) and
        // destructured or loop-head names stay out: `BindingFacts` does not tell a block-scoped pattern name
        // from the outer binding, so a shadowing `{ const { o } = src() }` would kill the outer `o.x`.
        let declared_bare = site.form == Form::Declare
            && !(r.nested || r.instance_field || r.own_params || rebound)
            && bases.contains(site.path.base.as_str())
            && path.fields.is_empty()
            && is_plain_declarator(parsed, site.target_start)
            && by_start
                .get(&site.target_start)
                .is_some_and(|stored| stored.iter().any(|p| **p == path));
        if covered || declared_bare {
            out.covered.insert((site.target_start, path.clone()));
        }
        // The line RD is handed for this write (#696): the E18 statement line of a kill-only def,
        // the target's own line of a recorded one. D3 keys the write by RD's node for that line.
        let kill_line = (candidate && !covered).then(|| placed.flatten()).flatten();
        if let Some(node) = placed {
            out.counting.note_target(site);
            if counting::counts(site) && !rebound {
                match node {
                    Some(_) => {
                        let probe = binding_probe(parsed, site);
                        let line = kill_line.unwrap_or(site.line);
                        out.counting.push(site, path.clone(), line, probe)
                    }
                    None => {
                        out.counting.unplaced.insert(site.path.base.clone());
                        if !site.path.comps.is_empty() {
                            let probe = binding_probe(parsed, site);
                            out.counting.unplaced_sites.push((
                                site.path.base.clone(),
                                probe,
                                site.line,
                            ));
                        }
                    }
                }
            }
        }
        if !candidate || covered {
            continue;
        }
        match kill_line {
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
    if !out.counting.is_empty() {
        out.counting.spans = std::mem::take(&mut c.spans);
    }
    out
}

/// The byte at which a counting write's binding is looked up. A declaration's name is not visible
/// at its own bytes (a `let` is visible from the end of its declaration, a `for`-in head's from the
/// end of its left side), so it is probed there; any other write at its target.
fn binding_probe(parsed: &ParsedFile, site: &Site) -> usize {
    if site.form != Form::Declare {
        return site.target_start;
    }
    let mut cur = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(site.target_start, site.target_start);
    while let Some(n) = cur {
        match n.kind() {
            "lexical_declaration" | "variable_declaration" => return n.end_byte(),
            "for_in_statement" => {
                return n
                    .child_by_field_name("left")
                    .map_or(n.end_byte(), |l| l.end_byte())
            }
            _ => {}
        }
        cur = n.parent();
    }
    site.target_start
}

/// Whether `this` at the site is another object than the owner's. Follows S0's child positions: a
/// static block, a field value, and the parameters or body of a non-arrow callable rebind it; a
/// computed key and every decorator (a method's, a parameter's, a class-body one) run in the
/// enclosing scope and keep it.
fn this_is_rebound(parsed: &ParsedFile, owner: Node<'_>, site: &Site) -> bool {
    this_is_rebound_at(parsed, owner, site.target_start)
}

/// `this_is_rebound` for a target that starts at `byte` (a recorded def's start byte, #674).
pub(crate) fn this_is_rebound_at(parsed: &ParsedFile, owner: Node<'_>, byte: usize) -> bool {
    let Some(node) = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(byte, byte)
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
    place(
        parsed,
        &c.spans,
        site.construct_start.unwrap_or(site.target_start),
        site.moment,
        site.target_start,
        site.line,
    )
}

/// E18 for any position: the node of the bytes `start..end`, given the statement `spans`, the byte
/// `at` the position's syntax starts at and its `line`. One rule for writes (`node_line`) and for the
/// reads D3 orders against them (#660 S1b-2), so the two cannot drift.
pub(crate) fn place(
    parsed: &ParsedFile,
    spans: &[(usize, usize)],
    start: usize,
    end: usize,
    at: usize,
    line: usize,
) -> Option<usize> {
    // Only a statement that starts at or before the position can execute it: one that starts later on
    // the line (a loop body after a header write) is a node a route may bypass.
    let on_line = || {
        spans
            .iter()
            .any(|(a, _)| *a <= start && parsed.line_for_byte(*a) == line)
            .then_some(line)
    };
    let contained = spans
        .iter()
        .filter(|(a, b)| *a <= start && end <= *b)
        .min_by_key(|(a, b)| (b - a, std::cmp::Reverse(*a)));
    let Some(&(a, b)) = contained else {
        return on_line();
    };
    let span_line = parsed.line_for_byte(a);
    if span_line == line || !lost_statement_under_compound(parsed, at, a, b) {
        return Some(span_line);
    }
    on_line()
}

/// The span `(a, b)` is a compound statement and an un-retained statement or declaration lies between
/// the site and it.
fn lost_statement_under_compound(parsed: &ParsedFile, at: usize, a: usize, b: usize) -> bool {
    let root = parsed.tree.root_node();
    let Some(mut n) = root.descendant_for_byte_range(at, at) else {
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

/// Whether the identifier at `byte` is the whole name of a `variable_declarator` (`let o = ..`, `var o`).
fn is_plain_declarator(parsed: &ParsedFile, byte: usize) -> bool {
    parsed
        .tree
        .root_node()
        .descendant_for_byte_range(byte, byte)
        .and_then(|n| n.parent().map(|p| (n, p)))
        .is_some_and(|(n, p)| {
            p.kind() == "variable_declarator"
                && p.child_by_field_name("name")
                    .is_some_and(|name| name.id() == n.id())
        })
}
