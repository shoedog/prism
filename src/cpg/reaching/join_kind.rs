//! One lazy, borrowed statement-kind map per RD invocation.
use super::{ArmProvenance, CfgEdge, EdgeOrigin, Language, Line, ParsedFile};
use std::cell::OnceCell;
use std::collections::BTreeMap;

pub(in crate::cpg) struct StatementKinds<'a> {
    statements: &'a [(Line, String)],
    kinds: OnceCell<BTreeMap<Line, &'a str>>,
}

impl<'a> StatementKinds<'a> {
    pub(in crate::cpg) fn new(statements: &'a [(Line, String)]) -> Self {
        Self {
            statements,
            kinds: OnceCell::new(),
        }
    }

    fn get(&self) -> &BTreeMap<Line, &'a str> {
        self.kinds.get_or_init(|| {
            #[cfg(test)]
            super::work_tests::bump(|counts| {
                counts.join_map_builds += 1;
                counts.join_map_entries += self.statements.len();
            });
            // Collect in statement order: the last entry on a line wins.
            self.statements
                .iter()
                .map(|(line, kind)| (*line, kind.as_str()))
                .collect()
        })
    }
}

impl<'a> StatementKinds<'a> {
    fn kind_at(&self, line: Line) -> &str {
        self.get().get(&line).copied().unwrap_or("")
    }
}

/// Go's synthetic function-exit dispatch (return -> defer). Defer arguments
/// are evaluated at the defer statement, so no data route runs through it.
pub(in crate::cpg) fn is_go_defer_dispatch(
    parsed: &ParsedFile,
    kinds: &StatementKinds<'_>,
    edge: &CfgEdge,
    provenance: ArmProvenance,
) -> bool {
    parsed.language == Language::Go
        && (provenance.origin == EdgeOrigin::GoDeferExit
            || (matches!(provenance.origin, EdgeOrigin::Structured)
                && parsed
                    .language
                    .is_return_node(kinds.kind_at(edge.from_line))
                && kinds.kind_at(edge.to_line) == "defer_statement"))
}

pub(in crate::cpg) fn is_incomplete_join(
    parsed: &ParsedFile,
    kinds: &StatementKinds<'_>,
    edge: &CfgEdge,
    provenance: ArmProvenance,
) -> bool {
    #[cfg(test)]
    if super::join_kind_tests::legacy_enabled() {
        return super::join_kind_tests::legacy_join(parsed, kinds.statements, edge, provenance);
    }
    if provenance.crosses_lexical_arm() || provenance.origin == EdgeOrigin::GoDeferExit {
        return true;
    }
    if !matches!(provenance.origin, EdgeOrigin::Structured) {
        return false;
    }
    #[cfg(test)]
    super::work_tests::bump(|counts| counts.join_queries += 1);
    let kinds = kinds.get();
    let from_kind = kinds.get(&edge.from_line).copied().unwrap_or("");
    let to_kind = kinds.get(&edge.to_line).copied().unwrap_or("");
    let try_join = matches!(
        parsed.language,
        Language::Python
            | Language::JavaScript
            | Language::TypeScript
            | Language::Tsx
            | Language::Java
    ) && from_kind == "try_statement";
    let go_defer = parsed.language == Language::Go
        && parsed.language.is_return_node(from_kind)
        && to_kind == "defer_statement";
    try_join || go_defer
}
