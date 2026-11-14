//! The E2/E3 fixpoint of the value-flow graph (`js_hold_flow.rs`): conditional calls, invoked callables, catch parameters.
use super::js_hold_flow::Flow;
use std::collections::BTreeSet;

/// A catch clause's parameters, `try` body range and, inside a generator, its callable node.
type CatchView = (Vec<usize>, (usize, usize), Option<usize>);

impl<'a, 't> Flow<'a, 't> {
    // ---- fixpoint ----

    pub(super) fn fixpoint(&mut self) {
        loop {
            let mut changed = false;
            let fns: Vec<(usize, usize)> =
                self.callables.iter().map(|c| (c.start, c.fn_id)).collect();
            let comps: BTreeSet<usize> = fns.iter().map(|&(_, f)| self.find(f)).collect();
            let class_comps: Vec<usize> = self
                .ids
                .iter()
                .filter(|(k, _)| k.starts_with("cls@"))
                .map(|(_, &i)| i)
                .collect();
            let class_comps: BTreeSet<usize> =
                class_comps.into_iter().map(|c| self.find(c)).collect();
            let reach = |s: &mut Self, names: &[usize]| {
                names.iter().any(|&n| {
                    let r = s.find(n);
                    comps.contains(&r) || class_comps.contains(&r)
                })
            };
            let pending = std::mem::take(&mut self.conditional);
            for names in pending {
                if reach(self, &names) {
                    self.union_all(&names);
                    if let Some(&f) = names.first() {
                        self.invoked.push(f);
                    }
                    changed = true;
                } else {
                    self.conditional.push(names);
                }
            }
            let invoked: BTreeSet<usize> = self
                .invoked
                .clone()
                .into_iter()
                .map(|i| self.find(i))
                .collect();
            for (start, f) in fns {
                let r = self.find(f);
                if invoked.contains(&r) {
                    let mut all = vec![f];
                    all.extend(self.formals.get(&start).cloned().unwrap_or_default());
                    if let Some(&a) = self.args_of.get(&start) {
                        all.push(a);
                    }
                    changed |= self.union_all(&all);
                }
            }
            let throws: Vec<usize> = self.throws.iter().flatten().copied().collect();
            let catches: Vec<CatchView> = self
                .catches
                .iter()
                .map(|c| (c.params.clone(), c.range, c.generator))
                .collect();
            for (params, range, generator) in catches {
                let mut all = params.clone();
                all.extend(throws.iter().copied());
                all.extend(generator);
                let inside: Vec<Vec<usize>> = self
                    .ops
                    .iter()
                    .filter(|o| range.0 <= o.start && o.start < range.1)
                    .map(|o| o.names.clone())
                    .collect();
                let reaches = inside.iter().any(|n| reach(self, n));
                all.extend(inside.into_iter().flatten());
                if reaches {
                    let nested: Vec<usize> = self
                        .ops
                        .iter()
                        .filter(|o| o.in_callable)
                        .flat_map(|o| o.names.iter().copied())
                        .collect();
                    all.extend(nested);
                }
                if params.is_empty() {
                    continue;
                }
                changed |= self.union_all(&all);
            }
            if !changed {
                break;
            }
        }
    }
}
