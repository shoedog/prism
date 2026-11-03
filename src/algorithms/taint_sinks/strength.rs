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
use crate::ast::ParsedFile;
use crate::languages::Language;

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
    /// `structured_sql` is the structured-sink registry's opinion for the line: `None` when it
    /// has no match, `Some(true)` for a SQL match (graded by the flat rule's argument shape),
    /// `Some(false)` for any other category (always strong). `is_extra` tells whether an
    /// identifier is a caller-supplied or wrapper sink name (always strong).
    pub(crate) fn line_strength(
        &self,
        file: &str,
        parsed: &ParsedFile,
        line: usize,
        structured_sql: Option<bool>,
        is_extra: &dyn Fn(&str) -> bool,
    ) -> Strength {
        let language = parsed.language;
        if !(language.is_js_family() || matches!(language, Language::Python | Language::Go))
            || structured_sql == Some(false)
        {
            return Strength::Strong;
        }
        let mut weak_seen = false;
        for id in parsed.identifiers_on_line(line) {
            let text = parsed.node_text(&id);
            if is_extra(text) {
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
