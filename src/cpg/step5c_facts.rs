//! Immutable caller-file work; collection order is also serial emission order.
use super::*;
use crate::ast::PathSpan;
use crate::resolution::ResolvedCallee;
use rayon::prelude::*;

pub(super) struct SiteFacts<'a> {
    pub(super) lvalues: Vec<PathSpan>,
    pub(super) resolved: Vec<ResolvedCallee<'a>>,
    pub(super) rhs_uses: Vec<NodeIndex>,
}

/// The third member is `Some` exactly when tracing was on when the facts were collected,
/// and then holds each site (index-aligned with the facts). Replay reads this, never
/// `s1_trace::enabled()` again, so a flag that flips in between cannot misalign it.
pub(super) type CallerFacts<'a> = (
    &'a FunctionId,
    Vec<SiteFacts<'a>>,
    Option<Vec<&'a CallSite>>,
);

/// Slice-taking twin of build::singleton_exact; replay needs no other outcome facts.
pub(super) fn singleton_exact<'a>(
    resolved: &'a [ResolvedCallee<'a>],
) -> Option<&'a ResolvedCallee<'a>> {
    (resolved.len() == 1 && resolved[0].confidence == ResolutionConfidence::Exact)
        .then(|| &resolved[0])
}

pub(super) fn collect<'a>(
    resolver: &crate::resolution::go::satisfiers::GoSatisfierMemo<'a>,
    files: &BTreeMap<String, ParsedFile>,
    uses: &uses::UseIndex,
) -> Vec<CallerFacts<'a>> {
    // CallGraph::calls is ordered by FunctionId (file first). Group contiguous
    // callers so every file is one independently stealable build-pool job.
    let mut by_file: Vec<Vec<_>> = Vec::new();
    for (caller, sites) in &resolver.graph().calls {
        if by_file.last().is_none_or(|group| {
            let (previous, _): &(&FunctionId, &BTreeSet<CallSite>) = &group[0];
            previous.file != caller.file
        }) {
            by_file.push(Vec::new());
        }
        by_file.last_mut().unwrap().push((caller, sites));
    }
    // One snapshot per collection: every caller sees the same flag.
    let trace = crate::cpg::s1_trace::enabled();
    #[cfg(test)]
    let work = tests::current_work();
    let per_file: Vec<Vec<CallerFacts<'a>>> = crate::build_pool::install(|| {
        crate::build_pool::per_item(&by_file)
            .map(|callers| {
                #[cfg(test)]
                let _scope = tests::inherit_work(work.clone());
                callers
                    .iter()
                    .map(|&(caller, sites)| {
                        let parsed = files.get(&caller.file);
                        let mut facts = Vec::new();
                        // Built only while tracing: the off path allocates nothing.
                        let mut traced = trace.then(Vec::new);
                        for site in sites {
                            let Some(parsed) = parsed else { break };
                            let Some(assignment) =
                                parsed.assignment_for_exact_call(site.start_byte, site.end_byte)
                            else {
                                continue;
                            };
                            let outcome = resolver.resolve_call_site_full(site);
                            // The old path queried these reads only after admitting a
                            // singleton exact callee and simple LHS. Keep that gate so
                            // unresolved calls never trigger semantic-use walks.
                            let rhs_uses = if singleton_exact(&outcome.resolved).is_some()
                                && !assignment.lvalues.is_empty()
                                && assignment
                                    .lvalues
                                    .iter()
                                    .all(|lhs| lhs.path.fields.is_empty())
                            {
                                uses.within(caller, site.start_byte, site.end_byte, parsed)
                            } else {
                                Vec::new()
                            };
                            if let Some(traced) = traced.as_mut() {
                                traced.push(site);
                            }
                            facts.push(SiteFacts {
                                lvalues: assignment.lvalues,
                                resolved: outcome.resolved,
                                rhs_uses,
                            });
                        }
                        (caller, facts, traced)
                    })
                    .collect()
            })
            .collect()
    });
    per_file.into_iter().flatten().collect()
}
