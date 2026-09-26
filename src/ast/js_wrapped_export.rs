//! S1 (wrapped-export SPEC §3.1): admission of `export const X = W(fn…)` where `W` is
//! React `forwardRef`/`memo` reached through an ESM import of `"react"`. Admission proves
//! static provenance only (SPEC §3.2): what the callee name denotes, never what the React
//! module object holds at run time. The fact names the inner function by exact span.
use super::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

const WRAPPERS: [&str; 2] = ["forwardRef", "memo"];

/// Top-level ESM value imports of exactly `"react"`: named local -> imported name, and
/// the default/namespace object locals.
struct ReactImports {
    named: BTreeMap<String, String>,
    objects: BTreeSet<String>,
}

impl ParsedFile {
    /// Checks R2–R12 in SPEC order for a `call_expression` initializer (R1 is the caller's
    /// kind dispatch). `Ok` is the inner function's registered name and line span; `Err`
    /// is the first failed reason.
    pub(super) fn js_ts_wrapped_export(
        &self,
        decl: Node<'_>,
        declarator: Node<'_>,
    ) -> Result<(String, usize, usize), &'static str> {
        if decl.kind() != "lexical_declaration"
            || decl.child_by_field_name("kind").map(|k| k.kind()) != Some("const")
        {
            return Err("not_const");
        }
        // The `export_statement` half is belt-and-braces: no known reachable input.
        if decl.parent().is_some_and(|p| p.has_error()) || decl.has_error() {
            return Err("parse_recovery");
        }
        let root = self.tree.root_node();
        let mut rc = root.walk();
        // A malformed import may also be recovered as a top-level `ERROR` sibling whose
        // first token is `import`; an unrelated parse error elsewhere does not refuse.
        if root.children(&mut rc).any(|n| {
            (n.kind() == "import_statement" && n.has_error())
                || (n.is_error() && first_token(n).kind() == "import")
        }) {
            return Err("import_parse_recovery");
        }
        let call = declarator
            .child_by_field_name("value")
            .ok_or("non_call_initializer")?;
        let callee = call
            .child_by_field_name("function")
            .ok_or("callee_not_admitted")?;
        let react = self.react_imports();
        let (local, wrapper) = match callee.kind() {
            "identifier" => {
                let l = self.node_text(&callee).to_string();
                let w = react.named.get(&l).ok_or("callee_not_admitted")?.clone();
                (l, w)
            }
            "member_expression" => {
                let o = callee
                    .child_by_field_name("object")
                    .ok_or("callee_not_admitted")?;
                let p = callee
                    .child_by_field_name("property")
                    .ok_or("callee_not_admitted")?;
                let ot = self.node_text(&o).to_string();
                if o.kind() != "identifier"
                    || p.kind() != "property_identifier"
                    || !react.objects.contains(&ot)
                {
                    return Err("callee_not_admitted");
                }
                (ot, self.node_text(&p).to_string())
            }
            _ => return Err("callee_not_admitted"),
        };
        if !WRAPPERS.contains(&wrapper.as_str()) {
            return Err("callee_not_admitted");
        }
        // R6 P1–P5, in SPEC order: unique binding, value-typed, unwritten, uncompeted at
        // module scope, unescaped spelling.
        let imports = self.extract_import_bindings();
        if imports.iter().filter(|b| b.local == local).count() != 1
            || self.js_ts_type_only_imports().contains_key(&local)
            || self.js_ts_module_value_written(&local)
            || self.module_scope_declares(root, &local, false)
            || local.contains('\\')
        {
            return Err("callee_provenance");
        }
        let args = call.child_by_field_name("arguments").ok_or("arity")?;
        if args.kind() != "arguments" {
            return Err("arity");
        }
        let mut ac = args.walk();
        let named: Vec<Node<'_>> = args
            .named_children(&mut ac)
            .filter(|n| n.kind() != "comment")
            .collect();
        let max = if wrapper == "forwardRef" { 1 } else { 2 };
        if named.is_empty() || named.len() > max {
            return Err("arity");
        }
        let inner = named[0];
        if inner.kind() == "call_expression" {
            return Err("nested_wrapper");
        }
        if !matches!(inner.kind(), "arrow_function" | "function_expression") {
            return Err("first_arg_not_function");
        }
        let span = self.node_line_range(&inner);
        let name = self.language.function_name(&inner);
        let name = name.map(|n| self.node_text(&n).to_string());
        if let Some(cmp) = named.get(1) {
            if matches!(cmp.kind(), "arrow_function" | "function_expression")
                && self.language.function_name(cmp).map(|n| self.node_text(&n)) == name.as_deref()
                && self.node_line_range(cmp) == span
            {
                return Err("comparator_line_collision");
            }
        }
        // R12: unreachable given R10 and Pattern 3 of `Language::function_name`.
        let name = name.ok_or("inner_unnamed")?;
        Ok((name, span.0, span.1))
    }

    /// The ESM-only React import table (SPEC §3.1 R5). A hand parser over top-level
    /// `import_statement` nodes, so `require("react")` bindings can never enter it.
    fn react_imports(&self) -> ReactImports {
        let mut out = ReactImports {
            named: BTreeMap::new(),
            objects: BTreeSet::new(),
        };
        let root = self.tree.root_node();
        let mut rc = root.walk();
        for st in root.named_children(&mut rc) {
            if st.kind() != "import_statement" || self.js_ts_import_statement_is_type_only(st) {
                continue;
            }
            let Some(src) = st.child_by_field_name("source") else {
                continue;
            };
            if self.node_text(&src).trim_matches(['"', '\'']) != "react" {
                continue;
            }
            let mut sc = st.walk();
            for clause in st.named_children(&mut sc) {
                if clause.kind() != "import_clause" {
                    continue;
                }
                let mut cc = clause.walk();
                for c in clause.named_children(&mut cc) {
                    match c.kind() {
                        "identifier" => {
                            out.objects.insert(self.node_text(&c).to_string());
                        }
                        "namespace_import" => {
                            let mut nc = c.walk();
                            for id in c.named_children(&mut nc) {
                                if id.kind() == "identifier" {
                                    out.objects.insert(self.node_text(&id).to_string());
                                }
                            }
                        }
                        "named_imports" => {
                            let mut ic = c.walk();
                            for spec in c.named_children(&mut ic) {
                                if spec.kind() != "import_specifier"
                                    || self.js_ts_import_specifier_is_type_only(spec)
                                {
                                    continue;
                                }
                                let Some(n) = spec.child_by_field_name("name") else {
                                    continue;
                                };
                                if n.kind() != "identifier" {
                                    continue;
                                }
                                let a = spec.child_by_field_name("alias").unwrap_or(n);
                                out.named.insert(
                                    self.node_text(&a).to_string(),
                                    self.node_text(&n).to_string(),
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        out
    }

    /// R6 P4: a module-scope declaration of `name`. That is a top-level or exported
    /// declaration, or a `var` hoisted out of top-level blocks, loops, `switch` and `try`.
    /// One recursive walk; it stops at nested functions, classes and block-scoped
    /// `let`/`const`, and skips `import_statement`.
    fn module_scope_declares(&self, node: Node<'_>, name: &str, top: bool) -> bool {
        let kind = node.kind();
        let named = |n: Node<'_>| self.node_text(&n) == name;
        if super::is_js_ts_function_like(kind) || kind.contains("class") {
            return top && node.child_by_field_name("name").is_some_and(named);
        }
        if kind == "import_statement" || (!top && kind == "lexical_declaration") {
            return false;
        }
        let binding = match kind {
            "variable_declarator" => node.child_by_field_name("name"),
            "for_in_statement"
                if node
                    .child_by_field_name("kind")
                    .is_some_and(|k| self.node_text(&k) == "var") =>
            {
                node.child_by_field_name("left")
            }
            _ => None,
        };
        if let Some(pattern) = binding {
            let mut names = BTreeSet::new();
            self.collect_js_ts_binding_pattern_names(pattern, &mut names);
            if names.contains(name) {
                return true;
            }
        }
        if top && node.child_by_field_name("name").is_some_and(named) {
            return true;
        }
        let child_top = matches!(kind, "program" | "export_statement")
            || (top && matches!(kind, "lexical_declaration" | "variable_declaration"));
        let mut c = node.walk();
        let found = node
            .named_children(&mut c)
            .any(|ch| self.module_scope_declares(ch, name, child_top));
        found
    }
}

/// The first non-comment leaf token of `node`.
fn first_token(mut node: Node<'_>) -> Node<'_> {
    loop {
        let mut c = node.walk();
        let first = node.children(&mut c).find(|n| n.kind() != "comment");
        match first {
            Some(child) => node = child,
            None => return node,
        }
    }
}
