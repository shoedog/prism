//! Stable metadata for public finding rules.

use serde::{Deserialize, Serialize};

mod taint_rows;
pub use taint_rows::*;

/// A finding's rule identity and optional severity override.
///
/// ```compile_fail
/// use prism::slice::RuleRef;
/// let _ = RuleRef::default();
/// ```
///
/// ```compile_fail
/// use prism::slice::RuleRef;
/// let _ = "x".parse::<RuleRef>();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RuleRef {
    id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cwe: Option<String>,
    /// The rule's own severity, only when lower than the finding's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    severity: Option<String>,
}

impl RuleRef {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn cwe(&self) -> Option<&str> {
        self.cwe.as_deref()
    }

    pub fn severity(&self) -> Option<&str> {
        self.severity.as_deref()
    }

    pub fn with_severity(mut self, severity: Option<String>) -> Self {
        self.severity = severity;
        self
    }
}

/// The canonical metadata for a registered finding rule.
#[derive(Debug, PartialEq, Eq)]
pub struct RuleInfo {
    id: &'static str,
    cwe: Option<&'static str>,
    name: &'static str,
    short: &'static str,
    help: &'static str,
    algorithms: &'static [&'static str],
}

impl RuleInfo {
    pub fn id(&self) -> &str {
        self.id
    }

    pub fn cwe(&self) -> Option<&str> {
        self.cwe
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub fn short(&self) -> &str {
        self.short
    }

    pub fn help(&self) -> &str {
        self.help
    }

    pub fn algorithms(&self) -> &[&str] {
        self.algorithms
    }

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
    let row = lookup(rule.id()).ok_or_else(|| format!("unknown rule id {:?}", rule.id()))?;
    if rule.cwe() != row.cwe() {
        return Err(format!(
            "rule {:?} requires CWE {:?}, got {:?}",
            rule.id(),
            row.cwe(),
            rule.cwe()
        ));
    }
    if !row.algorithms().contains(&algorithm) {
        return Err(format!(
            "rule {:?} is not registered for {algorithm:?}",
            rule.id()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
