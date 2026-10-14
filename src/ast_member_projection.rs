//! PR-C: one body walk per member-only formal. Collects, in a single pass, the static
//! member paths read through the formal (with their admitted occurrence spans) and every
//! member write that targets the formal, so the DFG builder neither re-walks the callable
//! per path nor leans on the shared assignment query (which other passes use).
//!
//! The walk covers the callable BODY only (by bytes). Parameter-list default expressions
//! are therefore never projection targets, on the declaration line or any other line.

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

    /// PR-C: member paths read through the formal `name` and member writes to it, in one
    /// pass over the callable body. `def_byte` is the formal's identifier start byte (the
    /// binder-fence anchor). Refused formals (eval/arguments/seam in a callback pass) never
    /// reach this point: the DFG builder registers no Def for them.
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
        let def_scope = self.js_ts_def_scope_id(def_byte, name);
        self.collect_member_facts(body, func_node, name, def_scope, false, &mut facts);
        facts
    }

    fn collect_member_facts(
        &self,
        node: Node<'_>,
        owner: &Node<'_>,
        name: &str,
        def_scope: Option<usize>,
        inside_chain: bool,
        facts: &mut FormalMemberFacts,
    ) {
        let mut in_chain = false;
        if node.kind() == "member_expression" {
            if self.language.is_in_erased_type_context(node) {
                // Erased TS positions (`typeof p.x`, interface members) are not reads.
                return;
            }
            if self.member_chain_root_is(node, name) {
                in_chain = true;
                let path = self
                    .bounded_member_path(node)
                    .map(|member| member.path)
                    .unwrap_or_else(|| AccessPath::from_expr(self.node_text(&node)));
                if !path.is_simple() && path.base == name {
                    let bound = !self.js_ts_reference_fenced_against(&node, owner, name, def_scope);
                    if self.member_is_write_target(node) {
                        if bound {
                            facts.record_write(path);
                        }
                    } else {
                        if !inside_chain {
                            facts.outermost.insert(path.clone());
                        }
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
            self.collect_member_facts(child, owner, name, def_scope, in_chain, facts);
        }
    }

    /// `p.x.y`: following `object` fields from `node` ends at the identifier `name`.
    fn member_chain_root_is(&self, node: Node<'_>, name: &str) -> bool {
        let mut root = node;
        while root.kind() == "member_expression" {
            match root.child_by_field_name("object") {
                Some(object) => root = object,
                None => return false,
            }
        }
        root.kind() == "identifier" && self.node_text(&root) == name
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
