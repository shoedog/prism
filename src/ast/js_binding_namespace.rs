//! S1b-4: namespace syntax provenance, proved at the exact qualifier node.
use super::js_binding::{JsBinding, JsBindingCache, NamespaceImports};
use super::js_binding_walk::Walk;
use super::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

impl ParsedFile {
    /// None leaves non-namespace qualifiers at base. A spelling is only an
    /// entry gate; Import authority additionally needs scope/declaration identity.
    pub(crate) fn js_ts_namespace_binding<'a>(
        &'a self,
        ident: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> Option<(JsBinding, Option<String>)> {
        let name = self.node_text(&ident);
        let imports = cache.namespace_imports.get_or_insert_with(|| {
            let mut out: NamespaceImports<'a> = BTreeMap::new();
            let root = self.tree.root_node();
            let mut cursor = root.walk();
            for stmt in root
                .named_children(&mut cursor)
                .filter(|s| s.kind() == "import_statement")
            {
                let mut type_cursor = stmt.walk();
                // JSX recovers TS's type token as an ERROR; it still must not
                // introduce namespace authority for the type-only non-goal.
                let recovered_type = stmt
                    .named_children(&mut type_cursor)
                    .any(|n| n.is_error() && self.node_text(&n).trim() == "type");
                if self.js_ts_import_statement_is_type_only(stmt) || recovered_type {
                    continue;
                }
                let mut cursor = stmt.walk();
                for clause in stmt
                    .named_children(&mut cursor)
                    .filter(|c| c.kind() == "import_clause")
                {
                    let mut cursor = clause.walk();
                    for ns in clause
                        .named_children(&mut cursor)
                        .filter(|n| n.kind() == "namespace_import")
                    {
                        let mut cursor = ns.walk();
                        if let Some(id) = ns
                            .named_children(&mut cursor)
                            .find(|n| n.kind() == "identifier")
                        {
                            let source = stmt
                                .child_by_field_name("source")
                                .and_then(|s| self.js_ts_module_export_name(s));
                            out.entry(self.node_text(&id).to_string())
                                .or_default()
                                .push((stmt, source));
                        };
                    }
                }
            }
            out
        });
        let declarations = imports.get(name)?.clone();
        let mut binding = self.js_ts_site_binding(ident, name, cache);
        if binding == JsBinding::Refused("duplicate_declaration") {
            if let Walk::Found(scope, None) = self.js_ts_binding_walk(ident, name, cache) {
                let (decls, _) = self.js_ts_scope_lookup(scope, name, cache);
                if scope.kind() != "program"
                    && decls.is_some_and(|ds| {
                        ds.iter()
                            .all(|d| !matches!(d.kind(), "import_statement" | "import_alias"))
                    })
                {
                    binding = JsBinding::Refused("namespace_shadow");
                }
            }
        }
        let module = if binding == JsBinding::Import {
            match self.js_ts_binding_walk(ident, name, cache) {
                Walk::Found(scope, None) if scope.kind() == "program" => {
                    let (decls, _) = self.js_ts_scope_lookup(scope, name, cache);
                    match (decls.as_deref(), declarations.as_slice()) {
                        (Some([decl]), [(stmt, source)]) if decl.id() == stmt.id() => {
                            source.clone()
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        } else {
            None
        };
        Some((binding, module))
    }
}
