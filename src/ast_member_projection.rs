//! PR-C: one body walk per member-only formal. Collects, in a single pass, the static
//! member paths read through the formal (with their admitted occurrence spans) and every
//! member write that targets the formal, so the DFG builder neither re-walks the callable
//! per path nor leans on the shared assignment query (which other passes use).
//!
//! The walk covers the callable BODY only (by bytes). Parameter-list default expressions
//! are therefore never projection targets, on the declaration line or any other line.

use super::callback_identity::ScopeBindMemo;
use super::*;

/// What one body walk found for a member-only formal.
#[derive(Debug, Default)]
pub(crate) struct FormalMemberFacts {
    /// Outermost static member paths read through the formal (`p.x`, `p.x.y`).
    pub outermost: BTreeSet<AccessPath>,
    /// Per path, the first admitted read occurrence on each line: line -> (start, end).
    /// Admitted means bound to the formal (binder fence), an evaluated read, outside
    /// `with` bodies and outside erased type positions.
    pub reads: BTreeMap<AccessPath, BTreeMap<usize, (usize, usize)>>,
    /// Member write targets bound to the formal, anywhere in the callable (nested
    /// callables included).
    writes: BTreeSet<AccessPath>,
    /// Every proper prefix of a write path (non-empty), for the deeper-write kill.
    write_ancestors: BTreeSet<AccessPath>,
    /// Distinct scope-binding verdicts the walk computed (cost witness: bounded by the
    /// number of scopes, not by the number of member occurrences).
    pub scope_verdicts: usize,
}

impl FormalMemberFacts {
    /// Flow-insensitive kill: a write to the path, to a prefix of it, or to a deeper path
    /// below it suppresses the path (a deeper write may change what the prefix holds).
    pub(crate) fn is_killed(&self, path: &AccessPath) -> bool {
        if self.write_ancestors.contains(path) {
            return true;
        }
        (0..=path.fields.len()).any(|k| {
            self.writes.contains(&AccessPath::with_fields(
                path.base.clone(),
                path.fields[..k].to_vec(),
            ))
        })
    }

    fn record_write(&mut self, path: AccessPath) {
        for k in 1..path.fields.len() {
            self.write_ancestors.insert(AccessPath::with_fields(
                path.base.clone(),
                path.fields[..k].to_vec(),
            ));
        }
        self.writes.insert(path);
    }
}

impl ParsedFile {
    /// PR-C: whether any identifier-like node of the callable is spelled with a `\` escape.
    /// The binder fence does not model escaped spellings, so such a callable keeps the old
    /// behaviour (no member-only classification), fail-closed as in #382.
    pub(crate) fn callable_has_escaped_identifier(&self, node: Node<'_>) -> bool {
        if node.kind().contains("identifier") && self.node_text(&node).contains('\\') {
            return true;
        }
        let mut cursor = node.walk();
        let found = node
            .children(&mut cursor)
            .any(|child| self.callable_has_escaped_identifier(child));
        found
    }

    /// PR-C: whether the callable (nested callables included) contains a `with` statement.
    /// A `with` object can bind any name for the statements under it, which the binder
    /// fence does not model, so such a callable is never member-only (fail-closed).
    pub(crate) fn callable_has_with_statement(&self, node: Node<'_>) -> bool {
        let mut stack = vec![node];
        while let Some(current) = stack.pop() {
            if current.kind() == "with_statement" {
                return true;
            }
            let mut cursor = current.walk();
            stack.extend(current.children(&mut cursor));
        }
        false
    }

    /// PR-C: member paths read through the formal `name` and member writes to it, in one
    /// pass over the callable body. `def_byte` is the formal's identifier start byte (the
    /// binder-fence anchor). Refused formals (eval/arguments/seam in a callback pass) never
    /// reach this point: the DFG builder registers no Def for them. Cost: one tree walk,
    /// with the seam verdict hoisted and every scope verdict memoised per `(scope, name)`,
    /// so the work is linear in the number of member occurrences at bounded path depth.
    pub(crate) fn formal_member_facts(
        &self,
        func_node: &Node<'_>,
        name: &str,
        def_byte: usize,
    ) -> FormalMemberFacts {
        let mut facts = FormalMemberFacts::default();
        let Some(body) = self.function_body_node(func_node) else {
            return facts;
        };
        let walk = FactsWalk {
            name,
            def_scope: self.js_ts_def_scope_id(def_byte, name),
            seam_open: self.js_ts_seam_open(func_node, name),
            memo: ScopeBindMemo::default(),
        };
        self.collect_member_facts(body, &walk, false, &mut facts);
        facts.scope_verdicts = walk.memo.computed();
        facts
    }

