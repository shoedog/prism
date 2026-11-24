//! Root-local empty recovery; original exact occurrences remain independent.
use crate::algorithms::taint::{
    js_ts::request_aliases::js_ts_seed_scope_contains_range, TaintRoot,
};
use prism::{
    api::{ReviewOccurrenceCensus, ReviewOccurrenceFacts},
    ast::{tree_helpers::is_js_ts_language, ParsedFile},
    call_graph::CallSite,
    data_flow::{VarAccessKind, VarLocation},
};
mod source;

pub(super) struct RecoveryContext<'a> {
    parsed: &'a ParsedFile,
    facts: ReviewOccurrenceFacts<'a>,
}
pub(super) struct PreparedRoot<'a> {
    root: &'a TaintRoot,
    empty: Vec<Option<VarLocation>>,
}
impl<'a> RecoveryContext<'a> {
    pub(super) fn new(parsed: &'a ParsedFile) -> Self {
        Self {
            parsed,
            facts: ReviewOccurrenceFacts::new(parsed),
        }
    }
    pub(super) fn prepare<'r>(&self, root: &'r TaintRoot) -> PreparedRoot<'r> {
        let proof = source::admit(self, root);
        let empty = root
            .reached
            .iter()
            .map(|r| proof.as_ref().and_then(|p| self.recover(p, &r.location)))
            .collect();
        PreparedRoot { root, empty }
    }
    pub(super) fn matches_argument(&self, call: &CallSite, value: &VarLocation) -> bool {
        super::occurrences::matches_argument(&self.facts, call, value)
            && (!value.path.fields.is_empty()
                || self.simple_value_syntax(value.start_byte..value.end_byte))
    }
    fn simple_value_syntax(&self, span: std::ops::Range<usize>) -> bool {
        let Some(node) = self
            .parsed
            .tree
            .root_node()
            .descendant_for_byte_range(span.start, span.end)
        else {
            return false;
        };
        if node.byte_range() != span
            || !matches!(node.kind(), "identifier" | "shorthand_property_identifier")
        {
            return false;
        }
        true
    }
    fn recover(&self, proof: &source::Proof<'_>, value: &VarLocation) -> Option<VarLocation> {
        if value.kind != VarAccessKind::Use
            || value.start_byte != value.end_byte
            || value.end_byte > self.parsed.source.len()
            || value.file != self.parsed.path
            || source::owner(self, value)? != proof.owner
        {
            return None;
        }
        let ReviewOccurrenceCensus::Complete(spans) =
            self.facts
                .literal_census(proof.owner.span.clone(), value.line, &value.path)
        else {
            return None;
        };
        let [span] = spans.as_slice() else {
            return None;
        };
        if !self.simple_value_syntax(span.clone()) {
            return None;
        }
        let seed = &proof.seed;
        if value.line < seed.line
            || seed
                .scope
                .is_some_and(|(a, b)| value.line < a || value.line > b)
            || seed
                .byte_scope
                .is_some_and(|(a, b)| span.start < a || span.end > b)
            || (value.line == seed.line && seed.start_byte.is_some_and(|start| span.start < start))
            || (is_js_ts_language(self.parsed.language)
                && !js_ts_seed_scope_contains_range(
                    self.parsed,
                    seed,
                    value.line,
                    span.start,
                    span.end,
                ))
            || !self.facts.values(span.clone()).iter().any(|fact| {
                fact.semantic_value
                    && fact.owner.as_ref() == Some(proof.owner)
                    && fact.occurrence.path == value.path
                    && fact.occurrence.line == value.line
            })
        {
            return None;
        }
        let mut recovered = value.clone();
        recovered.start_byte = span.start;
        recovered.end_byte = span.end;
        let function = proof.owner.function.as_ref()?;
        recovered.function = function.name.clone();
        recovered.function_start_line = function.start_line;
        Some(recovered)
    }
}
impl PreparedRoot<'_> {
    pub(super) fn root_value(&self) -> &VarLocation {
        &self.root.location
    }
    /// Inspectable originals preserve destination-file matching, even without proof.
    pub(super) fn value(&self, index: usize) -> Option<&VarLocation> {
        let original = &self.root.reached.get(index)?.location;
        if original.start_byte < original.end_byte {
            Some(original)
        } else {
            self.empty.get(index)?.as_ref()
        }
    }
}
#[cfg(test)]
mod exact_role_tests;
#[cfg(test)]
mod tests;
