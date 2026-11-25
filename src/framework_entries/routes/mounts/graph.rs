use super::*;
use events::{record_cut, Replay, Slot, Version};
use paths::{apply_guards, cut_path, eligible, suffix, tokenize};
use petgraph::{algo::kosaraju_scc, graph::DiGraph};
use std::ops::Range;

const MAX_EXPOSURES: usize = 64;
const MAX_WORK: usize = 4096;

pub(super) fn project<K>(
    routes: &mut [Route],
    mounts: &[Mount],
    roots: &[RouterId],
    events: &[Event<K>],
    replay: &Replay,
    relation: &impl Relation<K>,
) {
    let versions: Vec<_> = (0..events.len()).map(|i| replay.versions(i)).collect();
    let mut graph = Graph {
        final_epoch: events.len(),
        epochs: BTreeSet::from([events.len()]),
        descendants: BTreeMap::new(),
        literal_descendants: BTreeMap::new(),
        parents: BTreeMap::new(),
        live_cycles: BTreeSet::new(),
        failed: replay
            .failures
            .iter()
            .map(|failure| &failure.root)
            .collect(),
    };
    let mut origins = vec![Vec::new(); routes.len()];
    let mut refusals = vec![None; routes.len()];
    for (index, event) in events.iter().enumerate() {
        let slot = &replay.slots[index];
        match &event.target {
            Target::Route {
                record,
                path,
                effective_methods,
            } => {
                refusals[*record] = refusals[*record].or(slot.refusal);
                origins[*record].extend(versions[index].iter().map(|version| Seed {
                    dispatcher: &event.dispatcher,
                    site: &event.site,
                    path,
                    effective_methods,
                    version,
                }));
            }
            Target::Mount { record, epoch, key } => {
                let mount = &mounts[*record];
                if let Receiver::Router {
                    identity: child, ..
                } = &mount.child.receiver
                {
                    let edges = graph.parents.entry(child).or_default();
                    if edges.last().is_none_or(|edge| edge.start != slot.start) {
                        edges.push(Edge {
                            mount,
                            parent: &event.dispatcher,
                            mode: *epoch,
                            capture: index,
                            start: slot.start,
                            slots: BTreeMap::new(),
                        });
                    }
                    edges.last_mut().unwrap().slots.insert(
                        *key,
                        EdgeSlot {
                            slot,
                            versions: &versions[index],
                        },
                    );
                }
            }
            Target::Internal => {}
        }
    }
    for edges in graph.parents.values_mut() {
        edges.retain(|edge| {
            edge.slots.keys().any(Option::is_some)
                || edge.slots.values().any(|slot| !slot.versions.is_empty())
        });
        for edge in edges.iter() {
            if matches!(edge.mode, MountEpoch::Snapshot) {
                graph.epochs.insert(edge.capture);
            }
        }
        edges.sort_by_key(|edge| {
            (
                matches!(edge.mount.prefix, MountPrefix::Unevaluated { .. }),
                edge.mount.sort_key(),
                edge.capture,
            )
        });
    }
    graph.index(roots);
    for (index, (route, seeds)) in routes.iter_mut().zip(&mut origins).enumerate() {
        seeds.sort_by_key(|seed| {
            (
                !matches!(seed.path.status, ExposureStatus::Full { .. }),
                seed.site,
            )
        });
        let mut cuts = Vec::new();
        route.paths = graph.exposures(seeds, relation, &mut cuts);
        for cut in cuts {
            record_cut(route, cut);
        }
        if route.reason.is_none()
            && route.paths.is_empty()
            && !seeds
                .iter()
                .any(|seed| seed.version.epochs.contains(&graph.final_epoch))
        {
            route.reason = refusals[index]
                .or_else(|| (!route.superseded_keys.is_empty()).then_some(Reason::Superseded));
        }
    }
}

