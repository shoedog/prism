//! Repository-wide refusal of Go receiver recall edges (#639 S2 fold 1).
//!
//! A receiver edge `Def(p.f) -> Use(R)` of `R.M(..)` is wrong when `M` is a
//! func-typed field of the package: the call reads the field. A per-file
//! producer cannot know that, so each admitted edge is recorded with its
//! selector in the file's recall ledger and this pass removes (or restores)
//! edges against the assembled repository. The ledger holds the endpoints of
//! a removed edge, so the pass is a pure function of the current facts and
//! the persisted graph can flip an edge back when only a sibling file changes.
use super::update::combine;
use super::*;
use crate::cpg::{ReceiverEdge, ReceiverKey};
use std::collections::{HashMap, HashSet};

type Pending = HashMap<String, BTreeSet<ReceiverKey>>;

impl CompactDfg {
    /// Files whose ledger holds at least one receiver edge.
    pub(in crate::data_flow) fn receiver_files(&self) -> Vec<String> {
        self.rd_function_stats
            .iter()
            .filter(|(_, s)| !s.recall.receivers().is_empty())
            .map(|(f, _)| f.clone())
            .collect()
    }

    /// Spans of receiver Uses that exist only as endpoints of a present edge.
    pub(in crate::data_flow) fn recall_only_use_spans(
        &self,
    ) -> HashMap<String, HashSet<(usize, usize)>> {
        let mut out: HashMap<String, HashSet<(usize, usize)>> = HashMap::new();
        for (file, stats) in &self.rd_function_stats {
            for (key, edge) in stats.recall.receivers() {
                if edge.new_use && edge.refused.is_none() {
                    out.entry(file.clone())
                        .or_default()
                        .insert((key.to_start, key.to_end));
                }
            }
        }
        out
    }

    pub(in crate::data_flow) fn receivers_refused(&self) -> usize {
        self.rd_function_stats
            .values()
            .map(|s| s.recall.receivers_refused())
            .sum()
    }

    /// Bring every ledger entry in line with `refuse(file, selector)`.
    pub(in crate::data_flow) fn apply_receiver_refusals(
        &mut self,
        refuse: &dyn Fn(&str, &str) -> bool,
    ) {
        let mut to_refuse = Pending::new();
        let mut to_restore = Vec::new();
        for (file, stats) in &self.rd_function_stats {
            for (key, edge) in stats.recall.receivers() {
                match (refuse(file, &edge.selector), edge.refused.is_some()) {
                    (true, false) => {
                        to_refuse
                            .entry(file.clone())
                            .or_default()
                            .insert(key.clone());
                    }
                    (false, true) => to_restore.push((file.clone(), key.clone())),
                    _ => {}
                }
            }
        }
        if to_refuse.is_empty() && to_restore.is_empty() {
            return;
        }
        let mut old = std::mem::replace(self, Self::empty());
        old.remove_receiver_edges(to_refuse);
        let add = old.restore_receiver_edges(to_restore);
        *self = combine(vec![old]).expect("DFG exceeds u32 location capacity");
        if let Some(add) = add {
            self.merge(add);
        }
    }

