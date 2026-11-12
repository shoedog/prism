//! #783 option A, stage 1: the syntactic admission of a single-step body alias (design `DESIGN-astra.md`
//! 2.6 items 1-8), from the census's complete write and declaration inventory.
//!
//! A candidate is a `const|let|var u = o` declarator whose value is one bare identifier (alias formation takes
//! no wrapper). It is admitted when `u` and `o` each have exactly one binding occurrence in the owner subtree
//! (every census site with no path components counts: declarations, assignments, updates, destructuring and
//! loop targets, parameters and nested callables included), `o` is an ordinary formal or an initialized local
//! whose initializer, after peeling parentheses, `as`, `!`, `satisfies` and `<T>x`, is neither an identifier
//! nor `this` (not another alias step), the owner has no dynamic-binding hazard (`eval`, `with`, mapped
//! `arguments`, dead formals, an Annex B or refused binding at the alias) and the declarator is not in a
//! repeated initialization context or a deferred region. `eval` and `with` refuse every candidate of the
//! owner; the other hazards refuse by name. Everything else
//! is refused with a reason, never defaulted to admitted. Stage 2 (`cpg::reaching::alias_cert`) adds the
//! positive declaration identity, which needs `BindingFacts`.
//!
//! Inert: nothing here moves a label, an edge or a counter. An owner with no bare-identifier declarator takes
//! the presence test (a scan of the sites; no allocation) and returns an empty `Vec`.
use super::refusals::Classes;
use super::schema::WRAPPERS;
use super::walk::{wrap_operand, Form, Site};
use crate::ast::ParsedFile;
use tree_sitter::Node;

/// Why a candidate is refused (the trace's reason column).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reason {
    /// The alias declarator is in a nested callable, a parameter list or an instance field.
    DeferredRegion,
    /// `eval`, `with`, sloppy mapped `arguments`, a dead formal or a refused binding touches `u` or `o`.
    Dynamic,
    /// `const u = u`.
    SelfReference,
    /// The declarator is inside a loop: its alias is formed once per iteration.
    Loop,
    /// `u` is written again (`u = p`, `u++`, a destructuring or loop target, a nested write).
    AliasAssigned,
    /// Another declaration of the spelling `u` exists in the owner subtree (a shadow, a parameter, a `var`).
    AliasRedeclared,
    /// `o` is written as a whole again anywhere in the owner subtree.
    TargetRebound,
    /// Another declaration of the spelling `o` exists in the owner subtree.
    TargetRedeclared,
    /// `o` has no declaration in the owner (a free, outer or global name).
    TargetUnresolved,
    /// `o` is declared by a form A does not admit (a pattern, a default or rest formal, a catch, a function).
    TargetKind,
    /// `o` is itself an alias of an identifier (after peeling wrappers): a chain.
    Chain,
    /// `o` is itself an alias of `this` (after peeling wrappers).
    ThisAlias,
    /// `o` is declared after the alias.
    TargetAfterAlias,
    /// Stage 2: the alias's own def is missing or its binding is not an explicit declaration.
    AliasIdentity,
    /// Stage 2: `o` at the alias does not bind the declaration found, or that is not an explicit one.
    TargetIdentity,
}

impl Reason {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Reason::DeferredRegion => "deferred-region",
            Reason::Dynamic => "dynamic",
            Reason::SelfReference => "self-reference",
            Reason::Loop => "loop",
            Reason::AliasAssigned => "alias-assigned",
            Reason::AliasRedeclared => "alias-redeclared",
            Reason::TargetRebound => "target-rebound",
            Reason::TargetRedeclared => "target-redeclared",
            Reason::TargetUnresolved => "target-unresolved",
            Reason::TargetKind => "target-kind",
            Reason::Chain => "chain",
            Reason::ThisAlias => "this-alias",
            Reason::TargetAfterAlias => "target-after-alias",
            Reason::AliasIdentity => "alias-identity",
            Reason::TargetIdentity => "target-identity",
        }
    }
}

/// One `const|let|var u = o` declarator and the verdict of the syntactic stage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Candidate {
    pub alias: String,
    pub target: String,
    /// Start byte of `u` at the declarator.
    pub alias_start: usize,
    /// End byte of the declarator: a write is after the alias when it starts at or beyond it.
    pub alias_end: usize,
    /// Start byte of the `o` read at the declarator.
    pub rhs_start: usize,
    /// Start byte of `o`'s one declaration site, when it has exactly one.
    pub target_start: Option<usize>,
    pub refusal: Option<Reason>,
}

