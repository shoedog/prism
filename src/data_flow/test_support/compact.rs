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
    let mut p = parts(LegacyDataFlowGraph::empty());
    p.adjacency = None;
    for (file, parsed) in files {
        let r = crate::data_flow::build::file_refs(file, parsed);
        p.edges.extend(r.edges);
        for (k, v) in r.defs {
            p.defs.entry(k).or_default().extend(v);
        }
        for (k, v) in r.uses {
            p.uses.entry(k).or_default().extend(v);
        }
        for (k, c) in r.labels {
            p.labels
                .entry(k)
                .and_modify(|s| *s = s.worst(c))
                .or_insert(c);
        }
        for (k, c) in r.exact_labels {
            p.exact_labels
                .entry(k)
                .and_modify(|s| *s = s.worst(c))
                .or_insert(c);
        }
        p.rd_function_stats.insert(r.file_path, r.rd_function_stats);
    }
    CompactDfg::from_parts(p).expect("fixture capacity")
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
    assert_all(&old, &build(files), files);
    assert_conversion(&old, files);
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
