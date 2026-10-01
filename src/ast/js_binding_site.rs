//! S1b-3 (SPEC §3.1, §3.1a, §3.8 (2)): the scope index at every scope, and the binding at a
//! call site. `js_ts_site_binding` is wired to `CallSite.local_binding` in S1b-3b
//! (`src/call_graph.rs`).
use super::js_binding::{is_class, Index, JsBinding, JsBindingCache};
use super::js_binding_walk::Walk;
use super::{is_js_ts_function_like, ParsedFile};
use std::collections::BTreeSet;
use tree_sitter::Node;

/// Environment creators whose declarations the site walk looks up (SPEC §3.1 T1–T9). A
/// function's body block belongs to the function's own environment.
pub(super) fn is_scope(node: Node<'_>) -> bool {
    match node.kind() {
        "program" | "formal_parameters" | "for_statement" | "for_in_statement" | "catch_clause"
        | "switch_body" | "with_statement" => true,
        "statement_block" => !node
            .parent()
            .is_some_and(|p| is_js_ts_function_like(p.kind())),
        k => is_js_ts_function_like(k) || is_class(k),
    }
}

impl ParsedFile {
    /// What `name`, used at the identifier `ident`, denotes (SPEC §3.1, §3.1a).
    pub(crate) fn js_ts_site_binding<'a>(
        &'a self,
        ident: Node<'a>,
        name: &str,
        cache: &mut JsBindingCache<'a>,
    ) -> JsBinding {
        match self.js_ts_binding_walk(ident, name, cache) {
            Walk::Found(scope, explicit) => {
                // A call-site lookup is the local route (SPEC §3.7): `useCallback` admits.
                self.js_ts_scope_binding(scope, explicit, name, ident, cache, true)
            }
            Walk::Unbound => JsBinding::Refused("unbound"),
            Walk::Unchecked(reason) => JsBinding::Unchecked(reason),
        }
    }

    /// `scope`'s declarations of `name` (memoized per scope) and whether the name is an
    /// unknown-strictness Annex-B name there.
    pub(super) fn js_ts_scope_lookup<'a>(
        &'a self,
        scope: Node<'a>,
        name: &str,
        cache: &mut JsBindingCache<'a>,
    ) -> (Option<Vec<Node<'a>>>, bool) {
        let (index, annex) = cache
            .decls
            .entry(scope.id())
            .or_insert_with(|| self.js_ts_scope_index(scope));
        (index.get(name).cloned(), annex.contains(name))
    }

    /// Every name `scope` declares itself, with the declaring nodes (SPEC §3.1 table).
    fn js_ts_scope_index<'a>(&'a self, scope: Node<'a>) -> (Index<'a>, BTreeSet<String>) {
        let (mut out, mut annex) = (Index::new(), BTreeSet::new());
        let strict = self.js_ts_strictness(scope);
        let mut names = BTreeSet::new();
        let kind = scope.kind();
        let walk = |at: Node<'a>, mode, out: &mut Index<'a>, annex: &mut BTreeSet<String>| {
            self.js_ts_declare_walk(at, mode, &BTreeSet::new(), out, annex)
        };
        match kind {
            "catch_clause" | "for_in_statement" => {
                let lexical = kind == "catch_clause"
                    || scope
                        .child_by_field_name("kind")
                        .is_some_and(|k| matches!(self.node_text(&k), "let" | "const"));
                let field = if kind == "catch_clause" {
                    "parameter"
                } else {
                    "left"
                };
                if let Some(p) = scope.child_by_field_name(field).filter(|_| lexical) {
                    self.collect_js_ts_binding_pattern_names(p, &mut names);
                }
            }
            "for_statement" => {
                if let Some(init) = scope
                    .child_by_field_name("initializer")
                    .filter(|i| i.kind() == "lexical_declaration")
                {
                    walk(init, (true, false, strict), &mut out, &mut annex);
                }
            }
            // T10: the members of an enum body (StringValue), non-callable.
            "enum_body" => {
                let mut cursor = scope.walk();
                for m in scope.named_children(&mut cursor) {
                    let key = match m.kind() {
                        "enum_assignment" => m.child_by_field_name("name"),
                        _ => Some(m),
                    };
                    names.extend(key.and_then(|k| self.js_ts_module_export_name(k)));
                }
            }
            k if is_class(k) => {
                let name = scope.child_by_field_name("name");
                names.extend(name.map(|n| self.node_text(&n).to_string()));
            }
            _ => {}
        }
        for name in names {
            out.entry(name).or_default().push(scope);
        }
        match kind {
            "formal_parameters" => {
                if let Some(f) = scope.parent().filter(|f| is_js_ts_function_like(f.kind())) {
                    self.js_ts_function_names(f, scope, &mut out);
                }
            }
            k if is_js_ts_function_like(k) => {
                // Parameters are recorded at their list (or the single arrow parameter).
                let at = scope
                    .child_by_field_name("parameters")
                    .or_else(|| scope.child_by_field_name("parameter"))
                    .unwrap_or(scope);
                self.js_ts_function_names(scope, at, &mut out);
                let body = scope.child_by_field_name("body");
                if let Some(b) = body.filter(|b| b.kind() == "statement_block") {
                    walk(b, (true, true, strict), &mut out, &mut annex);
                }
            }
            "statement_block" => {
                // A static block or namespace body is a `var` scope (T4).
                let var_scope = scope.parent().is_some_and(|p| {
                    matches!(
                        p.kind(),
                        "class_static_block" | "internal_module" | "module"
                    )
                });
                walk(scope, (true, var_scope, strict), &mut out, &mut annex);
            }
            "switch_body" | "program" => {
                walk(
                    scope,
                    (true, kind == "program", strict),
                    &mut out,
                    &mut annex,
                );
            }
            _ => {}
        }
        if kind == "program" {
            let mut cursor = scope.walk();
            for e in scope
                .children(&mut cursor)
                .filter(|e| self.js_ts_recovered_import(*e))
            {
                for w in self.js_ts_recovered_import_names(e) {
                    out.entry(w).or_default().push(e);
                }
            }
        }
        (out, annex)
    }

    /// A function's own names: its parameters (at `at`), `arguments` (not for arrows), and a
    /// function expression's own name.
    fn js_ts_function_names<'a>(&self, f: Node<'a>, at: Node<'a>, out: &mut Index<'a>) {
        let mut params = BTreeSet::new();
        self.collect_js_ts_parameter_bindings(f, &mut params);
        if f.kind() != "arrow_function" {
            params.insert("arguments".to_string());
        }
        for p in params {
            out.entry(p).or_default().push(at);
        }
        if matches!(f.kind(), "function_expression" | "generator_function") {
            if let Some(n) = f.child_by_field_name("name") {
                out.entry(self.node_text(&n).into()).or_default().push(f);
            }
        }
    }
}

#[cfg(test)]
#[path = "js_binding_site_tests.rs"]
mod tests;