/// The candidates of one owner (`sites` is the census of the whole owner subtree, `classes` its refusal
/// classes). Empty, with no allocation, when no declarator has a bare identifier value.
pub(crate) fn candidates(
    parsed: &ParsedFile,
    owner: Node<'_>,
    sites: &[Site],
    classes: &Classes,
) -> Vec<Candidate> {
    let mut out = Vec::new();
    for site in sites {
        let plain = site.form == Form::Declare
            && site.initialized
            && site.path.comps.is_empty()
            && site.decl == "declarator";
        if !plain {
            continue;
        }
        let Some((declarator, rhs)) = bare_declarator(parsed, site) else {
            continue;
        };
        out.push(judge(parsed, owner, sites, classes, site, declarator, rhs));
    }
    out
}

/// The declarator and its bare-identifier value when `site` names `u` in `u = o` (type annotation allowed).
fn bare_declarator<'t>(parsed: &'t ParsedFile, site: &Site) -> Option<(Node<'t>, Node<'t>)> {
    let id = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(site.target_start, site.target_end)?;
    let declarator = id.parent()?;
    let named = |n: Option<Node<'_>>| n.map(|n| n.id());
    if id.kind() != "identifier"
        || id.start_byte() != site.target_start
        || declarator.kind() != "variable_declarator"
        || named(declarator.child_by_field_name("name")) != Some(id.id())
    {
        return None;
    }
    let value = declarator.child_by_field_name("value")?;
    (value.kind() == "identifier").then_some((declarator, value))
}

/// A formal that is one plain identifier: no default, rest, pattern or other wrapper.
fn plain_formal(id: Node<'_>) -> bool {
    let Some(p) = id.parent() else { return false };
    match p.kind() {
        "formal_parameters" | "arrow_function" => id.kind() == "identifier",
        "required_parameter" | "optional_parameter" => {
            id.kind() == "identifier" && p.child_by_field_name("value").is_none()
        }
        _ => false,
    }
}

/// The census sites that bind or rebind `name` as a whole (no path components), except the one at `skip`.
fn other_bindings<'a>(
    sites: &'a [Site],
    name: &'a str,
    skip: usize,
) -> impl Iterator<Item = &'a Site> {
    sites
        .iter()
        .filter(move |s| s.path.base == name && s.path.comps.is_empty() && s.target_start != skip)
}

fn judge(
    parsed: &ParsedFile,
    owner: Node<'_>,
    sites: &[Site],
    classes: &Classes,
    site: &Site,
    declarator: Node<'_>,
    rhs: Node<'_>,
) -> Candidate {
    let (u, o) = (
        site.path.base.as_str(),
        &parsed.source[rhs.start_byte()..rhs.end_byte()],
    );
    let mut cand = Candidate {
        alias: u.to_string(),
        target: o.to_string(),
        alias_start: site.target_start,
        alias_end: declarator.end_byte(),
        rhs_start: rhs.start_byte(),
        target_start: None,
        refusal: None,
    };
    cand.refusal = refusal(parsed, owner, sites, classes, site, declarator, &mut cand);
    cand
}

