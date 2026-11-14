//! Shared test helpers of the Go, Rust and Python occurrence classifiers.
use super::{Classified, Disposition, TwinPolicy};
use crate::ast::{ParsedFile, PathSpan};
use crate::data_flow::build::{FileRefs, Producer};
use crate::data_flow::VarAccessKind;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn parse(file: &str, source: &str, language: Language) -> ParsedFile {
    ParsedFile::parse(file, source, language).unwrap()
}

fn name(d: Disposition) -> &'static str {
    match d {
        Disposition::BindingWrite => "binding-write",
        Disposition::StorageWrite => "storage-write",
        Disposition::Unsupported => "unsupported",
    }
}

/// `line -> {"base:disposition:rule"}` for every lvalue span of every owner.
pub(super) fn classified(
    parsed: &ParsedFile,
    classify: fn(&ParsedFile, &PathSpan) -> Classified,
) -> BTreeMap<usize, BTreeSet<String>> {
    let mut out: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    for func in parsed.all_functions() {
        let (start, end) = parsed.node_line_range(&func);
        let lines: BTreeSet<usize> = (start..=end).collect();
        for span in parsed.assignment_lvalue_spans_on_lines(&func, &lines) {
            let c = classify(parsed, &span);
            out.entry(span.line).or_default().insert(format!(
                "{}:{}:{}",
                span.path,
                name(c.disposition),
                c.rule
            ));
        }
    }
    out
}

/// Check every `EXPECT tok..` token of the fixture. A `storage-write` token also accepts
/// `unsupported` (both keep the twin; the classifier does not read the declaration evidence that
/// separates them for selector and index targets).
pub(super) fn check_fixture(
    fixture: &str,
    source: &str,
    language: Language,
    classify: fn(&ParsedFile, &PathSpan) -> Classified,
) {
    let got = classified(&parse(fixture, source, language), classify);
    let mut missing = Vec::new();
    let mut count = 0;
    for (i, line) in source.lines().enumerate() {
        let Some(pos) = line.find("EXPECT ") else {
            continue;
        };
        for token in line[pos + "EXPECT ".len()..].split_whitespace() {
            count += 1;
            let set = got.get(&(i + 1));
            let relaxed = token.split(':').collect::<Vec<_>>();
            let ok = set.is_some_and(|s| {
                s.contains(token)
                    || (relaxed.get(1) == Some(&"storage-write")
                        && s.iter()
                            .any(|g| g.starts_with(&format!("{}:unsupported:", relaxed[0]))))
            });
            if !ok {
                missing.push(format!("{fixture}:{} {token} (got {set:?})", i + 1));
            }
        }
    }
    assert!(count >= 10, "{fixture}: fixture pins too little");
    assert!(missing.is_empty(), "{missing:#?}");
}

pub(super) fn dispositions(
    source: &str,
    file: &str,
    language: Language,
    classify: fn(&ParsedFile, &PathSpan) -> Classified,
    line: usize,
) -> BTreeSet<String> {
    classified(&parse(file, source, language), classify)
        .remove(&line)
        .unwrap_or_default()
}

/// Lines (1-based) of the occurrences the policy suppresses.
pub(super) fn suppressed_lines(file: &str, source: &str, language: Language) -> Vec<usize> {
    let parsed = parse(file, source, language);
    let mut out = Vec::new();
    for func in parsed.all_functions() {
        let (start, end) = parsed.node_line_range(&func);
        let lines: BTreeSet<usize> = (start..=end).collect();
        let (alias_map, raw) = Producer::build_alias_map(&parsed, &func, &lines);
        let spans = parsed.assignment_lvalue_spans_on_lines(&func, &lines);
        let policy = TwinPolicy::for_owner(&parsed, &func, &spans, &alias_map, &raw);
        out.extend(
            spans
                .iter()
                .filter(|s| policy.suppresses_span(s.start_byte))
                .map(|s| s.line),
        );
    }
    out.sort_unstable();
    out.dedup();
    out
}

pub(super) fn refs(file: &str, source: &str, language: Language) -> FileRefs {
    let parsed = parse(file, source, language);
    Producer::file_refs(&file.to_string(), &parsed)
}

/// Alias twins: Defs of another path sitting at the very bytes of a Def of `alias`.
pub(super) fn twins(r: &FileRefs, alias: &str) -> Vec<(String, usize)> {
    let all: Vec<_> = r
        .defs
        .values()
        .flatten()
        .filter(|d| d.kind == VarAccessKind::Def)
        .collect();
    let starts: BTreeSet<usize> = all
        .iter()
        .filter(|d| d.path.base == alias)
        .map(|d| d.start_byte)
        .collect();
    all.iter()
        .filter(|d| d.path.base != alias && starts.contains(&d.start_byte))
        .map(|d| (d.path.to_string(), d.line))
        .collect()
}

/// Fallback anchors the policy suppresses, as `(alias, line)`.
pub(super) fn anchors(file: &str, source: &str, language: Language) -> Vec<(String, usize)> {
    let parsed = parse(file, source, language);
    let mut out = Vec::new();
    for func in parsed.all_functions() {
        let (start, end) = parsed.node_line_range(&func);
        let lines: BTreeSet<usize> = (start..=end).collect();
        let (alias_map, raw) = Producer::build_alias_map(&parsed, &func, &lines);
        let spans = parsed.assignment_lvalue_spans_on_lines(&func, &lines);
        let policy = TwinPolicy::for_owner(&parsed, &func, &spans, &alias_map, &raw);
        for (alias, _, line) in &raw {
            if policy.suppresses_anchor(alias, *line)
                && !spans
                    .iter()
                    .any(|s| s.line == *line && s.path.is_simple() && s.path.base == *alias)
            {
                out.push((alias.clone(), *line));
            }
        }
    }
    out.sort();
    out.dedup();
    out
}
