//! Dual-backend observation and independently consumed producer inputs.
use super::*;
use crate::data_flow::compact::{CompactDfg, Parts};

pub(in crate::data_flow) fn parts(old: LegacyDataFlowGraph) -> Parts {
    Parts {
        edges: old.edges,
        defs: old.defs,
        uses: old.uses,
        labels: old.labels,
        exact_labels: old.exact_labels,
        rd_function_stats: old.rd_function_stats,
        adjacency: Some((old.forward, old.backward)),
    }
}

pub(in crate::data_flow) fn storage(g: &CompactDfg) -> Vec<u8> {
    bytes(&(
        g.edges().iter().map(|e| e.to_owned()).collect::<Vec<_>>(),
        g.defs().map(|(k, v)| (k, v.to_vec())).collect::<Vec<_>>(),
        g.uses().map(|(k, v)| (k, v.to_vec())).collect::<Vec<_>>(),
        g.forward()
            .map(|(k, v)| (k, v.to_vec()))
            .collect::<Vec<_>>(),
        g.backward()
            .map(|(k, v)| (k, v.to_vec()))
            .collect::<Vec<_>>(),
        g.labels()
            .map(|(e, c)| ((e.from, e.to), c))
            .collect::<Vec<_>>(),
        g.exact_labels().collect::<Vec<_>>(),
        g.rd_function_stats(),
    ))
}

pub(in crate::data_flow) fn assert_all(
    old: &LegacyDataFlowGraph,
    new: &CompactDfg,
    files: &BTreeMap<String, ParsedFile>,
) {
    assert_eq!(old_storage(old), storage(new), "compact ordered storage");
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

// Test-only construction path consumes a fresh shared producer invocation,
// without building or consulting the legacy graph being compared.
pub(in crate::data_flow) fn build(files: &BTreeMap<String, ParsedFile>) -> CompactDfg {
    use rayon::prelude::*;
    let ordered: Vec<_> = files.iter().collect();
    let compact_parts = ordered
        .par_iter()
        .map(|(file, parsed)| {
            let r = crate::data_flow::build::file_refs(file, parsed);
            CompactDfg::from_parts(Parts {
                edges: r.edges,
                defs: r.defs,
                uses: r.uses,
                labels: r.labels,
                exact_labels: r.exact_labels,
                rd_function_stats: BTreeMap::from([(r.file_path, r.rd_function_stats)]),
                adjacency: None,
            })
            .expect("fixture capacity")
        })
        .collect();
    crate::data_flow::compact::combine(compact_parts).expect("fixture capacity")
}

pub(in crate::data_flow) fn assert_conversion(
    old: &LegacyDataFlowGraph,
    files: &BTreeMap<String, ParsedFile>,
) {
    let new = CompactDfg::from_parts(parts(old.clone())).unwrap();
    assert_all(old, &new, files);
    assert_all(old, &new.clone(), files);
}

pub(in crate::data_flow) fn assert_built(files: &BTreeMap<String, ParsedFile>) {
    let old = LegacyDataFlowGraph::build(files);
    let new = build(files);
    assert_all(&old, &new, files);
    let encoded = bincode::serialize(&new).unwrap();
    let decoded: CompactDfg = bincode::deserialize(&encoded).unwrap();
    assert_all(&old, &decoded, files);
    assert_eq!(encoded, bincode::serialize(&decoded).unwrap());
    assert_conversion(&old, files);
    let production = DataFlowGraph::build(files);
    super::assert_pair(&old, &production, files);
    let encoded = bincode::serialize(&production).unwrap();
    let mut hit: DataFlowGraph = bincode::deserialize(&encoded).unwrap();
    super::assert_pair(&old, &hit, files);
    let mut updated = old.clone();
    let excluded = files.keys().cloned().collect();
    updated.remove_files(&excluded);
    hit.remove_files(&excluded);
    super::assert_pair(&updated, &hit, files);
    updated.merge(LegacyDataFlowGraph::build(files));
    hit.merge(DataFlowGraph::build(files));
    super::assert_pair(&updated, &hit, files);
}

#[cfg(test)]
pub(in crate::data_flow) fn pair(p: Parts) -> (LegacyDataFlowGraph, CompactDfg) {
    let q = p.clone();
    let (forward, backward) = q.adjacency.expect("explicit fixture adjacency");
    let old = LegacyDataFlowGraph {
        edges: q.edges,
        defs: q.defs,
        uses: q.uses,
        forward,
        backward,
        labels: q.labels,
        exact_labels: q.exact_labels,
        rd_function_stats: q.rd_function_stats,
    };
    (old, CompactDfg::from_parts(p).unwrap())
}

#[cfg(feature = "dfg-oracle")]
pub(super) fn assert_updates(files: &BTreeMap<String, ParsedFile>) {
    let baseline = LegacyDataFlowGraph::build(files);
    let compact = build(files);
    let held = compact.clone();
    let held_bytes = storage(&held);
    let mut exclusions = vec![
        BTreeSet::new(),
        files.keys().cloned().collect(),
        BTreeSet::from(["absent.py".into()]),
    ];
    exclusions.extend(files.keys().map(|f| BTreeSet::from([f.clone()])));
    for excluded in exclusions {
        let mut old = baseline.clone();
        let mut new = compact.clone();
        old.remove_files(&excluded);
        new.remove_files(&excluded);
        assert_all(&old, &new, files);
        let subset = files
            .iter()
            .filter(|(f, _)| excluded.contains(*f))
            .map(|(f, p)| (f.clone(), p.clone()))
            .collect();
        old.merge(LegacyDataFlowGraph::build(&subset));
        new.merge(build(&subset));
        assert_all(&old, &new, files);
        old.merge(baseline.clone());
        new.merge(compact.clone());
        assert_all(&old, &new, files);
        assert_eq!(storage(&held), held_bytes);
    }
}
