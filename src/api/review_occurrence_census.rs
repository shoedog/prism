//! Conservative literal competitors, independent of value or execution roles.
use super::ReviewOccurrenceFacts;
use crate::{access_path::AccessPath, languages::Language};
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReviewOccurrenceCensus {
    /// Complete possible-token population; these are not proven value uses.
    Complete(Vec<Range<usize>>),
    /// Unsupported or uncertain, distinct from a complete empty population.
    Unknown,
}

impl ReviewOccurrenceFacts<'_> {
    /// Count literal simple-name competitors on one physical line inside an
    /// inventoried owner envelope. Ranges refer only to this captured file.
    /// Headers, nested syntax, comments and strings deliberately contribute.
    pub fn literal_census(
        &self,
        owner: Range<usize>,
        line: usize,
        path: &AccessPath,
    ) -> ReviewOccurrenceCensus {
        let known_owner = self.owners.iter().any(|row| row.span == owner);
        if self.parsed.parse_error_count > 0
            || !known_owner
            || !path.fields.is_empty()
            || !literal_name(&path.base)
            || !matches!(
                self.parsed.language,
                Language::JavaScript
                    | Language::TypeScript
                    | Language::Tsx
                    | Language::Python
                    | Language::Go
            )
        {
            return ReviewOccurrenceCensus::Unknown;
        }
        self.census
            .get_or_init(|| LiteralIndex::new(&self.parsed.source))
            .census(owner, line, &path.base)
    }
}

fn name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')
}
fn literal_name(name: &str) -> bool {
    name.as_bytes()
        .first()
        .is_some_and(|byte| !byte.is_ascii_digit())
        && name.bytes().all(name_byte)
}

struct PhysicalLine {
    span: Range<usize>,
    uncertain: bool,
}
pub(super) struct LiteralIndex<'a> {
    source: &'a str,
    lines: Vec<PhysicalLine>,
    tokens: Vec<Range<usize>>,
}
impl<'a> LiteralIndex<'a> {
    fn new(source: &'a str) -> Self {
        let mut offset = 0;
        let lines = source
            .split('\n')
            .map(|text| {
                let span = offset..offset + text.len();
                offset = span.end + 1;
                PhysicalLine {
                    span,
                    uncertain: text.bytes().any(|byte| !byte.is_ascii() || byte == b'\\'),
                }
            })
            .collect();
        let mut tokens = Vec::new();
        let mut offset = 0;
        let bytes = source.as_bytes();
        while offset < bytes.len() {
            if !name_byte(bytes[offset]) {
                offset += 1;
                continue;
            }
            let start = offset;
            while offset < bytes.len() && name_byte(bytes[offset]) {
                offset += 1;
            }
            // Include digit-leading runs whole: never invent an identifier suffix.
            tokens.push(start..offset);
        }
        Self {
            source,
            lines,
            tokens,
        }
    }

    fn census(&self, owner: Range<usize>, line: usize, name: &str) -> ReviewOccurrenceCensus {
        let Some(line) = line.checked_sub(1).and_then(|index| self.lines.get(index)) else {
            return ReviewOccurrenceCensus::Unknown;
        };
        if line.uncertain || owner.start > owner.end || owner.end > self.source.len() {
            return ReviewOccurrenceCensus::Unknown;
        }
        let start = owner.start.max(line.span.start);
        let end = owner.end.min(line.span.end);
        if start > end {
            return ReviewOccurrenceCensus::Unknown;
        }
        let first = self.tokens.partition_point(|span| span.end <= start);
        let last = self.tokens.partition_point(|span| span.start < end);
        let tokens = &self.tokens[first..last];
        if tokens
            .iter()
            .any(|span| span.start < start || span.end > end)
        {
            return ReviewOccurrenceCensus::Unknown;
        }
        ReviewOccurrenceCensus::Complete(
            tokens
                .iter()
                .filter(|span| &self.source[(*span).clone()] == name)
                .cloned()
                .collect(),
        )
    }
}

#[cfg(test)]
#[path = "review_occurrence_census_tests.rs"]
mod tests;
