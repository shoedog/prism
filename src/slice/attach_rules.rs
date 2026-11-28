use super::SliceFinding;
use crate::rule_registry::RuleInfo;
use std::collections::BTreeMap;

/// Replace a built-in finding's rule list with the complete selection for that finding.
/// Each input is a named registry row paired with whether its occurrence is weak.
pub fn attach_rules(
    finding: &mut SliceFinding,
    rows: impl IntoIterator<Item = (&'static RuleInfo, bool)>,
) {
    let mut by_id = BTreeMap::new();
    for (row, weak) in rows {
        by_id
            .entry(row.id())
            .and_modify(|(_, all_weak)| *all_weak &= weak)
            .or_insert((row, weak));
    }
    finding.rules = by_id
        .into_values()
        .map(|(row, all_weak)| {
            let severity = (all_weak && finding.severity == "warning").then(|| "suggestion".into());
            row.rule_ref().with_severity(severity)
        })
        .collect();
}

#[cfg(test)]
mod tests;
