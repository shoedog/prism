use super::*;
use events::{Replay, Version};
use petgraph::{algo::kosaraju_scc, graph::DiGraph};
use std::ops::Range;

const MAX_EXPOSURES: usize = 64;

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
        parents: BTreeMap::new(),
        live_cycles: BTreeSet::new(),
        failed: replay
            .failures
            .iter()
            .map(|failure| &failure.root)
            .collect(),
    };
    let mut origins = vec![Vec::new(); routes.len()];
    for (index, event) in events.iter().enumerate() {
        match &event.target {
            Target::Route(origin, path) => {
                origins[*origin].extend(versions[index].iter().map(|version| Seed {
                    dispatcher: &event.dispatcher,
                    site: &event.site,
                    path,
                    version,
                }));
            }
            Target::Mount(record, mode) if !versions[index].is_empty() => {
                let mount = &mounts[*record];
                if let Receiver::Router {
                    identity: child, ..
                } = &mount.child.receiver
                {
                    if matches!(mode, MountEpoch::Snapshot) {
                        graph.epochs.insert(index);
                    }
                    graph.parents.entry(child).or_default().push(Edge {
                        mount,
                        parent: &event.dispatcher,
                        mode: *mode,
                        capture: index,
                        versions: &versions[index],
                    });
                }
            }
            _ => {}
        }
    }
    for edges in graph.parents.values_mut() {
        edges.sort_by_key(|edge| {
            (
                matches!(edge.mount.prefix, MountPrefix::Unevaluated { .. }),
                &edge.mount.site,
                edge.mount.alternative_ordinal,
                edge.mount.child_ordinal,
                edge.capture,
            )
        });
    }
    graph.index(roots);
    for (route, seeds) in routes.iter_mut().zip(&mut origins) {
        seeds.sort_by_key(|seed| {
            (
                !matches!(seed.path.status, ExposureStatus::Full { .. }),
                seed.site,
            )
        });
        route.paths = graph.exposures(seeds, relation);
        if route.reason.is_none()
            && route.paths.is_empty()
            && !route.superseded_keys.is_empty()
            && !seeds
                .iter()
                .any(|seed| seed.version.epochs.contains(&graph.final_epoch))
        {
            route.reason = Some(Reason::Superseded);
        }
    }
}

struct Graph<'a> {
    final_epoch: usize,
    epochs: BTreeSet<usize>,
    descendants: BTreeMap<&'a RouterId, BTreeSet<&'a RouterId>>,
    parents: BTreeMap<&'a RouterId, Vec<Edge<'a>>>,
    live_cycles: BTreeSet<&'a RouterId>,
    failed: BTreeSet<&'a RouterId>,
}

struct Edge<'a> {
    mount: &'a Mount,
    parent: &'a RouterId,
    mode: MountEpoch,
    capture: usize,
    versions: &'a [Version],
}

#[derive(Clone, Copy)]
struct Seed<'a> {
    dispatcher: &'a RouterId,
    site: &'a Site,
    path: &'a Exposure,
    version: &'a Version,
}

#[derive(Clone, PartialEq, Eq)]
struct Branch<'a> {
    node: &'a RouterId,
    seen: Vec<&'a RouterId>,
    epochs: Range<usize>,
    path: Exposure,
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
            self.path.sort_key(),
        )
            .cmp(&(
                other.node,
                other.epochs.start,
                other.epochs.end,
                &other.seen,
                other.uncertain,
                other.path.sort_key(),
            ))
    }
}

