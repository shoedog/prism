//! #693/#697 slice 4 (F3): the path-level initializer replay criterion for a `for` loop.
//!
//! A rewired tree draws the back route through the loop's header node, which also holds the initializer, so the
//! initializer's writes are replayed there and would kill a body definition the real loop keeps. The criterion
//! (design §5) is per eager body definition `D`:
//!
//! ```text
//! hazard(D) = some initializer write can affect D
//!             AND no proven repeated write necessarily affects D on the continue route
//! ```
//!
//! Identities are RD's own: each written target is enumerated by the lvalue extraction the def producer uses
//! (`ParsedFile::recorded_write_paths`, pattern leaves one by one) and carries the `AccessPath`s RD records,
//! plus the census identity (`static_path`, wrappers peeled as the census peels them). A write can affect `D`
//! when RD's `affects` relation can hold between recorded paths (a prefix, `[]` matching any component) or the
//! census identities relate; a target with no recorded path is unknown and affects everything. A repeated write
//! excuses the replay only when its recorded path AND its census identity both prove one exact path that is a
//! prefix of every recorded path of `D`, it is a MUST-write of the test or update (never conditional, deferred
//! or nested in a call), and its root names one binding in the loop. No solver is asked, so there is no cycle
//! with admission.
use super::Cx;
use crate::access_path::{AccessPath, ARRAY_FIELD};
use std::collections::BTreeSet;
use tree_sitter::Node;

/// A write target: the paths RD records for it and its census identity when it has one (`a.b[]`: `[]` is a
/// computed component).
struct Target {
    recorded: Vec<AccessPath>,
    census: Option<Vec<String>>,
    /// The write is a declaration (a declarator or a `let`/`const`/`var` loop head), not an assignment.
    declares: bool,
}

impl<'a, 'tree> Cx<'a, 'tree> {
    /// Whether a `for` initializer's replay at the shared header node would kill a body definition that
    /// no repeated write of the test or update kills too.
    pub(super) fn for_init_hazard(&self, node: Node<'tree>) -> bool {
        let Some(init) = node.child_by_field_name("initializer") else {
            return false;
        };
        let mut init_writes = Vec::new();
        self.collect_targets(init, &mut init_writes);
        if init_writes.is_empty() {
            return false;
        }
        let mut body_writes = Vec::new();
        if let Some(body) = node.child_by_field_name("body") {
            self.collect_targets(body, &mut body_writes);
        }
        // A `let`/`const` header binding cannot be redeclared in the body, so a declaration of the same name
        // there (an inner `for (const i of xs)`, a `let i` in a block) is another binding: not a body write of it.
        // Binding positions only: the declarator NAMES, never a write inside an initial value.
        if init.kind() == "lexical_declaration" {
            let mut cursor = init.walk();
            let names: BTreeSet<String> = init
                .named_children(&mut cursor)
                .filter_map(|d| d.child_by_field_name("name"))
                .flat_map(|n| self.parsed.recorded_write_paths(n))
                .map(|p| p.base)
                .collect();
            body_writes.retain(|t| {
                !(t.declares
                    && !t.recorded.is_empty()
                    && t.recorded
                        .iter()
                        .all(|r| r.fields.is_empty() && names.contains(&r.base)))
            });
        }
        if body_writes.is_empty() {
            return false;
        }
        let mut repeated = Vec::new();
        for field in ["condition", "increment"] {
            if let Some(n) = node.child_by_field_name(field) {
                self.necessary_writes(n, &mut repeated);
            }
        }
        let clean = Bindings::of(self, node, init);
        body_writes.iter().any(|d| {
            init_writes.iter().any(|i| may_affect(i, d))
                && !repeated.iter().any(|&w| self.proves(w, d, &clean))
        })
    }

    /// Whether the repeated write `w` certainly kills `d`: its recorded path and census identity agree on one
    /// exact path (every component spelled out) that is a prefix of the census identity and of every recorded
    /// path of `d`, on a root that names one binding in the loop.
    fn proves(&self, w: Node<'tree>, d: &Target, clean: &Bindings) -> bool {
        let (Some(pw), Some(pd)) = (self.exact_path(w), d.census.as_ref()) else {
            return false;
        };
        let recorded = self.parsed.recorded_write_paths(w);
        let exact = AccessPath {
            base: pw[0].clone(),
            fields: pw[1..].to_vec(),
        };
        pw.len() <= pd.len()
            && pw.iter().zip(pd).all(|(a, b)| a == b && a != ARRAY_FIELD)
            && recorded == [exact.clone()]
            && !d.recorded.is_empty()
            && d.recorded.iter().all(|r| exact.is_prefix_of(r))
            && clean.single(&exact.base)
    }

