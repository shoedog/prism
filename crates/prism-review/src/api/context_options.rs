//! Validated private options for captured-review context.
use super::context_run::ContextRequest;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub(crate) enum TaintContextMode {
    #[default]
    Off,
    Imports,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub(crate) enum TaintContextEdges {
    Exact,
    #[default]
    NameOnly,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) struct TaintContextOptions {
    pub(crate) mode: TaintContextMode,
    pub(crate) edges: TaintContextEdges,
    pub(crate) depth: u8,
    pub(crate) max_files: usize,
}
impl Default for TaintContextOptions {
    fn default() -> Self {
        Self {
            mode: TaintContextMode::Off,
            edges: TaintContextEdges::NameOnly,
            depth: 2,
            max_files: 64,
        }
    }
}
#[derive(Debug)]
pub(crate) struct ContextOptionError(pub(crate) String);
impl std::fmt::Display for ContextOptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for ContextOptionError {}
impl TaintContextOptions {
    pub(crate) fn validate(&self) -> Result<Option<ContextRequest>, ContextOptionError> {
        if !(1..=4).contains(&self.depth) {
            return Err(ContextOptionError("depth must be in 1..=4".into()));
        }
        if self.max_files == 0 {
            return Err(ContextOptionError("max_files must be at least 1".into()));
        }
        if self.mode == TaintContextMode::Off {
            return Ok(None);
        }
        let limits = crate::algorithms::taint::context_budget::ContextLimits::new(
            self.depth,
            self.max_files,
        )
        .map_err(|_| ContextOptionError("depth must be in 1..=4".into()))?;
        let ceiling = match self.edges {
            TaintContextEdges::Exact => prism::cpg::CrossingCeiling::Exact,
            TaintContextEdges::NameOnly => prism::cpg::CrossingCeiling::NameOnly,
        };
        Ok(Some(ContextRequest { limits, ceiling }))
    }
}
