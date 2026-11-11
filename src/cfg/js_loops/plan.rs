//! The loop-tree plan of a JS/TS/TSX file (#693, #697; option B, slice 2).
//!
//! ONE immutable plan, a pure function of the parsed file, decides which loop trees are rewired. The `do`
//! condition admission (the statement universe) and the edge rewrite (`js_loops::build`) both ask it, so
//! no `do` condition is admitted without its tree's rewrite and no tree is half mutated.
//!
//! A tree is ADMITTED only when (1) main's own policy routes it (`Cx::tree_checked`: a `for` initializer
//! hazard, a jump through a `finally` and an unrouted form refuse it exactly as before, keeping main's
//! edges and labels), (2) the PLACEMENT rule holds (`placement`), and (3) every record of the owner that
//! intersects it has a disposition of `OwnedReplaceable`, `Independent` or `Foreign` (not admitted by
//! the owner's RD, or independent): an `Unresolved` one (`js_disposition`) refuses the tree. A refused
//! tree is refused WHOLE with its owner (final refusal policy): the owner keeps main's statement universe,
//! edges, labels and RD availability exactly, and its admissible sibling trees are refused with it
//! (`Why::OwnerSibling`); `build_inner` asks `LoopPlan::admitted` and reaching definitions has no refusal path.
//!
//! The dispositions are read off a build, and a build depends on the universe, which depends on the
//! admitted set: so the plan is a fixpoint. Round k builds every callable of the file with exactly the
//! trees admitted so far assumed admitted, and refuses the trees whose records or placement fail; it
//! stops at the first round that refuses nothing, so the surviving trees were verified against the very
//! build that stands.
use super::{placement, tryflow, Cx, Legacy};
use crate::ast::ParsedFile;
use crate::cfg::js_disposition::{Disposition, Ownership, Unres};
use crate::cfg::js_ownership::EdgeRecord;
use crate::cfg::EdgeSink;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

type Range = (usize, usize);

/// The file an assumption is about (the address of its `ParsedFile`, which a plan pass borrows for its whole
/// run) and its trees.
type Assumed = (usize, BTreeSet<Range>);

std::thread_local! {
    /// The trees a plan pass assumes admitted (while the plan itself is still being computed).
    static ASSUMED: RefCell<Option<Assumed>> = const { RefCell::new(None) };
}

fn file_key(parsed: &ParsedFile) -> usize {
    std::ptr::from_ref(parsed) as usize
}

/// Runs `f` with exactly `admitted` as the admitted trees of `parsed`. A question about another file (a
/// rayon join can run another file's task on this thread) is answered by that file's own plan.
pub(in crate::cfg) fn with_admission<T>(
    parsed: &ParsedFile,
    admitted: &BTreeSet<Range>,
    f: impl FnOnce() -> T,
) -> T {
    struct Reset(Option<Assumed>);
    impl Drop for Reset {
        fn drop(&mut self) {
            ASSUMED.with(|c| *c.borrow_mut() = self.0.take());
        }
    }
    let _reset =
        Reset(ASSUMED.with(|c| c.borrow_mut().replace((file_key(parsed), admitted.clone()))));
    f()
}

/// Whether the tree whose outermost loop spans `root` is admitted. Nothing is, in main's loop CFG.
pub(super) fn admits(parsed: &ParsedFile, root: Range) -> bool {
    if super::main_loop_cfg() {
        return false;
    }
    let key = file_key(parsed);
    match ASSUMED.with(|c| {
        c.borrow()
            .as_ref()
            .filter(|(k, _)| *k == key)
            .map(|(_, a)| a.contains(&root))
    }) {
        Some(assumed) => assumed,
        None => parsed.js_loop_plan().admitted.contains(&root),
    }
}

/// Whether some admitted tree (under a plan pass, the assumed admission) lies inside `range`: when none does,
/// no tree of a callable spanning `range` (or nested in it) is rewired. A speed gate, never a verdict.
pub(in crate::cfg) fn any_admitted_within(parsed: &ParsedFile, range: Range) -> bool {
    if super::main_loop_cfg() {
        return false;
    }
    let within = |set: &BTreeSet<Range>| {
        set.range((range.0, 0)..=(range.1, usize::MAX))
            .any(|r| r.1 <= range.1)
    };
    let key = file_key(parsed);
    match ASSUMED.with(|c| {
        c.borrow()
            .as_ref()
            .filter(|(k, _)| *k == key)
            .map(|(_, a)| within(a))
    }) {
        Some(found) => found,
        None => within(&parsed.js_loop_plan().admitted),
    }
}

