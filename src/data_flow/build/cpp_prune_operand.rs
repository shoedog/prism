//! The operand certificate and the worklist that settles it (#630 slice 6c, fold 4), consumed by
//! `cpp_prune.rs`. Design of record: `docs/superpowers/plans/2026-11-11-630-s6-cpp-ud/DESIGN.md`
//! §5.3.
//!
//! A user-defined operator needs a class or enumeration operand, and a converting constructor of
//! a by-value class parameter can bind a scalar operand (`S(int&)` with `S operator+(S, S)`), so
//! an operator is built-in only when EVERY operand is a built-in scalar expression. For each
//! operand of a binary operator (all of them, `<<`, `>>`, `<=>`, `&&`, `||` and `,` included), a
//! compound assignment, a subscript and the two branches of `?:`, the lvalue roots of that
//! operand escape unless every OTHER operand is a built-in scalar expression
//! (`certify`, trace kind `operand_certificate`). Whether an identifier is one depends on the
//! owner-wide escaped set, so the escape waits as a `Deferred` record until `settle`.
use super::c_prune::{declaration_of, resolve, COwner};
use super::cpp_prune::{keyword_scalar, keyword_spelling, lvalue_roots, scalar_cast_type};
use super::cpp_prune_names::is_alignof_keyword;
use super::prune_roots::operand;
use crate::ast::ParsedFile;
use std::collections::HashMap;
use tree_sitter::Node;

/// One fact a deferred escape depends on. Byte ranges name identifier occurrences.
pub(super) enum Need {
    /// A bindable scalar local (`COwner::binding`): depends on the escaped set.
    Scalar(usize, usize),
    /// A bindable scalar local or an owner-local array of keyword scalars or pointers.
    Value(usize, usize),
    /// An owner-local array whose elements are keyword scalars or pointers (static).
    Array(usize, usize),
    /// A `NULL`/`nullptr` spelling, a built-in only when no in-repo macro has that name.
    Spelling(String),
}

/// An escape that holds only when some `need` fails: `roots` (names) escape then.
pub(super) struct Deferred {
    pub(super) needs: Vec<Need>,
    pub(super) kind: &'static str,
    pub(super) roots: Vec<String>,
    /// 6d measurement: site info for the log.
    pub(super) info: String,
}

/// Bounds the work one built-in-ness check does (a left-deep chain of N terms would otherwise
/// walk O(N) nodes per level): more than `MAX_NEEDS` identifiers (the 65th refuses) or
/// `MAX_NODES` visited nodes count as not scalar.
const MAX_NEEDS: usize = 64;
const MAX_NODES: usize = 512;

/// A built-in scalar expression: a literal other than a user-defined one, an identifier (a
/// `need`), a cast to a keyword scalar or a pointer to one, `sizeof`/`alignof`, `a[i]` of a
/// local array, or parentheses, a unary arithmetic or logical operator, a binary, comma or
/// conditional expression over these. Anything else (a call, a member access, a dereference,
/// `this`, a template) is unknown and counts as not scalar.
fn builtin_scalar(
    parsed: &ParsedFile,
    node: Node<'_>,
    needs: &mut Vec<Need>,
    budget: &mut usize,
) -> bool {
    if *budget == 0 || needs.len() > MAX_NEEDS {
        return false;
    }
    *budget -= 1;
    let mut sub = |f: &str, needs: &mut Vec<Need>| {
        node.child_by_field_name(f)
            .is_some_and(|c| builtin_scalar(parsed, c, needs, budget))
    };
    match node.kind() {
        "number_literal" | "char_literal" | "sizeof_expression" => true,
        // The grammar reads `TRUE`, `FALSE` and `NULL` as these node kinds, and a program can
        // declare any of them as an object or an enumerator: only the lowercase keyword counts.
        "true" | "false" => keyword_spelling(parsed, node),
        "alignof_expression" => is_alignof_keyword(parsed, node),
        "null" | "nullptr" => {
            if !keyword_spelling(parsed, node) {
                return false;
            }
            needs.push(Need::Spelling(parsed.node_text(&node).to_owned()));
            true
        }
        "identifier" => {
            needs.push(Need::Value(node.start_byte(), node.end_byte()));
            true
        }
        "parenthesized_expression" => {
            operand(node).is_some_and(|c| builtin_scalar(parsed, c, needs, budget))
        }
        "cast_expression" => node
            .child_by_field_name("type")
            .is_some_and(|t| scalar_cast_type(parsed, t)),
        "unary_expression" => {
            node.child_by_field_name("operator")
                .is_some_and(|op| matches!(parsed.node_text(&op), "-" | "+" | "!" | "~"))
                && sub("argument", needs)
        }
        "binary_expression" | "comma_expression" => sub("left", needs) && sub("right", needs),
        "conditional_expression" => {
            if node.child_by_field_name("consequence").is_some() {
                sub("consequence", needs) && sub("alternative", needs)
            } else {
                sub("condition", needs) && sub("alternative", needs)
            }
        }
        "subscript_expression" => {
            let Some(base) = node.child_by_field_name("argument") else {
                return false;
            };
            if base.kind() != "identifier" {
                return false;
            }
            needs.push(Need::Array(base.start_byte(), base.end_byte()));
            subscript_indices(node)
                .iter()
                .all(|i| builtin_scalar(parsed, *i, needs, budget))
        }
        _ => false,
    }
}

