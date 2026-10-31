//! Filter primary IDs, reclaim dead rows and remap ordered parts.
use super::freeze::{remap_bindings, OrderedIntern};
use super::*;
fn filter_primary(g: &mut CompactDfg, excluded: &BTreeSet<String>) {
    let a = &g.arena;
    g.edges
        .retain(|&(f, t)| !excluded.contains(&a.get(f).file) && !excluded.contains(&a.get(t).file));
    g.defs.retain(|k, _| !excluded.contains(&k.0));
    g.uses.retain(|k, _| !excluded.contains(&k.0));
    g.labels.retain(|_, v| {
        !excluded.contains(&a.get(v.from).file) && !excluded.contains(&a.get(v.to).file)
    });
    g.exact_labels.retain(|&(f, t), _| {
        !excluded.contains(&a.get(f).file) && !excluded.contains(&a.get(t).file)
    });
    g.rd_function_stats.retain(|f, _| !excluded.contains(f));
    g.forward.clear();
    g.backward.clear();
}
pub(in crate::data_flow) fn combine(parts: Vec<CompactDfg>) -> Result<CompactDfg, &'static str> {
    let mut i = OrderedIntern::default();
    let mut edges = Vec::new();
    let mut defs: BTreeMap<BindingKey, Vec<OccId>> = BTreeMap::new();
    let mut uses: BTreeMap<BindingKey, Vec<OccId>> = BTreeMap::new();
    let mut labels = Vec::new();
    let mut exact = Vec::new();
    let mut stats: BTreeMap<String, crate::cpg::RdFileStats> = BTreeMap::new();
    for p in parts {
        let mut live = vec![false; p.arena.rows().len()];
        let mut mark = |id: OccId| live[id.0 as usize] = true;
        for &(f, t) in &p.edges {
            mark(f);
            mark(t);
        }
        for ids in p.defs.values().chain(p.uses.values()) {
            for &id in ids {
                mark(id);
            }
        }
        for v in p.labels.values() {
            mark(v.from);
            mark(v.to);
        }
        for &(f, t) in p.exact_labels.keys() {
            mark(f);
            mark(t);
        }
        let mut r = vec![None; live.len()];
        for (old, row) in p.arena.into_wire_parts().0.into_iter().enumerate() {
            if live[old] {
                r[old] = Some(i.put(row)?);
            }
        }
        let map = |id: OccId| r[id.0 as usize].expect("marked primary reference");
        edges.extend(p.edges.into_iter().map(|(f, t)| (map(f), map(t))));
        for (k, ids) in p.defs {
            defs.entry(k).or_default().extend(ids.into_iter().map(map));
        }
        for (k, ids) in p.uses {
            uses.entry(k).or_default().extend(ids.into_iter().map(map));
        }
        labels.extend(p.labels.into_values().map(|v| Label {
            from: map(v.from),
            to: map(v.to),
            confidence: v.confidence,
        }));
        exact.extend(
            p.exact_labels
                .into_iter()
                .map(|((f, t), confidence)| Label {
                    from: map(f),
                    to: map(t),
                    confidence,
                }),
        );
        for (file, s) in p.rd_function_stats {
            stats.entry(file).or_default().merge(s);
        }
    }
    let (arena, r) = i.finish()?;
    for (f, t) in &mut edges {
        *f = r[f.0 as usize];
        *t = r[t.0 as usize];
    }
    remap_bindings(&mut defs, &r);
    remap_bindings(&mut uses, &r);
    let (mut forward, mut backward) = (BTreeMap::new(), BTreeMap::new());
    for &(f, t) in &edges {
        add_adj(&mut forward, &arena, f, t);
        add_adj(&mut backward, &arena, t, f);
    }
    let mut reduced = BTreeMap::new();
    for v in labels {
        add_label(
            &mut reduced,
            &arena,
            r[v.from.0 as usize],
            r[v.to.0 as usize],
            v.confidence,
        );
    }
    let mut exact_labels = BTreeMap::new();
    for v in exact {
        exact_labels
            .entry((r[v.from.0 as usize], r[v.to.0 as usize]))
            .and_modify(|c: &mut FlowConfidence| *c = c.worst(v.confidence))
            .or_insert(v.confidence);
    }
    Ok(CompactDfg {
        arena,
        edges,
        defs,
        uses,
        forward,
        backward,
        labels: reduced,
        exact_labels,
        rd_function_stats: stats,
    })
}
impl CompactDfg {
    pub(in crate::data_flow) fn empty() -> Self {
        combine(Vec::new()).expect("empty DFG")
    }
    pub(in crate::data_flow) fn remove_files(&mut self, excluded: &BTreeSet<String>) {
        let mut old = std::mem::replace(self, Self::empty());
        filter_primary(&mut old, excluded);
        *self = combine(vec![old]).expect("DFG exceeds u32 location capacity");
    }
    pub(in crate::data_flow) fn merge(&mut self, other: Self) {
        let old = std::mem::replace(self, Self::empty());
        *self = combine(vec![old, other]).expect("DFG exceeds u32 location capacity");
    }
}
