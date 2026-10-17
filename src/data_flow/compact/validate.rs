//! Validate every reference before any graph can be served.
use super::wire::WireDfg;
use super::*;
fn ordered<K: Ord, V>(rows: &[(K, V)]) -> bool {
    rows.windows(2).all(|w| w[0].0 < w[1].0)
}
impl TryFrom<WireDfg> for CompactDfg {
    type Error = &'static str;
    fn try_from(mut w: WireDfg) -> Result<Self, Self::Error> {
        let mut arena = Arena::from_wire(w.arena, w.legacy_reps, w.legacy_by_occ)?;
        let valid = |id: OccId| (id.0 as usize) < arena.rows().len();
        if !ordered(&w.defs)
            || !ordered(&w.uses)
            || !ordered(&w.forward)
            || !ordered(&w.backward)
            || !ordered(&w.labels)
            || !ordered(&w.exact_labels)
        {
            return Err("DFG map order/duplicate");
        }
        if w.edges.iter().any(|&(f, t)| !valid(f) || !valid(t)) {
            return Err("DFG edge ID");
        }
        if w.defs
            .iter()
            .chain(&w.uses)
            .any(|(_, ids)| ids.iter().any(|&id| !valid(id)))
        {
            return Err("DFG binding ID");
        }
        for (key, row) in w.forward.iter().chain(&w.backward) {
            if !valid(row.representative) || row.neighbors.iter().any(|&id| !valid(id)) {
                return Err("DFG adjacency ID");
            }
            if *key != arena.group(row.representative) {
                return Err("DFG adjacency group");
            }
        }
        for (key, row) in &w.labels {
            if !valid(row.from) || !valid(row.to) {
                return Err("DFG label ID");
            }
            if *key != (arena.group(row.from), arena.group(row.to)) {
                return Err("DFG label group");
            }
        }
        if w.exact_labels
            .iter()
            .any(|((f, t), _)| !valid(*f) || !valid(*t))
        {
            return Err("DFG exact label ID");
        }
        if !ordered(&w.rd_function_stats) {
            return Err("DFG RD map order/duplicate");
        }
        if w.rd_function_stats
            .iter()
            .any(|(_, s)| !s.counts_match_keys())
        {
            return Err("DFG RD stats counts");
        }
        // Serde caps initial reservations then grows geometrically. Discard
        // spare capacity only after validation, including retained nested Vecs.
        arena.shrink_to_fit();
        w.edges.shrink_to_fit();
        for (key, ids) in w.defs.iter_mut().chain(&mut w.uses) {
            key.3.fields.shrink_to_fit();
            ids.shrink_to_fit();
        }
        for (_, row) in w.forward.iter_mut().chain(&mut w.backward) {
            row.neighbors.shrink_to_fit();
        }
        Ok(Self {
            arena,
            edges: w.edges,
            defs: w.defs.into_iter().collect(),
            uses: w.uses.into_iter().collect(),
            forward: w.forward.into_iter().collect(),
            backward: w.backward.into_iter().collect(),
            labels: w.labels.into_iter().collect(),
            exact_labels: w.exact_labels.into_iter().collect(),
            rd_function_stats: w.rd_function_stats.into_iter().collect(),
        })
    }
}
