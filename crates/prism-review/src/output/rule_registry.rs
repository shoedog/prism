//! Stable metadata for public finding rules.

use prism::slice::RuleRef;

mod taint_rows;
pub use taint_rows::*;

#[derive(Debug, PartialEq, Eq)]
pub struct RuleInfo {
    pub id: &'static str,
    pub cwe: Option<&'static str>,
    pub name: &'static str,
    pub short: &'static str,
    pub help: &'static str,
    pub algorithms: &'static [&'static str],
}

impl RuleInfo {
    pub fn rule_ref(&'static self) -> RuleRef {
        RuleRef {
            id: self.id.into(),
            cwe: self.cwe.map(str::to_owned),
            severity: None,
        }
    }
}

/// All registered rules, in increasing id order.
pub fn rules() -> &'static [RuleInfo] {
    taint_rows::RULES
}

pub fn lookup(id: &str) -> Option<&'static RuleInfo> {
    let rows = rules();
    let index = rows.binary_search_by_key(&id, |row| row.id).ok()?;
    Some(&rows[index])
}

/// Check a built-in producer's reference against the registry's exact metadata.
pub fn check_emitted(algorithm: &str, rule: &RuleRef) -> Result<(), String> {
    let row = lookup(&rule.id).ok_or_else(|| format!("unknown rule id {:?}", rule.id))?;
    if rule.cwe.as_deref() != row.cwe {
        return Err(format!(
            "rule {:?} requires CWE {:?}, got {:?}",
            rule.id, row.cwe, rule.cwe
        ));
    }
    if !row.algorithms.contains(&algorithm) {
        return Err(format!(
            "rule {:?} is not registered for {algorithm:?}",
            rule.id
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
