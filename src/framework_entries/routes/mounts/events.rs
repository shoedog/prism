use super::*;
use std::ops::Range;

pub(super) struct Replay {
    slots: Vec<Slot>,
    pub(super) failures: Vec<FailedRoot>,
    pub(super) diagnostics: Vec<Diagnostic>,
}

struct Slot {
    end: usize,
    uncertain_from: usize,
    contests: Vec<Contest>,
}

struct Contest {
    other: usize,
    methods: MethodSet,
    shadowed: Option<bool>,
    loses: bool,
}

pub(super) struct Version {
    pub(super) epochs: Range<usize>,
    pub(super) uncertain: bool,
    binding_blocked: Option<MethodSet>,
    shadowed: Option<Option<bool>>,
}

impl Version {
    pub(super) fn apply(&self, path: &mut Exposure) {
        if let Some(methods) = &self.binding_blocked {
            block(&mut path.binding_blocked, methods);
        }
        if let Some(shadowed) = self.shadowed {
            set_shadow(&mut path.shadowed, shadowed);
        }
    }
}

impl Replay {
    pub(super) fn versions(&self, index: usize) -> Vec<Version> {
        let slot = &self.slots[index];
        let start = index + 1;
        let span = start..slot.end;
        let mut cuts = BTreeSet::from([start, slot.end]);
        cuts.extend(std::iter::once(slot.uncertain_from).filter(|at| span.contains(at)));
        for contest in &slot.contests {
            let other = &self.slots[contest.other];
            cuts.extend(
                [contest.other + 1, other.end, other.uncertain_from]
                    .into_iter()
                    .filter(|at| span.contains(at)),
            );
        }
        let cuts: Vec<_> = cuts.into_iter().collect();
        cuts.windows(2)
            .map(|bounds| {
                let at = bounds[0];
                let uncertain = slot.uncertain_from <= at;
                let mut binding_blocked = uncertain.then_some(MethodSet::ANY);
                let mut shadowed = None;
                for contest in &slot.contests {
                    let other = &self.slots[contest.other];
                    if (contest.other + 1..other.end).contains(&at) {
                        let conditional = uncertain || other.uncertain_from <= at;
                        if conditional || contest.loses {
                            block(&mut binding_blocked, &contest.methods);
                        }
                        set_shadow(
                            &mut shadowed,
                            if conditional { None } else { contest.shadowed },
                        );
                    }
                }
                Version {
                    epochs: at..bounds[1],
                    uncertain,
                    binding_blocked,
                    shadowed,
                }
            })
            .collect()
    }
}

