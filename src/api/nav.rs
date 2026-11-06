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
    pub owner: Option<super::OwnerOptions>,
}

pub fn nav_session(repo: &Path, opts: &NavOptions) -> Result<NavigationSession> {
    if let Some(owner) = &opts.owner {
        anyhow::ensure!(
            opts.no_cache && opts.cache_dir.is_none(),
            "owner activation requires --no-cache and no cache directory"
        );
        let mut active = None;
        owner.replace(&mut active, repo)?;
        return active.ok_or_else(|| anyhow::anyhow!("owner session was not published"));
    }
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
        crate::census::mark("nav_index_built");
        if crate::census::drop_point().as_deref() == Some("end") {
            crate::census_cg::census_cg(&index.cpg.call_graph);
            let mut index = index;
            let c = &mut index.cpg;
            crate::census::drop_measure("cpg.graph", std::mem::take(&mut c.graph));
            crate::census::drop_measure("cpg.func_index", std::mem::take(&mut c.func_index));
            crate::census::drop_measure("cpg.name_index", std::mem::take(&mut c.name_index));
            crate::census::drop_measure("cpg.var_index", std::mem::take(&mut c.var_index));
            crate::census::drop_measure("cpg.location_index", std::mem::take(&mut c.location_index));
            crate::census::drop_measure("cpg.dfg", std::mem::replace(&mut c.dfg, crate::data_flow::DataFlowGraph::empty()));
            crate::census_cg::drop_cg(&mut c.call_graph);
            crate::census::drop_measure("nav_index_rest", index);
            if let Ok(r) = Arc::try_unwrap(repo) {
                let mut r = r;
                let nfiles = r.files.len();
                let src: usize = r.files.values().map(|f| f.source.capacity()).sum();
                eprintln!("CENSUS_FILES {{\"files\":{nfiles},\"source_bytes\":{src}}}");
                crate::census::drop_measure("repo.files(ParsedFile incl. trees' Rust side)", std::mem::take(&mut r.files));
                crate::census::drop_measure("repo_rest", r);
            }
            std::process::exit(0);
        }
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
    crate::build_pool::install(|| {
        let (symbol, file, location) = seed_parts(seed);
        crate::navigation::queries::callers_with_confidence(
            session, symbol, file, location, depth, exact_only,
        )
    })
}

pub fn callees(
    session: &NavigationSession,
    seed: Seed<'_>,
    depth: usize,
    exact_only: bool,
) -> Result<Evidence, QueryError> {
    crate::build_pool::install(|| {
        let (symbol, file, location) = seed_parts(seed);
        crate::navigation::queries::callees_with_confidence(
            session, symbol, file, location, depth, exact_only,
        )
    })
}

fn seed_parts(seed: Seed<'_>) -> (Option<&str>, Option<&str>, Option<&str>) {
    match seed {
        Seed::Symbol(symbol) => (Some(symbol), None, None),
        Seed::Location(location) => (None, None, Some(location)),
        Seed::SymbolInFile { symbol, file } => (Some(symbol), Some(file), None),
    }
}
