//! Sink strength (#636): a recognised sink whose argument shape defuses it is `Weak`.
//!
//! Weak means "the operation is sensitive and the data reaches it, but not in an exploitable
//! form": SQL with a constant query text (taint can only arrive as a bound parameter), an ORM
//! expression clause, an auto-escaping template render. A weak finding is emitted at
//! `suggestion` severity instead of `warning`; the set of findings is unchanged. A line is
//! weak only when every sink call a rule recognises on it is weak and nothing else (a
//! caller-supplied sink name, a structured non-SQL sink) is on the line. Languages without
//! call-position rules are always strong.

use super::SinkClassifier;
use prism::ast::ParsedFile;
use prism::languages::Language;
use tree_sitter::Node;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Strength {
    Strong,
    Weak,
}

impl Strength {
    /// The finding severity this strength is reported at.
    pub(crate) fn severity(self) -> &'static str {
        match self {
            Strength::Strong => "warning",
            Strength::Weak => "suggestion",
        }
    }
}

impl SinkClassifier {
    /// Strength of the `taint_sink` finding on `line`.
    ///
    /// A non-SQL structured-registry match on the line (SSRF, path, command, ...) keeps it
    /// strong. `is_extra` tells whether an identifier is a caller-supplied or wrapper sink name
    /// (always strong).
    pub(crate) fn line_strength(
        &self,
        file: &str,
        parsed: &ParsedFile,
        line: usize,
        is_extra: &dyn Fn(&str) -> bool,
    ) -> Strength {
        let language = parsed.language;
        if !(language.is_js_family() || matches!(language, Language::Python | Language::Go))
            || super::structured::non_sql_match_on_line(parsed, line)
        {
            return Strength::Strong;
        }
        let mut weak_seen = false;
        // A line inside a multi-line call is judged with the calls that enclose it.
        let mut ids = parsed.identifiers_on_line(line);
        if let Some(first) = ids.first().copied() {
            ids.extend(enclosing_callees(parsed, first));
        }
        for id in ids {
            let text = parsed.node_text(&id);
            if is_extra(text) {
                return Strength::Strong;
            }
            if language == Language::Python && self.py_line_has_raw(file, parsed, id) {
                return Strength::Strong;
            }
            // No legacy list: these languages have call-position rules only.
            match self.classify(file, parsed, id, text, &[]) {
                Some(hit) if hit.strength == Strength::Weak => weak_seen = true,
                Some(_) => return Strength::Strong,
                None => {}
            }
        }
        if weak_seen {
            Strength::Weak
        } else {
            Strength::Strong
        }
    }
}

/// The callee identifier of every call that encloses `node`.
fn enclosing_callees<'t>(parsed: &ParsedFile, node: Node<'t>) -> Vec<Node<'t>> {
    let mut out = Vec::new();
    let mut cur = node.parent();
    while let Some(n) = cur {
        if matches!(n.kind(), "call_expression" | "call") {
            let callee = parsed.call_function_node(&n).and_then(|f| match f.kind() {
                "identifier" => Some(f),
                "member_expression" => f.child_by_field_name("property"),
                "attribute" => f.child_by_field_name("attribute"),
                "selector_expression" => f.child_by_field_name("field"),
                _ => None,
            });
            out.extend(callee);
        }
        cur = n.parent();
    }
    out
}