    fn collect_member_facts(
        &self,
        node: Node<'_>,
        walk: &FactsWalk<'_>,
        inside_chain: bool,
        facts: &mut FormalMemberFacts,
    ) {
        // A transparent wrapper between two members of one chain (`(p.x).y`, `p.x!.y`)
        // keeps its inner members inside the chain.
        let mut in_chain = is_transparent_wrapper(node.kind()) && inside_chain;
        if node.kind() == "member_expression" {
            if self.language.is_in_erased_type_context(node) {
                // Erased TS positions (`typeof p.x`, interface members) are not reads.
                return;
            }
            if let Some(path) = self.static_member_path(node, walk.name) {
                in_chain = true;
                if !path.is_simple() {
                    let bound = !self.js_ts_reference_fenced_memo(
                        &node,
                        walk.name,
                        walk.def_scope,
                        walk.seam_open,
                        &walk.memo,
                    );
                    if self.member_is_write_target(node) {
                        if bound {
                            facts.record_write(path);
                        }
                    } else {
                        if !inside_chain {
                            facts.outermost.insert(path.clone());
                        }
                        // A callable containing a `with` statement is never member-only
                        // (`classify_parameter_use`); this still refuses reads of a callable
                        // that is itself nested inside a `with` body (reachable, so kept).
                        if bound
                            && !self.js_ts_identifier_is_not_a_read(node)
                            && !self.js_ts_span_in_with_body(node.start_byte(), node.end_byte())
                        {
                            facts
                                .reads
                                .entry(path)
                                .or_default()
                                .entry(node.start_position().row + 1)
                                .or_insert((node.start_byte(), node.end_byte()));
                        }
                    }
                }
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_member_facts(child, walk, in_chain, facts);
        }
    }

    /// The one normalizer for member chains, used for reads and writes alike: the static
    /// path of `node` (`p.x.y`) with transparent wrappers (parentheses, `!`, `as`,
    /// `satisfies`, type assertions) stripped at every level of the receiver chain, or
    /// `None` when the chain does not end at the identifier `name` through plain
    /// (non-computed) member accesses.
    fn static_member_path(&self, node: Node<'_>, name: &str) -> Option<AccessPath> {
        let canonical = self.canonical_chain_text(node, name)?;
        Some(AccessPath::from_expr(&canonical))
    }

    fn canonical_chain_text(&self, node: Node<'_>, name: &str) -> Option<String> {
        let node = strip_transparent_wrappers(node)?;
        match node.kind() {
            "identifier" => (self.node_text(&node) == name).then(|| name.to_string()),
            "member_expression" => {
                let object = node.child_by_field_name("object")?;
                let mut text = self.canonical_chain_text(object, name)?;
                let tail = self.source.get(object.end_byte()..node.end_byte())?;
                text.extend(tail.chars().filter(|c| !c.is_whitespace()));
                Some(text)
            }
            _ => None,
        }
    }

    /// Whether this member expression is the target of a write: assignment (plain,
    /// compound, logical) left side, `++`/`--`, `delete`, an array/object pattern target
    /// (including rest and nested patterns), or a `for (… in/of …)` head.
    fn member_is_write_target(&self, node: Node<'_>) -> bool {
        let mut child = node;
        let mut parent = node.parent();
        while let Some(p) = parent {
            match p.kind() {
                "parenthesized_expression"
                | "non_null_expression"
                | "as_expression"
                | "satisfies_expression"
                | "type_assertion"
                | "rest_pattern" => {}
                "assignment_expression" | "augmented_assignment_expression" => {
                    return p.child_by_field_name("left").map(|n| n.id()) == Some(child.id());
                }
                "update_expression" => return true,
                "unary_expression" => {
                    return p
                        .child_by_field_name("operator")
                        .is_some_and(|op| op.kind() == "delete");
                }
                "array_pattern" => return true,
                // A member under a pair pattern is its value; a computed key sits under a
                // `computed_property_name`, which falls through to "not a write".
                "pair_pattern" => {}
                "assignment_pattern" | "object_assignment_pattern" => {
                    if p.child_by_field_name("left").map(|n| n.id()) != Some(child.id()) {
                        return false;
                    }
                }
                "object_pattern" => return true,
                "for_in_statement" => {
                    return p.child_by_field_name("left").map(|n| n.id()) == Some(child.id());
                }
                _ => return false,
            }
            child = p;
            parent = p.parent();
        }
        false
    }
}

/// Per-walk constants of `formal_member_facts`.
struct FactsWalk<'a> {
    name: &'a str,
    def_scope: Option<usize>,
    seam_open: bool,
    memo: ScopeBindMemo,
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

/// Peel transparent wrappers off an expression. `None` when a wrapper has no inner
/// expression. A `type_assertion` (`<T>e`) carries its expression last; the others first.
fn strip_transparent_wrappers(mut node: Node<'_>) -> Option<Node<'_>> {
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
