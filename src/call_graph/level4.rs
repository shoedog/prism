//! Level-4's inverted field → file → targets index. Raw-text legacy semantics
//! belong to `ast::line_field_targets`; selection must not interpret syntax.

use crate::ast::ParsedFile;
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
mod tests;

type Fields = BTreeMap<String, BTreeSet<String>>;
pub(super) type Index = BTreeMap<String, BTreeMap<String, BTreeSet<String>>>;

pub(super) fn build(files: &BTreeMap<String, ParsedFile>, known: &BTreeSet<String>) -> Index {
    let sources: Vec<_> = files
        .iter()
        .map(|(path, parsed)| (path.as_str(), parsed.source.as_str()))
        .collect();
    build_sources(&sources, known)
}

pub(super) fn build_sources(sources: &[(&str, &str)], known: &BTreeSet<String>) -> Index {
    let per_file: Vec<_> = crate::build_pool::install(|| {
        crate::build_pool::per_item(sources)
            .map(|(path, source)| {
                let mut fields = Fields::new();
                for line in source.lines() {
                    let trimmed = line.trim();
                    for field in assignment_fields(trimmed) {
                        crate::ast::line_field_targets(
                            trimmed,
                            &field,
                            known,
                            fields.entry(field.clone()).or_default(),
                        );
                    }
                }
                (*path, fields)
            })
            .collect()
    });
    let mut index = Index::new();
    // Indexed collection retains input order. Production inputs are a BTreeMap;
    // the merge and every nested map/set also retain lexical order.
    for (path, fields) in per_file {
        for (field, targets) in fields {
            if !targets.is_empty() {
                index.entry(field).or_default().insert(path.into(), targets);
            }
        }
    }
    index
}

/// Select only maximal accessor identifiers followed by whitespace and a single
/// `=`. A field omitted here cannot add any target. For each selected field the
/// matcher still sees the ENTIRE line: nonproductive arrow/prefix occurrences
/// may suppress an earlier dot assignment and must not be sliced away.
fn assignment_fields(line: &str) -> BTreeSet<String> {
    let mut fields = BTreeSet::new();
    if !line.contains('=') {
        return fields;
    }
    let bytes = line.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let accessor_len = match bytes[cursor] {
            b'.' => 1,
            b'-' if bytes.get(cursor + 1) == Some(&b'>') => 2,
            _ => {
                cursor += 1;
                continue;
            }
        };
        // ASCII accessor bytes cannot be inside a UTF-8 code point. All string
        // slices below therefore start/end at character boundaries even though
        // the outer scan advances in bytes.
        let after = &line[cursor + accessor_len..];
        let end = after
            .find(|c: char| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(after.len());
        let rest = after[end..].trim_start();
        if end > 0 && rest.starts_with('=') && !rest.starts_with("==") {
            fields.insert(after[..end].to_string());
        }
        cursor += accessor_len + end;
    }
    fields
}

/// Original serial builder, retained independently of the optimized selector
/// and merge so whole-index differential tests detect both excess and misses.
#[cfg(test)]
pub(super) fn reference(sources: &[(&str, &str)], known: &BTreeSet<String>) -> Index {
    let mut index = Index::new();
    for (path, source) in sources {
        let mut per_field = Fields::new();
        for line in source.lines() {
            let trimmed = line.trim();
            for field in crate::ast::candidate_fields_on_line(trimmed) {
                let targets = per_field.entry(field.clone()).or_default();
                crate::ast::line_field_targets(trimmed, &field, known, targets);
            }
        }
        for (field, targets) in per_field {
            if !targets.is_empty() {
                index
                    .entry(field)
                    .or_default()
                    .insert((*path).into(), targets);
            }
        }
    }
    index
}
