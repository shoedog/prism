//! Exact observations shared by unit tests and the temporary feature umbrella.
use super::oracle::LegacyDataFlowGraph;
use super::{DataFlowGraph, ExactVarLocation};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use serde::Serialize;
use std::collections::BTreeMap;
pub(crate) fn bytes<T: Serialize>(value: &T) -> Vec<u8> {
    bincode::serialize(value).expect("observation serializes")
}
pub(crate) fn assert_bytes<T: Serialize>(name: &str, old: &T, new: &T) {
    assert_eq!(bytes(old), bytes(new), "{name}");
}
use crate::data_flow::{FlowPath, VarLocation};
use crate::finding_confidence::{EvidenceHop, EvidencePath};
use crate::frameworks::SanitizerCategory;
use std::collections::BTreeSet;
fn hop_obs(h: &EvidenceHop) -> Vec<u8> {
    match h {
        EvidenceHop::DataFlow {
            from,
            to,
            confidence,
        } => bytes(&(0u8, from, to, confidence)),
        EvidenceHop::Call { edge, confidence } => bytes(&(
            1u8,
            &edge.caller,
            edge.call_site_line,
            edge.confidence,
            edge.kind,
            confidence,
        )),
    }
}
fn category_obs(c: &SanitizerCategory) -> u8 {
    match c {
        SanitizerCategory::Xss => 0,
        SanitizerCategory::Sqli => 1,
        SanitizerCategory::Ssrf => 2,
        SanitizerCategory::Deserialization => 3,
        SanitizerCategory::OsCommand => 4,
        SanitizerCategory::PathTraversal => 5,
    }
}
pub(crate) fn path_bytes(paths: &[FlowPath]) -> Vec<u8> {
    bytes(
        &paths
            .iter()
            .map(|p| {
                (
                    &p.edges,
                    p.cleansed_for.iter().map(category_obs).collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>(),
    )
}
pub(crate) fn labeled_obs(value: &(BTreeSet<VarLocation>, Vec<EvidenceHop>)) -> Vec<u8> {
    bytes(&(&value.0, value.1.iter().map(hop_obs).collect::<Vec<_>>()))
}
pub(crate) fn evidence_obs(paths: &[Option<EvidencePath>]) -> Vec<u8> {
    bytes(
        &paths
            .iter()
            .map(|p| {
                p.as_ref().map(|p| {
                    (
                        p.hops.iter().map(hop_obs).collect::<Vec<_>>(),
                        p.crossed_unlabeled,
                    )
                })
            })
            .collect::<Vec<_>>(),
    )
}
#[cfg(feature = "dfg-oracle")]
#[doc(hidden)]
pub fn observe_evidence(paths: &[Option<EvidencePath>]) -> Vec<u8> {
    evidence_obs(paths)
}
fn old_storage(g: &LegacyDataFlowGraph) -> Vec<u8> {
    bytes(&(
        &g.edges,
        g.defs.iter().collect::<Vec<_>>(),
        g.uses.iter().collect::<Vec<_>>(),
        g.forward.iter().collect::<Vec<_>>(),
        g.backward.iter().collect::<Vec<_>>(),
        g.labels.iter().collect::<Vec<_>>(),
        g.exact_labels.iter().collect::<Vec<_>>(),
        &g.rd_function_stats,
    ))
}
fn current_storage(g: &DataFlowGraph) -> Vec<u8> {
    bytes(&(
        &g.edges,
        g.defs.iter().collect::<Vec<_>>(),
        g.uses.iter().collect::<Vec<_>>(),
        g.forward.iter().collect::<Vec<_>>(),
        g.backward.iter().collect::<Vec<_>>(),
        g.labels.iter().collect::<Vec<_>>(),
        g.exact_labels.iter().collect::<Vec<_>>(),
        &g.rd_function_stats,
    ))
}
macro_rules! compare_queries {
    ($old:expr, $new:expr, $locations:expr, $lines:expr) => {{
        let (old, new) = ($old, $new);
        for loc in $locations {
            assert_bytes(
                "forward",
                &old.forward_reachable(loc),
                &new.forward_reachable(loc),
            );
            assert_bytes(
                "backward",
                &old.backward_reachable(loc),
                &new.backward_reachable(loc),
            );
            assert_eq!(
                labeled_obs(&old.backward_reachable_labeled(loc)),
                labeled_obs(&new.backward_reachable_labeled(loc))
            );
            assert_bytes(
                "defs",
                &old.all_defs_of(&loc.file, loc.var_name()),
                &new.all_defs_of(&loc.file, loc.var_name()),
            );
        }
        for (file, line) in $lines {
            let sources = vec![(file.clone(), *line), (file.clone(), *line)];
            assert_eq!(
                path_bytes(&old.taint_forward(&sources)),
                path_bytes(&new.taint_forward(&sources))
            );
            assert_bytes(
                "locations at",
                &old.all_locations_at(file, *line),
                &new.all_locations_at(file, *line),
            );
        }
        for ((file, line), (sink_file, sink_line)) in chop_pairs(old, $lines) {
            assert_bytes(
                "chop",
                &old.chop(&file, line, &sink_file, sink_line),
                &new.chop(&file, line, &sink_file, sink_line),
            );
        }
        assert_eq!(
            path_bytes(&old.taint_forward($lines)),
            path_bytes(&new.taint_forward($lines))
        );
        assert_eq!(
            path_bytes(&old.taint_forward(&[])),
            path_bytes(&new.taint_forward(&[]))
        );
    }};
}
fn domain(g: &LegacyDataFlowGraph) -> (Vec<VarLocation>, Vec<(String, usize)>) {
    let mut exact = BTreeSet::new();
    let mut add = |v: &VarLocation| {
        exact.insert(ExactVarLocation::from_legacy(v));
    };
    for e in &g.edges {
        add(&e.from);
        add(&e.to);
    }
    for vs in g.defs.values().chain(g.uses.values()) {
        for v in vs {
            add(v);
        }
    }
    for (k, vs) in g.forward.iter().chain(&g.backward) {
        add(k);
        for v in vs {
            add(v);
        }
    }
    for (f, t) in g.labels.keys() {
        add(f);
        add(t);
    }
    for e in g.exact_labels.keys() {
        add(&e.from.as_legacy());
        add(&e.to.as_legacy());
    }
    let original: Vec<_> = exact.into_iter().map(|v| v.as_legacy()).collect();
    let mut locations = original.clone();
    for v in &original {
        let mut shifted = v.clone();
        shifted.start_byte += 10000;
        shifted.end_byte += 10000;
        locations.push(shifted);
        let mut absent = v.clone();
        absent.file = "absent.py".into();
        locations.push(absent);
        let mut path = v.clone();
        path.path = AccessPath::simple("__absent_compaction_path");
        locations.push(path);
        let mut function = v.clone();
        function.function_start_line += 10000;
        locations.push(function);
        let mut zero = v.clone();
        zero.line = 0;
        locations.push(zero);
    }
    let mut lines: BTreeSet<_> = original.iter().map(|v| (v.file.clone(), v.line)).collect();
    let mut max: BTreeMap<String, usize> = BTreeMap::new();
    for v in &original {
        max.entry(v.file.clone())
            .and_modify(|n| *n = (*n).max(v.line))
            .or_insert(v.line);
    }
    for (file, line) in max {
        for n in 0..=line + 1 {
            lines.insert((file.clone(), n));
        }
    }
    lines.insert(("absent.py".into(), 0));
    (locations, lines.into_iter().collect())
}
fn chop_pairs(
    old: &LegacyDataFlowGraph,
    lines: &[(String, usize)],
) -> Vec<((String, usize), (String, usize))> {
    let selected: BTreeSet<_> = if lines.len() <= 60 {
        lines.iter().cloned().collect()
    } else {
        let endpoints: BTreeSet<_> = old
            .edges
            .iter()
            .flat_map(|e| [(&e.from.file, e.from.line), (&e.to.file, e.to.line)])
            .map(|(f, l)| (f.clone(), l))
            .collect();
        let mut selected: BTreeSet<_> = endpoints.into_iter().take(32).collect();
        // lines includes each file's max+1 sentinel; absent.py is explicit.
        let mut last = BTreeMap::new();
        for (f, l) in lines {
            last.entry(f.clone())
                .and_modify(|n: &mut usize| *n = (*n).max(*l))
                .or_insert(*l);
        }
        selected.extend(last);
        selected.insert(("absent.py".into(), 0));
        selected
    };
    selected
        .iter()
        .flat_map(|a| selected.iter().map(move |b| (a.clone(), b.clone())))
        .collect()
}

pub(crate) fn assert_pair(
    old: &LegacyDataFlowGraph,
    new: &DataFlowGraph,
    files: &BTreeMap<String, ParsedFile>,
) {
    assert_eq!(old_storage(old), current_storage(new), "ordered storage");
    let (locations, mut lines) = domain(old);
    let mut all_lines: BTreeSet<_> = lines.drain(..).collect();
    for (file, parsed) in files {
        for line in 0..=parsed.source.lines().count() + 1 {
            all_lines.insert((file.clone(), line));
        }
    }
    lines.extend(all_lines);
    compare_queries!(old, new, &locations, &lines);
}
/// Compare independent storage and CPG assembly for the whole multi-file corpus.
#[cfg(feature = "dfg-oracle")]
pub fn assert_storage_assembly(files: &BTreeMap<String, ParsedFile>) {
    let old = LegacyDataFlowGraph::build(files);
    let new = DataFlowGraph::build(files);
    assert_eq!(old_storage(&old), current_storage(&new));
}
/// Run the complete finite-domain oracle on one fixture or generator.
#[cfg(feature = "dfg-oracle")]
pub fn assert_corpus(files: &BTreeMap<String, ParsedFile>) {
    let old = LegacyDataFlowGraph::build(files);
    let new = DataFlowGraph::build(files);
    assert_pair(&old, &new, files);
    assert_pair(&old.clone(), &new.clone(), files);
    assert_pair(
        &LegacyDataFlowGraph::empty(),
        &DataFlowGraph::empty(),
        &BTreeMap::new(),
    );
    let all: BTreeSet<_> = files.keys().cloned().collect();
    let mut subsets = vec![BTreeSet::new(), all, BTreeSet::from(["absent.py".into()])];
    subsets.extend(files.keys().map(|f| BTreeSet::from([f.clone()])));
    for subset in subsets {
        assert_pair(
            &LegacyDataFlowGraph::build_subset(files, &subset),
            &DataFlowGraph::build_subset(files, &subset),
            files,
        );
    }
}
