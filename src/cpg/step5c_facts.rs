//! Immutable caller-file work; collection order is also serial emission order.
use super::*;
use crate::ast::AssignmentCallInfo;
use rayon::prelude::*;

pub(super) struct SiteFacts<'a> {
    pub(super) assignment: AssignmentCallInfo,
    pub(super) outcome: ResolutionOutcome<'a>,
    pub(super) rhs_uses: Vec<NodeIndex>,
}

pub(super) type CallerFacts<'a> = (&'a FunctionId, Vec<Option<SiteFacts<'a>>>);

pub(super) fn collect<'a>(
    resolver: &crate::resolution::go_satisfiers::GoSatisfierMemo<'a>,
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
                        let facts = sites
                            .iter()
                            .map(|site| {
                                let assignment = parsed?
                                    .assignment_for_exact_call(site.start_byte, site.end_byte)?;
                                let outcome = resolver.resolve_call_site_full(site);
                                // The old path queried these reads only after admitting a
                                // singleton exact callee and simple LHS. Keep that gate so
                                // unresolved calls never trigger semantic-use walks.
                                let rhs_uses = if singleton_exact(&outcome).is_some()
                                    && !assignment.lvalues.is_empty()
                                    && assignment
                                        .lvalues
                                        .iter()
                                        .all(|lhs| lhs.path.fields.is_empty())
                                {
                                    uses.within(
                                        caller,
                                        site.start_byte,
                                        site.end_byte,
                                        parsed.unwrap(),
                                    )
                                } else {
                                    Vec::new()
                                };
                                Some(SiteFacts {
                                    assignment,
                                    outcome,
                                    rhs_uses,
                                })
                            })
                            .collect();
                        (caller, facts)
                    })
                    .collect()
            })
            .collect()
    });
    per_file.into_iter().flatten().collect()
}
