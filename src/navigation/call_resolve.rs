use crate::call_graph::{CallGraph, CallSite, FunctionId};
use crate::resolution::{ResolutionConfidence, ResolutionKind, ResolvedCallee};

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
    cg.resolve_call_site_full(site)
        .resolved
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

pub(crate) fn scoped_caller_site_match_count(
    cg: &CallGraph,
    bucket_key: &str,
    site: &CallSite,
    target_name: &str,
    suffix: &str,
) -> usize {
    let mut count = usize::from(bucket_key == target_name || bucket_key.ends_with(suffix));

    if site.qualifier.is_some() {
        return count;
    }

    if let Some(bindings) = cg.import_bindings.get(&site.caller.file) {
        for b in bindings {
            if b.eligible
                && matches!(b.kind, crate::call_graph::ImportBindingKind::MemberImport)
                && b.local != target_name
                && b.local == bucket_key
                && b.member.as_deref() == Some(target_name)
            {
                count += 1;
            }
        }
    }

    count
}

/// Caller sites targeting a function named `target_name`, including scoped keys
/// (`callers` is keyed by the raw callee name, so a scoped call lives under
/// `"A::target_name"`). Bare and `::target_name`-suffixed keys are both returned;
/// the caller resolves each site to confirm the target file.
pub fn scoped_caller_sites<'a>(cg: &'a CallGraph, target_name: &str) -> Vec<&'a CallSite> {
    let suffix = format!("::{target_name}");
    let mut out: Vec<&CallSite> = Vec::new();
    for (key, sites) in &cg.callers {
        for site in sites {
            for _ in 0..scoped_caller_site_match_count(cg, key, site, target_name, &suffix) {
                out.push(site);
            }
        }
    }
    out
}
