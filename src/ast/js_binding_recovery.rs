//! S1b-3 (SPEC §3.8 (7), carry-forward 9): the recovered top-level import predicate and the
//! names such a recovery may bind. The marker is one arm of the program scope index; outside
//! the walk it would need its own per-file fact and resolution guard.
use super::ParsedFile;
use std::collections::BTreeSet;
use tree_sitter::Node;

impl ParsedFile {
    /// A top-level `ERROR` that is a recovered static import: its first token is `import` and
    /// the next is neither `.` (`import.meta`) nor `(` (`import()`). Tokens are leaves that
    /// are not comments or other non-`ERROR` extras (spec r2 sol W2: `import /* c */ .meta`;
    /// tree-sitter also flags recovery `ERROR` nodes as extras, so those are descended, not
    /// skipped).
    pub(super) fn js_ts_recovered_import(&self, e: Node<'_>) -> bool {
        let mut leaves = Vec::new();
        let mut stack = vec![e];
        while let Some(n) = stack.pop().filter(|_| leaves.len() < 2) {
            if (n.is_extra() && !n.is_error()) || n.kind() == "comment" {
                continue;
            }
            if n.child_count() == 0 {
                leaves.push(self.node_text(&n));
                continue;
            }
            let mut c = n.walk();
            let children: Vec<_> = n.children(&mut c).collect();
            stack.extend(children.into_iter().rev());
        }
        e.is_error()
            && leaves.first() == Some(&"import")
            && !matches!(leaves.get(1).copied(), Some("." | "("))
    }

    /// The names a recovered import may bind, which the tree does not keep (an alias can land
    /// in string text): every `identifier` in it, and every word of its source, where a word is
    /// a run of ASCII identifier characters and non-ASCII non-whitespace characters (so `é`
    /// with U+0301 and ZWNJ/ZWJ stay whole), minus the import keywords and digit-led runs.
    pub(super) fn js_ts_recovered_import_names(&self, e: Node<'_>) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut stack = vec![e];
        while let Some(n) = stack.pop() {
            if n.kind() == "identifier" {
                out.insert(self.node_text(&n).to_string());
            }
            let mut c = n.walk();
            stack.extend(n.named_children(&mut c));
        }
        let text = self.node_text(&e);
        let breaks = |c: char| {
            c.is_whitespace()
                || (c.is_ascii() && !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
        };
        out.extend(text.split(breaks).map(str::to_string));
        out.retain(|w| {
            !w.is_empty()
                && !w.starts_with(|c: char| c.is_ascii_digit())
                && !matches!(w.as_str(), "import" | "from" | "as" | "type" | "typeof")
        });
        out
    }
}

#[cfg(test)]
#[path = "js_binding_recovery_tests.rs"]
mod tests;
