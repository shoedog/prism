//! A line-result memo scoped to one RD invocation and its immutable spans.
use super::{Line, StatementLineSpan};
use std::cmp::Reverse;
use std::collections::BTreeMap;

pub(super) struct StatementLookup<'a> {
    spans: &'a [StatementLineSpan],
    memo: BTreeMap<Line, Option<Line>>,
}

impl<'a> StatementLookup<'a> {
    pub(super) fn new(spans: &'a [StatementLineSpan]) -> Self {
        Self {
            spans,
            memo: BTreeMap::new(),
        }
    }

    pub(super) fn get(&mut self, line: Line) -> Option<Line> {
        #[cfg(test)]
        super::work_tests::bump(|counts| {
            counts.statement_lookup_calls += 1;
            if counts.statement_query_lines.insert(line) {
                counts.projected_statement_span_visits += self.spans.len();
            }
        });
        #[cfg(test)]
        if super::statement_lookup_tests::legacy_enabled() {
            return super::innermost_statement(line, self.spans);
        }
        if let Some(result) = self.memo.get(&line) {
            return *result;
        }
        let result = self
            .spans
            .iter()
            .filter(|span| {
                #[cfg(test)]
                super::work_tests::bump(|counts| counts.statement_span_visits += 1);
                span.start_line <= line && line <= span.end_line
            })
            .min_by_key(|span| {
                (
                    span.end_byte.saturating_sub(span.start_byte),
                    Reverse(span.start_byte),
                    Reverse(span.start_line),
                )
            })
            .map(|span| span.start_line);
        self.memo.insert(line, result);
        result
    }
}
