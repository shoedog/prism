//! Receipt and selection of adapter-proved operands for already admitted sinks.

use super::matches_sink;
use prism::ast::ParsedFile;
use prism::rule_registry::{RuleInfo, CONFIGURED_SINK, FORMAT_WRAPPER};
use std::collections::BTreeSet;

#[cfg_attr(not(test), allow(dead_code))]
mod certificate;
mod evidence;
use evidence::{Detector, Occurrence, Role};

type Selection = (&'static RuleInfo, bool);
type FindingKey<'a> = (&'a str, usize);

pub(super) struct NameSinks<'a> {
    configured: &'a [String],
    wrappers: &'a [String],
}

impl<'a> NameSinks<'a> {
    pub(super) fn new(configured: &'a [String], wrappers: &'a [String]) -> Self {
        Self {
            configured,
            wrappers,
        }
    }

    pub(super) fn admission_patterns(&self) -> impl Iterator<Item = &'a str> {
        self.configured
            .iter()
            .chain(self.wrappers)
            .map(String::as_str)
    }

    /// Select the supplied name-detector origin after the adapter's flat filters.
    fn select(&self, witness: &Occurrence<'_, '_>, rows: &mut Vec<Selection>) {
        let site = witness.site();
        if !matches!(site.role(), Role::Argument(_) | Role::KeywordArgument(_)) {
            return;
        }
        let callee = witness.parsed().node_text(&site.target());
        match witness.detector() {
            Detector::Configured(entry)
                if self.configured.iter().any(|name| name == entry)
                    && matches_sink(callee, entry) =>
            {
                rows.push((&CONFIGURED_SINK, false));
            }
            Detector::Wrapper(entry)
                if self.wrappers.iter().any(|name| name == entry)
                    && entry.strip_prefix('=') == Some(callee) =>
            {
                rows.push((&FORMAT_WRAPPER, false));
            }
            _ => {}
        }
    }
}

/// Collect all channels for one admitted finding in the selected analysis variant.
/// Keep every received path available to its guard, even for repeated occurrences.
fn collect_rules<'a, 'p>(
    parsed: &'a ParsedFile,
    line: usize,
    admitted: &BTreeSet<(String, usize)>,
    witnesses: impl IntoIterator<Item = Occurrence<'a, 'p>>,
    mut project: impl FnMut(&Occurrence<'a, 'p>, &mut Vec<Selection>),
) -> Vec<Selection> {
    let mut rows = Vec::new();
    if line == 0 || !admitted.contains(&(parsed.path.clone(), line)) {
        return rows;
    }
    for witness in witnesses {
        if witness.verifies(parsed, (&parsed.path, line)) {
            project(&witness, &mut rows);
        }
    }
    rows
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod price;

#[cfg(test)]
pub(super) mod old_visits;
