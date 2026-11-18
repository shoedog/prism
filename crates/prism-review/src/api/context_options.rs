//! Validated options for captured-review taint context.
use super::context_run::ContextRequest;

/// Default maximum context depth; supported values are 1 through 4.
pub const DEFAULT_TAINT_CONTEXT_DEPTH: u8 = 2;
/// Default maximum number of admitted context files; supported values are at least 1.
pub const DEFAULT_TAINT_CONTEXT_MAX_FILES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum TaintContextMode {
    #[default]
    Off,
    Imports,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum TaintContextEdges {
    Exact,
    #[default]
    NameOnly,
}
/// Context loading defaults to Off. Imports permits only supported import-directed taint
/// continuations, with fixed 8 MiB read and 20 s cooperative time limits. An in-progress
/// read or build is not interrupted. Hints are loading advice; notes are not findings.
/// Construct with `Default` and mutate fields; validation also applies while Off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct TaintContextOptions {
    pub mode: TaintContextMode,
    pub edges: TaintContextEdges,
    pub depth: u8,
    pub max_files: usize,
}
impl Default for TaintContextOptions {
    fn default() -> Self {
        Self {
            mode: TaintContextMode::Off,
            edges: TaintContextEdges::NameOnly,
            depth: DEFAULT_TAINT_CONTEXT_DEPTH,
            max_files: DEFAULT_TAINT_CONTEXT_MAX_FILES,
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
