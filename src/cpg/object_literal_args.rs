//! Object-literal call arguments bind field-selectively to the callee formal (#594).
//!
//! `f({ k: v })` gives no path from `v` into `f`: the argument text is not a variable, so
//! Step 5b mints no `Use(v) -> Def(p)` edge. This pass adds, per data-carrying leaf of the
//! literal, `Use(v) -> Use(p.<read>)` edges straight to the callee reads that can observe
//! that leaf. It never routes through `Def(p)`: that node reaches every sibling read, which
//! is exactly the imprecision a literal's known keys let us avoid.
//!
//! Where it runs: inside the Step-5b per-site loop, after the positional formal is found.
//! It is gated on a call that resolves to an `Exact` singleton (NameOnly singletons were
//! wrong at 44 census sites; no multi-target pool is bound) and on a JS/TS/TSX caller and
//! callee. The callee formal is whatever Step 5b already admits as a positional target
//! (plain identifier, member-only, parenthesised-arrow); destructured, rest and bare-arrow
//! slots are holes there and stay holes here.
//!
//! Callee reads. The targets are the `Use` nodes one DataFlow hop from the formal's `Def`
//! whose path base is the formal: the DFG has already decided which reads that definition
//! reaches (member-only projections, reaching definitions, kills by reassignment or by a
//! member write), and the new edge inherits that decision. A member-only formal's reads
//! carry their field path (`p.a.b`); a mixed formal's member reads are base `p` Uses, so
//! the static member chain is read back from the syntax (`p.a.b` -> `[a, b]`; any other
//! use of `p`, including a `p[k]` read, is a bare read with an empty chain).
//!
//! Pairing. A leaf at key path `L` (a chain of static keys) feeds a read with chain `R`
//! when one is a prefix of the other: `L = [a]` feeds `p.a`, `p.a.x` and bare `p`, never
//! `p.b`. A nested literal contributes its leaf paths (`{ a: { b: x } }` is `L = [a, b]`),
//! so `p.a` (the container) sees `x` while `p.a.c` does not.
//!
//! Member writes. A member-only formal's reads already honour the callee's member writes
//! (the DFG drops a suppressed path). A mixed formal's reads are base `p` Uses that
//! assignment to `p.a` does not redefine, so the pass applies the member-only lane's own
//! flow-insensitive suppression itself (`ParsedFile::formal_member_facts_at`): a read whose
//! path is written, has a written prefix or has a deeper write below it is not fed, and a
//! leaf at or below a written path is not fed to any read (the write replaced that key).
//! Cost: a read that would have run before the write loses its edge. A computed-member
//! write to the formal itself (`p[k] = v`, `p["a"] = v`, `delete p[k]`, a pattern or
//! `for-in/of` target, wrappers and nested callables included) may replace any key, so it
//! is a wildcard write: no leaf of the literal feeds any read of that formal (fail closed;
//! `FormalMemberFacts::has_computed_write`, read by this pass only). A write below a key
//! (`p.a[k] = v`) and aliases of `p` are not modelled, as in the member-only lane.
//!
//! Label. The shortcut replaces the two hops `Use(v) -> Def(p) -> Use(p.<read>)`, so its
//! label is the worse of the call's label (`Exact` here) and the definition edge's own
//! label (for example `CfgIncomplete` in a one-statement callee). A `NameOnly(Killed)`
//! label keeps the definition edge's `kill_line`, which is a line of the *callee*, the
//! function the edge ends in; the argument's own function is the edge's `from`.
//!
//! Key decisions (each pinned by a test in `object_literal_arg_tests.rs`):
//! * A leaf is an identifier, a member path or a shorthand property, parenthesised or
//!   type-asserted; calls, arithmetic, arrays and function values carry no binding (the
//!   rule plain arguments already follow).
//! * Spread or a computed key (property or method) is a wildcard writer: it may overwrite
//!   any key written before it, so those keys are refused; keys after the last wildcard
//!   are definitive. The spread's own source is not bound (unknown keys).
//! * Duplicate keys: the last property for a key wins; if it is an accessor or method the
//!   key is refused. `__proto__` is never an own property and is refused.
//! * A spread argument before the literal makes the positional slot ambiguous: refused.
//! * A defaulted formal is bound exactly when Step 5b binds it for a plain argument; a
//!   literal is never `undefined`, so the default cannot replace it.

use super::*;
use tree_sitter::Node;

/// Everything one resolved call site contributes to the pass.
pub(super) struct SiteInputs<'a> {
    pub caller_id: &'a FunctionId,
    pub caller_parsed: &'a ParsedFile,
    pub callee_parsed: &'a ParsedFile,
    pub args: &'a [(String, std::ops::Range<usize>)],
    pub var_index: &'a BTreeMap<(String, String, usize, usize, AccessPath, VarAccess), NodeIndex>,
    pub exact_var_index: &'a ExactVarIndex,
    pub graph: &'a DiGraph<CpgNode, CpgEdge>,
    pub cross_file_free_single_exact: bool,
}

