//! S1b-3 (SPEC §3.1 W1, §3.8 (3)): the scoped write resolver (M2), replacing the module write
//! scan. A write counts against a binding only when its target, walked like a site, resolves
//! to that binding's scope.
use super::js_binding::JsBindingCache;
use super::js_binding_decls::js_ts_is_using;
use super::js_binding_walk::Walk;
use super::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

impl ParsedFile {
    /// M2 (SPEC §3.1 W rows, §3.8 (3)): some write's target resolves to `scope`'s binding of
    /// `name` (or passes a `with`, or sits at an unproven position). The file's write targets
    /// are indexed once and each `(scope, name)` answer is memoized (C-19).
    pub(super) fn js_ts_scoped_written<'a>(
        &'a self,
        scope: Node<'a>,
        name: &str,
        cache: &mut JsBindingCache<'a>,
    ) -> bool {
        let canon = canonical_write_scope(scope);
        let key = (canon.id(), name.to_string());
        if let Some(w) = cache.written.get(&key) {
            return *w;
        }
        let targets = cache
            .write_targets
            .get_or_insert_with(|| self.js_ts_write_targets())
            .get(name)
            .cloned()
            .unwrap_or_default();
        let written = targets
            .into_iter()
            .any(|t| match self.js_ts_binding_walk(t, name, cache) {
                Walk::Found(s, _) => {
                    canonical_write_scope(s).id() == canon.id() || s.kind() == "with_statement"
                }
                Walk::Unchecked(_) => true,
                Walk::Unbound => false,
            });
        cache.written.insert(key, written);
        written
    }

    /// W1 targets by name: assignment, update and `delete` targets and bare `for (x in/of …)`
    /// heads (a `using` declaration and a declaring head are not writes), and every identifier
    /// that is a child or a sibling of an `ERROR` (recovery may have mangled a write, as in
    /// `f = ;`, parsed as `f` beside an `ERROR` holding `=`).
    fn js_ts_write_targets<'a>(&'a self) -> BTreeMap<String, Vec<Node<'a>>> {
        let mut out = BTreeMap::<String, Vec<Node<'a>>>::new();
        let mut stack = vec![self.tree.root_node()];
        while let Some(n) = stack.pop() {
            let declares =
                n.kind() == "for_in_statement" && n.child_by_field_name("kind").is_some();
            let mut targets: Vec<Node<'a>> = Vec::new();
            if !declares && !js_ts_is_using(n) {
                targets.extend(self.js_ts_write_target(n));
            }
            let mut cursor = n.walk();
            let errorful = n.is_error() || n.children(&mut cursor).any(|c| c.is_error());
            for c in n.named_children(&mut cursor) {
                if errorful && c.kind() == "identifier" {
                    targets.push(c);
                }
                stack.push(c);
            }
            for t in targets {
                let mut names = BTreeSet::new();
                self.collect_js_ts_binding_pattern_names(t, &mut names);
                for w in names {
                    out.entry(w).or_default().push(t);
                }
            }
        }
        out
    }
}

/// A `formal_parameters` node (T3) and its owning function (T2) index the same shared
/// bindings (parameters, `arguments`, a named function expression's own name) at two distinct
/// node ids: a default-expression write walks to T3, a body reference walks to T2. Bug fold
/// (gpt-5.6-sol W1, gpt-6.1-sol W1): comparing raw ids misses the write in either direction.
/// `formal_parameters` never indexes a body-only declaration (its own scope index calls only
/// `js_ts_function_names`), so mapping it to its parent is always safe here.
fn canonical_write_scope(n: Node<'_>) -> Node<'_> {
    if n.kind() == "formal_parameters" {
        n.parent().unwrap_or(n)
    } else {
        n
    }
}

#[cfg(test)]
#[path = "js_binding_writes_tests.rs"]
mod tests;