/// Whether `parsed` is being built under a plan pass's assumption (not the standing build).
pub(in crate::cfg) fn assuming(parsed: &ParsedFile) -> bool {
    let key = file_key(parsed);
    ASSUMED.with(|c| c.borrow().as_ref().is_some_and(|(k, _)| *k == key))
}

/// The outermost loops main's own policy routes (the plan's first assumption), as byte ranges.
#[cfg(test)]
pub(in crate::cfg) fn legacy_candidates(parsed: &ParsedFile) -> BTreeSet<Range> {
    LoopPlan::initial(parsed)
        .into_values()
        .filter(|v| v.why.is_none())
        .map(|v| v.root)
        .collect()
}

/// Why a tree is not rewired.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Why {
    /// Main's own policy (no new reasoning, no label refusal).
    Legacy(Legacy),
    /// Some occurrence of the tree is not placed as the rewrite needs.
    Placement,
    /// A record of the owner that intersects the tree has this unresolved kind (the first by emission).
    Unresolved(Unres),
    /// The owner contains, or its selected statement lines overlap, a `try` (T2, `tryflow`).
    TryFlow,
    /// Another tree of the same owner is refused: a refused owner keeps main's geometry whole (final policy),
    /// so none of its trees is rewired.
    OwnerSibling,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Verdict {
    pub(crate) root: Range,
    pub(crate) owner: Range,
    /// `None`: admitted.
    pub(crate) why: Option<Why>,
    /// Every class that applies in the round that refused the tree: `placement`, `unresolved`,
    /// `foreign_admitted`.
    pub(crate) overlaps: Vec<&'static str>,
    /// The first offending occurrence or record, for the trace.
    pub(crate) first: String,
    /// A tree main's own policy refuses (`why` is `Legacy`) keeps main's edges, but is classified under the
    /// final admission as well: a failed placement or an unresolved record refuses its OWNER by label
    /// (#693 fold 2), which this records. Always `None` for any other verdict.
    pub(crate) legacy_o1: Option<Why>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct LoopPlan {
    admitted: BTreeSet<Range>,
    refused_owners: BTreeSet<Range>,
    pub(crate) verdicts: Vec<Verdict>,
    /// The edges the last verifying round drew for the callables it built (key: the callable's byte range),
    /// under exactly the admission that stands: the standing build of those callables is these edges.
    built: Built,
}

/// A callable's drained edges, each with its origin (what `EdgeSink::drain` yields).
type Built = BTreeMap<Range, Vec<(crate::cfg::CfgEdge, crate::cfg::EdgeOrigin)>>;

impl LoopPlan {
    /// The standing edges of the callable at `owner`, when the verifying round built it (a pure reuse of work
    /// the plan already did under the admission that stands, never a different answer).
    pub(in crate::cfg) fn replay(
        &self,
        owner: Range,
    ) -> Option<&[(crate::cfg::CfgEdge, crate::cfg::EdgeOrigin)]> {
        self.built.get(&owner).map(Vec::as_slice)
    }

    /// Whether the owner is refused: a tree of it is refused, so the owner keeps main's geometry and labels whole.
    #[cfg(test)]
    pub(crate) fn refuses_owner(&self, owner: Range) -> bool {
        self.refused_owners.contains(&owner)
    }

    /// The per-reason counters of the file (persisted with the other DFG counters).
    pub(crate) fn counters(&self) -> BTreeMap<String, usize> {
        let mut c = BTreeMap::new();
        let mut bump = |k: String| *c.entry(k).or_insert(0usize) += 1;
        for v in &self.verdicts {
            bump("trees".into());
            match &v.why {
                None => bump("admitted".into()),
                Some(Why::Legacy(l)) => bump(format!("refused.legacy.{}", legacy_key(*l))),
                Some(Why::Placement) => bump("refused.placement".into()),
                Some(Why::TryFlow) => bump("refused.try_flow".into()),
                Some(Why::OwnerSibling) => bump("refused.owner_sibling".into()),
                Some(Why::Unresolved(Unres::ForeignAdmitted)) => {
                    bump("refused.foreign_admitted".into())
                }
                Some(Why::Unresolved(u)) => bump(format!("refused.unresolved.{}", u.key())),
            }
            match &v.legacy_o1 {
                None => {}
                Some(Why::Unresolved(u)) => bump(format!("legacy_o1.unresolved.{}", u.key())),
                Some(Why::TryFlow) => bump("legacy_o1.try_flow".into()),
                Some(_) => bump("legacy_o1.placement".into()),
            }
            for o in &v.overlaps {
                bump(format!("overlap.{o}"));
            }
        }
        if !self.refused_owners.is_empty() {
            c.insert("owners_refused".into(), self.refused_owners.len());
        }
        c
    }

    /// Every outermost loop of the file, refused only by main's own policy.
    fn initial(parsed: &ParsedFile) -> BTreeMap<Range, Verdict> {
        let owners = parsed.all_functions();
        let mut verdicts: BTreeMap<Range, Verdict> = BTreeMap::new();
        for owner in &owners {
            let Some(body) = owner.child_by_field_name("body") else {
                continue;
            };
            let boundaries = parsed.language.callable_boundary_node_types();
            let cx = Cx {
                parsed,
                body,
                boundaries: &boundaries,
            };
            for root in cx.outermost() {
                let range = (root.start_byte(), root.end_byte());
                verdicts.insert(
                    range,
                    Verdict {
                        root: range,
                        owner: (owner.start_byte(), owner.end_byte()),
                        why: cx.tree_checked(root).err().map(Why::Legacy),
                        overlaps: Vec::new(),
                        first: String::new(),
                        legacy_o1: None,
                    },
                );
            }
        }
        verdicts
    }

    pub(crate) fn compute(parsed: &ParsedFile) -> Self {
        let mut verdicts = Self::initial(parsed);
        // No loop tree: nothing to admit, refuse or label, so the try regions are never read (speed gate).
        if verdicts.is_empty() {
            let plan = Self {
                admitted: BTreeSet::new(),
                refused_owners: BTreeSet::new(),
                verdicts: Vec::new(),
                built: Built::new(),
            };
            plan.trace(parsed);
            return plan;
        }
        let regions = tryflow::Regions::of(parsed);
        let mut admitted: BTreeSet<Range> = verdicts
            .values()
            .filter(|v| v.why.is_none())
            .map(|v| v.root)
            .collect();
        // The trees main's own policy refuses are classified too, in the round that stands (#693 fold 2):
        // they keep main's edges, but a failed placement or an unresolved record still refuses the owner.
        let legacy: BTreeSet<Range> = verdicts
            .values()
            .filter(|v| matches!(v.why, Some(Why::Legacy(_))))
            .map(|v| v.root)
            .collect();
        let mut built = Built::new();
        let mut standing_built = Built::new();
        while !admitted.is_empty() || !legacy.is_empty() {
            let roots: BTreeSet<Range> = admitted.union(&legacy).copied().collect();
            built.clear();
            let mut refused = with_admission(parsed, &admitted, || {
                round(parsed, &roots, &regions, &mut built)
            });
            // A refused owner keeps main's geometry whole: every other tree of an owner that has a refused
            // (or legacy) tree is refused with it.
            let bad: BTreeSet<Range> = verdicts
                .values()
                .filter(|v| v.why.is_some())
                .map(|v| v.owner)
                .chain(
                    refused
                        .keys()
                        .filter_map(|r| verdicts.get(r).map(|v| v.owner)),
                )
                .collect();
            for r in &admitted {
                if !refused.contains_key(r)
                    && verdicts.get(r).is_some_and(|v| bad.contains(&v.owner))
                {
                    refused.insert(
                        *r,
                        (
                            Why::OwnerSibling,
                            vec!["owner_sibling"],
                            "another tree of the owner is refused".into(),
                        ),
                    );
                }
            }
            let standing: Refusals = {
                let keep = refused.keys().filter(|r| legacy.contains(r)).copied();
                keep.collect::<Vec<_>>()
                    .into_iter()
                    .filter_map(|r| refused.remove(&r).map(|v| (r, v)))
                    .collect()
            };
            if refused.is_empty() {
                // This round's admission is the standing one: its builds are the standing builds.
                standing_built = std::mem::take(&mut built);
                for (root, (why, overlaps, first)) in standing {
                    if let Some(v) = verdicts.get_mut(&root) {
                        (v.legacy_o1, v.overlaps, v.first) = (Some(why), overlaps, first);
                    }
                }
                break;
            }
            for (root, (why, overlaps, first)) in refused {
                admitted.remove(&root);
                if let Some(v) = verdicts.get_mut(&root) {
                    (v.why, v.overlaps, v.first) = (Some(why), overlaps, first);
                }
            }
        }
        // Every overlapping reason is recorded: under the standing admission, a tree refused for another reason
        // that also sits in a try-affected owner says so.
        let tried = with_admission(parsed, &admitted, || tryflow::affected(parsed, &regions));
        for v in verdicts.values_mut() {
            if let (Some(why), true) = (tried.get(&v.root), v.why.is_some()) {
                if !v.overlaps.contains(&"try_flow") {
                    v.overlaps.push("try_flow");
                }
                if v.legacy_o1.is_none() && matches!(v.why, Some(Why::Legacy(_))) {
                    v.legacy_o1 = Some(Why::TryFlow);
                    if v.first.is_empty() {
                        v.first = why.clone();
                    }
                }
            }
        }
        let refused_owners = verdicts
            .values()
            .filter(|v| {
                matches!(
                    v.why,
                    Some(
                        Why::Placement
                            | Why::Unresolved(_)
                            | Why::Legacy(_)
                            | Why::TryFlow
                            | Why::OwnerSibling
                    )
                ) || v.legacy_o1.is_some()
            })
            .map(|v| v.owner)
            .collect();
        let plan = Self {
            admitted,
            refused_owners,
            verdicts: verdicts.into_values().collect(),
            built: standing_built,
        };
        plan.trace(parsed);
        plan
    }

    fn trace(&self, parsed: &ParsedFile) {
        if !crate::cfg::js_disposition::trace_enabled() {
            return;
        }
        for v in &self.verdicts {
            let line = |b| parsed.line_for_byte(b);
            eprintln!(
                "own-plan {} tree=[{},{}) lines={}-{} owner=[{},{}) verdict={} overlaps={:?} first={}",
                parsed.path,
                v.root.0,
                v.root.1,
                line(v.root.0),
                line(v.root.1.saturating_sub(1)),
                v.owner.0,
                v.owner.1,
                v.why.as_ref().map_or("admitted".to_string(), |w| format!("{w:?}"))
                    + &v.legacy_o1.as_ref().map_or(String::new(), |w| format!("+o1:{w:?}")),
                v.overlaps,
                if v.first.is_empty() { "-" } else { &v.first },
            );
        }
    }
}

fn legacy_key(l: Legacy) -> &'static str {
    match l {
        Legacy::ForInit => "for_init",
        Legacy::Finally => "finally",
        Legacy::Unrouted => "unrouted",
    }
}