/// A data-carrying leaf of an object literal: its static key path and value span.
struct Leaf {
    keys: Vec<String>,
    span: std::ops::Range<usize>,
}

fn is_js_family(language: crate::languages::Language) -> bool {
    use crate::languages::Language;
    matches!(
        language,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    )
}

fn is_transparent_wrapper(kind: &str) -> bool {
    matches!(
        kind,
        "parenthesized_expression"
            | "non_null_expression"
            | "as_expression"
            | "satisfies_expression"
            | "type_assertion"
    )
}

/// The expression inside a transparent wrapper (`type_assertion` carries it last).
fn unwrap_expression(mut node: Node<'_>) -> Option<Node<'_>> {
    while is_transparent_wrapper(node.kind()) {
        let mut cursor = node.walk();
        let mut inner = node.named_children(&mut cursor).filter(|c| !c.is_extra());
        node = if node.kind() == "type_assertion" {
            inner.last()?
        } else {
            inner.next()?
        };
    }
    Some(node)
}

fn plain_identifier(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || matches!(b, b'_' | b'$'))
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'$'))
}

/// How one object property writes its key.
enum Write {
    /// Spread or a computed key: may overwrite any earlier key.
    Wildcard,
    /// A definite key that cannot be bound (accessor, method, odd spelling, `__proto__`).
    Opaque(Option<String>),
    /// A definite key with a value expression.
    Value(String),
}

/// `None`: a key that may equal any other key (computed, or spelled with an escape or a
/// non-ASCII character, which can alias an identifier key): a wildcard writer.
/// `Some(None)`: a definite key no static member read can name (numeric, `"a-b"`).
fn literal_key(parsed: &ParsedFile, key: Node<'_>) -> Option<Option<String>> {
    let text = parsed.node_text(&key);
    match key.kind() {
        "property_identifier" => {
            (text.is_ascii() && !text.contains('\\')).then(|| Some(text.to_string()))
        }
        "string" => {
            if !text.is_ascii() || text.contains('\\') {
                return None;
            }
            let inner = text.get(1..text.len().checked_sub(1)?)?;
            Some(plain_identifier(inner).then(|| inner.to_string()))
        }
        "computed_property_name" => None,
        _ => Some(None),
    }
}

