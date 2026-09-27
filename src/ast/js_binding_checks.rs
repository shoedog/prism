//! S1b (SPEC §3.1 B0, B1): the fail-safe over node kinds and the sealed-error rule.
use super::js_binding::{classified, sealing, JsBindingCache};
use super::ParsedFile;
use tree_sitter::Node;

impl ParsedFile {
    /// B0 (fail-safe): every named node kind in `scope`, leaves included, is classified, and
    /// no binding identifier is spelled with an escape (the collector compares raw spellings).
    pub(super) fn js_ts_scope_clean<'a>(
        &self,
        scope: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> Result<(), &'static str> {
        if let Some(r) = cache.clean.get(&scope.id()) {
            return *r;
        }
        let mut result = Ok(());
        let mut stack = vec![scope];
        while let Some(n) = stack.pop() {
            if n.is_named() && !n.is_error() && !n.is_missing() && !classified(n.kind()) {
                result = Err("unclassified_kind");
                break;
            }
            if matches!(
                n.kind(),
                "identifier" | "shorthand_property_identifier_pattern"
            ) && self.node_text(&n).contains('\\')
            {
                result = Err("escaped_identifier");
                break;
            }
            let mut cursor = n.walk();
            stack.extend(n.named_children(&mut cursor));
        }
        cache.clean.insert(scope.id(), result);
        result
    }

    /// B1 (owner E6, the narrower rule, with the re-plan folds). A binding scope with a parse
    /// error is usable only when (i) no structural brace is missing or a direct token of an
    /// `ERROR` anywhere in the file (so every braced node's extent is its true extent; braces
    /// in string, template, regex or comment text never count), and (ii) every error in the
    /// scope lies inside a delimited child (a `body` or `class_body` with paired braces, a
    /// `parameters` list with paired parentheses) of a class, static block or braced function
    /// that is strictly inside the scope and does not contain the site. A header error (name,
    /// type parameters, return type, heritage) is not sealed. `Some` holds the sealing nodes
    /// that contain errors (empty for a clean scope); `None` refuses.
    pub(super) fn js_ts_recovery_sealed<'a>(
        &self,
        scope: Node<'a>,
        site: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> Option<Vec<Node<'a>>> {
        let mut sealers = Vec::new();
        if !scope.has_error() {
            return Some(sealers);
        }
        if !*cache
            .braces
            .get_or_insert_with(|| self.js_ts_braces_paired())
        {
            return None;
        }
        // ASSUMPTION (Opus r1 S1): (i) proves braces paired; a parameter list's parentheses are
        // taken as paired when both edge tokens are present and not `MISSING`.
        let delimited = |d: Node<'_>| {
            let (open, close) = match d.kind() {
                "statement_block" | "class_body" => ("{", "}"),
                "formal_parameters" => ("(", ")"),
                _ => return false,
            };
            let edge = |c: Option<Node<'_>>, token| {
                c.is_some_and(|c| c.kind() == token && !c.is_missing())
            };
            d.parent().is_some_and(sealing)
                && edge(d.child(0), open)
                && edge(d.child(d.child_count().saturating_sub(1)), close)
        };
        let inside = |outer: Node<'_>, inner: Node<'_>| {
            outer.start_byte() <= inner.start_byte() && inner.end_byte() <= outer.end_byte()
        };
        let mut stack = vec![scope];
        while let Some(n) = stack.pop() {
            if n.is_error() || n.is_missing() {
                let mut up = n.parent();
                while let Some(d) = up.filter(|d| d.id() != scope.id() && !delimited(*d)) {
                    up = d.parent();
                }
                let sealer = up
                    .filter(|d| d.id() != scope.id())
                    .and_then(|d| d.parent())
                    .filter(|f| f.id() != scope.id() && !inside(*f, site));
                sealers.push(sealer?);
            }
            let mut cursor = n.walk();
            stack.extend(
                n.children(&mut cursor)
                    .filter(|c| c.has_error() || c.is_missing()),
            );
        }
        Some(sealers)
    }

    /// B1 (i): no `MISSING` brace, and no `ERROR` holding an anonymous `{`/`}` token (directly
    /// or through a nested `ERROR`), anywhere in the file.
    fn js_ts_braces_paired(&self) -> bool {
        fn structural(n: Node<'_>) -> bool {
            let mut cursor = n.walk();
            let found = n.children(&mut cursor).any(|c| {
                (!c.is_named() && matches!(c.kind(), "{" | "}")) || (c.is_error() && structural(c))
            });
            found
        }
        let mut stack = vec![self.tree.root_node()];
        while let Some(n) = stack.pop() {
            if (n.is_missing() && matches!(n.kind(), "{" | "}")) || (n.is_error() && structural(n))
            {
                return false;
            }
            let mut cursor = n.walk();
            stack.extend(
                n.children(&mut cursor)
                    .filter(|c| c.has_error() || c.is_missing()),
            );
        }
        true
    }
}
