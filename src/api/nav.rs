use crate::navigation::types::{Evidence, QueryError};
use crate::navigation::{NavigationIndex, NavigationSession};
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Default)]
#[non_exhaustive]
pub struct NavOptions {
    pub no_cache: bool,
    pub cache_dir: Option<PathBuf>,
}

pub fn nav_session(repo: &Path, opts: &NavOptions) -> Result<NavigationSession> {
    crate::build_pool::install(|| {
        let repo = Arc::new(crate::repo_loader::load_repo(repo)?);
        let index = if opts.no_cache {
            NavigationIndex::build(&repo)
        } else {
            match opts.cache_dir.as_deref() {
                Some(base) => NavigationIndex::build_cached_under(&repo, base),
                None => NavigationIndex::build_cached(&repo),
            }
        };
        let index = Arc::new(index);
        Ok(NavigationSession { repo, index })
    })
}

#[non_exhaustive]
pub enum Seed<'a> {
    Symbol(&'a str),
    Location(&'a str),
    SymbolInFile { symbol: &'a str, file: &'a str },
}

pub fn callers(
    session: &NavigationSession,
    seed: Seed<'_>,
    depth: usize,
    exact_only: bool,
) -> Result<Evidence, QueryError> {
    let (symbol, file, location) = seed_parts(seed);
    crate::navigation::queries::callers_with_confidence(
        session, symbol, file, location, depth, exact_only,
    )
}

pub fn callees(
    session: &NavigationSession,
    seed: Seed<'_>,
    depth: usize,
    exact_only: bool,
) -> Result<Evidence, QueryError> {
    let (symbol, file, location) = seed_parts(seed);
    crate::navigation::queries::callees_with_confidence(
        session, symbol, file, location, depth, exact_only,
    )
}

fn seed_parts(seed: Seed<'_>) -> (Option<&str>, Option<&str>, Option<&str>) {
    match seed {
        Seed::Symbol(symbol) => (Some(symbol), None, None),
        Seed::Location(location) => (None, None, Some(location)),
        Seed::SymbolInFile { symbol, file } => (Some(symbol), Some(file), None),
    }
}
