//! Captured loading authority for review context reads (#602 S4a).
//!
//! `load_review_inputs` records, once and privately, the original lexical and canonical root, its
//! listing and a clone of its completed scope inputs. [`ReviewContextLoader`] reads one more
//! listed source file under that authority; it takes no root, listing, loaded set or options, so
//! a later change to the public inputs or the working directory cannot retarget it.

use super::ReviewInputs;
use crate::call_graph::ScopeGraphBuildInputs;
use crate::repo_loader::context_read::{self, ReadAuthority};
use crate::repo_loader::SourceListing;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use crate::repo_loader::context_read::{
    ContextFile, ContextReadAttempt, ContextReadOutcome, ContextSkip,
};

/// Why the review recorded no loading authority. The baseline inputs are unaffected.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContextUnavailable {
    ListingUnavailable { message: String },
    RootUnavailable { message: String },
    RootChangedDuringCapture,
}

/// What `load_review_inputs` captured: private, never exposed through a public field.
pub(super) struct Capsule {
    lexical_root: PathBuf,
    canonical_root: PathBuf,
    listing: Arc<BTreeSet<String>>,
    scope: ScopeGraphBuildInputs,
}

pub(super) type ContextCapture = Result<Capsule, ContextUnavailable>;

/// P1: the canonical directory of the original lexical root, observed before any other load work.
pub(super) fn observe_root(lexical_root: &Path) -> Result<PathBuf, ContextUnavailable> {
    std::fs::canonicalize(lexical_root).map_err(|error| ContextUnavailable::RootUnavailable {
        message: error.to_string(),
    })
}

/// P2: after the scope inputs and listing are complete, recheck the root and keep them.
pub(super) fn capture(
    lexical_root: &Path,
    observed: Result<PathBuf, ContextUnavailable>,
    scope: &ScopeGraphBuildInputs,
    listing: &SourceListing,
) -> ContextCapture {
    let canonical_root = observed?;
    if observe_root(lexical_root)? != canonical_root {
        return Err(ContextUnavailable::RootChangedDuringCapture);
    }
    let listing = match listing {
        Ok(listing) => Arc::clone(listing),
        Err(error) => {
            return Err(ContextUnavailable::ListingUnavailable {
                message: error.to_string(),
            })
        }
    };
    Ok(Capsule {
        lexical_root: lexical_root.to_path_buf(),
        canonical_root,
        listing,
        scope: scope.clone(),
    })
}

/// Reads individual listed repository files under one review's captured authority.
#[non_exhaustive]
pub struct ReviewContextLoader<'a> {
    capsule: &'a Capsule,
}

impl<'a> ReviewContextLoader<'a> {
    pub fn new(inputs: &'a ReviewInputs) -> Result<Self, ContextUnavailable> {
        match &inputs.context_capture {
            Ok(capsule) => Ok(Self { capsule }),
            Err(unavailable) => Err(unavailable.clone()),
        }
    }

    /// Reads and parses one repository-relative listed path, returning at most
    /// `min(2 MiB, remaining_source_bytes) + 1` source bytes. The caller keeps the capacity
    /// ledger: nothing is charged or published here, and dropping a result undoes nothing.
    pub fn read(&self, path: &str, remaining_source_bytes: u64) -> ContextReadAttempt {
        let authority = ReadAuthority {
            lexical_root: &self.capsule.lexical_root,
            canonical_root: &self.capsule.canonical_root,
            listing: &self.capsule.listing,
            original: &self.capsule.scope.all_file_paths,
        };
        context_read::read(&authority, path, remaining_source_bytes)
    }

    /// Plain scope values as of the original load (the shared JS memo cells are not detached).
    #[allow(dead_code)] // consumed by the later rebase slices
    pub(crate) fn captured_scope(&self) -> &ScopeGraphBuildInputs {
        &self.capsule.scope
    }

    #[allow(dead_code)] // consumed by the later rebase slices
    pub(crate) fn listing(&self) -> &Arc<BTreeSet<String>> {
        &self.capsule.listing
    }
}

#[cfg(test)]
#[path = "review_loading_tests.rs"]
mod tests;