    fn remove_receiver_edges(&mut self, pending: Pending) {
        if pending.is_empty() {
            return;
        }
        let spans: HashMap<&str, HashSet<(usize, usize)>> = pending
            .iter()
            .map(|(f, keys)| {
                (
                    f.as_str(),
                    keys.iter().map(|k| (k.to_start, k.to_end)).collect(),
                )
            })
            .collect();
        let mut removed = vec![false; self.edges.len()];
        let mut found: Vec<(String, ReceiverKey, VarLocation, VarLocation, OccId)> = Vec::new();
        for (i, &(f, t)) in self.edges.iter().enumerate() {
            let to = self.arena.get(t);
            let Some(wanted) = spans.get(to.file.as_str()) else {
                continue;
            };
            if !wanted.contains(&(to.start_byte, to.end_byte)) {
                continue;
            }
            let from = self.arena.get(f);
            let key = ReceiverKey {
                from_start: from.start_byte,
                from_end: from.end_byte,
                from_path: from.path.clone(),
                to_start: to.start_byte,
                to_end: to.end_byte,
            };
            if pending[&to.file].contains(&key) {
                removed[i] = true;
                found.push((to.file.clone(), key, from.clone(), to.clone(), t));
            }
        }
        let mut groups = BTreeSet::new();
        let mut maybe_orphans = BTreeSet::new();
        for (i, &(f, t)) in self.edges.iter().enumerate() {
            if removed[i] {
                groups.insert((self.arena.group(f), self.arena.group(t)));
                maybe_orphans.insert(t);
            }
        }
        let mut flags = removed.iter();
        self.edges.retain(|_| !*flags.next().unwrap());
        // A label survives while another edge shares its legacy pair; a Use
        // survives while any edge still touches it.
        for &(f, t) in &self.edges {
            groups.remove(&(self.arena.group(f), self.arena.group(t)));
            maybe_orphans.remove(&f);
            maybe_orphans.remove(&t);
        }
        for g in groups {
            self.labels.remove(&g);
        }
        for (file, key, from, to, to_occ) in found {
            let stats = self.rd_function_stats.get_mut(&file).expect("ledger file");
            let entry = stats.recall.receivers_mut().get_mut(&key).expect("ledger");
            if entry.new_use && maybe_orphans.contains(&to_occ) {
                let use_key = (
                    to.file.clone(),
                    to.function.clone(),
                    to.function_start_line,
                    to.path.clone(),
                );
                if let Some(ids) = self.uses.get_mut(&use_key) {
                    let arena = &self.arena;
                    ids.retain(|&id| {
                        let u = arena.get(id);
                        (u.start_byte, u.end_byte) != (to.start_byte, to.end_byte)
                    });
                    if ids.is_empty() {
                        self.uses.remove(&use_key);
                    }
                }
            }
            entry.refused = Some((from, to));
        }
        self.forward.clear();
        self.backward.clear();
    }

    fn restore_receiver_edges(&mut self, restore: Vec<(String, ReceiverKey)>) -> Option<Self> {
        if restore.is_empty() {
            return None;
        }
        let mut parts = Parts {
            edges: Vec::new(),
            defs: BTreeMap::new(),
            uses: BTreeMap::new(),
            labels: BTreeMap::new(),
            exact_labels: BTreeMap::new(),
            rd_function_stats: BTreeMap::new(),
            adjacency: None,
        };
        for (file, key) in restore {
            let stats = self.rd_function_stats.get_mut(&file).expect("ledger file");
            let entry: &mut ReceiverEdge =
                stats.recall.receivers_mut().get_mut(&key).expect("ledger");
            let Some((from, to)) = entry.refused.take() else {
                continue;
            };
            if entry.new_use {
                let use_key = (
                    to.file.clone(),
                    to.function.clone(),
                    to.function_start_line,
                    to.path.clone(),
                );
                let present = self.uses.get(&use_key).is_some_and(|ids| {
                    ids.iter().any(|&id| {
                        let u = self.arena.get(id);
                        (u.start_byte, u.end_byte) == (to.start_byte, to.end_byte)
                    })
                });
                let queued = parts.uses.entry(use_key).or_default();
                if !present && !queued.iter().any(|u| u.start_byte == to.start_byte) {
                    queued.push(to.clone());
                }
            }
            parts
                .labels
                .entry((from.clone(), to.clone()))
                .and_modify(|c| *c = c.worst(entry.label))
                .or_insert(entry.label);
            parts.edges.push(FlowEdge { from, to });
        }
        parts.uses.retain(|_, v| !v.is_empty());
        Some(CompactDfg::from_parts(parts).expect("DFG exceeds u32 location capacity"))
    }
}
