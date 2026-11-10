use super::records::*;
use prism::api::{ReviewHintBasis, ReviewImportHint, ReviewUnresolvedReason};
use prism::languages::Language;
use std::fmt::Write;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LocatedBoundaryNote {
    pub(crate) record: BoundaryRecord,
    pub(crate) message: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct BoundaryDisplay {
    pub(crate) located: Vec<LocatedBoundaryNote>,
    pub(crate) omitted_count: usize,
    pub(crate) availability: Availability,
}
impl LocatedBoundaryNote {
    pub fn file(&self) -> &str {
        &self.record.site.caller.file
    }
    pub fn line(&self) -> usize {
        self.record.line
    }
    pub fn callee(&self) -> &str {
        &self.record.callee
    }
    pub fn byte_range(&self) -> std::ops::Range<usize> {
        self.record.site.start_byte..self.record.site.end_byte
    }
    pub fn hint(&self) -> &ReviewImportHint {
        &self.record.hint
    }
    pub fn kind(&self) -> BoundaryKind {
        self.record.kind
    }
    pub fn language(&self) -> Language {
        self.record.language
    }
    pub fn message(&self) -> &str {
        &self.message
    }
}
impl BoundaryDisplay {
    pub fn located(&self) -> &[LocatedBoundaryNote] {
        &self.located
    }
    pub fn omitted_count(&self) -> usize {
        self.omitted_count
    }
    pub fn availability(&self) -> &Availability {
        &self.availability
    }
    pub fn summary(&self) -> Option<String> {
        (self.omitted_count > 0).then(|| {
            format!(
                "taint-context: {} more boundary calls not listed",
                self.omitted_count
            )
        })
    }
}
/// Accepts coherent collector records or identical duplicates, preserving input.
pub(crate) fn project(collection: &BoundaryCollection) -> BoundaryDisplay {
    let mut ordered: Vec<_> = collection.records.iter().collect();
    ordered.sort_by(|a, b| display_order_key(a).cmp(&display_order_key(b)));
    ordered.dedup_by(|a, b| same_identity(a, b));
    let shown = ordered.len().min(10);
    let omitted_count = ordered.len() - shown;
    BoundaryDisplay {
        located: ordered
            .into_iter()
            .take(shown)
            .map(|record| LocatedBoundaryNote {
                record: record.clone(),
                message: message(record),
            })
            .collect(),
        omitted_count,
        availability: collection.availability.clone(),
    }
}
fn raw_targets(record: &BoundaryRecord) -> &[String] {
    match &record.hint {
        ReviewImportHint::InRepo { files, .. } => files,
        _ => &[],
    }
}
fn same_identity(a: &BoundaryRecord, b: &BoundaryRecord) -> bool {
    a.site == b.site && raw_targets(a) == raw_targets(b)
}
fn display_order_key(record: &BoundaryRecord) -> (&str, usize, &str, &[String], &PhysicalSite) {
    (
        &record.site.caller.file,
        record.line,
        &record.callee,
        raw_targets(record),
        &record.site,
    )
}
fn message(record: &BoundaryRecord) -> String {
    let suffix = match &record.hint {
        ReviewImportHint::InRepo {
            files,
            declaring,
            spec,
            basis,
            ..
        } => {
            let origin = origin(record.language, spec);
            if record.language == Language::Go {
                if declaring.is_empty() {
                    format!("Go {origin} has candidate package files {} outside the analysed file set; the declaring file for this call is unknown.", paths(files))
                } else {
                    let advice = if declaring.len() == 1 {
                        "is likely in"
                    } else {
                        "may be in one of"
                    };
                    format!("Go {origin}; the declaration scan suggests the callee {advice} {} (outside the analysed file set; not analysed).", paths(declaring))
                }
            } else {
                let advice = match basis {
                    ReviewHintBasis::Bound => "suggests the callee is likely in",
                    ReviewHintBasis::Candidates if files.len() == 1 => {
                        "suggests the callee is likely in"
                    }
                    ReviewHintBasis::Candidates => "suggests the callee may be in one of",
                    _ => "has candidate repo files",
                };
                format!(
                    "{origin} {advice} {} (outside the analysed file set; not analysed).",
                    paths(files)
                )
            }
        }
        ReviewImportHint::Unresolvable { spec, why } => {
            let reason = match why {
                ReviewUnresolvedReason::NoFile => {
                    "no matching source file was found in the captured repository listing"
                }
                ReviewUnresolvedReason::TooMany => {
                    "its repository attribution is ambiguous or exceeds the candidate limit"
                }
                ReviewUnresolvedReason::ReplaceUnproven => {
                    "the Go replacement's repository identity could not be proved"
                }
                _ => "its repository target could not be determined",
            };
            format!(
                "{} looks in-repo, but {reason}.",
                origin(record.language, spec)
            )
        }
        _ => "repository target could not be determined.".into(),
    };
    format!(
        "taint-context: tainted value reaches {}(...) at {}:{}; {suffix}",
        quoted_token(&record.callee),
        quoted_token(&record.site.caller.file),
        record.line
    )
}
fn origin(language: Language, spec: &str) -> String {
    if language == Language::Go && spec.is_empty() {
        "same-package hint".into()
    } else {
        format!("import {}", quoted_token(spec))
    }
}
fn paths(paths: &[String]) -> String {
    format!(
        "[{}]",
        paths
            .iter()
            .map(|p| quoted_token(p))
            .collect::<Vec<_>>()
            .join(", ")
    )
}
fn quoted_token(text: &str) -> String {
    format!("\"{}\"", escape_content(text))
}
pub(crate) fn escape_content(text: &str) -> String {
    let mut quoted = String::new();
    for c in text.chars() {
        match c {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            '\t' => quoted.push_str("\\t"),
            c if c.is_control() || extra_control(c) => {
                write!(quoted, "\\u{{{:x}}}", u32::from(c)).unwrap();
            }
            c => quoted.push(c),
        }
    }
    quoted
}
fn extra_control(c: char) -> bool {
    matches!(c, '\u{2028}' | '\u{2029}' | '\u{61c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}
#[cfg(test)]
#[path = "display_tests.rs"]
mod tests;