/// The index expressions of a `subscript_expression` (C++23 allows several).
fn subscript_indices(node: Node<'_>) -> Vec<Node<'_>> {
    let mut out = Vec::new();
    if let Some(list) = node.child_by_field_name("indices") {
        let mut cursor = list.walk();
        out.extend(
            list.named_children(&mut cursor)
                .filter(|c| c.kind() != "comment"),
        );
    }
    out
}

/// The operands of an operator node, `None` when the node is no operator the certificate covers.
pub(super) fn operands_of<'a>(parsed: &ParsedFile, node: Node<'a>) -> Option<Vec<Node<'a>>> {
    let field = |f: &str| node.child_by_field_name(f);
    match node.kind() {
        "binary_expression" | "comma_expression" => Some(vec![field("left")?, field("right")?]),
        "assignment_expression" => {
            let op = field("operator")?;
            (parsed.node_text(&op) != "=").then_some(vec![field("left")?, field("right")?])
        }
        "subscript_expression" => {
            let mut v = vec![field("argument")?];
            v.extend(subscript_indices(node));
            Some(v)
        }
        "conditional_expression" => Some(if field("consequence").is_some() {
            vec![field("consequence")?, field("alternative")?]
        } else {
            vec![field("condition")?, field("alternative")?]
        }),
        _ => None,
    }
}

/// The operand certificate: for each operand, its lvalue roots escape unless every other
/// operand is a built-in scalar expression.
pub(super) fn certify(parsed: &ParsedFile, node: Node<'_>, out: &mut COwner) {
    let Some(operands) = operands_of(parsed, node) else {
        return;
    };
    let family = op_family(parsed, node);
    if super::mute6d::mute(&format!("c5{family}")) {
        return;
    }
    let line = node.start_position().row + 1;
    for (i, operand) in operands.iter().enumerate() {
        let mut roots = Vec::new();
        lvalue_roots(parsed, *operand, &mut roots);
        if roots.is_empty() {
            continue;
        }
        let names: Vec<String> = roots
            .iter()
            .map(|n| parsed.node_text(n).to_owned())
            .collect();
        let mut needs = Vec::new();
        let mut budget = MAX_NODES;
        let all_builtin = operands
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .all(|(_, o)| builtin_scalar(parsed, *o, &mut needs, &mut budget))
            && needs.len() <= MAX_NEEDS;
        if !all_builtin {
            if super::mute6d::log_enabled() {
                let why = operands
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .find_map(|(_, o)| fail_reason(parsed, *o))
                    .unwrap_or_else(|| "too_large".to_owned());
                super::mute6d::set_site(
                    line,
                    format!("op={family}:{};why={why}", op_text(parsed, node)),
                );
            }
            for n in &names {
                out.escape("operand_certificate", n);
            }
        } else if !needs.is_empty() {
            out.deferred.push(Deferred {
                info: format!("op={family}:{};line={line}", op_text(parsed, node)),
                needs,
                kind: "operand_certificate",
                roots: names,
            });
        }
    }
}

