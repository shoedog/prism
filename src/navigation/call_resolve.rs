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
    // Aliased imports: `from m import f as g; g()` has callee_name "g" (the local),
    // so the site lives under callers key "g", not target_name ("f"). Gather those
    // sites by consulting eligible member-import bindings whose member == target_name.
    // direct_callers re-resolves each site and keeps it only if it actually reaches
    // THIS target (full FunctionId identity), so a same-named alias to a different
    // module is filtered out — this arm only widens the candidate set.
    //
    // qualifier.is_none(): R4c only resolves UNQUALIFIED calls, and the shared
    // `collision_dropped_sites` consumer counts collisions WITHOUT an identity
    // filter — so a qualified `x.g()` site under key "g" must not be pulled in here
    // (it can't resolve via R4c anyway, and would otherwise be miscounted).
    for (file, bindings) in &cg.import_bindings {
        for b in bindings {
            if b.eligible
                && matches!(b.kind, crate::call_graph::ImportBindingKind::MemberImport)
                && b.local != target_name
                && b.member.as_deref() == Some(target_name)
            {
                if let Some(sites) = cg.callers.get(&b.local) {
                    out.extend(
                        sites
                            .iter()
                            .filter(|s| s.caller.file == *file && s.qualifier.is_none()),
                    );
                }
            }
        }
    }
    out
}