type Refusals = BTreeMap<Range, (Why, Vec<&'static str>, String)>;

/// One round: the trees of `roots` (the admitted ones, assumed, see `with_admission`, and the ones main's
/// policy refuses) that fail placement or whose records are unresolved, with the reason classes that apply
/// and the first offender.
fn round(
    parsed: &ParsedFile,
    roots: &BTreeSet<Range>,
    regions: &tryflow::Regions,
    built: &mut Built,
) -> Refusals {
    let mut out = Refusals::new();
    for (node, own) in ownerships(parsed, true, Some(built)) {
        for t in own
            .trees
            .iter()
            .filter(|t| roots.contains(&(t.root.start, t.root.end)))
        {
            let root = (t.root.start, t.root.end);
            let placed = placement::violation(parsed, node, root);
            let bad: Vec<_> = t
                .entries
                .iter()
                .filter_map(|e| match e.disposition {
                    Disposition::Unresolved(u) => Some((u, &e.record)),
                    _ => None,
                })
                .collect();
            let mut overlaps = Vec::new();
            if placed.is_some() {
                overlaps.push("placement");
            }
            if bad.iter().any(|(u, _)| *u != Unres::ForeignAdmitted) {
                overlaps.push("unresolved");
            }
            if bad.iter().any(|(u, _)| *u == Unres::ForeignAdmitted) {
                overlaps.push("foreign_admitted");
            }
            let refusal = match (bad.first(), placed) {
                (Some((u, r)), _) => Some((Why::Unresolved(*u), describe(r))),
                (None, Some(p)) => Some((Why::Placement, p)),
                (None, None) => None,
            };
            if let Some((why, first)) = refusal {
                out.insert(root, (why, overlaps, first));
            }
        }
    }
    // T2: an owner that contains or overlaps a `try` is refused whole, every tree of it (legacy ones too).
    for (root, first) in tryflow::affected(parsed, regions) {
        if !roots.contains(&root) {
            continue;
        }
        match out.get_mut(&root) {
            Some((_, overlaps, _)) => overlaps.push("try_flow"),
            None => {
                out.insert(root, (Why::TryFlow, vec!["try_flow"], first));
            }
        }
    }
    out
}

fn describe(r: &EdgeRecord) -> String {
    format!(
        "#{} {:?}/{:?} L{}->L{}",
        r.id, r.producer, r.role, r.from_line, r.to_line
    )
}

/// The callables of the file that hold a loop tree (or every one, `looped` false), each with its `Ownership`
/// under the current admission. A callable's RD also selects the edges OTHER callables' builds drew between
/// two of its lines, so those callables are built too (only the ones sharing a statement line with a looped
/// callable, or enclosing it, can matter), and their surviving records are handed over as the foreign records it may admit.
pub(in crate::cfg) fn ownerships<'p>(
    parsed: &'p ParsedFile,
    looped: bool,
    mut capture: Option<&mut Built>,
) -> Vec<(tree_sitter::Node<'p>, Ownership)> {
    let boundaries = parsed.language.callable_boundary_node_types();
    let owners = parsed.all_functions();
    let lines: Vec<BTreeSet<usize>> = owners
        .iter()
        .map(|o| {
            parsed
                .statement_spans_before_dedup(o)
                .iter()
                .map(|s| s.line)
                .collect()
        })
        .collect();
    let wanted: Vec<bool> = owners
        .iter()
        .map(|owner| {
            !looped
                || owner.child_by_field_name("body").is_some_and(|body| {
                    !Cx {
                        parsed,
                        body,
                        boundaries: &boundaries,
                    }
                    .outermost()
                    .is_empty()
                })
        })
        .collect();
    let mut by_line: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (i, set) in lines.iter().enumerate() {
        for l in set {
            by_line.entry(*l).or_default().push(i);
        }
    }
    let index: BTreeMap<usize, usize> = owners
        .iter()
        .enumerate()
        .map(|(i, o)| (o.id(), i))
        .collect();
    let mut needed = wanted.clone();
    for (i, set) in lines.iter().enumerate().filter(|(i, _)| wanted[*i]) {
        for l in set {
            for j in &by_line[l] {
                needed[*j] |= *j != i;
            }
        }
        // An enclosing callable's generic collectors draw edges for the constructs inside this one.
        let mut at = owners[i].parent();
        while let Some(n) = at {
            if let Some(j) = index.get(&n.id()) {
                needed[*j] = true;
            }
            at = n.parent();
        }
    }
    let mut sink = EdgeSink::new();
    let mut built = Vec::new();
    for (i, owner) in owners.iter().enumerate().filter(|(i, _)| needed[*i]) {
        crate::cfg::build_function_cfg(*owner, parsed, &mut sink);
        if let Some(rec) = sink.take_records() {
            built.push((i, *owner, rec));
        }
        match capture.as_deref_mut() {
            Some(built) => {
                built.insert(
                    (owner.start_byte(), owner.end_byte()),
                    sink.drain().collect(),
                );
            }
            None => sink.drain().for_each(drop),
        }
    }
    // The other callables' surviving edges by source line, as `FileCfgEdgeIndex` indexes them.
    let mut by_source: BTreeMap<usize, Vec<(usize, &EdgeRecord)>> = BTreeMap::new();
    for (i, _, rec) in &built {
        for r in &rec.live {
            by_source.entry(r.from_line).or_default().push((*i, r));
        }
    }
    let others: Vec<Vec<EdgeRecord>> = built
        .iter()
        .map(|(i, _, _)| {
            if !wanted[*i] {
                return Vec::new();
            }
            lines[*i]
                .iter()
                .filter_map(|l| by_source.get(l))
                .flatten()
                .filter(|(j, r)| j != i && lines[*i].contains(&r.to_line))
                .map(|(_, r)| (*r).clone())
                .collect()
        })
        .collect();
    drop(by_source);
    built
        .into_iter()
        .zip(others)
        .filter(|((i, _, _), _)| wanted[*i])
        .map(|((_, owner, rec), others)| (owner, Ownership::new(parsed, owner, rec, others)))
        .collect()
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;