struct Graph<'a> {
    final_epoch: usize,
    epochs: BTreeSet<usize>,
    descendants: BTreeMap<&'a RouterId, BTreeSet<&'a RouterId>>,
    literal_descendants: BTreeMap<&'a RouterId, BTreeSet<&'a RouterId>>,
    parents: BTreeMap<&'a RouterId, Vec<Edge<'a>>>,
    live_cycles: BTreeSet<&'a RouterId>,
    failed: BTreeSet<&'a RouterId>,
}

struct Edge<'a> {
    mount: &'a Mount,
    parent: &'a RouterId,
    mode: MountEpoch,
    capture: usize,
    start: usize,
    slots: BTreeMap<Option<MountKey>, EdgeSlot<'a>>,
}

struct EdgeSlot<'a> {
    slot: &'a Slot,
    versions: &'a [Version],
}

#[derive(Clone, Copy)]
struct Seed<'a> {
    dispatcher: &'a RouterId,
    site: &'a Site,
    path: &'a Exposure,
    effective_methods: &'a MethodSet,
    version: &'a Version,
}

#[derive(Clone, PartialEq, Eq)]
struct Branch<'a> {
    node: &'a RouterId,
    seen: Vec<&'a RouterId>,
    epochs: Range<usize>,
    path: Exposure,
    effective_methods: MethodSet,
    uncertain: bool,
}

impl Ord for Branch<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (
            self.node,
            self.epochs.start,
            self.epochs.end,
            &self.seen,
            self.uncertain,
            &self.effective_methods,
            self.path.sort_key(),
        )
            .cmp(&(
                other.node,
                other.epochs.start,
                other.epochs.end,
                &other.seen,
                other.uncertain,
                &other.effective_methods,
                other.path.sort_key(),
            ))
    }
}

impl PartialOrd for Branch<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Copy)]
struct Destination<'a> {
    root: Option<&'a RouterId>,
    exact: bool,
}

struct Search {
    remaining: usize,
    paths: Vec<Exposure>,
}

impl Search {
    fn spend(&mut self) -> bool {
        if self.remaining == 0 {
            return false;
        }
        self.remaining -= 1;
        true
    }

    fn emit(&mut self, path: Exposure) -> bool {
        if self.paths.contains(&path) {
            return true;
        }
        if self.paths.len() == MAX_EXPOSURES {
            return false;
        }
        self.paths.push(path);
        true
    }
}

type Children<'a> = BTreeMap<&'a RouterId, Vec<&'a RouterId>>;

impl<'a> Graph<'a> {
    fn index(&mut self, roots: &'a [RouterId]) {
        let mut children = Children::new();
        let mut literal_children = Children::new();
        let mut live = DiGraph::new();
        let mut nodes = BTreeMap::new();
        for (&child, parents) in &self.parents {
            for edge in parents {
                children.entry(edge.parent).or_default().push(child);
                if !matches!(edge.mount.prefix, MountPrefix::Unevaluated { .. }) {
                    literal_children.entry(edge.parent).or_default().push(child);
                }
                if matches!(edge.mode, MountEpoch::Live) {
                    let parent = *nodes
                        .entry(edge.parent)
                        .or_insert_with(|| live.add_node(edge.parent));
                    let child = *nodes.entry(child).or_insert_with(|| live.add_node(child));
                    live.add_edge(parent, child, ());
                }
            }
        }
        self.live_cycles.extend(
            kosaraju_scc(&live)
                .into_iter()
                .filter(|group| group.len() > 1 || live.contains_edge(group[0], group[0]))
                .flatten()
                .map(|node| live[node]),
        );
        for root in roots {
            self.descendants.insert(root, reachable(root, &children));
            self.literal_descendants
                .insert(root, reachable(root, &literal_children));
        }
    }