/// Collect the data-carrying leaves of `object` below `prefix`. `None` refuses the whole
/// literal (parse recovery inside it).
fn collect_leaves(
    parsed: &ParsedFile,
    object: Node<'_>,
    prefix: &[String],
    out: &mut Vec<Leaf>,
) -> Option<()> {
    if object.has_error() {
        return None;
    }
    let mut writes: Vec<(Write, Node<'_>)> = Vec::new();
    let mut cursor = object.walk();
    for child in object.named_children(&mut cursor) {
        match child.kind() {
            "comment" => {}
            "spread_element" => writes.push((Write::Wildcard, child)),
            "shorthand_property_identifier" => {
                let name = parsed.node_text(&child).to_string();
                if name.is_ascii() && !name.contains('\\') {
                    writes.push((Write::Value(name), child));
                } else {
                    writes.push((Write::Wildcard, child));
                }
            }
            "pair" => {
                let key = child.child_by_field_name("key")?;
                match literal_key(parsed, key) {
                    None => writes.push((Write::Wildcard, child)),
                    Some(None) => writes.push((Write::Opaque(None), child)),
                    Some(Some(name)) => writes.push((Write::Value(name), child)),
                }
            }
            "method_definition" => {
                let name = child.child_by_field_name("name")?;
                match literal_key(parsed, name) {
                    None => writes.push((Write::Wildcard, child)),
                    Some(key) => writes.push((Write::Opaque(key), child)),
                }
            }
            _ => return None,
        }
    }
    let last_wildcard = writes
        .iter()
        .rposition(|(write, _)| matches!(write, Write::Wildcard));
    // The last property for a key decides it, whatever its kind.
    let mut last_for_key: BTreeMap<String, usize> = BTreeMap::new();
    for (i, (write, _)) in writes.iter().enumerate() {
        match write {
            Write::Value(name) | Write::Opaque(Some(name)) => {
                last_for_key.insert(name.clone(), i);
            }
            _ => {}
        }
    }
    for (i, (write, node)) in writes.iter().enumerate() {
        let Write::Value(name) = write else {
            continue;
        };
        if last_for_key.get(name) != Some(&i)
            || last_wildcard.is_some_and(|wild| wild > i)
            || name == "__proto__"
        {
            continue;
        }
        let mut keys = prefix.to_vec();
        keys.push(name.clone());
        if node.kind() == "shorthand_property_identifier" {
            out.push(Leaf {
                keys,
                span: node.start_byte()..node.end_byte(),
            });
            continue;
        }
        let Some(value) = node
            .child_by_field_name("value")
            .and_then(unwrap_expression)
        else {
            continue;
        };
        match value.kind() {
            "object" => collect_leaves(parsed, value, &keys, out)?,
            "identifier" | "member_expression" => out.push(Leaf {
                keys,
                span: value.start_byte()..value.end_byte(),
            }),
            _ => {}
        }
    }
    Some(())
}

/// The literal behind an argument span, with its leaves, or `None` when the argument is
/// not a literal or is not safely positional.
fn argument_leaves(parsed: &ParsedFile, span: &std::ops::Range<usize>) -> Option<Vec<Leaf>> {
    let node = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(span.start, span.end)?;
    if node.start_byte() != span.start || node.end_byte() != span.end {
        return None;
    }
    // A spread argument before this one shifts which positional slot this is.
    let mut earlier = node.prev_sibling();
    while let Some(sibling) = earlier {
        if sibling.kind() == "spread_element" {
            return None;
        }
        earlier = sibling.prev_sibling();
    }
    let object = unwrap_expression(node)?;
    if object.kind() != "object" {
        return None;
    }
    let mut leaves = Vec::new();
    collect_leaves(parsed, object, &[], &mut leaves)?;
    Some(leaves)
}

/// The static member chain read through the identifier at `[start, end)`: `p.a.b` -> `[a, b]`,
/// bare or computed uses -> `[]`. `None` when the span is not that identifier.
fn read_chain(parsed: &ParsedFile, start: usize, end: usize) -> Option<Vec<String>> {
    let ident = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(start, end)?;
    if ident.kind() != "identifier" || ident.start_byte() != start || ident.end_byte() != end {
        return None;
    }
    let mut chain = Vec::new();
    let mut current = ident;
    while let Some(parent) = current.parent() {
        if is_transparent_wrapper(parent.kind()) {
            if first_expression(parent).map(|n| n.id()) == Some(current.id()) {
                current = parent;
                continue;
            }
            break;
        }
        if parent.kind() == "member_expression"
            && parent.child_by_field_name("object").map(|n| n.id()) == Some(current.id())
        {
            let property = parent.child_by_field_name("property")?;
            if property.kind() != "property_identifier" {
                break;
            }
            chain.push(parsed.node_text(&property).to_string());
            current = parent;
            continue;
        }
        break;
    }
    // A write target is not special-cased here: its Use is refused through the callee's
    // member writes (`FormalInfo`), which classify with the member-only lane's own rules.
    Some(chain)
}

/// The expression a transparent wrapper wraps, one level.
fn first_expression(wrapper: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = wrapper.walk();
    let mut inner = wrapper
        .named_children(&mut cursor)
        .filter(|c| !c.is_extra());
    if wrapper.kind() == "type_assertion" {
        inner.last()
    } else {
        inner.next()
    }
}

/// A leaf and a read overlap when one key chain is a prefix of the other.
fn comparable(leaf: &[String], read: &[String]) -> bool {
    let n = leaf.len().min(read.len());
    leaf[..n] == read[..n]
}

/// One read of the formal that its definition node reaches.
struct Read {
    node: NodeIndex,
    chain: Vec<String>,
    /// The definition edge's own label.
    label: FlowConfidence,
}

/// The reads of a formal and, for a mixed formal, the callee's member writes to it.
struct FormalInfo {
    reads: Vec<Read>,
    base: String,
    /// Present exactly when the reads are base `p` Uses (a mixed formal).
    facts: Option<crate::ast::FormalMemberFacts>,
}

impl FormalInfo {
    /// The read is not fed: its path is written, has a written prefix or a deeper write.
    fn read_suppressed(&self, read: &Read) -> bool {
        self.facts.as_ref().is_some_and(|facts| {
            facts.is_killed(&AccessPath::with_fields(
                self.base.clone(),
                read.chain.clone(),
            ))
        })
    }

    /// The leaf's key was replaced by a write at it or above it.
    fn leaf_overwritten(&self, leaf: &Leaf) -> bool {
        self.facts
            .as_ref()
            .is_some_and(|facts| facts.overwrites_at_or_above(&self.base, &leaf.keys))
    }
}

/// Reads of the formal that its definition node reaches.
fn formal_info(
    graph: &DiGraph<CpgNode, CpgEdge>,
    callee_parsed: &ParsedFile,
    def: NodeIndex,
) -> Option<FormalInfo> {
    let CpgNode::Variable {
        path: formal,
        access: VarAccess::Def,
        start_byte: def_start,
        end_byte: def_end,
        ..
    } = &graph[def]
    else {
        return None;
    };
    let mut reads: BTreeMap<NodeIndex, Read> = BTreeMap::new();
    let mut mixed = false;
    for edge in graph.edges(def) {
        let CpgEdge::DataFlow(label) = *edge.weight() else {
            continue;
        };
        let CpgNode::Variable {
            path,
            access: VarAccess::Use,
            start_byte,
            end_byte,
            ..
        } = &graph[edge.target()]
        else {
            continue;
        };
        if path.base != formal.base {
            continue;
        }
        let chain = if path.fields.is_empty() {
            mixed = true;
            match read_chain(callee_parsed, *start_byte, *end_byte) {
                Some(chain) => chain,
                None => continue,
            }
        } else {
            path.fields.clone()
        };
        reads
            .entry(edge.target())
            .and_modify(|read| read.label = read.label.worst(label))
            .or_insert(Read {
                node: edge.target(),
                chain,
                label,
            });
    }
    // The callee's member writes come from the same walk the member-only lane uses; fail
    // closed without them. A mixed formal's reads are not field-aware in the DFG, so those
    // facts also drive the per-read suppression; a member-only formal's DFG already has it.
    let all_facts = callee_parsed.formal_member_facts_at(*def_start, *def_end, &formal.base)?;
    // A computed write (`p[k] = v`) may replace any key: a wildcard writer, fail closed.
    if all_facts.has_computed_write() {
        return None;
    }
    let facts = mixed.then_some(all_facts);
    Some(FormalInfo {
        reads: reads.into_values().collect(),
        base: formal.base.clone(),
        facts,
    })
}

fn may_be_literal(text: &str, language: crate::languages::Language) -> bool {
    text.starts_with(['{', '('])
        || (text.starts_with('<') && language == crate::languages::Language::TypeScript)
}

/// A call site binds object literals only when it resolves to one Exact callee.
pub(super) fn exact_singleton(resolved: &[ResolvedCallee<'_>]) -> bool {
    resolved.len() == 1 && resolved[0].confidence == ResolutionConfidence::Exact
}

/// Emit the literal edges for one Exact-singleton call site. `params` is the callee's
/// positional formals; `resolve` turns one into its definition node (holes stay holes).
pub(super) fn collect<T>(
    out: &mut Vec<PendingArgumentFlow>,
    site: &SiteInputs<'_>,
    params: &[Option<T>],
    resolve: impl Fn(&T) -> Option<NodeIndex>,
) {
    let language = site.caller_parsed.language;
    if !is_js_family(language)
        || !is_js_family(site.callee_parsed.language)
        || !site
            .args
            .iter()
            .any(|(text, _)| may_be_literal(text, language))
    {
        return;
    }
    let mut seen: BTreeSet<(NodeIndex, NodeIndex)> = BTreeSet::new();
    for (i, param) in params.iter().enumerate() {
        let Some((arg_text, arg_span)) = site.args.get(i) else {
            break;
        };
        if !may_be_literal(arg_text, language) {
            continue;
        }
        let Some(def) = param.as_ref().and_then(&resolve) else {
            continue;
        };
        let Some(leaves) = argument_leaves(site.caller_parsed, arg_span) else {
            continue;
        };
        if leaves.is_empty() {
            continue;
        }
        let Some(info) = formal_info(site.graph, site.callee_parsed, def) else {
            continue;
        };
        for leaf in &leaves {
            if info.leaf_overwritten(leaf) {
                continue;
            }
            let value_text = site
                .caller_parsed
                .source
                .get(leaf.span.clone())
                .unwrap_or("")
                .trim();
            // Same lookup as a plain argument: the field path, then the base.
            let full = site
                .caller_parsed
                .bounded_argument_member_path_at(leaf.span.start, leaf.span.end)
                .unwrap_or_else(|| AccessPath::from_expr(value_text));
            let mut paths = vec![full.clone()];
            let base = AccessPath::simple(&full.base);
            if base != full {
                paths.push(base);
            }
            let sources: Vec<NodeIndex> = paths
                .into_iter()
                .filter_map(|path| {
                    CodePropertyGraph::argument_var_node_in_span_with_exact(
                        site.caller_id,
                        site.caller_parsed,
                        &path,
                        &leaf.span,
                        site.var_index,
                        site.exact_var_index,
                        site.graph,
                    )
                })
                .collect();
            for read in &info.reads {
                if !comparable(&leaf.keys, &read.chain) || info.read_suppressed(read) {
                    continue;
                }
                for &from in &sources {
                    if seen.insert((from, read.node)) {
                        out.push(PendingArgumentFlow {
                            edge: (
                                from,
                                read.node,
                                CpgEdge::DataFlow(FlowConfidence::Exact.worst(read.label)),
                            ),
                            cross_file_free_single_exact: site.cross_file_free_single_exact,
                        });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "object_literal_parity_tests.rs"]
mod parity_tests;
