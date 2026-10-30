//! Consuming exact interning, canonical freeze and reference remap.
use super::*;
#[derive(Default)]
pub(super) struct Intern {
    rows: BTreeMap<ExactVarLocation, OccId>,
}
fn own_exact(v: VarLocation) -> ExactVarLocation {
    ExactVarLocation {
        file: v.file,
        function: v.function,
        function_start_line: v.function_start_line,
        line: v.line,
        path: v.path,
        start_byte: v.start_byte,
        end_byte: v.end_byte,
        kind: v.kind,
    }
}
fn own_legacy(v: ExactVarLocation) -> VarLocation {
    VarLocation {
        file: v.file,
        function: v.function,
        function_start_line: v.function_start_line,
        line: v.line,
        path: v.path,
        start_byte: v.start_byte,
        end_byte: v.end_byte,
        kind: v.kind,
    }
}
impl Intern {
    pub(super) fn put(&mut self, v: VarLocation) -> Result<OccId, &'static str> {
        let key = own_exact(v);
        if let Some(&id) = self.rows.get(&key) {
            return Ok(id);
        }
        let next = self
            .rows
            .len()
            .checked_add(1)
            .ok_or("DFG arena capacity overflow")?;
        checked_id(next)?;
        let id = OccId(checked_id(self.rows.len())?);
        self.rows.insert(key, id);
        Ok(id)
    }
    pub(super) fn finish(self) -> Result<(Arena, Vec<OccId>), &'static str> {
        let mut remap = vec![OccId(0); self.rows.len()];
        let mut rows = Vec::with_capacity(self.rows.len());
        for (key, old) in self.rows {
            remap[old.0 as usize] = OccId(checked_id(rows.len())?);
            rows.push(own_legacy(key));
        }
        Ok((Arena::from_sorted(rows)?, remap))
    }
}
fn intern_bindings(
    m: BTreeMap<BindingKey, Vec<VarLocation>>,
    i: &mut Intern,
) -> Result<BTreeMap<BindingKey, Vec<OccId>>, &'static str> {
    m.into_iter()
        .map(|(k, vs)| {
            Ok((
                k,
                vs.into_iter().map(|v| i.put(v)).collect::<Result<_, _>>()?,
            ))
        })
        .collect()
}
fn intern_adj(
    m: BTreeMap<VarLocation, Vec<VarLocation>>,
    i: &mut Intern,
) -> Result<Vec<Adj>, &'static str> {
    m.into_iter()
        .map(|(k, vs)| {
            Ok(Adj {
                representative: i.put(k)?,
                neighbors: vs.into_iter().map(|v| i.put(v)).collect::<Result<_, _>>()?,
            })
        })
        .collect()
}
pub(super) fn remap_bindings(m: &mut BTreeMap<BindingKey, Vec<OccId>>, r: &[OccId]) {
    for vs in m.values_mut() {
        for id in vs {
            *id = r[id.0 as usize];
        }
    }
}
fn remap_adj(rows: Vec<Adj>, a: &Arena, r: &[OccId]) -> BTreeMap<LegacyId, Adj> {
    rows.into_iter()
        .map(|mut row| {
            row.representative = r[row.representative.0 as usize];
            for id in &mut row.neighbors {
                *id = r[id.0 as usize];
            }
            (a.group(row.representative), row)
        })
        .collect()
}
impl CompactDfg {
    pub(in crate::data_flow) fn from_parts(p: Parts) -> Result<Self, &'static str> {
        #[cfg(test)]
        FROM_PARTS_CALLS.with(|n| n.set(n.get() + 1));
        let mut i = Intern::default();
        let mut edges: Vec<IdEdge> = p
            .edges
            .into_iter()
            .map(|e| Ok((i.put(e.from)?, i.put(e.to)?)))
            .collect::<Result<_, &'static str>>()?;
        let mut defs = intern_bindings(p.defs, &mut i)?;
        let mut uses = intern_bindings(p.uses, &mut i)?;
        let labels: Vec<Label> = p
            .labels
            .into_iter()
            .map(|((from, to), confidence)| {
                Ok(Label {
                    from: i.put(from)?,
                    to: i.put(to)?,
                    confidence,
                })
            })
            .collect::<Result<_, &'static str>>()?;
        let exact: Vec<Label> = p
            .exact_labels
            .into_iter()
            .map(|(e, confidence)| {
                Ok(Label {
                    from: i.put(own_legacy(e.from))?,
                    to: i.put(own_legacy(e.to))?,
                    confidence,
                })
            })
            .collect::<Result<_, &'static str>>()?;
        let adjacency = match p.adjacency {
            Some((f, b)) => Some((intern_adj(f, &mut i)?, intern_adj(b, &mut i)?)),
            None => None,
        };
        let (arena, r) = i.finish()?;
        for (from, to) in &mut edges {
            *from = r[from.0 as usize];
            *to = r[to.0 as usize];
        }
        remap_bindings(&mut defs, &r);
        remap_bindings(&mut uses, &r);
        let (forward, backward) = match adjacency {
            Some((f, b)) => (remap_adj(f, &arena, &r), remap_adj(b, &arena, &r)),
            None => {
                let (mut f, mut b) = (BTreeMap::new(), BTreeMap::new());
                for &(from, to) in &edges {
                    add_adj(&mut f, &arena, from, to);
                    add_adj(&mut b, &arena, to, from);
                }
                (f, b)
            }
        };
        let mut reduced = BTreeMap::new();
        for l in labels {
            add_label(
                &mut reduced,
                &arena,
                r[l.from.0 as usize],
                r[l.to.0 as usize],
                l.confidence,
            );
        }
        let mut exact_labels = BTreeMap::new();
        for l in exact {
            exact_labels
                .entry((r[l.from.0 as usize], r[l.to.0 as usize]))
                .and_modify(|v: &mut FlowConfidence| *v = v.worst(l.confidence))
                .or_insert(l.confidence);
        }
        Ok(Self {
            arena,
            edges,
            defs,
            uses,
            forward,
            backward,
            labels: reduced,
            exact_labels,
            rd_function_stats: p.rd_function_stats,
        })
    }
}

#[cfg(test)]
thread_local! { pub(in crate::data_flow) static FROM_PARTS_CALLS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) }; }