    fn exposures<K>(
        &self,
        seeds: &[Seed<'a>],
        relation: &impl Relation<K>,
        cuts: &mut Vec<DispatchCut>,
    ) -> Vec<Exposure> {
        let mut search = Search {
            remaining: MAX_WORK,
            paths: vec![],
        };
        for exact in [true, false] {
            for root in self
                .descendants
                .keys()
                .copied()
                .map(Some)
                .chain((!exact).then_some(None))
            {
                let destination = Destination { root, exact };
                for seed in seeds {
                    let Some(epochs) = self.active_epochs(seed.version.epochs.clone()) else {
                        continue;
                    };
                    let mut path = seed.path.clone();
                    let Some(effective_methods) =
                        eligible(seed.effective_methods, Some(&path.method_guard))
                    else {
                        continue;
                    };
                    seed.version.apply(&mut path);
                    let first = Branch {
                        node: seed.dispatcher,
                        seen: vec![],
                        epochs,
                        path,
                        effective_methods,
                        uncertain: seed.version.uncertain,
                    };
                    if !self.walk(first, destination, relation, &mut search, cuts) {
                        let mut marker = seed.path.clone();
                        marker.root = root.cloned();
                        marker.binding_blocked = Some(MethodSet::ANY);
                        cut_path(&mut marker, Reason::ExpansionTruncated);
                        tokenize(&mut marker, relation);
                        search.paths.push(marker);
                        return search.paths;
                    }
                }
            }
        }
        search.paths
    }

    /// Returns false if this origin ran out of work or exposure capacity.
    fn walk<K>(
        &self,
        first: Branch<'a>,
        destination: Destination<'a>,
        relation: &impl Relation<K>,
        search: &mut Search,
        cuts: &mut Vec<DispatchCut>,
    ) -> bool {
        let Destination { root, exact } = destination;
        let reachability = if exact {
            &self.literal_descendants
        } else {
            &self.descendants
        };
        let reachable = root.map(|root| &reachability[root]);
        let reaches_root = |node: &RouterId| reachable.is_none_or(|nodes| nodes.contains(node));
        if !reaches_root(first.node) {
            return true;
        }
        let mut pending = vec![first];
        let mut visited = BTreeSet::new();
        while let Some(mut branch) = pending.pop() {
            if !search.spend() {
                return false;
            }
            if exact
                && (branch.uncertain || !matches!(branch.path.status, ExposureStatus::Full { .. }))
            {
                continue;
            }
            if !visited.insert(branch.clone()) {
                continue;
            }
            let current = branch.epochs.contains(&self.final_epoch);
            if branch.seen.contains(&branch.node) {
                if !exact && current && (root.is_none() || root == Some(branch.node)) {
                    cut_path(&mut branch.path, Reason::MountCycle);
                    if !self.emit(branch.path, None, relation, search) {
                        return false;
                    }
                }
                continue;
            }
            if current && root == Some(branch.node) {
                let mut path = branch.path.clone();
                if branch.uncertain {
                    suffix(&mut path, Reason::DispatchConflict);
                }
                if exact == matches!(path.status, ExposureStatus::Full { .. })
                    && !self.emit(path, root, relation, search)
                {
                    return false;
                }
            }
            if current && root.is_none() && self.descendants.contains_key(branch.node) {
                continue;
            }
            // A snapshot moves the capture earlier when read backwards,
            // permitting another visit to the same identity at a new epoch.
            if self.live_cycles.contains(branch.node) {
                branch.seen.push(branch.node);
            }
            let mut next = Vec::new();
            let mut attached_at_listing = false;
            for edge in self.parents.get(branch.node).into_iter().flatten() {
                if !reaches_root(edge.parent)
                    || (exact && matches!(edge.mount.prefix, MountPrefix::Unevaluated { .. }))
                    || !edge.can_read(&branch.epochs)
                {
                    continue;
                }
                if !search.spend() {
                    return false;
                }
                let Some((path, methods)) = edge.join(&branch, relation) else {
                    attached_at_listing |= edge.active_at(self.final_epoch, &branch.epochs);
                    continue;
                };
                let key = relation.mount_key(edge.mount, &path, &methods);
                let Some(slot) = edge.slots.get(&key) else {
                    continue;
                };
                for cut in &slot.slot.cuts {
                    let mut cut = cut.clone();
                    cut.path = path.status.value().map(str::to_owned);
                    if !cuts.contains(&cut) {
                        cuts.push(cut);
                    }
                }
                if key.is_some() && (!slot.slot.cuts.is_empty() || slot.slot.refusal.is_some()) {
                    attached_at_listing |= current || matches!(edge.mode, MountEpoch::Snapshot);
                }
                for version in slot.versions {
                    if !search.spend() {
                        return false;
                    }
                    let span = edge.span(&branch.epochs, &version.epochs);
                    let Some(epochs) = self.active_epochs(span) else {
                        continue;
                    };
                    attached_at_listing |= epochs.contains(&self.final_epoch);
                    let mut path = path.clone();
                    version.apply(&mut path);
                    next.push(Branch {
                        node: edge.parent,
                        seen: match edge.mode {
                            MountEpoch::Live => branch.seen.clone(),
                            MountEpoch::Snapshot => vec![],
                        },
                        epochs,
                        path,
                        effective_methods: methods.clone(),
                        uncertain: branch.uncertain || version.uncertain,
                    });
                }
            }
            if root.is_none() && current && !attached_at_listing {
                suffix(&mut branch.path, Reason::NoKnownMount);
                if !self.emit(branch.path, None, relation, search) {
                    return false;
                }
            }
            pending.extend(next.into_iter().rev());
        }
        true
    }

    fn emit<K>(
        &self,
        mut path: Exposure,
        root: Option<&RouterId>,
        relation: &impl Relation<K>,
        search: &mut Search,
    ) -> bool {
        path.root = root.cloned();
        if root.is_some_and(|root| self.failed.contains(root)) {
            path.binding_blocked = Some(MethodSet::ANY);
        }
        tokenize(&mut path, relation);
        search.emit(path)
    }

    fn active_epochs(&self, span: Range<usize>) -> Option<Range<usize>> {
        if span.is_empty() {
            return None;
        }
        let mut epochs = self.epochs.range(span);
        let first = *epochs.next()?;
        Some(first..epochs.next_back().copied().unwrap_or(first) + 1)
    }
}

impl Edge<'_> {
    fn can_read(&self, epochs: &Range<usize>) -> bool {
        match self.mode {
            MountEpoch::Live => epochs.end > self.start,
            MountEpoch::Snapshot => epochs.contains(&self.capture),
        }
    }