    /// Every write under `node` (assignment, augmented assignment, update, declarator, `for`..`in` target and
    /// member `delete`), nested callables included: a deferred write is still a hazard source and a body
    /// definition. Each is enumerated as RD's producer enumerates lvalues.
    fn collect_targets(&self, node: Node<'tree>, out: &mut Vec<Target>) {
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            let target = match n.kind() {
                "assignment_expression" | "augmented_assignment_expression" => {
                    n.child_by_field_name("left")
                }
                "update_expression" => n.child_by_field_name("argument"),
                "variable_declarator" => n.child_by_field_name("name"),
                "for_in_statement" => n.child_by_field_name("left"),
                "unary_expression" => self.deleted_member(n),
                _ => None,
            };
            if let Some(t) = target {
                out.push(Target {
                    recorded: self.parsed.recorded_write_paths(t),
                    census: self.static_path(t),
                    declares: n.kind() == "variable_declarator"
                        || (n.kind() == "for_in_statement"
                            && n.child_by_field_name("kind").is_some()),
                });
            }
            let mut cursor = n.walk();
            stack.extend(n.named_children(&mut cursor));
        }
    }

    /// The member or subscript a `delete` removes (`delete o.x`, `delete o[k]`, wrappers peeled); `delete name`
    /// is not a write of a tracked definition.
    fn deleted_member(&self, n: Node<'tree>) -> Option<Node<'tree>> {
        let op = n.child_by_field_name("operator")?;
        if &self.parsed.source[op.start_byte()..op.end_byte()] != "delete" {
            return None;
        }
        let arg = peel(n.child_by_field_name("argument")?);
        matches!(arg.kind(), "member_expression" | "subscript_expression").then_some(arg)
    }

    /// The static path of a write target: an identifier, `this`, or a member chain over those, wrappers peeled as
    /// the write census peels them. `exact` is cleared when a wrapper or an optional chain was crossed: the
    /// census identity and RD's recorded path may then disagree, so the path may be a hazard source or a body
    /// definition but never an exemption.
    fn static_path_with(&self, t: Node<'tree>, exact: &mut bool) -> Option<Vec<String>> {
        let text = |n: Node<'_>| self.parsed.source[n.start_byte()..n.end_byte()].to_string();
        match t.kind() {
            "identifier" | "this" => Some(vec![text(t)]),
            "parenthesized_expression"
            | "non_null_expression"
            | "as_expression"
            | "satisfies_expression"
            | "type_assertion" => {
                *exact = false;
                self.static_path_with(operand(t)?, exact)
            }
            "member_expression" => {
                let mut cursor = t.walk();
                if t.children(&mut cursor)
                    .any(|c| c.kind() == "optional_chain")
                {
                    *exact = false;
                }
                let mut path = self.static_path_with(t.child_by_field_name("object")?, exact)?;
                path.push(text(t.child_by_field_name("property")?));
                Some(path)
            }
            "subscript_expression" => {
                let mut path = self.static_path_with(t.child_by_field_name("object")?, exact)?;
                path.push("[]".to_string());
                Some(path)
            }
            _ => None,
        }
    }

    fn static_path(&self, t: Node<'tree>) -> Option<Vec<String>> {
        self.static_path_with(t, &mut true)
    }

    /// The path only when the census identity and RD's recorded path certainly agree.
    fn exact_path(&self, t: Node<'tree>) -> Option<Vec<String>> {
        let mut exact = true;
        let path = self.static_path_with(t, &mut exact)?;
        exact.then_some(path)
    }

    /// The paths an expression of the test or update NECESSARILY writes each time it is evaluated: only
    /// through parentheses, comma sequences, the right side of an assignment, non-short-circuit binary
    /// operands and the left side of `&&`, `||`, `??`. A call, conditional, callable or pattern stops it.
    fn necessary_writes(&self, n: Node<'tree>, out: &mut Vec<Node<'tree>>) {
        let op = |n: Node<'_>| {
            n.child_by_field_name("operator")
                .map(|o| self.parsed.source[o.start_byte()..o.end_byte()].to_string())
                .unwrap_or_default()
        };
        let mut cursor = n.walk();
        let children: Vec<_> = n.named_children(&mut cursor).collect();
        match n.kind() {
            "expression_statement" | "parenthesized_expression" | "sequence_expression" => {
                children
                    .into_iter()
                    .for_each(|c| self.necessary_writes(c, out));
            }
            "assignment_expression" | "augmented_assignment_expression" => {
                if matches!(op(n).as_str(), "&&=" | "||=" | "??=") {
                    return;
                }
                if let Some(left) = n.child_by_field_name("left") {
                    out.push(left);
                }
                if let Some(right) = n.child_by_field_name("right") {
                    self.necessary_writes(right, out);
                }
            }
            "update_expression" => {
                if let Some(arg) = n.child_by_field_name("argument") {
                    out.push(arg);
                }
            }
            "unary_expression" => {
                if let Some(member) = self.deleted_member(n) {
                    out.push(member);
                }
            }
            "binary_expression" => {
                let short = matches!(op(n).as_str(), "&&" | "||" | "??");
                for (i, c) in children.into_iter().enumerate() {
                    if i == 0 || !short {
                        self.necessary_writes(c, out);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Whether the write of `i` can affect the definition `d`: RD's `affects` relation can hold between a pair of
/// recorded paths (a prefix, `[]` matching any component), or the census identities relate; a target RD records
/// no path for is unknown and affects everything (fail closed).
fn may_affect(i: &Target, d: &Target) -> bool {
    if i.recorded.is_empty() || d.recorded.is_empty() {
        return true;
    }
    i.recorded.iter().any(|ri| {
        d.recorded
            .iter()
            .any(|rd| ri.base == rd.base && prefix_compatible(&ri.fields, &rd.fields))
    }) || matches!((&i.census, &d.census), (Some(ci), Some(cd)) if prefix_compatible(ci, cd))
}

/// The wrapped operand the write census sees through: comments are skipped, and a parenthesized, `as`,
/// `satisfies`, `<T>` or non-null expression is unwrapped (`js_write_census/walk.rs::wrap_operand`).
fn operand(n: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = n.walk();
    let kids: Vec<Node<'_>> = n
        .named_children(&mut cursor)
        .filter(|c| c.kind() != "comment")
        .collect();
    match n.kind() {
        "type_assertion" => kids.last().copied(),
        "as_expression" | "satisfies_expression" => kids.first().copied(),
        _ if kids.len() == 1 => kids.first().copied(),
        _ => None,
    }
}

/// `n` with every census wrapper unwrapped.
fn peel(mut n: Node<'_>) -> Node<'_> {
    while matches!(
        n.kind(),
        "parenthesized_expression"
            | "non_null_expression"
            | "as_expression"
            | "satisfies_expression"
            | "type_assertion"
    ) {
        match operand(n) {
            Some(inner) => n = inner,
            None => break,
        }
    }
    n
}

/// `ip` is a prefix of `dp`, a `[]` component matching any component.
fn prefix_compatible(ip: &[String], dp: &[String]) -> bool {
    ip.len() <= dp.len()
        && ip
            .iter()
            .zip(dp)
            .all(|(a, b)| a == b || a == "[]" || b == "[]")
}

/// The names declared inside a loop, to prove a root names one binding there.
struct Bindings {
    total: std::collections::BTreeMap<String, usize>,
    in_init: std::collections::BTreeMap<String, usize>,
}

impl Bindings {
    fn of(cx: &Cx<'_, '_>, node: Node<'_>, init: Node<'_>) -> Self {
        let (total, in_init) = (Self::count(cx, node), Self::count(cx, init));
        Self { total, in_init }
    }

    /// A root names one binding when the loop declares it nowhere, or only in its own initializer.
    fn single(&self, root: &str) -> bool {
        let total = self.total.get(root).copied().unwrap_or(0);
        total == 0 || (total == 1 && self.in_init.get(root) == Some(&1))
    }

    /// Identifier occurrences in declaring positions under `node`: declarator names, parameters, `catch`
    /// parameters, function and class names, and `let`/`const`/`var` loop heads.
    fn count(cx: &Cx<'_, '_>, node: Node<'_>) -> std::collections::BTreeMap<String, usize> {
        let mut counts = std::collections::BTreeMap::new();
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            let declaring: Vec<Node<'_>> = match n.kind() {
                "variable_declarator" => n.child_by_field_name("name").into_iter().collect(),
                "formal_parameters" => vec![n],
                "arrow_function" => n.child_by_field_name("parameter").into_iter().collect(),
                "catch_clause" => n.child_by_field_name("parameter").into_iter().collect(),
                "function_declaration"
                | "class_declaration"
                | "function_expression"
                | "generator_function_declaration"
                | "generator_function"
                | "class" => n.child_by_field_name("name").into_iter().collect(),
                "for_in_statement" if n.child_by_field_name("kind").is_some() => {
                    n.child_by_field_name("left").into_iter().collect()
                }
                _ => Vec::new(),
            };
            for d in declaring {
                let mut inner = vec![d];
                while let Some(x) = inner.pop() {
                    if matches!(
                        x.kind(),
                        "identifier" | "shorthand_property_identifier_pattern"
                    ) {
                        *counts
                            .entry(cx.parsed.source[x.start_byte()..x.end_byte()].to_string())
                            .or_insert(0) += 1;
                    }
                    let mut cursor = x.walk();
                    inner.extend(x.named_children(&mut cursor));
                }
            }
            let mut cursor = n.walk();
            stack.extend(n.named_children(&mut cursor));
        }
        counts
    }
}