/// An owner-local array of keyword scalars (or of pointers): `a[i]` is a built-in subscript
/// with a scalar or pointer element. A one-dimensional `T a[N]` or `T *a[N]` declarator only.
fn array_local(parsed: &ParsedFile, out: &COwner, occ: Node<'_>) -> bool {
    // An escaped name (a `using` or hidden declaration the resolver cannot see, an aliasing
    // escape, ...) may be another entity than the declaration `resolve` finds.
    let name = parsed.node_text(&occ);
    if out.is_macro_name(name) || out.is_escaped(name) {
        return false;
    }
    let Some(ident) = resolve(parsed, occ) else {
        return false;
    };
    let Some((decl, top)) = declaration_of(ident) else {
        return false;
    };
    let mut n = top;
    let mut pointer = false;
    while n.kind() == "pointer_declarator" {
        pointer = true;
        match n.child_by_field_name("declarator") {
            Some(x) => n = x,
            None => return false,
        }
    }
    if n.kind() != "array_declarator"
        || n.child_by_field_name("declarator")
            .is_none_or(|d| d.id() != ident.id())
    {
        return false;
    }
    let scalar_type = decl
        .child_by_field_name("type")
        .is_some_and(|t| keyword_scalar(parsed, t));
    if !(pointer || scalar_type) {
        return false;
    }
    let spec = &parsed.source[decl.start_byte()..top.start_byte().max(decl.start_byte())];
    !spec
        .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .any(|t| !t.is_empty() && out.is_macro_name(t))
}

/// The escaped-set-independent verdict on one identifier occurrence.
#[derive(Clone, Copy)]
struct Shape {
    /// `COwner::binding` accepts it when its name is unescaped.
    scalar: bool,
    array: bool,
}

/// Settle the deferred escapes to the least fixpoint. Each needed occurrence is resolved once
/// (a block scan; the resolution does not depend on the escaped set), the records are indexed by
/// the names their decision depends on (scalars and arrays alike), and a name that escapes
/// re-examines only the records that depend on it: escaping is monotone, so each record fires at
/// most once and propagation is near-linear. Shape construction is not: it resolves each
/// distinct occurrence once, and a resolution scans the enclosing blocks.
pub(super) fn settle(parsed: &ParsedFile, out: &mut COwner) {
    let root = parsed.tree.root_node();
    let deferred = std::mem::take(&mut out.deferred);
    let mut fired = vec![false; deferred.len()];
    let mut shapes: HashMap<(usize, usize), Option<(String, Shape)>> = HashMap::new();
    let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
    let mut dead = Vec::new();
    let mut dead_why: HashMap<usize, String> = HashMap::new();
    for (i, d) in deferred.iter().enumerate() {
        let mut is_dead = false;
        for need in &d.needs {
            let (s, e, want) = match need {
                Need::Spelling(name) => {
                    if out.is_macro_name(name) {
                        is_dead = true;
                        dead_why
                            .entry(i)
                            .or_insert_with(|| "dead:macro_name".to_owned());
                    }
                    continue;
                }
                Need::Scalar(s, e) => (*s, *e, 0),
                Need::Value(s, e) => (*s, *e, 1),
                Need::Array(s, e) => (*s, *e, 2),
            };
            let entry = shapes.entry((s, e)).or_insert_with(|| {
                let n = root.descendant_for_byte_range(s, e)?;
                (n.kind() == "identifier").then(|| {
                    let shape = Shape {
                        scalar: out.binding_unescaped(parsed, n),
                        array: array_local(parsed, out, n),
                    };
                    (parsed.node_text(&n).to_owned(), shape)
                })
            });
            let Some((name, shape)) = entry else {
                is_dead = true;
                dead_why
                    .entry(i)
                    .or_insert_with(|| "dead:not_identifier".to_owned());
                continue;
            };
            // An array need and a scalar need both depend on the name staying unescaped: dead
            // now, or re-examined when the name escapes later.
            let holds = if want == 2 || (want == 1 && shape.array) {
                shape.array
            } else {
                shape.scalar
            };
            if !holds || out.is_escaped(name) {
                is_dead = true;
                let why = if !holds {
                    match root.descendant_for_byte_range(s, e) {
                        Some(n) if resolve(parsed, n).is_none() => "dead:global_or_unknown",
                        Some(_) => "dead:local_nonscalar_type",
                        None => "dead:?",
                    }
                } else {
                    "dead:escaped_name"
                };
                dead_why.entry(i).or_insert_with(|| why.to_owned());
            } else {
                by_name.entry(name.clone()).or_default().push(i);
            }
        }
        if is_dead {
            dead.push(i);
        }
    }
    let mut queue: Vec<String> = Vec::new();
    let mut fire = |i: usize, out: &mut COwner, queue: &mut Vec<String>, why: &str| {
        if std::mem::replace(&mut fired[i], true) {
            return;
        }
        super::mute6d::set_site(0, format!("{};{why}", deferred[i].info));
        for r in &deferred[i].roots {
            if out.escape(deferred[i].kind, r) {
                queue.push(r.clone());
            }
        }
    };
    for i in dead {
        let why = dead_why.get(&i).cloned().unwrap_or_default();
        fire(i, out, &mut queue, &why);
    }
    while let Some(name) = queue.pop() {
        for i in by_name.remove(&name).unwrap_or_default() {
            fire(i, out, &mut queue, "cascade");
        }
    }
}

