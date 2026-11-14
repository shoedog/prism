//! Value names `V(e)`, target roots and call names of the value-flow graph (`js_hold_flow.rs`).
use super::js_hold_flow::Flow;
use super::js_hold_resolve::{is_callable_kind, is_class_kind};
use crate::ast::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

pub(super) fn leaf_kind(kind: &str) -> bool {
    matches!(
        kind,
        "identifier"
            | "shorthand_property_identifier"
            | "shorthand_property_identifier_pattern"
            | "this"
            | "super"
    )
}

/// The wrappers a target or value is peeled through.
pub(super) fn peel(node: Node<'_>) -> Node<'_> {
    let mut cur = node;
    loop {
        let next = match cur.kind() {
            "parenthesized_expression"
            | "non_null_expression"
            | "as_expression"
            | "satisfies_expression" => cur.named_child(0),
            "type_assertion" => cur.named_child(cur.named_child_count().saturating_sub(1)),
            _ => None,
        };
        match next {
            Some(n) => cur = n,
            None => return cur,
        }
    }
}

/// The root of a member/subscript chain (wrappers peeled).
pub(super) fn chain_root(node: Node<'_>) -> Node<'_> {
    let mut cur = peel(node);
    while matches!(cur.kind(), "member_expression" | "subscript_expression") {
        match cur.child_by_field_name("object") {
            Some(o) => cur = peel(o),
            None => break,
        }
    }
    cur
}

const NAMESPACES: [&str; 10] = [
    "console", "Math", "JSON", "Number", "String", "Boolean", "Date", "Symbol", "Intl", "Atomics",
];

impl<'a, 't> Flow<'a, 't> {
    // ---- value names ----

    pub(super) fn fn_node(&mut self, start: usize) -> usize {
        self.intern(&format!("fn@{start}"), "")
    }

    pub(super) fn val_all(&mut self, node: Node<'_>) -> Vec<usize> {
        let mut out = Vec::new();
        let mut c = node.walk();
        for child in node.named_children(&mut c) {
            out.extend(self.val(child));
        }
        out
    }

    /// `V(e)`: the nodes an expression can evaluate to or contain.
    pub(super) fn val(&mut self, node: Node<'_>) -> Vec<usize> {
        let kind = node.kind();
        if leaf_kind(kind) {
            return vec![self.ident_of(node).0];
        }
        match kind {
            "number" | "string" | "true" | "false" | "null" | "undefined" | "regex"
            | "template_string" | "meta_property" | "comment" | "unary_expression"
            | "update_expression" => Vec::new(),
            "member_expression" | "subscript_expression" => node
                .child_by_field_name("object")
                .map(|o| self.val(o))
                .unwrap_or_default(),
            "parenthesized_expression"
            | "non_null_expression"
            | "as_expression"
            | "satisfies_expression"
            | "await_expression"
            | "spread_element"
            | "type_assertion" => {
                let p = peel(node);
                if p.id() != node.id() {
                    self.val(p)
                } else {
                    node.named_child(0).map(|c| self.val(c)).unwrap_or_default()
                }
            }
            "sequence_expression" => {
                let last = node.named_child(node.named_child_count().saturating_sub(1));
                last.map(|r| self.val(r)).unwrap_or_default()
            }
            "ternary_expression" => {
                let mut out = Vec::new();
                for f in ["consequence", "alternative"] {
                    if let Some(c) = node.child_by_field_name(f) {
                        out.extend(self.val(c));
                    }
                }
                out
            }
            "binary_expression" => {
                let op = node.child_by_field_name("operator").map(|o| o.kind());
                if matches!(op, Some("||" | "&&" | "??")) {
                    let mut out = Vec::new();
                    for f in ["left", "right"] {
                        if let Some(c) = node.child_by_field_name(f) {
                            out.extend(self.val(c));
                        }
                    }
                    out
                } else {
                    Vec::new()
                }
            }
            "assignment_expression" | "augmented_assignment_expression" => {
                let mut out = node
                    .child_by_field_name("right")
                    .map(|r| self.val(r))
                    .unwrap_or_default();
                let logical = node
                    .child_by_field_name("operator")
                    .is_some_and(|o| matches!(o.kind(), "||=" | "&&=" | "??="));
                if logical {
                    if let Some(l) = node.child_by_field_name("left") {
                        out.extend(self.val(l));
                    }
                }
                out
            }
            "yield_expression" => {
                let f = self.enclosing_callable(node);
                let mut out = vec![self.fn_node(f.start_byte())];
                out.extend(self.val_all(node));
                out
            }
            "call_expression" | "new_expression" => self.call_names(node).0,
            "object" => {
                let mut out = Vec::new();
                let mut c = node.walk();
                let children: Vec<Node<'_>> = node.named_children(&mut c).collect();
                for ch in children {
                    match ch.kind() {
                        "pair" => {
                            if let Some(v) = ch.child_by_field_name("value") {
                                out.extend(self.val(v));
                            }
                        }
                        "method_definition" => out.push(self.fn_node(ch.start_byte())),
                        _ => out.extend(self.val(ch)),
                    }
                }
                if !self.object_methods(node).is_empty() {
                    out.push(self.intern(&format!("lit@{}", node.start_byte()), ""));
                }
                out
            }
            k if is_callable_kind(k) => vec![self.fn_node(node.start_byte())],
            k if is_class_kind(k) => vec![self.intern(&format!("cls@{}", node.start_byte()), "")],
            _ => self.val_all(node),
        }
    }

    pub(super) fn object_methods<'n>(&self, node: Node<'n>) -> Vec<Node<'n>> {
        let mut out = Vec::new();
        let mut c = node.walk();
        for ch in node.named_children(&mut c) {
            match ch.kind() {
                "method_definition" => out.push(ch),
                "pair" => {
                    if let Some(v) = ch.child_by_field_name("value") {
                        let v = peel(v);
                        if is_callable_kind(v.kind()) && v.kind() != "arrow_function" {
                            out.push(v);
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }

    pub(super) fn enclosing_callable(&self, node: Node<'t>) -> Node<'t> {
        let mut cur = node.parent();
        while let Some(p) = cur {
            if is_callable_kind(p.kind()) || p.id() == self.owner.id() {
                return p;
            }
            cur = p.parent();
        }
        self.owner
    }

    pub(super) fn in_callable(&self, node: Node<'t>) -> bool {
        let mut cur = node.parent();
        while let Some(p) = cur {
            if p.id() == self.owner.id() {
                return false;
            }
            if is_callable_kind(p.kind()) {
                return true;
            }
            cur = p.parent();
        }
        false
    }

    /// The roots of a write or binding target: leaves of patterns, the chain root of a member target.
    pub(super) fn roots(&mut self, node: Node<'_>) -> Vec<usize> {
        let node = peel(node);
        match node.kind() {
            k if leaf_kind(k) => vec![self.ident_of(node).0],
            "member_expression" | "subscript_expression" => {
                let r = chain_root(node);
                if leaf_kind(r.kind()) {
                    vec![self.ident_of(r).0]
                } else {
                    self.val(r)
                }
            }
            "pair_pattern" => node
                .child_by_field_name("value")
                .map(|v| self.roots(v))
                .unwrap_or_default(),
            "assignment_pattern" | "object_assignment_pattern" => node
                .child_by_field_name("left")
                .map(|v| self.roots(v))
                .unwrap_or_default(),
            "array_pattern" | "object_pattern" | "rest_pattern" | "array" | "object"
            | "spread_element" | "pair" => {
                let mut out = Vec::new();
                let mut c = node.walk();
                let children: Vec<Node<'_>> = node.named_children(&mut c).collect();
                for ch in children {
                    out.extend(self.roots(ch));
                }
                out
            }
            _ => self.val(node),
        }
    }

    pub(super) fn call_args(&mut self, node: Node<'_>) -> Vec<usize> {
        let Some(args) = node.child_by_field_name("arguments") else {
            return Vec::new();
        };
        if args.kind() == "template_string" {
            let mut out = Vec::new();
            let mut c = args.walk();
            let subs: Vec<Node<'_>> = args
                .named_children(&mut c)
                .filter(|s| s.kind() == "template_substitution")
                .collect();
            for s in subs {
                out.extend(self.val_all(s));
            }
            return out;
        }
        self.val_all(args)
    }

    /// The names of a call (callee value, receiver, arguments) and whether the call is opaque (joins nothing).
    pub(super) fn call_names(&mut self, node: Node<'_>) -> (Vec<usize>, bool) {
        let new = node.kind() == "new_expression";
        let callee = node
            .child_by_field_name(if new { "constructor" } else { "function" })
            .map(peel_keep_parens);
        let mut names = Vec::new();
        let mut opaque = false;
        if let Some(c) = callee {
            names.extend(self.val(c));
            let tagged = node
                .child_by_field_name("arguments")
                .is_some_and(|a| a.kind() == "template_string");
            if !new && !tagged && c.kind() == "identifier" {
                let id = self.res.ident(self.parsed, c, self.owner);
                opaque = self.res.is_opaque(&id);
            } else if !new && !tagged && c.kind() == "member_expression" {
                opaque = self.namespace_call(c);
            }
        }
        names.extend(self.call_args(node));
        (names, opaque)
    }

    /// R-NS: `console.log(..)`, `Math.max(..)`: a member call on a free built-in namespace that no binding or
    /// import of the file replaces stores nothing.
    pub(super) fn namespace_call(&mut self, member: Node<'_>) -> bool {
        let Some(obj) = member.child_by_field_name("object") else {
            return false;
        };
        if obj.kind() != "identifier" {
            return false;
        }
        let name = self.parsed.node_text(&obj).trim().to_string();
        if !NAMESPACES.contains(&name.as_str()) {
            return false;
        }
        if !self.res.key(&name, obj.start_byte()).key.starts_with("f:") {
            return false;
        }
        if let Some(&ok) = self.rns_ok.get(&name) {
            return ok;
        }
        let ok = file_free_of(self.parsed, &name);
        self.rns_ok.insert(name, ok);
        ok
    }
}

/// Keeps parentheses (a parenthesised callee is not a plain identifier) but peels type wrappers.
pub(super) fn peel_keep_parens(node: Node<'_>) -> Node<'_> {
    let mut cur = node;
    while matches!(
        cur.kind(),
        "non_null_expression" | "as_expression" | "satisfies_expression"
    ) {
        match cur.named_child(0) {
            Some(n) => cur = n,
            None => break,
        }
    }
    cur
}

/// True when every occurrence of `name` in the file is the object of a member expression and the file has no `with`.
pub(super) fn file_free_of(parsed: &ParsedFile, name: &str) -> bool {
    use std::cell::RefCell;
    use std::hash::{Hash, Hasher};
    thread_local! {
        static CACHE: RefCell<BTreeMap<(String, u64, String), bool>> = const { RefCell::new(BTreeMap::new()) };
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    parsed.source.hash(&mut h);
    let key = (parsed.path.clone(), h.finish(), name.to_string());
    if let Some(v) = CACHE.with(|c| c.borrow().get(&key).copied()) {
        return v;
    }
    let mut ok = true;
    let mut cursor = parsed.tree.walk();
    'walk: loop {
        let n = cursor.node();
        if n.kind() == "with_statement" {
            ok = false;
            break;
        }
        if matches!(
            n.kind(),
            "identifier"
                | "shorthand_property_identifier"
                | "shorthand_property_identifier_pattern"
        ) && parsed.node_text(&n).trim() == name
        {
            let object_of_member = n.kind() == "identifier"
                && n.parent().is_some_and(|p| {
                    p.kind() == "member_expression"
                        && p.child_by_field_name("object").map(|o| o.id()) == Some(n.id())
                });
            if !object_of_member {
                ok = false;
                break;
            }
        }
        if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                break 'walk;
            }
        }
    }
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() > 64 {
            c.clear();
        }
        c.insert(key, ok);
    });
    ok
}
