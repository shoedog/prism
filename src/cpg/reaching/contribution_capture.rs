//! Capture, deferred-receiver and same-line classification of the contribution
//! lane (#639 S2 fold 2). Liveness is computed at an evaluation point inside a
//! collapsed node (`route::Pos`), not from the node's incoming set.
use super::{BTreeMap, BTreeSet, BitSet, FlowConfidence, FlowDoubt, Lane};
use crate::cpg::reaching::go_aggregate::Mode;
use crate::cpg::reaching::route::{order, Order, Pos};

const CFG_INCOMPLETE: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);

impl Lane<'_> {
    /// `k` is a genuine write that overwrites contribution `d`: the same
    /// binding, and a path equal to or a prefix of `d`'s.
    fn overwrites(&self, k: usize, d: usize) -> bool {
        let (w, s) = (&self.defs[k], &self.defs[d]);
        k != d
            && w.path.base == s.path.base
            && s.path.fields.starts_with(&w.path.fields)
            && (self.bindings.proven_same_def_binding(d, k)
                || self.bindings.same_fallback_binding(d, k))
    }

    fn pos(&self, d: usize) -> Option<&Pos> {
        self.def_pos[d]
    }

    /// Writes of node `n` that run before evaluation point `at`.
    fn writes_before(&self, n: usize, at: &Pos) -> Vec<usize> {
        self.node_writes
            .get(&n)
            .into_iter()
            .flatten()
            .copied()
            .filter(|&k| self.pos(k).is_some_and(|p| order(p, at).is_before()))
            .collect()
    }

    /// Contributions live at evaluation point `at` of node `n`: those entering
    /// the node or written earlier in it, minus the ones a write before `at`
    /// overwrote.
    fn live_at(&self, n: usize, at: &Pos) -> BitSet {
        let before = self.writes_before(n, at);
        let mut set = BitSet::new(self.defs.len());
        for d in self.incoming[n].members() {
            if !before.iter().any(|&k| self.overwrites(k, d)) {
                set.insert(d);
            }
        }
        for &d in self.node_sources.get(&n).into_iter().flatten() {
            if self.pos(d).is_some_and(|p| order(p, at).is_before())
                && !before.iter().any(|&k| {
                    self.overwrites(k, d)
                        && self.pos(d).zip(self.pos(k)).is_some_and(|(pd, pk)| {
                            matches!(order(pd, pk), Order::Before | Order::Same)
                        })
                })
            {
                set.insert(d);
            }
        }
        set
    }

    /// Contributions live at evaluation point `at` of node `n` or at any later
    /// point on a path that passes through the point: the creation rule of an
    /// escaping literal. Propagated forward from the point, so a contribution
    /// that reaches a later node only on a path bypassing the point is not
    /// admitted.
    fn live_from_point(&mut self, n: usize, at: &Pos) -> BitSet {
        let width = self.defs.len();
        let mut set = self.live_at(n, at);
        // What leaves `n` after the point: survivors of the node's later
        // writes, plus those later writes themselves.
        let mut leaving = set.clone();
        leaving.subtract(&self.kill[n]);
        for &d in self.node_sources.get(&n).into_iter().flatten() {
            let later = self
                .pos(d)
                .is_some_and(|p| !matches!(order(p, at), Order::Before | Order::Exclusive));
            if later && self.gen[n].contains(d) {
                set.insert(d);
                leaving.insert(d);
            }
        }
        let before = self.writes_before(n, at);
        let mut incoming: BTreeMap<usize, BitSet> = BTreeMap::new();
        let mut work: Vec<usize> = Vec::new();
        // A node is queued the first time it is reached and again whenever its
        // incoming set grows.
        let push = |incoming: &mut BTreeMap<usize, BitSet>,
                    work: &mut Vec<usize>,
                    from: &BitSet,
                    node: usize| {
            let first = !incoming.contains_key(&node);
            let entry = incoming.entry(node).or_insert_with(|| BitSet::new(width));
            if entry.union_changed(from) || first {
                work.push(node);
            }
        };
        for &(t, _) in &self.successors[n] {
            push(&mut incoming, &mut work, &leaving, t);
        }
        while let Some(m) = work.pop() {
            let mut input = incoming[&m].clone();
            if m == n {
                // Round the loop back to the creation node: only what the
                // point's own earlier writes do not overwrite is live there.
                let mut kept = BitSet::new(width);
                for d in input.members() {
                    if !before.iter().any(|&k| self.overwrites(k, d)) {
                        kept.insert(d);
                    }
                }
                input = kept;
            }
            set.union_with(&input);
            set.union_with(&self.gen[m]);
            let mut out = input;
            out.subtract(&self.kill[m]);
            out.union_with(&self.gen[m]);
            for &(t, _) in &self.successors[m] {
                push(&mut incoming, &mut work, &out, t);
            }
        }
        set
    }

    fn node_of_call(&self, closure: usize, call: usize) -> Option<usize> {
        self.call_nodes[closure][call]
    }

    /// Contributions the literal can read, by when its body runs.
    fn closure_live(&mut self, closure: usize) -> &BitSet {
        if !self.capture_live.contains_key(&closure) {
            let facts = self.facts;
            let c = &facts.closures[closure];
            let mut set = BitSet::new(self.defs.len());
            match c.mode {
                Mode::Escaping => {
                    if let Some(n) = self.closure_nodes[closure] {
                        set = self.live_from_point(n, &c.pos);
                    }
                }
                Mode::AtCreation => {
                    if let Some(n) = self.closure_nodes[closure] {
                        set = self.live_at(n, &c.pos);
                    }
                }
                Mode::AtCalls => {
                    for (i, call) in c.calls.iter().enumerate() {
                        if let Some(n) = self.node_of_call(closure, i) {
                            set.union_with(&self.live_at(n, &call.pos));
                        }
                    }
                }
            }
            self.capture_live.insert(closure, set);
        }
        &self.capture_live[&closure]
    }

    /// Read inside a function literal: provisional timing, always
    /// `cfg_incomplete`. See `go_literal_mode` for the run-site rule.
    pub(super) fn classify_capture(
        &mut self,
        d: usize,
        u: usize,
        closure: usize,
    ) -> Option<FlowConfidence> {
        let (defs, facts) = (self.defs, self.facts);
        let path = &defs[d].path;
        if facts.closures[closure]
            .writes
            .iter()
            .any(|w| w.base == path.base && path.fields.starts_with(&w.fields))
        {
            return None;
        }
        self.binding_uncertain(d, u)?;
        self.mapped_defs[d]?;
        self.closure_nodes[closure]?;
        self.closure_live(closure)
            .contains(d)
            .then_some(CFG_INCOMPLETE)
    }

    /// Receiver of `defer R.M()` / `go R.M()`: Go evaluates the receiver at
    /// the statement, so only contributions that reach the statement count
    /// (owner decision, #639 S2 fold 3; later writes are issue #675).
    pub(super) fn classify_deferred(&mut self, d: usize, u: usize) -> Option<FlowConfidence> {
        self.binding_uncertain(d, u)?;
        self.mapped_defs[d]?;
        let n = self.mapped_reads[u]?;
        if !self.deferred_live.contains_key(&u) {
            let facts = self.facts;
            let set = self.live_at(n, &facts.reads[u].pos);
            self.deferred_live.insert(u, set);
        }
        self.deferred_live[&u].contains(d).then_some(CFG_INCOMPLETE)
    }

    /// Whether the `for` statement `id` can run its post and come back to its
    /// own header: a path from the node back to itself through nodes of the
    /// loop's own line span (an enclosing loop's cycle does not count).
    fn loop_returns(&self, id: usize, node: usize) -> bool {
        let Some(&(lo, hi)) = self.facts.loops.get(&id) else {
            return false;
        };
        let inside = |m: usize| (lo..=hi).contains(&self.node_line[m]);
        let mut seen = BTreeSet::new();
        // A loop whose header, body and back edge share this one node leaves
        // no route to follow (a body that returns looks the same), so a direct
        // self edge proves nothing.
        let mut stack: Vec<usize> = self.successors[node]
            .iter()
            .filter(|&&(t, incomplete)| !incomplete && t != node && inside(t))
            .map(|&(t, _)| t)
            .collect();
        while let Some(m) = stack.pop() {
            if m == node {
                return true;
            }
            if seen.insert(m) {
                stack.extend(
                    self.successors[m]
                        .iter()
                        .filter(|&&(t, incomplete)| !incomplete && inside(t))
                        .map(|&(t, _)| t),
                );
            }
        }
        false
    }

    /// Same CFG node: order is evaluation order (`route::order`), not byte
    /// order. Within one simple statement the right side runs before the
    /// write; across `;` statements and header clauses the earlier one writes
    /// first; arms of one branch never both run.
    pub(super) fn classify_same_line(&mut self, d: usize, u: usize) -> Option<FlowConfidence> {
        let (defs, facts) = (self.defs, self.facts);
        let (def, read) = (&defs[d], &facts.reads[u]);
        let meta = facts.write_meta.get(&(def.path.clone(), def.start_byte))?;
        self.binding_uncertain(d, u)?;
        let forward = match order(&meta.pos, &read.pos) {
            Order::Before => true,
            // Only a `for` post reaches its own condition or body, and only
            // over this loop's own cycle.
            Order::LoopBack(id) => {
                if !self.mapped_defs[d].is_some_and(|n| self.loop_returns(id, n)) {
                    return None;
                }
                false
            }
            _ => return None,
        };
        for &k in self.line_writes.get(&def.line).into_iter().flatten() {
            if !self.overwrites(k, d) {
                continue;
            }
            let killer = facts
                .write_meta
                .get(&(defs[k].path.clone(), defs[k].start_byte))?;
            if !forward
                || (matches!(order(&meta.pos, &killer.pos), Order::Before | Order::Same)
                    && order(&killer.pos, &read.pos).is_before())
            {
                return None;
            }
        }
        Some(FlowConfidence::NameOnly(FlowDoubt::SameLine))
    }
}
