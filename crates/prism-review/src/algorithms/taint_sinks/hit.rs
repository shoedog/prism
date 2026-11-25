use super::Strength;

/// A matched flat sink. Strength drives finding severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SinkHit {
    rule: &'static str,
    cwe: Option<&'static str>,
    strength: Strength,
}

impl SinkHit {
    pub(super) const LEGACY: Self = Self {
        rule: "legacy",
        cwe: None,
        strength: Strength::Strong,
    };

    pub(super) const fn new(rule: &'static str, cwe: &'static str) -> Self {
        Self {
            rule,
            cwe: Some(cwe),
            strength: Strength::Strong,
        }
    }

    /// The same hit, graded weak when `weak` holds.
    pub(super) fn weak_if(mut self, weak: bool) -> Self {
        if weak {
            self.strength = Strength::Weak;
        }
        self
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn rule(&self) -> &'static str {
        self.rule
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn cwe(&self) -> Option<&'static str> {
        self.cwe
    }

    pub(super) fn strength(&self) -> Strength {
        self.strength
    }
}