/// 6d measurement: operator family of an operand-certificate node.
fn op_family(parsed: &ParsedFile, node: Node<'_>) -> &'static str {
    let op = op_text(parsed, node);
    match node.kind() {
        "binary_expression" => match op.as_str() {
            "<" | ">" | "<=" | ">=" | "==" | "!=" | "<=>" => "cmp",
            "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^" => "arith",
            "<<" | ">>" => "shift",
            _ => "other",
        },
        _ => "other",
    }
}

fn op_text(parsed: &ParsedFile, node: Node<'_>) -> String {
    match node.kind() {
        "binary_expression" | "assignment_expression" => node
            .child_by_field_name("operator")
            .map(|o| parsed.node_text(&o).to_owned())
            .unwrap_or_default(),
        k => k.to_owned(),
    }
}

/// 6d measurement: why `builtin_scalar` fails on an operand (first failing leaf), mirrored.
fn fail_reason(parsed: &ParsedFile, node: Node<'_>) -> Option<String> {
    let sub = |f: &str| {
        node.child_by_field_name(f)
            .and_then(|c| fail_reason(parsed, c))
    };
    match node.kind() {
        "number_literal" | "char_literal" | "sizeof_expression" | "identifier" => None,
        "true" | "false" | "alignof_expression" | "null" | "nullptr" => None,
        "parenthesized_expression" => operand(node).and_then(|c| fail_reason(parsed, c)),
        "cast_expression" => node
            .child_by_field_name("type")
            .is_some_and(|t| scalar_cast_type(parsed, t))
            .then_some(())
            .map_or(Some("cast_nonscalar".to_owned()), |_| None),
        "unary_expression" => sub("argument"),
        "binary_expression" | "comma_expression" => sub("left").or_else(|| sub("right")),
        "conditional_expression" => sub("consequence")
            .or_else(|| sub("condition"))
            .or_else(|| sub("alternative")),
        "subscript_expression" => match node.child_by_field_name("argument") {
            Some(b) if b.kind() == "identifier" => None,
            _ => Some("subscript_nonlocal".to_owned()),
        },
        "call_expression" => Some("call".to_owned()),
        "field_expression" => Some("member".to_owned()),
        "pointer_expression" => Some("deref_or_addr".to_owned()),
        "qualified_identifier" => Some("qualified_global".to_owned()),
        "this" => Some("this".to_owned()),
        "string_literal" | "raw_string_literal" | "concatenated_string" => {
            Some("string".to_owned())
        }
        k => Some(format!("kind:{k}")),
    }
}
