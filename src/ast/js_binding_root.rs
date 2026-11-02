//! #602: where the root identifier of a call is declared, for import-target
//! attribution. The import map of a file is keyed by spelling; this decides
//! whether the spelling at one call site still denotes that import or a nearer
//! declaration (a parameter, a local, a nested function). It reuses the S1b
//! site walk, so the answer is the same scope fact `CallSite.local_binding` uses.
use super::js_binding::{JsBinding, JsBindingCache};
use super::js_binding_walk::Walk;
use super::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

/// Which scope declares a call's root identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JsRootScope {
    /// The program scope: an import, a module-level declaration, or a program
    /// function. Module-level facts (the import map) apply.
    Program,
    /// A nearer scope. `callable` is true when it provably holds one in-file
    /// function (a nested function declaration).
    Nearer { callable: bool },
    /// No declaration found, or a position the walk does not model.
    Unproven,
}

impl ParsedFile {
    /// The scope that declares the root identifier `root` of the call spanning
    /// `start_byte..end_byte`: the callee itself, or the leftmost identifier of
    /// its member chain (`ns` of `ns.sub.f()`). `Unproven` when the call node,
    /// its callee shape (`this`, a call result, a subscript) or the walk does
    /// not give one identifier spelled `root`.
    pub(crate) fn js_ts_call_root_scope(
        &self,
        start_byte: usize,
        end_byte: usize,
        root: &str,
    ) -> JsRootScope {
        let Some(call) = self
            .tree
            .root_node()
            .descendant_for_byte_range(start_byte, end_byte)
            .filter(|n| n.start_byte() == start_byte && n.end_byte() == end_byte)
        else {
            return JsRootScope::Unproven;
        };
        let Some(ident) = self.js_ts_call_root_identifier(call) else {
            return JsRootScope::Unproven;
        };
        if self.node_text(&ident) != root {
            return JsRootScope::Unproven;
        }
        let mut cache = JsBindingCache::for_sites();
        match self.js_ts_binding_walk(ident, root, &mut cache) {
            Walk::Found(scope, _) if scope.kind() == "program" => JsRootScope::Program,
            Walk::Found(..) => JsRootScope::Nearer {
                callable: matches!(
                    self.js_ts_site_binding(ident, root, &mut cache),
                    JsBinding::Callable(_)
                ),
            },
            Walk::Unbound | Walk::Unchecked(_) => JsRootScope::Unproven,
        }
    }

    /// The file's program-scope import spellings: local name -> specifier, from
    /// top-level `import` statements and top-level `require` declarations
    /// (also under `export`). Unlike `extract_imports`, a `require` inside a
    /// function or block is not read, so a nested declaration cannot overwrite
    /// the spelling the program-scope binding has.
    pub(crate) fn js_ts_program_imports(&self) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        let root = self.tree.root_node();
        let mut cursor = root.walk();
        for statement in root.children(&mut cursor) {
            let declaration = if statement.kind() == "export_statement" {
                statement.child_by_field_name("declaration")
            } else {
                Some(statement)
            };
            if let Some(node) = declaration.filter(|n| {
                matches!(
                    n.kind(),
                    "import_statement" | "lexical_declaration" | "variable_declaration"
                )
            }) {
                self.collect_js_imports(node, &mut out);
            }
        }
        out
    }

    fn js_ts_call_root_identifier<'a>(&self, call: Node<'a>) -> Option<Node<'a>> {
        let mut node = match call.kind() {
            "call_expression" => call.child_by_field_name("function")?,
            "new_expression" => call.child_by_field_name("constructor")?,
            "jsx_self_closing_element" | "jsx_opening_element" => {
                call.child_by_field_name("name")?
            }
            _ => return None,
        };
        loop {
            match node.kind() {
                "identifier" => return Some(node),
                "member_expression" => node = node.child_by_field_name("object")?,
                _ => return None,
            }
        }
    }
}
