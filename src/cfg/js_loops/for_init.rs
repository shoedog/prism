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
//! A write affects `D` when its path is a prefix of `D`'s (RD's relation; `[]` may match any component, and an
//! unknown shape falls back to the names it mentions). A repeated write excuses the replay only when it is a
//! known static path, a MUST-write of the owner's eager test or update (never conditional, deferred or nested in
//! a call), a prefix of `D` with every component spelled out, and its root names one binding in the loop. The
//! test and the update both run on the route the replay would otherwise kill (body end or `continue`, then
//! update, then test). Pure syntax: no solver is asked, so there is no cycle with admission.
use super::Cx;
use std::collections::BTreeSet;
use tree_sitter::Node;

/// A write target: its static path when it is one (`a.b[]`: `[]` is a computed component), and every
/// identifier it mentions (the over-approximation used when a shape has no path).
struct Target {
    path: Option<Vec<String>>,
    names: BTreeSet<String>,
    /// The write is a declaration (a declarator or a `let`/`const`/`var` loop head), not an assignment.
    declares: bool,
    /// A member or subscript write whose chain has no static path (a call, a tagged template, ...): it cannot be
    /// told from any other write, so it overlaps everything.
    opaque: bool,
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
        if init.kind() == "lexical_declaration" {
            // Binding positions only: the declarator NAMES, never a write inside an initial value
            // (`let i = (x = 0)` writes `x`; it does not bind it).
            let mut names = BTreeSet::new();
            let mut cursor = init.walk();
            for d in init.named_children(&mut cursor) {
                if let Some(name) = d.child_by_field_name("name") {
                    self.target_names(name, &mut names);
                }
            }
            body_writes.retain(|t| !(t.declares && t.names.iter().all(|n| names.contains(n))));
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
                && !d.path.as_ref().is_some_and(|dp| {
                    repeated
                        .iter()
                        .any(|w| must_affect(w, dp) && clean.single(&w[0]))
                })
        })
    }

    /// Every write under `node` (assignment, augmented assignment, update, declarator and `for`..`in` target),
    /// nested callables included: a deferred write is still a hazard source and a body definition.
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
                let mut names = BTreeSet::new();
                self.target_names(t, &mut names);
                out.push(Target {
                    path: self.static_path(t),
                    opaque: self.static_path(t).is_none()
                        && matches!(peel(t).kind(), "member_expression" | "subscript_expression"),
                    names,
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

    /// The identifiers a write target writes or hangs off, through patterns.
    fn target_names(&self, t: Node<'tree>, out: &mut BTreeSet<String>) {
        let text = |n: Node<'_>| self.parsed.source[n.start_byte()..n.end_byte()].to_string();
        match t.kind() {
            "identifier" | "shorthand_property_identifier_pattern" => {
                out.insert(text(t));
            }
            "member_expression" | "subscript_expression" => {
                // The written element belongs to the object: an index or property expression is only read.
                if let Some(object) = t.child_by_field_name("object") {
                    self.chain_names(object, out);
                }
            }
            "assignment_pattern" | "object_assignment_pattern" => {
                // `a = default`: the written side only.
                if let Some(left) = t.child_by_field_name("left") {
                    self.target_names(left, out);
                }
            }
            "array_pattern" | "object_pattern" | "rest_pattern" | "parenthesized_expression" => {
                let mut cursor = t.walk();
                for c in t.named_children(&mut cursor) {
                    match c.kind() {
                        "property_identifier" => {}
                        // `{ key: value }`: the value side is written.
                        "pair_pattern" => {
                            if let Some(v) = c.child_by_field_name("value") {
                                self.target_names(v, out);
                            }
                        }
                        _ => self.target_names(c, out),
                    }
                }
            }
            _ => {
                let mut stack = vec![t];
                while let Some(n) = stack.pop() {
                    if matches!(
                        n.kind(),
                        "identifier" | "shorthand_property_identifier_pattern"
                    ) {
                        out.insert(text(n));
                    }
                    let mut cursor = n.walk();
                    stack.extend(n.named_children(&mut cursor));
                }
            }
        }
    }

    /// The identifiers a member chain's object hangs off: through member and subscript objects, a call's callee
    /// (never its arguments) and wrappers; every identifier of any other shape (an assignment, a literal).
    fn chain_names(&self, n: Node<'tree>, out: &mut BTreeSet<String>) {
        let next = match n.kind() {
            "member_expression" | "subscript_expression" => n.child_by_field_name("object"),
            "call_expression" => n.child_by_field_name("function"),
            "non_null_expression"
            | "parenthesized_expression"
            | "as_expression"
            | "satisfies_expression" => {
                let mut cursor = n.walk();
                let first = n.named_children(&mut cursor).next();
                first
            }
            _ => None,
        };
        if let Some(next) = next {
            return self.chain_names(next, out);
        }
        let mut stack = vec![n];
        while let Some(x) = stack.pop() {
            if x.kind() == "identifier" {
                out.insert(self.parsed.source[x.start_byte()..x.end_byte()].to_string());
            }
            let mut cursor = x.walk();
            stack.extend(x.named_children(&mut cursor));
        }
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
    fn necessary_writes(&self, n: Node<'tree>, out: &mut Vec<Vec<String>>) {
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
                if let Some(path) = n
                    .child_by_field_name("left")
                    .and_then(|l| self.exact_path(l))
                {
                    out.push(path);
                }
                if let Some(right) = n.child_by_field_name("right") {
                    self.necessary_writes(right, out);
                }
            }
            "update_expression" => {
                if let Some(path) = n
                    .child_by_field_name("argument")
                    .and_then(|a| self.exact_path(a))
                {
                    out.push(path);
                }
            }
            "unary_expression" => {
                if let Some(path) = self.deleted_member(n).and_then(|m| self.exact_path(m)) {
                    out.push(path);
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

/// Whether the write of `i` can affect the definition `d`: `i` is a prefix of `d` (a `[]` component matches any
/// component); a shape without a static path falls back to a shared identifier.
fn may_affect(i: &Target, d: &Target) -> bool {
    match (&i.path, &d.path) {
        // The precise census paths, and RD's recorded ones (an lvalue is cut at its first `[`: `o[k].a` and
        // `o[k].b` are both recorded `o.[]`, and equal recorded paths kill each other).
        (Some(ip), Some(dp)) => {
            prefix_compatible(ip, dp) || prefix_compatible(recorded(ip), recorded(dp))
        }
        // An opaque chain, or a shape that names nothing, overlaps everything (fail closed).
        _ => {
            i.opaque
                || d.opaque
                || i.names.is_empty()
                || d.names.is_empty()
                || !i.names.is_disjoint(&d.names)
        }
    }
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

/// The path as RD records it: cut after the first `[]`.
fn recorded(p: &[String]) -> &[String] {
    p.iter().position(|c| c == "[]").map_or(p, |k| &p[..=k])
}

/// Whether the repeated write `w` certainly kills `d`: every component of `w` is spelled out and equals `d`'s.
fn must_affect(w: &[String], d: &[String]) -> bool {
    w.len() <= d.len() && w.iter().zip(d).all(|(a, b)| a == b && a != "[]")
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
