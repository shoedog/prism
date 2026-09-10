//! Same-epoch graph integration and cache-free session construction.
use super::*;
use crate::call_graph::CallSite;

/// Private, same-owner installation. No serde, target setter, or raw packet API.
pub(crate) struct InstalledOwner {
    epoch: AuthenticatedProgramEpoch,
    entries: BTreeMap<CallKey, (CallSite, ExecutableOwnerProof)>,
}
impl std::fmt::Debug for InstalledOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstalledOwner")
            .field("calls", &self.entries.len())
            .finish_non_exhaustive()
    }
}
impl InstalledOwner {
    /// None means this is not one of our calls. Some(None) is terminal.
    pub(crate) fn target<'a>(
        &self,
        graph: &'a CallGraph,
        site: &CallSite,
    ) -> Option<Option<&'a FunctionId>> {
        let key = (site.caller.file.clone(), site.start_byte, site.end_byte);
        let (original, proof) = self.entries.get(&key)?;
        let target = self.epoch.target_for(proof, &key.0, key.1, key.2);
        Some(
            target
                .filter(|_| {
                    original == site
                        && graph
                            .calls
                            .get(&site.caller)
                            .is_some_and(|sites| sites.iter().any(|s| s == site))
                })
                .and_then(|target| {
                    graph.functions.get(&target.name).and_then(|pool| {
                        let matches: Vec<_> = pool.iter().filter(|f| *f == target).collect();
                        (matches.len() == 1).then(|| matches[0])
                    })
                }),
        )
    }
}

fn install(epoch: &AuthenticatedProgramEpoch, graph: &mut CallGraph) {
    let entries = graph
        .calls
        .values()
        .flatten()
        .filter_map(|site| {
            let key = (site.caller.file.clone(), site.start_byte, site.end_byte);
            epoch
                .prove(&key.0, key.1, key.2)
                .map(|proof| (key, (site.clone(), proof)))
        })
        .collect();
    graph.executable_owner = Some(Arc::new(InstalledOwner {
        epoch: AuthenticatedProgramEpoch(Arc::clone(&epoch.0)),
        entries,
    }));
    graph.owner_ephemeral = true;
}

pub(crate) fn staged_graph(epoch: &AuthenticatedProgramEpoch) -> CallGraph {
    let mut graph = CallGraph::build(&epoch.0._files);
    install(epoch, &mut graph);
    graph
}

/// Validate a supplied full JS/TS census before constructing ANY derived graph.
/// Return the owned/reparsed inputs with it so CPG assembly cannot substitute a
/// second file map. Non-JS languages retain their ordinary loader semantics.
pub(crate) fn graph_and_inputs(
    epoch: &AuthenticatedProgramEpoch,
    files: &BTreeMap<String, ParsedFile>,
    scope_inputs: Option<&crate::call_graph::ScopeGraphBuildInputs>,
) -> Result<(CallGraph, BTreeMap<String, ParsedFile>)> {
    ensure(
        files.iter().all(|(f, p)| {
            relative(f) && p.path == *f && Language::from_path(f) == Some(p.language)
        }),
        "integration_input_identity",
    )?;
    let census: BTreeMap<_, _> = files
        .iter()
        .filter(|(_, p)| js(p.language))
        .map(|(f, p)| (f.clone(), hash(p.source.as_bytes())))
        .collect();
    let expected: BTreeMap<_, _> = epoch
        .0
        ._files
        .iter()
        .map(|(f, p)| (f.clone(), hash(p.source.as_bytes())))
        .collect();
    ensure(census == expected, "integration_input_set")?;
    let mut owned = files.clone();
    owned.extend(epoch.0._files.clone());
    let mut graph = CallGraph::build_with_scope_graph_inputs(&owned, scope_inputs);
    install(epoch, &mut graph);
    Ok((graph, owned))
}

pub(crate) fn staged_subset(
    epoch: &AuthenticatedProgramEpoch,
    only: &BTreeSet<String>,
) -> Result<CallGraph> {
    ensure(
        only.iter().all(|f| epoch.0._files.contains_key(f)),
        "subset_input",
    )?;
    let mut graph = CallGraph::build_direct_subset(&epoch.0._files, only);
    install(epoch, &mut graph);
    Ok(graph)
}

/// Internal staging entry, retaining the original mixed-input test seam.
pub(crate) fn replace_session(
    active: &mut Option<crate::navigation::NavigationSession>,
    root: &Path,
    config: &str,
    compiler: &Path,
) -> Result<()> {
    replace_session_impl(
        active,
        root,
        config,
        compiler,
        false,
        &mut admission::Report::new(),
    )
}

/// Selected public route: only JS/TS inputs until mixed-language type enrichment
/// has its own proof. Do not drop files to make the input census agree.
pub(crate) fn replace_selected_session(
    active: &mut Option<crate::navigation::NavigationSession>,
    root: &Path,
    config: &str,
    compiler: &Path,
) -> std::result::Result<(), admission::Failure> {
    let mut report = admission::Report::new();
    replace_session_impl(active, root, config, compiler, true, &mut report)
        .map_err(|reason| report.refusal(reason))
}

fn replace_session_impl(
    active: &mut Option<crate::navigation::NavigationSession>,
    root: &Path,
    config: &str,
    compiler: &Path,
    selected: bool,
    report: &mut admission::Report,
) -> Result<()> {
    *active = None;
    let mut repo = crate::repo_loader::load_repo(root).map_err(|_| "index_unavailable")?;
    report.phase = admission::Phase::SelectInputs;
    if selected {
        report.observe(&repo);
        ensure(
            repo.type_db.is_none() && repo.files.values().all(|f| js(f.language)),
            "owner_requires_js_ts_only",
        )?;
    }
    let epoch = AuthenticatedProgramEpoch::acquire_observed(
        root,
        config,
        compiler,
        repo.files.clone(),
        report,
    )?;
    report.phase = admission::Phase::SessionBuild;
    repo.files.extend(epoch.0._files.clone());
    let index = crate::build_pool::install(|| -> Result<_> {
        let cpg = crate::cpg::CodePropertyGraph::build_with_staged_owner(
            &epoch,
            &repo.files,
            repo.scope_graph_inputs.as_ref(),
        )?;
        Ok(crate::navigation::NavigationIndex::from_ctx(
            crate::cpg::CpgContext::build_with_fresh_cpg(&repo.files, cpg, repo.type_db.as_ref()),
        ))
    })?;
    *active = Some(crate::navigation::NavigationSession {
        repo: Arc::new(repo),
        index: Arc::new(index),
    });
    Ok(())
}

#[cfg(all(test, feature = "detached-owner-audit"))]
mod tests;
