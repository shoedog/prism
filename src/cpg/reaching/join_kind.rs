//! One lazy, borrowed statement-kind map per RD invocation.
use super::{ArmProvenance, CfgEdge, EdgeOrigin, Language, Line, ParsedFile};
use std::cell::OnceCell;
use std::collections::BTreeMap;

pub(super) struct StatementKinds<'a> {
    statements: &'a [(Line, String)],
    kinds: OnceCell<BTreeMap<Line, &'a str>>,
}

impl<'a> StatementKinds<'a> {
    pub(super) fn new(statements: &'a [(Line, String)]) -> Self {
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

pub(super) fn is_incomplete_join(
    parsed: &ParsedFile,
    kinds: &StatementKinds<'_>,
    edge: &CfgEdge,
    provenance: ArmProvenance,
) -> bool {
    #[cfg(test)]
    if super::join_kind_tests::legacy_enabled() {
        return super::join_kind_tests::legacy_join(parsed, kinds.statements, edge, provenance);
    }
    if provenance.crosses_lexical_arm() {
        return true;
    }
    if matches!(provenance.origin, EdgeOrigin::ImportExceptionBypass) {
        return false;
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
