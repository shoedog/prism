use super::*;
use std::ops::Range;

pub(super) struct Replay {
    pub(super) slots: Vec<Slot>,
    pub(super) failures: Vec<FailedRoot>,
    pub(super) diagnostics: Vec<Diagnostic>,
}

pub(super) struct Slot {
    pub(super) start: usize,
    end: usize,
    guarded: bool,
    uncertain_from: usize,
    contests: Vec<Contest>,
    pub(super) cuts: Vec<DispatchCut>,
    pub(super) refusal: Option<Reason>,
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

impl Slot {
    fn possibly_absent(&self, at: usize) -> bool {
        self.guarded || self.uncertain_from <= at
    }

    fn withhold_installation(&mut self, at: usize) {
        self.uncertain_from = self.uncertain_from.min(at);
    }
}

impl Replay {
    pub(super) fn versions(&self, index: usize) -> Vec<Version> {
        let slot = &self.slots[index];
        let span = slot.start..slot.end;
        let mut cuts = BTreeSet::from([slot.start, slot.end]);
        cuts.extend(std::iter::once(slot.uncertain_from).filter(|at| span.contains(at)));
        for contest in &slot.contests {
            let other = &self.slots[contest.other];
            cuts.extend(
                [other.start, other.end, other.uncertain_from]
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
                    if (other.start..other.end).contains(&at) {
                        if contest.loses {
                            block(&mut binding_blocked, &contest.methods);
                        }
                        set_shadow(
                            &mut shadowed,
                            if slot.possibly_absent(at) || other.possibly_absent(at) {
                                None
                            } else {
                                contest.shadowed
                            },
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
        slots: slots(events, mounts),
        failures: vec![],
        diagnostics: vec![],
    };
    let mut dispatchers: BTreeMap<&RouterId, Vec<usize>> = BTreeMap::new();
    // Per call: (slot, reason, guarded) of each proved within-call abort.
    let mut rejected_calls: BTreeMap<usize, Vec<(usize, Reason, bool)>> = BTreeMap::new();
    for (i, event) in events.iter().enumerate() {
        let at = replay.slots[i].start;
        if !admitted(event, routes, mounts) {
            replay.slots[i].end = at;
            continue;
        }
        let guarded = replay.slots[i].guarded;
        let mut aborted_maybe = false;
        if let Some(aborts) = event.call.and_then(|call| rejected_calls.get(&call)) {
            // An unguarded earlier abort stops this slot. A guarded one only
            // might, whatever either guard's text: equal text is not one shared
            // evaluation (`flag ? [item] : []` can differ between two slots).
            let earlier = aborts
                .iter()
                .find(|(rejected_at, _, abort_guarded)| *rejected_at < at && !abort_guarded)
                .map(|&(_, reason, _)| reason);
            if let Some(reason) = earlier {
                replay.slots[i].end = at;
                replay.slots[i].refusal = Some(reason);
                continue;
            }
            aborted_maybe = aborts.iter().any(|(rejected_at, ..)| *rejected_at < at);
        }
        if let Some(reason) = event.rejection {
            replay.slots[i].end = at;
            replay.slots[i].refusal = Some(reason);
            if !replay.slots[i].guarded {
                replay.reject(roots, event, reason);
            }
            // A guarded abort fails no root and only possibly aborts later siblings.
            if let Some(call) = event.call {
                rejected_calls
                    .entry(call)
                    .or_default()
                    .push((at, reason, guarded));
            }
            continue;
        }

        let active = dispatchers.entry(&event.dispatcher).or_default();
        active.retain(|&prior| at < replay.slots[prior].end);
        // Resolve admission against the original state before applying writes
        // to any earlier slot on this dispatcher.
        let mut comparisons = Vec::new();
        let mut uncertain_admission = false;
        for &prior in active.iter() {
            if replay.slots[prior].start == at {
                continue;
            }
            let interaction = relation.interact(&events[prior], event);
            let prior_possible = replay.slots[prior].possibly_absent(at);
            match &interaction {
                Interaction::Reject(methods) if !prior_possible && !replay.slots[i].guarded => {
                    let reason = Reason::DispatchConflict;
                    replay.slots[i].end = at;
                    replay.slots[i].refusal = Some(reason);
                    replay.reject(roots, event, reason);
                    if let Some(call) = event.call {
                        rejected_calls
                            .entry(call)
                            .or_default()
                            .push((at, reason, guarded));
                    }
                    replay.slots[i]
                        .cuts
                        .push(cut(roots, event, &events[prior], methods, reason));
                    break;
                }
                Interaction::Skip if !prior_possible => {
                    replay.slots[i].end = at;
                    replay.slots[i].refusal = Some(Reason::DispatchConflict);
                    break;
                }
                Interaction::Reject(_) | Interaction::Skip | Interaction::Unknown { .. } => {
                    uncertain_admission = true;
                }
                _ => {}
            }
            comparisons.push((prior, interaction));
        }
        if replay.slots[i].end == at {
            continue;
        }
        if uncertain_admission || aborted_maybe {
            replay.slots[i].withhold_installation(at);
        }
        let conditional_write = replay.slots[i].possibly_absent(at);
        for (prior, interaction) in comparisons {
            replay.apply(roots, events, i, prior, conditional_write, interaction);
        }
        active.push(i);
    }

    for (event, slot) in events.iter().zip(&replay.slots) {
        if let Target::Route { record, .. } = event.target {
            for cut in &slot.cuts {
                record_cut(&mut routes[record], cut.clone());
            }
        }
    }
    replay.failures.sort();
    replay.failures.dedup();
    replay.diagnostics.sort();
    replay.diagnostics.dedup();
    replay
}

fn slots<K>(events: &[Event<K>], mounts: &[Mount]) -> Vec<Slot> {
    let mut out = Vec::with_capacity(events.len());
    for group in events.chunk_by(|a, b| {
        matches!((&a.target, &b.target),
            (Target::Mount { record: left, epoch: left_epoch, .. },
             Target::Mount { record: right, epoch: right_epoch, .. })
                if a.dispatcher == b.dispatcher && left == right && left_epoch == right_epoch)
    }) {
        let start = out.len() + group.len();
        out.extend(group.iter().map(|event| Slot {
            start,
            end: events.len() + 1,
            guarded: match &event.target {
                Target::Route { path, .. } => path.setup_guard.is_some(),
                Target::Mount { record, .. } => mounts[*record].setup_guard.is_some(),
                Target::Internal => false,
            },
            uncertain_from: usize::MAX,
            contests: vec![],
            cuts: vec![],
            refusal: None,
        }));
    }
    out
}

impl Replay {
    fn reject<K>(&mut self, roots: &[RouterId], event: &Event<K>, reason: Reason) {
        let mut record = |root: &RouterId| {
            self.failures.push(FailedRoot {
                root: root.clone(),
                site: event.site.clone(),
                reason,
            });
        };
        match event.setup_root.as_ref() {
            Some(root) if roots.contains(root) => record(root),
            Some(_) => {}
            None if roots.contains(&event.dispatcher) => record(&event.dispatcher),
            None => {
                roots.iter().for_each(record);
                self.diagnostics.push(Diagnostic {
                    site: event.site.clone(),
                    reason,
                });
            }
        }
    }

    fn apply<K>(
        &mut self,
        roots: &[RouterId],
        events: &[Event<K>],
        i: usize,
        prior: usize,
        conditional_write: bool,
        interaction: Interaction,
    ) {
        let at = self.slots[i].start;
        let add_cut = |slot: &mut Slot, affected, competing, methods, reason| {
            slot.cuts.push(cut(
                roots,
                &events[affected],
                &events[competing],
                methods,
                reason,
            ));
        };
        match interaction {
            Interaction::Disjoint | Interaction::CoInstall | Interaction::Skip => {}
            Interaction::Overwrite(methods) => {
                let slot = &mut self.slots[prior];
                let reason = if conditional_write {
                    slot.withhold_installation(at);
                    Reason::DispatchConflict
                } else {
                    slot.end = at;
                    Reason::Superseded
                };
                add_cut(slot, prior, i, &methods, reason);
            }
            Interaction::Reject(methods) => add_cut(
                &mut self.slots[i],
                i,
                prior,
                &methods,
                Reason::DispatchConflict,
            ),
            Interaction::Unknown {
                methods,
                keep_earlier,
            } => {
                add_cut(
                    &mut self.slots[i],
                    i,
                    prior,
                    &methods,
                    Reason::DispatchConflict,
                );
                if !keep_earlier {
                    self.slots[prior].withhold_installation(at);
                }
                // Earlier admission can be guaranteed even while its
                // effective callback could lose to an admitted later slot.
                self.slots[prior].contests.push(Contest {
                    other: i,
                    methods: methods.clone(),
                    shadowed: None,
                    loses: true,
                });
                add_cut(
                    &mut self.slots[prior],
                    prior,
                    i,
                    &methods,
                    Reason::DispatchConflict,
                );
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
                for (slot, other, (shadowed, loses)) in [(prior, i, first), (i, prior, second)] {
                    self.slots[slot].contests.push(Contest {
                        other,
                        methods: methods.clone(),
                        shadowed,
                        loses,
                    });
                    if loses {
                        add_cut(
                            &mut self.slots[slot],
                            slot,
                            other,
                            &methods,
                            Reason::DispatchConflict,
                        );
                    }
                }
            }
        }
    }
}

fn admitted<K>(event: &Event<K>, routes: &[Route], mounts: &[Mount]) -> bool {
    let receiver = match &event.target {
        Target::Route { record, .. } => {
            let route = &routes[*record];
            if route.reason == Some(Reason::BuildOnly) {
                return false;
            }
            &route.receiver
        }
        Target::Mount { record, .. } => &mounts[*record].parent,
        Target::Internal => return true,
    };
    matches!(receiver, Receiver::Router { identity, .. } if identity == &event.dispatcher)
}

fn cut<K>(
    roots: &[RouterId],
    affected: &Event<K>,
    competing: &Event<K>,
    methods: &MethodSet,
    reason: Reason,
) -> DispatchCut {
    let setup_root = |event: &Event<K>| {
        let root = event.setup_root.as_ref().unwrap_or(&event.dispatcher);
        roots.contains(root).then(|| root.clone())
    };
    DispatchCut {
        root: setup_root(affected).or_else(|| setup_root(competing)),
        method_guard: methods.clone(),
        path: match &affected.target {
            Target::Route { path, .. } => path.status.value().map(str::to_owned),
            _ => None,
        },
        site: competing.site.clone(),
        reason,
    }
}

pub(super) fn record_cut(route: &mut Route, cut: DispatchCut) {
    let cuts = if cut.reason == Reason::Superseded {
        &mut route.superseded_keys
    } else {
        &mut route.conflicts
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
