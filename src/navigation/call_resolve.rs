use crate::call_graph::{CallGraph, CallSite, FunctionId};
use crate::resolution::{DropReason, ResolutionConfidence, ResolutionKind, ResolvedCallee};

/// One resolved nav call edge with the metadata needed for score and
/// `Reason::Resolution`.
#[derive(Debug, Clone)]
pub struct NavCallEdge<'a> {
    pub target: &'a FunctionId,
    pub call_site_line: usize,
    pub qualifier: Option<String>,
    pub confidence: ResolutionConfidence,
    pub kind: ResolutionKind,
}

pub fn resolve_site_nav<'a>(cg: &'a CallGraph, site: &'a CallSite) -> Vec<NavCallEdge<'a>> {
    cg.resolve_call_site(site)
        .into_iter()
        .map(|r: ResolvedCallee<'a>| NavCallEdge {
            target: r.target,
            call_site_line: site.line,
            qualifier: site.qualifier.clone(),
            confidence: r.confidence,
            kind: r.kind,
        })
        .collect()
}

/// Collision-dropped same-name receiver sites for a seed name. Counts only R6
/// multi-owner collisions, not external/import/unknown drops.
pub fn collision_dropped_sites(cg: &CallGraph, seed_name: &str) -> usize {
    scoped_caller_sites(cg, seed_name)
        .into_iter()
        .filter(|site| {
            cg.resolve_call_site_full(site).drop == Some(DropReason::MultiOwnerCollision)
        })
        .count()
}

/// Caller sites targeting a function named `target_name`, including scoped keys
/// (`callers` is keyed by the raw callee name, so a scoped call lives under
/// `"A::target_name"`). Bare and `::target_name`-suffixed keys are both returned;
/// the caller resolves each site to confirm the target file.
pub fn scoped_caller_sites<'a>(cg: &'a CallGraph, target_name: &str) -> Vec<&'a CallSite> {
    let suffix = format!("::{target_name}");
    let mut out: Vec<&CallSite> = Vec::new();
    for (key, sites) in &cg.callers {
        if key == target_name || key.ends_with(&suffix) {
            out.extend(sites.iter());
        }
    }
    out
}