impl PartialOrd for Branch<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Graph<'a> {
    fn index(&mut self, roots: &'a [RouterId]) {
        let mut children: BTreeMap<&RouterId, Vec<&RouterId>> = BTreeMap::new();
        let mut live = DiGraph::new();
        let mut nodes = BTreeMap::new();
        for (&child, parents) in &self.parents {
            for edge in parents {
                children.entry(edge.parent).or_default().push(child);
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
            let mut descendants = BTreeSet::new();
            let mut pending = vec![root];
            while let Some(node) = pending.pop() {
                if descendants.insert(node) {
                    pending.extend(children.get(node).into_iter().flatten().copied());
                }
            }
            self.descendants.insert(root, descendants);
        }
    }

    fn exposures<K>(&self, seeds: &[Seed<'a>], relation: &impl Relation<K>) -> Vec<Exposure> {
        let mut paths = Vec::new();
        for exact in [true, false] {
            for root in self
                .descendants
                .keys()
                .copied()
                .map(Some)
                .chain((!exact).then_some(None))
            {
                for seed in seeds {
                    let Some(epochs) = self.active_epochs(seed.version.epochs.clone()) else {
                        continue;
                    };
                    let mut path = seed.path.clone();
                    seed.version.apply(&mut path);
                    let first = Branch {
                        node: seed.dispatcher,
                        seen: vec![],
                        epochs,
                        path,
                        uncertain: seed.version.uncertain,
                    };
                    let overflow = self.walk(first, root, exact, relation, |mut path| {
                        if path
                            .root
                            .as_ref()
                            .is_some_and(|root| self.failed.contains(root))
                        {
                            path.binding_blocked = Some(MethodSet::ANY);
                        }
                        tokenize(&mut path, relation);
                        if paths.contains(&path) {
                            return false;
                        }
                        if paths.len() == MAX_EXPOSURES {
                            path.status = ExposureStatus::Suffix {
                                value: seed.path.status.value().unwrap_or_default().into(),
                                reason: Reason::ExpansionTruncated,
                            };
                            tokenize(&mut path, relation);
                            paths.push(path);
                            return true;
                        }
                        paths.push(path);
                        false
                    });
                    if overflow {
                        return paths;
                    }
                }
            }
        }
        paths
    }

    fn walk<K>(
        &self,
        first: Branch<'a>,
        root: Option<&RouterId>,
        exact: bool,
        relation: &impl Relation<K>,
        mut emit: impl FnMut(Exposure) -> bool,
    ) -> bool {
        let reachable = root.map(|root| &self.descendants[root]);
        let reaches_root = |node: &RouterId| reachable.is_none_or(|nodes| nodes.contains(node));
        if !reaches_root(first.node) {
            return false;
        }
        let mut pending = vec![first];
        let mut visited = BTreeSet::new();
        // Bound memoization by graph size even when many distinct paths die
        // at an impossible join before producing any exposure.
        let cache_limit = self.parents.len().saturating_mul(MAX_EXPOSURES);
        while let Some(mut branch) = pending.pop() {
            if exact
                && (branch.uncertain || !matches!(branch.path.status, ExposureStatus::Full { .. }))
            {
                continue;
            }
            let repeated = if visited.len() < cache_limit {
                !visited.insert(branch.clone())
            } else {
                visited.contains(&branch)
            };
            if repeated {
                continue;
            }
            let current = branch.epochs.contains(&self.final_epoch);
            if current {
                if root == Some(branch.node) {
                    if branch.uncertain {
                        suffix(&mut branch.path, Reason::DispatchConflict);
                    }
                    if exact == matches!(branch.path.status, ExposureStatus::Full { .. }) {
                        branch.path.root = root.cloned();
                        if emit(branch.path) {
                            return true;
                        }
                    }
                    continue;
                }
                if root.is_none() && self.descendants.contains_key(branch.node) {
                    continue;
                }
            }
            if branch.seen.contains(&branch.node) {
                if root.is_none() && current {
                    cut_path(&mut branch.path, Reason::MountCycle);
                    if emit(branch.path) {
                        return true;
                    }
                }
                continue;
            }
            // Only a live cycle can revisit a node at the same epoch.
            if self.live_cycles.contains(branch.node) {
                branch.seen.push(branch.node);
            }
            let mut next = Vec::new();
            let mut attached_at_listing = false;
            for edge in self.parents.get(branch.node).into_iter().flatten() {
                if !reaches_root(edge.parent) {
                    continue;
                }
                for version in edge.versions {
                    let span = match edge.mode {
                        MountEpoch::Live => {
                            branch.epochs.start.max(version.epochs.start)
                                ..branch.epochs.end.min(version.epochs.end)
                        }
                        MountEpoch::Snapshot if branch.epochs.contains(&edge.capture) => {
                            version.epochs.clone()
                        }
                        MountEpoch::Snapshot => continue,
                    };
                    let Some(epochs) = self.active_epochs(span) else {
                        continue;
                    };
                    attached_at_listing |= epochs.contains(&self.final_epoch);
                    let mut path = branch.path.clone();
                    if !relation.join(edge.mount, &mut path) || !apply_guards(edge.mount, &mut path)
                    {
                        continue;
                    }
                    if matches!(edge.mount.prefix, MountPrefix::Unevaluated { .. }) {
                        suffix(&mut path, Reason::VariablePrefix);
                    }
                    version.apply(&mut path);
                    let candidate = Branch {
                        node: edge.parent,
                        seen: match edge.mode {
                            MountEpoch::Live => branch.seen.clone(),
                            MountEpoch::Snapshot => vec![],
                        },
                        epochs,
                        path,
                        uncertain: branch.uncertain || version.uncertain,
                    };
                    if !next.contains(&candidate) {
                        next.push(candidate);
                    }
                }
            }
            if root.is_none() && current && !attached_at_listing {
                suffix(&mut branch.path, Reason::NoKnownMount);
                if emit(branch.path) {
                    return true;
                }
            }
            pending.extend(next.into_iter().rev());
        }
        false
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

fn suffix(path: &mut Exposure, reason: Reason) {
    if matches!(path.status, ExposureStatus::Full { .. }) {
        cut_path(path, reason);
    }
}

fn cut_path(path: &mut Exposure, reason: Reason) {
    path.status = match path.status.value() {
        Some(value) => ExposureStatus::Suffix {
            value: value.into(),
            reason,
        },
        None => ExposureStatus::Dynamic { reason },
    };
}

fn apply_guards(mount: &Mount, path: &mut Exposure) -> bool {
    if let Some(guard) = &mount.setup_guard {
        path.setup_guard = Some(match path.setup_guard.take() {
            Some(child) if &child != guard => format!("({guard}) && ({child})"),
            _ => guard.clone(),
        });
    }
    match (&path.method_guard, &mount.method_guard) {
        (_, None | Some(MethodSet::Any(_))) => {}
        (MethodSet::Any(_), Some(guard)) => path.method_guard = guard.clone(),
        (MethodSet::Known { known }, Some(MethodSet::Known { known: guard })) => {
            let joined: BTreeSet<_> = known.intersection(guard).cloned().collect();
            if joined.is_empty() && !known.is_empty() {
                return false;
            }
            path.method_guard = MethodSet::Known { known: joined };
        }
        (child, Some(parent)) if child != parent => {
            let text = |methods: &MethodSet| match methods {
                MethodSet::Known { known } => known.iter().cloned().collect::<Vec<_>>().join(", "),
                MethodSet::Unknown { text, .. } => text.clone(),
                MethodSet::Any(_) => unreachable!(),
            };
            path.method_guard =
                MethodSet::unknown(format!("({}) && ({})", text(parent), text(child)));
        }
        _ => {}
    }
    true
}

fn tokenize<K>(path: &mut Exposure, relation: &impl Relation<K>) {
    path.path_tokens = path
        .status
        .value()
        .and_then(|value| relation.path_tokens(value))
        .map(|tokens| {
            let mut joined = Vec::new();
            for token in tokens {
                match (joined.last_mut(), token) {
                    (Some(PathToken::Literal(left)), PathToken::Literal(right)) => {
                        left.push_str(&right)
                    }
                    (_, token) => joined.push(token),
                }
            }
            joined
        });
    path.normalized = path.path_tokens.as_ref().map(|tokens| {
        tokens
            .iter()
            .map(|token| match token {
                PathToken::Literal(text) => text.as_str(),
                PathToken::Parameter => "{}",
            })
            .collect()
    });
}
