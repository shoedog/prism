//! S1b (SPEC §3.1 D, P rows): the declarations a scope's statement list holds, by one walk.
#![allow(dead_code)] // S1b-2b and S1b-3 wire these; remove the allow there
use super::js_binding::{is_class, Index, Strictness};
use super::{is_js_ts_function_like, ParsedFile};
use std::collections::BTreeSet;
use tree_sitter::Node;

/// Kinds whose `name` field declares that name (its first segment, for a dotted namespace)
/// in the enclosing statement list (D1, D2, D6).
const DECLARES_NAME: [&str; 7] = [
    "function_declaration",
    "generator_function_declaration",
    "class_declaration",
    "abstract_class_declaration",
    "internal_module",
    "module",
    "enum_declaration",
];

/// Statement wrappers the walk stays direct through; any other statement makes its subtree
/// indirect (SPEC §3.1 "Declarations in a scope").
const TRANSPARENT: [&str; 8] = [
    "export_statement",
    "expression_statement",
    "labeled_statement",
    "switch_case",
    "switch_default",
    "ambient_declaration",
    "lexical_declaration",
    "variable_declaration",
];

impl ParsedFile {
    /// Declarations under `node`: lexical ones while `direct` (still in the scope's own
    /// statement list), `var` ones at any depth while `hoist` (D7). A hoisted `var` whose name
    /// a `catch` parameter binds (`taint`), or any under `with` (the empty name), records its
    /// declaration as a non-callable marker (Annex B.3.4). A block function reached while
    /// hoisting is an Annex B.3.2 marker (its list) only in sloppy code, and an unproven name
    /// in `annex` when strictness is unknown (D1). Functions, classes, static blocks,
    /// namespace and enum bodies and decorators are their own scopes: the walk stops there.
    pub(super) fn js_ts_declare_walk<'a>(
        &'a self,
        node: Node<'a>,
        (direct, hoist, strict): (bool, bool, Strictness),
        taint: &BTreeSet<String>,
        out: &mut Index<'a>,
        annex: &mut BTreeSet<String>,
    ) {
        let mut cursor = node.walk();
        for ch in node.named_children(&mut cursor) {
            let kind = ch.kind();
            let mut add = |name: String, at: Node<'a>| out.entry(name).or_default().push(at);
            if is_js_ts_function_like(kind)
                || is_class(kind)
                || matches!(
                    kind,
                    "class_static_block"
                        | "enum_declaration"
                        | "decorator"
                        | "internal_module"
                        | "module"
                )
            {
                let name = ch
                    .child_by_field_name("name")
                    .filter(|_| DECLARES_NAME.contains(&kind))
                    .map(|n| {
                        let text = self.node_text(&n);
                        text.split('.').next().unwrap_or(text).trim().to_string()
                    });
                let mut c = ch.walk();
                let ordinary = kind == "function_declaration"
                    && !ch.children(&mut c).any(|t| t.kind() == "async");
                match name {
                    Some(n) if direct => add(n, ch),
                    Some(n) if hoist && ordinary && strict == Strictness::Sloppy => add(n, node),
                    Some(n) if hoist && ordinary && strict == Strictness::Unknown => {
                        annex.insert(n);
                    }
                    _ => {}
                }
                continue;
            }
            let marked = |n: &String| taint.contains(n) || taint.contains("");
            match kind {
                "variable_declarator" => {
                    let mut names = BTreeSet::new();
                    if let Some(p) = ch.child_by_field_name("name") {
                        self.collect_js_ts_binding_pattern_names(p, &mut names);
                    }
                    for n in names {
                        let at = if marked(&n) { node } else { ch };
                        add(n, at);
                    }
                    continue;
                }
                "variable_declaration" if !hoist => continue,
                "lexical_declaration" if !direct => continue,
                // `declare global { … }` augments the global scope, not the module (Opus r1 W4).
                "statement_block" if node.kind() == "ambient_declaration" => continue,
                // D6: type space declares nothing in value space.
                "interface_declaration" | "type_alias_declaration" => continue,
                "import_statement" | "import_alias" => {
                    if direct {
                        for n in self.js_ts_import_value_names(ch) {
                            add(n, ch);
                        }
                    }
                    continue;
                }
                // D6: `declare function f()` declares `f`; an overload signature does not.
                "function_signature" => {
                    let name = ch.child_by_field_name("name");
                    if let Some(n) = name.filter(|_| direct && node.kind() == "ambient_declaration")
                    {
                        add(self.node_text(&n).to_string(), ch);
                    }
                    continue;
                }
                // D7: `for (var … in/of …)` declares its head (not callable).
                "for_in_statement" if hoist => {
                    let is_var = ch
                        .child_by_field_name("kind")
                        .is_some_and(|k| self.node_text(&k) == "var");
                    let mut names = BTreeSet::new();
                    if let Some(left) = ch.child_by_field_name("left").filter(|_| is_var) {
                        self.collect_js_ts_binding_pattern_names(left, &mut names);
                    }
                    for n in names {
                        add(n, ch);
                    }
                }
                "catch_clause" => {
                    let mut names = taint.clone();
                    if let Some(p) = ch.child_by_field_name("parameter") {
                        self.collect_js_ts_binding_pattern_names(p, &mut names);
                    }
                    if let Some(body) = ch.child_by_field_name("body") {
                        self.js_ts_declare_walk(body, (false, hoist, strict), &names, out, annex);
                    }
                    continue;
                }
                "with_statement" => {
                    let names = BTreeSet::from([String::new()]);
                    if let Some(body) = ch.child_by_field_name("body") {
                        self.js_ts_declare_walk(body, (false, hoist, strict), &names, out, annex);
                    }
                    continue;
                }
                _ => {}
            }
            if let Some(using) = self.js_ts_using_declaration(ch).filter(|_| direct) {
                let mut names = BTreeSet::new();
                if let Some(left) = using.child_by_field_name("left") {
                    self.collect_js_ts_binding_pattern_names(left, &mut names);
                }
                for n in names {
                    add(n, using);
                }
                continue;
            }
            let mode = (direct && TRANSPARENT.contains(&kind), hoist, strict);
            self.js_ts_declare_walk(ch, mode, taint, out, annex);
        }
    }

    /// D4: TS `using x = e` / `await using x = e`. The grammar spells the declaration as an
    /// assignment expression carrying an anonymous `using` token.
    pub(super) fn js_ts_using_declaration<'a>(&self, stmt: Node<'a>) -> Option<Node<'a>> {
        let mut expr = stmt
            .named_child(0)
            .filter(|_| stmt.kind() == "expression_statement")?;
        if expr.kind() == "await_expression" {
            expr = expr.named_child(0)?;
        }
        let mut cursor = expr.walk();
        let using = expr.kind() == "assignment_expression"
            && expr
                .children(&mut cursor)
                .any(|c| !c.is_named() && c.kind() == "using");
        using.then_some(expr)
    }

    /// D5: the value names an `import` statement or an `import x = N.y` alias binds. An
    /// `import type` statement and `type` specifiers bind nothing in value space.
    pub(super) fn js_ts_import_value_names(&self, node: Node<'_>) -> Vec<String> {
        if node.kind() == "import_alias" {
            let first = node.named_child(0);
            return first
                .map(|n| self.node_text(&n).to_string())
                .into_iter()
                .collect();
        }
        if self.js_ts_import_statement_is_type_only(node) {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            let mut cursor = n.walk();
            for c in n.named_children(&mut cursor) {
                match c.kind() {
                    "identifier" => out.push(self.node_text(&c).to_string()),
                    "import_specifier" if !self.js_ts_import_specifier_is_type_only(c) => {
                        let local = c
                            .child_by_field_name("alias")
                            .or_else(|| c.child_by_field_name("name"));
                        out.extend(local.map(|l| self.node_text(&l).to_string()));
                    }
                    "import_clause"
                    | "namespace_import"
                    | "named_imports"
                    | "import_require_clause" => stack.push(c),
                    _ => {}
                }
            }
        }
        out
    }
}
