use super::*;
use crate::algorithms::taint::boundary::{
    collect::site_has_loaded_callable, records::BoundaryCollection,
};
use prism::cpg::CpgContext;
use std::collections::BTreeSet;

/// One planning pass over the complete eligible collection of a completed round.
/// The caller supplies its same-round graph and round-bound one-hop adapter.
pub(crate) fn plan_round(
    next_round: u8,
    depth: u8,
    collection: &BoundaryCollection,
    ctx: &CpgContext<'_>,
    carried: &FrontierState,
    refused: &BTreeSet<String>,
    hop: &mut dyn FnMut(&str, &str) -> Option<(String, String)>,
) -> FrontierState {
    let mut state = carried.clone();
    state.open.clear();
    state.load_order.clear();
    let mut candidates = Vec::new();
    for mut request in carried.open.clone() {
        state.dispositions.retain(|(origin, target, disposition)| {
            !(*origin == request.origin
                && *target == request.target
                && *disposition == Disposition::Requested)
        });
        if request.depth > depth {
            state
                .dispositions
                .push((request.origin, request.target, Disposition::DepthBound));
        } else if ctx.files.contains_key(&request.target) {
            if site_has_loaded_callable(ctx, &state.provenance[request.origin].site) {
                state
                    .dispositions
                    .push((request.origin, request.target, Disposition::Resolved));
            } else if let Some(member) = request.member.as_deref() {
                if let Some((target, member)) = hop(&request.target, member) {
                    let pair = (target, Some(member));
                    if request.visited.contains(&pair) {
                        state
                            .dispositions
                            .push((request.origin, pair.0, Disposition::BarrelCycle));
                    } else {
                        request.visited.push(pair.clone());
                        request.target = pair.0;
                        request.member = pair.1;
                        request.depth = next_round;
                        candidates.push(request);
                    }
                } else {
                    state.dispositions.push((
                        request.origin,
                        request.target,
                        Disposition::LoadedUnresolved,
                    ));
                }
            } else {
                state.dispositions.push((
                    request.origin,
                    request.target,
                    Disposition::LoadedUnresolved,
                ));
            }
        } else if refused.contains(&request.target) {
            state
                .dispositions
                .push((request.origin, request.target, Disposition::TargetRefused));
        } else {
            candidates.push(request);
        }
    }
    let mut records: Vec<_> = collection.records.iter().collect();
    records.sort_by(|a, b| {
        (
            &a.site.caller.file,
            a.line,
            &a.callee,
            &a.site,
            format!("{:?}", a.hint),
        )
            .cmp(&(
                &b.site.caller.file,
                b.line,
                &b.callee,
                &b.site,
                format!("{:?}", b.hint),
            ))
    });
    records.dedup_by(|a, b| *a == *b);
    let mut known = BTreeSet::new();
    for provenance in &state.provenance {
        if let ReviewImportHint::InRepo {
            files,
            declaring,
            member,
            ..
        } = &provenance.hint
        {
            let paths = if provenance.language == Language::Go {
                declaring
            } else {
                files
            };
            for target in paths {
                known.insert((provenance.site.clone(), target.clone(), member.clone()));
            }
        }
    }
    for record in records {
        let unseen: Vec<_> = match &record.hint {
            ReviewImportHint::InRepo {
                files,
                declaring,
                member,
                ..
            } => {
                let paths = if record.language == Language::Go {
                    declaring
                } else {
                    files
                };
                paths
                    .iter()
                    .filter(|target| {
                        known.insert((record.site.clone(), (*target).clone(), member.clone()))
                    })
                    .cloned()
                    .collect()
            }
            _ => Vec::new(),
        };
        let seen_record = state
            .provenance
            .iter()
            .any(|p| p.site == record.site && p.hint == record.hint);
        if unseen.is_empty()
            && (seen_record
                || matches!(&record.hint, ReviewImportHint::InRepo { files, declaring, .. } if !(if record.language == Language::Go { declaring } else { files }).is_empty()))
        {
            continue;
        }
        let origin = state.provenance.len();
        state.provenance.push(CallProvenance {
            site: record.site.clone(),
            line: record.line,
            callee: record.callee.clone(),
            language: record.language,
            hint: record.hint.clone(),
            first_round: next_round.saturating_sub(1),
        });
        match &record.hint {
            ReviewImportHint::InRepo {
                files,
                declaring,
                member,
                ..
            } => {
                let targets = if record.language == Language::Go {
                    declaring
                } else {
                    files
                };
                if targets.is_empty() && record.language == Language::Go {
                    state.dispositions.push((
                        origin,
                        String::new(),
                        Disposition::GoDeclarationUnknown,
                    ));
                }
                for target in &unseen {
                    let pair = (target.clone(), member.clone());
                    candidates.push(Request {
                        target: target.clone(),
                        member: member.clone(),
                        depth: next_round,
                        origin,
                        visited: vec![pair],
                    });
                }
            }
            ReviewImportHint::Unresolvable { .. } => {
                state
                    .dispositions
                    .push((origin, String::new(), Disposition::Unresolvable))
            }
            _ => state
                .dispositions
                .push((origin, String::new(), Disposition::Unresolvable)),
        }
    }
    candidates.sort_by(|a, b| {
        let x = &state.provenance[a.origin];
        let y = &state.provenance[b.origin];
        (
            a.depth,
            &x.site.caller.file,
            x.line,
            &x.callee,
            &a.target,
            &x.site,
        )
            .cmp(&(
                b.depth,
                &y.site.caller.file,
                y.line,
                &y.callee,
                &b.target,
                &y.site,
            ))
    });
    let mut seen = BTreeSet::new();
    for request in candidates {
        if request.depth > depth {
            state
                .dispositions
                .push((request.origin, request.target, Disposition::DepthBound));
        } else if refused.contains(&request.target) && !ctx.files.contains_key(&request.target) {
            state
                .dispositions
                .push((request.origin, request.target, Disposition::TargetRefused));
        } else {
            if request.depth == next_round
                && !ctx.files.contains_key(&request.target)
                && seen.insert(request.target.clone())
            {
                state.load_order.push(request.target.clone());
            }
            state.dispositions.push((
                request.origin,
                request.target.clone(),
                Disposition::Requested,
            ));
            state.open.push(request);
        }
    }
    state
}
