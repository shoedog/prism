//! One expansion round's hints and context graph (#602 S4b2).
//!
//! A round binds ONE parsed map (the original files plus admitted context files), its own
//! rebased scope inputs and the review's captured listing. Hints, the one-hop barrel step and
//! the context graph all read that same map. A round reads no cache, walks nothing, and never
//! changes the review's inputs; a hint is loading advice, never an edge or a proof.

use super::review_context::ReviewBoundaryHints;
use super::review_loading::ReviewContextLoader;
use crate::ast::ParsedFile;
use crate::call_graph::ScopeGraphBuildInputs;
use crate::cpg::CpgContext;
use crate::repo_loader::context_rebase::rebase_scope_inputs;
use crate::type_db::TypeDatabase;
use crate::type_provider::TypeRegistry;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[non_exhaustive]
pub struct ReviewContextRound<'a> {
    files: &'a BTreeMap<String, ParsedFile>,
    scope: ScopeGraphBuildInputs,
    listing: Arc<BTreeSet<String>>,
}

impl<'a> ReviewContextRound<'a> {
    /// `files` is the union map. The scope inputs are rebased from the loader's captured ones
    /// and the listing is the captured one; the caller supplies neither, nor a root.
    pub fn new(loader: &ReviewContextLoader<'_>, files: &'a BTreeMap<String, ParsedFile>) -> Self {
        Self {
            files,
            scope: rebase_scope_inputs(loader.captured_scope(), files),
            listing: loader.listing().clone(),
        }
    }

    /// Hints whose caller lookup and loaded identities both come from this round's map.
    pub fn hints(&'a self) -> anyhow::Result<ReviewBoundaryHints<'a>> {
        ReviewBoundaryHints::for_round(self.files, &self.scope, &self.listing)
    }

    /// A full, uncached graph over this round's map with this round's rebased scope inputs. The
    /// baseline registry's `target_versions` and `mode` are kept; providers are rebuilt.
    pub fn build_context(
        &'a self,
        type_db: Option<&'a TypeDatabase>,
        baseline: &TypeRegistry,
    ) -> CpgContext<'a> {
        let mut ctx = crate::build_pool::install(|| {
            CpgContext::build_with_scope_graph_inputs(self.files, type_db, Some(&self.scope))
        });
        ctx.types.target_versions = baseline.target_versions.clone();
        ctx.types.mode = baseline.mode;
        ctx
    }
}

#[cfg(test)]
#[path = "review_round_tests.rs"]
mod tests;