pub(super) fn replay<K>(
    routes: &mut [Route],
    mounts: &[Mount],
    roots: &[RouterId],
    events: &[Event<K>],
    relation: &impl Relation<K>,
) -> Replay {
    let mut replay = Replay {
        slots: events
            .iter()
            .map(|_| Slot {
                end: events.len() + 1,
                uncertain_from: usize::MAX,
                contests: vec![],
            })
            .collect(),
        failures: vec![],
        diagnostics: vec![],
    };
    let mut dispatchers: BTreeMap<&RouterId, Vec<usize>> = BTreeMap::new();
    let mut rejected_calls = BTreeSet::new();
    for (i, event) in events.iter().enumerate() {
        let at = i + 1;
        if !admitted(event, routes, mounts)
            || event
                .call
                .is_some_and(|call| rejected_calls.contains(&call))
        {
            replay.slots[i].end = at;
            continue;
        }
        let active = dispatchers.entry(&event.dispatcher).or_default();
        active.retain(|&prior| at < replay.slots[prior].end);
        let mut comparisons = Vec::new();
        for &prior in active.iter() {
            let earlier = &events[prior];
            let guarded = |event: &Event<K>| match &event.target {
                Target::Route(_, path) => path.setup_guard.is_some(),
                Target::Mount(index, _) => mounts[*index].setup_guard.is_some(),
                Target::Internal => false,
            };
            let interaction = match relation.interact(earlier, event) {
                Interaction::Reject(methods)
                    if guarded(event)
                        || guarded(earlier)
                        || replay.slots[prior].uncertain_from <= at =>
                {
                    Interaction::Unknown {
                        methods,
                        keep_earlier: true,
                    }
                }
                Interaction::Overwrite(methods) if guarded(event) => Interaction::Unknown {
                    methods,
                    keep_earlier: false,
                },
                Interaction::Priority { methods, priority }
                    if (matches!(priority, Priority::Earlier { .. }) && guarded(earlier))
                        || (matches!(priority, Priority::Later { .. }) && guarded(event)) =>
                {
                    Interaction::Priority {
                        methods,
                        priority: Priority::Unknown,
                    }
                }
                interaction => interaction,
            };
            if matches!(interaction, Interaction::Reject(_) | Interaction::Skip) {
                replay.slots[i].end = at;
                if let Interaction::Reject(methods) = interaction {
                    rejected_calls.extend(event.call);
                    cut(
                        routes,
                        roots,
                        event,
                        earlier,
                        &methods,
                        Reason::DispatchConflict,
                    );
                    if let Some(root) = setup_root(event, roots) {
                        replay.failures.push(FailedRoot {
                            root: root.clone(),
                            site: event.site.clone(),
                            reason: Reason::DispatchConflict,
                        });
                    } else {
                        replay.diagnostics.push(Diagnostic {
                            site: event.site.clone(),
                            reason: Reason::DispatchConflict,
                        });
                    }
                }
                break;
            }
            comparisons.push((prior, interaction));
        }
        if replay.slots[i].end == at {
            continue;
        }
        for (prior, interaction) in comparisons {
            let earlier = &events[prior];
            match interaction {
                Interaction::Disjoint | Interaction::CoInstall => {}
                Interaction::Overwrite(methods) => {
                    replay.slots[prior].end = at;
                    cut(routes, roots, earlier, event, &methods, Reason::Superseded);
                }
                Interaction::Unknown {
                    methods,
                    keep_earlier,
                } => {
                    replay.slots[i].uncertain_from = at;
                    cut(
                        routes,
                        roots,
                        event,
                        earlier,
                        &methods,
                        Reason::DispatchConflict,
                    );
                    if !keep_earlier {
                        let slot = &mut replay.slots[prior];
                        slot.uncertain_from = slot.uncertain_from.min(at);
                        cut(
                            routes,
                            roots,
                            earlier,
                            event,
                            &methods,
                            Reason::DispatchConflict,
                        );
                    }
                }
                Interaction::Priority { methods, priority } => {
                    let (first, second) = match priority {
                        Priority::Unknown => ((None, true), (None, true)),
                        Priority::Earlier { entire } => {
                            ((Some(false), false), (entire.then_some(true), true))
                        }
                        Priority::Later { entire } => {
                            ((entire.then_some(true), true), (Some(false), false))
                        }
                    };
                    for (slot, other, (shadowed, loses)) in [(prior, i, first), (i, prior, second)]
                    {
                        replay.slots[slot].contests.push(Contest {
                            other,
                            methods: methods.clone(),
                            shadowed,
                            loses,
                        });
                        if loses {
                            cut(
                                routes,
                                roots,
                                &events[slot],
                                &events[other],
                                &methods,
                                Reason::DispatchConflict,
                            );
                        }
                    }
                }
                Interaction::Reject(_) | Interaction::Skip => {
                    unreachable!("terminal outcome handled before writes")
                }
            }
        }
        active.push(i);
    }
    replay.failures.sort();
    replay.failures.dedup();
    replay
}

fn admitted<K>(event: &Event<K>, routes: &[Route], mounts: &[Mount]) -> bool {
    match &event.target {
        Target::Route(index, _) => {
            let route = &routes[*index];
            route.reason != Some(Reason::BuildOnly)
                && matches!(route.receiver, Receiver::Router { .. })
        }
        Target::Mount(index, _) => {
            matches!(&mounts[*index].parent, Receiver::Router { identity, .. } if identity == &event.dispatcher)
        }
        Target::Internal => true,
    }
}

fn setup_root<'a, K>(event: &'a Event<K>, roots: &[RouterId]) -> Option<&'a RouterId> {
    event
        .setup_root
        .as_ref()
        .or(Some(&event.dispatcher))
        .filter(|root| roots.contains(root))
}

fn cut<K>(
    routes: &mut [Route],
    roots: &[RouterId],
    affected: &Event<K>,
    competing: &Event<K>,
    methods: &MethodSet,
    reason: Reason,
) {
    let Target::Route(index, path) = &affected.target else {
        return;
    };
    let route = &mut routes[*index];
    let cuts = if reason == Reason::Superseded {
        &mut route.superseded_keys
    } else {
        &mut route.conflicts
    };
    let cut = DispatchCut {
        root: setup_root(affected, roots)
            .or_else(|| setup_root(competing, roots))
            .cloned(),
        method_guard: methods.clone(),
        path: path.status.value().map(str::to_owned),
        site: competing.site.clone(),
        reason,
    };
    if !cuts.contains(&cut) {
        cuts.push(cut);
    }
}

fn block(blocked: &mut Option<MethodSet>, methods: &MethodSet) {
    match (&mut *blocked, methods) {
        (None, MethodSet::Known { .. }) => *blocked = Some(methods.clone()),
        (Some(MethodSet::Known { known }), MethodSet::Known { known: added }) => {
            known.extend(added.iter().cloned());
        }
        _ => *blocked = Some(MethodSet::ANY),
    }
}

fn set_shadow(previous: &mut Option<Option<bool>>, next: Option<bool>) {
    *previous = match (*previous, next) {
        (Some(Some(true)), _) | (_, Some(true)) => Some(Some(true)),
        (Some(None), _) | (_, None) => Some(None),
        _ => Some(Some(false)),
    };
}