fn refusal(
    parsed: &ParsedFile,
    owner: Node<'_>,
    sites: &[Site],
    classes: &Classes,
    site: &Site,
    declarator: Node<'_>,
    cand: &mut Candidate,
) -> Option<Reason> {
    let (u, o) = (cand.alias.as_str(), cand.target.as_str());
    let r = site.region;
    if r.nested || r.instance_field || r.own_params {
        return Some(Reason::DeferredRegion);
    }
    let named = |n: &str| classes.names.contains_key(n) || classes.dead.iter().any(|d| d.name == n);
    if r.with_body
        || classes.eval
        || classes.within_with
        || !classes.with_bodies.is_empty()
        || named(u)
        || named(o)
        || classes.defs.contains_key(&site.target_start)
    {
        return Some(Reason::Dynamic);
    }
    if u == o {
        return Some(Reason::SelfReference);
    }
    let mut cur = declarator.parent();
    while let Some(n) = cur.filter(|n| n.id() != owner.id()) {
        if matches!(
            n.kind(),
            "for_statement" | "for_in_statement" | "while_statement" | "do_statement"
        ) {
            return Some(Reason::Loop);
        }
        cur = n.parent();
    }
    // The binding inventory: every other occurrence with no path components is a (re)binding.
    let bindings = |name| other_bindings(sites, name, site.target_start);
    if bindings(u).any(|s| s.form != Form::Declare) {
        return Some(Reason::AliasAssigned);
    }
    if bindings(u).next().is_some() {
        return Some(Reason::AliasRedeclared);
    }
    if bindings(o).any(|s| s.form != Form::Declare) {
        return Some(Reason::TargetRebound);
    }
    let mut decls = bindings(o);
    let (Some(decl), None) = (decls.next(), decls.next()) else {
        return Some(if bindings(o).next().is_none() {
            Reason::TargetUnresolved
        } else {
            Reason::TargetRedeclared
        });
    };
    cand.target_start = Some(decl.target_start);
    let dr = decl.region;
    if dr.nested || dr.instance_field {
        return Some(Reason::TargetUnresolved);
    }
    // Fail closed: a declaration the tree cannot find again is no admitted root.
    let Some(id) = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(decl.target_start, decl.target_end)
        .filter(|id| id.start_byte() == decl.target_start)
    else {
        return Some(Reason::TargetKind);
    };
    if dr.own_params {
        return (decl.decl != "param" || !plain_formal(id)).then_some(Reason::TargetKind);
    }
    let value = match (decl.decl == "declarator", id.parent()) {
        (true, Some(p))
            if p.kind() == "variable_declarator"
                && id.kind() == "identifier"
                && p.child_by_field_name("name").map(|n| n.id()) == Some(id.id()) =>
        {
            p.child_by_field_name("value")
        }
        _ => return Some(Reason::TargetKind),
    };
    let root = root_kind(value);
    match root {
        Root::Identifier => Some(Reason::Chain),
        Root::This => Some(Reason::ThisAlias),
        Root::Other => Some(Reason::TargetKind),
        Root::Supported if decl.target_start >= site.target_start => Some(Reason::TargetAfterAlias),
        Root::Supported => None,
    }
}

/// What an initialized local's initializer makes of it.
enum Root {
    /// An identifier, after peeling wrappers: another alias step.
    Identifier,
    This,
    /// A kind on the allow-list: the local holds its own value.
    Supported,
    /// Anything else (a sequence, conditional, logical, binary, member, ... or an unknown kind): refused.
    Other,
}

/// Initializer kinds that make a local a supported root, after peeling transparent wrappers: a call or `new`, a
/// literal, an object, array, function, arrow or class expression, and `await`/`yield` of one of these. The class
/// is an allow-list on purpose: the set of forms that merely forward another value (`(0, p)`, `p || p`, `c ? p : p`)
/// is open, the set of forms that produce a fresh one is not.
const ROOT_KINDS: [&str; 15] = [
    "call_expression",
    "new_expression",
    "number",
    "string",
    "regex",
    "true",
    "false",
    "null",
    "object",
    "array",
    "function_expression",
    "function",
    "generator_function",
    "arrow_function",
    "class",
];

fn root_kind(value: Option<Node<'_>>) -> Root {
    let mut value = value;
    // Transparent wrappers, TS instantiation expressions (`p<T>`), `await` and `yield` of an operand.
    while let Some(v) = value {
        value = match v.kind() {
            k if WRAPPERS.contains(&k) => wrap_operand(v),
            "instantiation_expression" => v.child_by_field_name("function"),
            "await_expression" | "yield_expression" => {
                let operand = v.named_child(0);
                if operand.is_none() {
                    return Root::Other;
                }
                operand
            }
            _ => break,
        };
    }
    let Some(v) = value else { return Root::Other };
    match v.kind() {
        "identifier" => Root::Identifier,
        "this" => Root::This,
        "template_string" => {
            let mut cur = v.walk();
            let plain = !v
                .named_children(&mut cur)
                .any(|c| c.kind() == "template_substitution");
            if plain {
                Root::Supported
            } else {
                Root::Other
            }
        }
        k if ROOT_KINDS.contains(&k) => Root::Supported,
        _ => Root::Other,
    }
}
