//! Seed resolution type definitions for the Tier-2 reasoning layer.

use crate::data_flow::VarLocation;
use crate::navigation::types::{SymbolRef, Warning};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeedSpec {
    Loc { file: String, line: usize },
    Symbol { name: String, file: Option<String> },
}

#[derive(Debug, Clone)]
pub struct ResolvedSeed {
    pub locations: Vec<VarLocation>,
    pub symbol: Option<SymbolRef>,
    pub origin: SeedSpec,
}

#[derive(Debug, Clone, Default)]
pub struct SeedSet {
    pub seeds: Vec<ResolvedSeed>,
    pub warnings: Vec<Warning>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_types_construct() {
        let spec = SeedSpec::Loc {
            file: "a.py".into(),
            line: 3,
        };
        assert!(matches!(spec, SeedSpec::Loc { line: 3, .. }));
        assert!(SeedSet::default().seeds.is_empty());
    }
}