    fn span(&self, child: &Range<usize>, mount: &Range<usize>) -> Range<usize> {
        match self.mode {
            MountEpoch::Live => child.start.max(mount.start)..child.end.min(mount.end),
            MountEpoch::Snapshot => mount.clone(),
        }
    }

    fn active_at(&self, at: usize, epochs: &Range<usize>) -> bool {
        self.slots.values().any(|slot| {
            slot.versions
                .iter()
                .any(|version| self.span(epochs, &version.epochs).contains(&at))
        })
    }

    fn join<K>(
        &self,
        branch: &Branch<'_>,
        relation: &impl Relation<K>,
    ) -> Option<(Exposure, MethodSet)> {
        let methods = eligible(&branch.effective_methods, self.mount.method_guard.as_ref())?;
        let mut path = branch.path.clone();
        if !relation.join(self.mount, &mut path, &methods) || !apply_guards(self.mount, &mut path) {
            return None;
        }
        if matches!(self.mount.prefix, MountPrefix::Unevaluated { .. }) {
            suffix(&mut path, Reason::VariablePrefix);
        }
        Some((path, methods))
    }
}

fn reachable<'a>(root: &'a RouterId, children: &Children<'a>) -> BTreeSet<&'a RouterId> {
    let mut reached = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if reached.insert(node) {
            pending.extend(children.get(node).into_iter().flatten().copied());
        }
    }
    reached
}
