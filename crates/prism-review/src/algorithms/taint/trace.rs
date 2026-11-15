//! Opt-in retained taint roots. Legacy slicing does not collect this payload.

use super::source_scope::admission::RunScope;
use super::{TaintConfig, TaintSeed};
use prism::cpg::CpgContext;
use prism::data_flow::{FlowPath, VarLocation};
use prism::diff::DiffInput;
use prism::finding_confidence::EvidencePath;
use prism::slice::SliceResult;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum TaintRootOrigin {
    Explicit,
    Diff,
    Ipc,
    Framework,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TaintReached {
    pub location: VarLocation,
    pub evidence: EvidencePath,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TaintRoot {
    /// Full source descriptor, including target/scope for synthesized roots.
    pub seed: TaintSeed,
    pub origin: TaintRootOrigin,
    pub location: VarLocation,
    pub reached: Vec<TaintReached>,
}

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct TaintTrace {
    /// Roots remain separate by source descriptor, origin and byte occurrence.
    /// This is the pre-cleanser frontier, not a collection of sink findings.
    pub roots: Vec<TaintRoot>,
}

pub fn slice_with_trace(
    ctx: &CpgContext,
    diff: &DiffInput,
    config: &TaintConfig,
) -> anyhow::Result<(SliceResult, TaintTrace)> {
    let mut trace = TaintTrace::default();
    let result = super::run::slice_impl(ctx, diff, config, Some(&mut trace))?;
    Ok((result, trace))
}

type FlowEvidence = BTreeMap<(VarLocation, VarLocation), EvidencePath>;
type Frontiers = BTreeMap<(VarLocation, usize, usize), Vec<TaintReached>>;
type AmbiguousPairs = BTreeSet<(VarLocation, VarLocation)>;

/// Source inputs of `capture`. `ipc` and `framework` arrive already admitted by the
/// run's scope; `scope` re-admits the explicit and diff roots that `capture` rebuilds.
#[derive(Clone, Copy)]
pub(super) struct CaptureSources<'a> {
    pub(super) ipc: &'a BTreeSet<(String, usize)>,
    pub(super) framework: &'a [TaintSeed],
    pub(super) scope: RunScope<'a>,
}

pub(super) fn capture(
    ctx: &CpgContext,
    diff: &DiffInput,
    config: &TaintConfig,
    sources: CaptureSources<'_>,
    paths: &mut Vec<FlowPath>,
    evidence: &FlowEvidence,
) -> TaintTrace {
    let CaptureSources {
        ipc,
        framework,
        scope,
    } = sources;
    let mut trace = TaintTrace::default();
    let mut seeds = BTreeSet::new();
    for (file, line) in &config.sources {
        if !scope.admits_source_file(ctx.files, file) {
            continue;
        }
        seeds.insert((
            TaintSeed::line(file.clone(), *line),
            TaintRootOrigin::Explicit,
        ));
    }
    if config.taint_from_diff {
        for file in &diff.files {
            if ctx.files.contains_key(&file.file_path)
                && scope.admits_source_file(ctx.files, &file.file_path)
            {
                for &line in &file.diff_lines {
                    seeds.insert((
                        TaintSeed::line(file.file_path.clone(), line),
                        TaintRootOrigin::Diff,
                    ));
                }
            }
        }
    }
    for (file, line) in ipc {
        seeds.insert((TaintSeed::line(file.clone(), *line), TaintRootOrigin::Ipc));
    }
    for seed in framework.iter().filter(|seed| seed.target.is_none()) {
        seeds.insert((seed.clone(), TaintRootOrigin::Framework));
    }
    let original_len = paths.len();
    let mut target_ranges = Vec::new();
    for seed in framework.iter().filter(|seed| seed.target.is_some()) {
        // Keep each synthetic seed's scope identity before flattening, while
        // preserving the legacy path construction order.
        let start = paths.len();
        super::seeds::synthesize_target_seed_paths(std::slice::from_ref(seed), ctx, paths, scope);
        target_ranges.push((seed, start..paths.len()));
    }
    // Every frontier, including a single synthetic seed's subset, sees the
    // complete collision population. A subset cannot revive an overwritten label.
    let ambiguous = ambiguous_pairs(paths);
    let frontiers = index_frontiers(&paths[..original_len], evidence, &ambiguous);
    for (seed, origin) in seeds {
        // Include isolated source uses: legacy FlowPath drops empty frontiers.
        let locations = ctx
            .cpg
            .nodes_at(&seed.file, seed.line)
            .into_iter()
            .filter_map(|node| ctx.cpg.to_var_location(node))
            .collect();
        trace.record(&seed, origin, locations, &frontiers);
    }
    for (seed, range) in target_ranges {
        let added = &paths[range];
        let locations = added
            .iter()
            .flat_map(|path| &path.edges)
            .map(|edge| edge.from.clone())
            .collect();
        trace.record(
            seed,
            TaintRootOrigin::Framework,
            locations,
            &index_frontiers(added, evidence, &ambiguous),
        );
    }
    trace.roots.sort_by(|a, b| {
        (&a.seed, a.origin)
            .cmp(&(&b.seed, b.origin))
            .then_with(|| occurrence_cmp(&a.location, &b.location))
    });
    trace
}

impl TaintTrace {
    fn record(
        &mut self,
        seed: &TaintSeed,
        origin: TaintRootOrigin,
        mut locations: Vec<VarLocation>,
        frontiers: &Frontiers,
    ) {
        locations.sort_by(occurrence_cmp);
        locations.dedup_by(|a, b| occurrence_cmp(a, b).is_eq());
        for location in locations {
            let reached = frontiers
                .get(&occurrence_key(&location))
                .cloned()
                .unwrap_or_default();
            self.roots.push(TaintRoot {
                seed: seed.clone(),
                origin,
                location,
                reached,
            });
        }
    }
}

fn ambiguous_pairs(paths: &[FlowPath]) -> AmbiguousPairs {
    let mut first = BTreeMap::new();
    let mut ambiguous = AmbiguousPairs::new();
    for edge in paths.iter().flat_map(|path| &path.edges) {
        let pair = (edge.from.clone(), edge.to.clone());
        let bytes = (
            edge.from.start_byte,
            edge.from.end_byte,
            edge.to.start_byte,
            edge.to.end_byte,
        );
        if first.get(&pair).is_some_and(|prior| *prior != bytes) {
            ambiguous.insert(pair);
        } else {
            first.entry(pair).or_insert(bytes);
        }
    }
    ambiguous
}

fn index_frontiers(
    paths: &[FlowPath],
    evidence: &FlowEvidence,
    ambiguous: &AmbiguousPairs,
) -> Frontiers {
    let mut frontiers = Frontiers::new();
    for edge in paths.iter().flat_map(|path| &path.edges) {
        let pair = (edge.from.clone(), edge.to.clone());
        frontiers
            .entry(occurrence_key(&edge.from))
            .or_default()
            .push(TaintReached {
                location: edge.to.clone(),
                // Keep the labeled traversal's evidence. Hints and synthetic
                // source ranges cannot strengthen it.
                evidence: evidence
                    .get_key_value(&pair)
                    // insert() retains the first legacy-equal key but replaces
                    // its value. Even a matching key cannot prove which pair's
                    // witness survived when different byte pairs coalesced.
                    .filter(|_| !ambiguous.contains(&pair))
                    .filter(|((from, to), _)| {
                        occurrence_cmp(from, &edge.from).is_eq()
                            && occurrence_cmp(to, &edge.to).is_eq()
                    })
                    .map(|(_, path)| path.clone())
                    .unwrap_or_else(|| EvidencePath::unlabeled_for("taint")),
            });
    }
    for reached in frontiers.values_mut() {
        reached.sort_by(|a, b| occurrence_cmp(&a.location, &b.location));
        reached.dedup_by(|a, b| occurrence_cmp(&a.location, &b.location).is_eq());
    }
    frontiers
}

fn occurrence_key(location: &VarLocation) -> (VarLocation, usize, usize) {
    (location.clone(), location.start_byte, location.end_byte)
}

fn occurrence_cmp(a: &VarLocation, b: &VarLocation) -> std::cmp::Ordering {
    // VarLocation's legacy Eq/Ord deliberately excludes byte offsets.
    a.cmp(b)
        .then_with(|| (a.start_byte, a.end_byte).cmp(&(b.start_byte, b.end_byte)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism::access_path::AccessPath;
    use prism::cpg::{FlowConfidence, FlowDoubt};
    use prism::data_flow::{FlowEdge, VarAccessKind};
    use prism::finding_confidence::EvidenceHop;

    #[test]
    fn trace_preserves_occurrence_and_seed_scope_collisions() {
        let first = VarLocation {
            file: "app.js".into(),
            function: "h".into(),
            function_start_line: 1,
            line: 2,
            path: AccessPath::simple("x"),
            start_byte: 10,
            end_byte: 11,
            kind: VarAccessKind::Use,
        };
        let second = VarLocation {
            start_byte: 20,
            end_byte: 21,
            ..first.clone()
        };
        assert_eq!(first, second, "legacy location equality ignores bytes");
        let paths = vec![FlowPath {
            edges: vec![
                FlowEdge {
                    from: first.clone(),
                    to: first.clone(),
                },
                FlowEdge {
                    from: second.clone(),
                    to: second.clone(),
                },
            ],
            cleansed_for: BTreeSet::new(),
        }];
        let frontiers = index_frontiers(&paths, &FlowEvidence::new(), &ambiguous_pairs(&paths));
        let mut trace = TaintTrace::default();
        for span in [(10, 15), (20, 25)] {
            let seed = TaintSeed::target_scoped(
                "app.js".into(),
                2,
                AccessPath::simple("x"),
                Some(span.0),
                Some((1, 3)),
                Some(span),
            );
            trace.record(
                &seed,
                TaintRootOrigin::Framework,
                vec![first.clone(), second.clone()],
                &frontiers,
            );
        }
        assert_eq!(trace.roots.len(), 4);
        assert_ne!(
            trace.roots[0].seed.byte_scope,
            trace.roots[2].seed.byte_scope
        );
        for root in &trace.roots {
            assert_eq!(root.reached.len(), 1);
            assert_eq!(
                root.location.start_byte,
                root.reached[0].location.start_byte
            );
            assert!(root.reached[0].evidence.crossed_unlabeled);
        }
        // Legacy evidence-map equality must not lend a different byte
        // occurrence's label to this occurrence.
        let evidence =
            FlowEvidence::from([((first.clone(), first.clone()), EvidencePath::default())]);
        let first_only = vec![FlowPath {
            edges: vec![paths[0].edges[0].clone()],
            cleansed_for: BTreeSet::new(),
        }];
        let labeled = index_frontiers(&first_only, &evidence, &ambiguous_pairs(&first_only));
        assert!(
            !labeled[&occurrence_key(&first)][0]
                .evidence
                .crossed_unlabeled
        );
        let second_only = vec![FlowPath {
            edges: vec![paths[0].edges[1].clone()],
            cleansed_for: BTreeSet::new(),
        }];
        assert!(
            index_frontiers(&second_only, &evidence, &ambiguous_pairs(&second_only))
                [&occurrence_key(&second)][0]
                .evidence
                .crossed_unlabeled
        );
    }

    fn collision_fixture() -> (Vec<FlowPath>, Vec<EvidencePath>) {
        let first = VarLocation {
            file: "app.js".into(),
            function: "h".into(),
            function_start_line: 1,
            line: 2,
            path: AccessPath::simple("x"),
            start_byte: 10,
            end_byte: 11,
            kind: VarAccessKind::Def,
        };
        let second = VarLocation {
            start_byte: 20,
            end_byte: 21,
            ..first.clone()
        };
        let target = VarLocation {
            line: 3,
            path: AccessPath::simple("y"),
            start_byte: 30,
            end_byte: 31,
            kind: VarAccessKind::Use,
            ..first.clone()
        };
        let mut paths = Vec::new();
        let mut witnesses = Vec::new();
        for (from, confidence) in [
            (first, FlowConfidence::NameOnly(FlowDoubt::SameLine)),
            (second, FlowConfidence::Exact),
        ] {
            paths.push(FlowPath {
                edges: vec![FlowEdge {
                    from: from.clone(),
                    to: target.clone(),
                }],
                cleansed_for: BTreeSet::new(),
            });
            witnesses.push(EvidencePath::from_hops(vec![EvidenceHop::DataFlow {
                from,
                to: target.clone(),
                confidence,
            }]));
        }
        (paths, witnesses)
    }

    fn assert_overwritten_pair_is_unlabeled(order: [usize; 2]) {
        let (paths, witnesses) = collision_fixture();
        let mut evidence = FlowEvidence::new();
        for i in order {
            let edge = &paths[i].edges[0];
            evidence.insert((edge.from.clone(), edge.to.clone()), witnesses[i].clone());
        }
        assert_eq!(evidence.len(), 1);
        let ((retained, _), overwritten) = evidence.first_key_value().unwrap();
        assert_eq!(
            retained.start_byte,
            paths[order[0]].edges[0].from.start_byte
        );
        assert_eq!(
            format!("{overwritten:?}"),
            format!("{:?}", witnesses[order[1]])
        );
        let ambiguous = ambiguous_pairs(&paths);
        let indexed = index_frontiers(&paths, &evidence, &ambiguous);
        for path in &paths {
            let reached = &indexed[&occurrence_key(&path.edges[0].from)][0];
            assert!(
                reached.evidence.crossed_unlabeled,
                "overwritten witness borrowed: {:?}",
                reached.evidence
            );
            assert!(reached.evidence.hops.is_empty());
        }
        let subset = std::slice::from_ref(&paths[order[0]]);
        let indexed = index_frontiers(subset, &evidence, &ambiguous);
        assert!(
            indexed[&occurrence_key(retained)][0]
                .evidence
                .crossed_unlabeled
        );
    }

    #[test]
    fn trace_rejects_overwritten_pair_weak_then_exact() {
        assert_overwritten_pair_is_unlabeled([0, 1]);
    }

    #[test]
    fn trace_rejects_overwritten_pair_exact_then_weak() {
        assert_overwritten_pair_is_unlabeled([1, 0]);
    }

    #[test]
    fn trace_unambiguous_pairs_keep_their_own_witnesses() {
        let (paths, witnesses) = collision_fixture();
        for i in 0..2 {
            let edge = &paths[i].edges[0];
            let evidence =
                FlowEvidence::from([((edge.from.clone(), edge.to.clone()), witnesses[i].clone())]);
            // Repeated identical occurrences are not ambiguity.
            let repeated = vec![paths[i].clone(), paths[i].clone()];
            let indexed = index_frontiers(&repeated, &evidence, &ambiguous_pairs(&repeated));
            let reached = &indexed[&occurrence_key(&edge.from)];
            assert_eq!(reached.len(), 1);
            assert_eq!(
                format!("{:?}", reached[0].evidence),
                format!("{:?}", witnesses[i])
            );
        }
    }
}
