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
//! tree keeps main's statement universe and edges, and its owner's labels are refused callable-wide
//! (`NameOnly(CfgIncomplete)`, option O1; reaching definitions asks `refuses_owner`).
//!
//! The dispositions are read off a build, and a build depends on the universe, which depends on the
//! admitted set: so the plan is a fixpoint. Round k builds every callable of the file with exactly the
//! trees admitted so far assumed admitted, and refuses the trees whose records or placement fail; it
//! stops at the first round that refuses nothing, so the surviving trees were verified against the very
//! build that stands.
use super::{placement, Cx, Legacy};
use crate::ast::ParsedFile;
use crate::cfg::js_disposition::{Disposition, Ownership, Unres};
use crate::cfg::js_ownership::EdgeRecord;
use crate::cfg::EdgeSink;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

type Range = (usize, usize);

std::thread_local! {
    /// The trees a plan pass assumes admitted (while the plan itself is still being computed).
    static ASSUMED: RefCell<Option<BTreeSet<Range>>> = const { RefCell::new(None) };
}

/// Runs `f` with exactly `admitted` as the admitted trees.
pub(in crate::cfg) fn with_admission<T>(admitted: &BTreeSet<Range>, f: impl FnOnce() -> T) -> T {
    struct Reset(Option<BTreeSet<Range>>);
    impl Drop for Reset {
        fn drop(&mut self) {
            ASSUMED.with(|c| *c.borrow_mut() = self.0.take());
        }
    }
    let _reset = Reset(ASSUMED.with(|c| c.borrow_mut().replace(admitted.clone())));
    f()
}

/// Whether the tree whose outermost loop spans `root` is admitted. Nothing is, in main's loop CFG.
pub(super) fn admits(parsed: &ParsedFile, root: Range) -> bool {
    if super::main_loop_cfg() {
        return false;
    }
    match ASSUMED.with(|c| c.borrow().as_ref().map(|a| a.contains(&root))) {
        Some(assumed) => assumed,
        None => parsed.js_loop_plan().admitted.contains(&root),
    }
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
}

#[derive(Clone, Debug, Default)]
pub(crate) struct LoopPlan {
    admitted: BTreeSet<Range>,
    refused_owners: BTreeSet<Range>,
    pub(crate) verdicts: Vec<Verdict>,
}

impl LoopPlan {
    /// Whether the owner's labels are refused callable-wide: a tree of it was newly refused (O1).
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
                Some(Why::Unresolved(Unres::ForeignAdmitted)) => {
                    bump("refused.foreign_admitted".into())
                }
                Some(Why::Unresolved(u)) => bump(format!("refused.unresolved.{}", u.key())),
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
                    },
                );
            }
        }
        verdicts
    }

    pub(crate) fn compute(parsed: &ParsedFile) -> Self {
        let mut verdicts = Self::initial(parsed);
        let mut admitted: BTreeSet<Range> = verdicts
            .values()
            .filter(|v| v.why.is_none())
            .map(|v| v.root)
            .collect();
        while !admitted.is_empty() {
            let refused = with_admission(&admitted, || round(parsed, &admitted));
            if refused.is_empty() {
                break;
            }
            for (root, (why, overlaps, first)) in refused {
                admitted.remove(&root);
                if let Some(v) = verdicts.get_mut(&root) {
                    (v.why, v.overlaps, v.first) = (Some(why), overlaps, first);
                }
            }
        }
        let refused_owners = verdicts
            .values()
            .filter(|v| matches!(v.why, Some(Why::Placement | Why::Unresolved(_))))
            .map(|v| v.owner)
            .collect();
        let plan = Self {
            admitted,
            refused_owners,
            verdicts: verdicts.into_values().collect(),
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
                v.why.as_ref().map_or("admitted".to_string(), |w| format!("{w:?}")),
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

/// One round: the trees of the admitted set (assumed, see `with_admission`) that fail placement or whose
/// records are unresolved, with the reason classes that apply and the first offender.
fn round(parsed: &ParsedFile, admitted: &BTreeSet<Range>) -> Refusals {
    let mut out = Refusals::new();
    for (owner, own) in ownerships(parsed, true) {
        let node = parsed
            .all_functions()
            .into_iter()
            .find(|n| (n.start_byte(), n.end_byte()) == owner);
        for t in own
            .trees
            .iter()
            .filter(|t| admitted.contains(&(t.root.start, t.root.end)))
        {
            let root = (t.root.start, t.root.end);
            let placed = node.and_then(|n| placement::violation(parsed, n, root));
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
    out
}

fn describe(r: &EdgeRecord) -> String {
    format!(
        "#{} {:?}/{:?} L{}->L{}",
        r.id, r.producer, r.role, r.from_line, r.to_line
    )
}

/// Every callable of the file built with ownership records, and for each one that holds a loop tree (or
/// every one, `looped` false) its `Ownership`, under the current admission. The surviving records of
/// the OTHER callables' builds are handed to each as the foreign records its RD may admit.
pub(in crate::cfg) fn ownerships(parsed: &ParsedFile, looped: bool) -> Vec<(Range, Ownership)> {
    let mut sink = EdgeSink::new();
    let mut built = Vec::new();
    for owner in parsed.all_functions() {
        crate::cfg::build_function_cfg(owner, parsed, &mut sink);
        if let Some(rec) = sink.take_records() {
            built.push((owner, rec));
        }
        sink.drain().for_each(drop);
    }
    // The other callables' surviving edges by source line, as `FileCfgEdgeIndex` indexes them.
    let mut by_source: BTreeMap<usize, Vec<(usize, &EdgeRecord)>> = BTreeMap::new();
    for (i, (_, rec)) in built.iter().enumerate() {
        for r in &rec.live {
            by_source.entry(r.from_line).or_default().push((i, r));
        }
    }
    let wanted: Vec<bool> = built
        .iter()
        .map(|(owner, _)| {
            let boundaries = parsed.language.callable_boundary_node_types();
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
    let others: Vec<Vec<EdgeRecord>> = built
        .iter()
        .enumerate()
        .map(|(i, (owner, _))| {
            if !wanted[i] {
                return Vec::new();
            }
            let mine: BTreeSet<usize> = parsed
                .statement_spans_before_dedup(owner)
                .iter()
                .map(|s| s.line)
                .collect();
            mine.iter()
                .filter_map(|l| by_source.get(l))
                .flatten()
                .filter(|(j, r)| *j != i && mine.contains(&r.to_line))
                .map(|(_, r)| (*r).clone())
                .collect()
        })
        .collect();
    drop(by_source);
    built
        .into_iter()
        .zip(others)
        .zip(wanted)
        .filter(|(_, w)| *w)
        .map(|(((owner, rec), others), _)| {
            (
                (owner.start_byte(), owner.end_byte()),
                Ownership::new(parsed, owner, rec, others),
            )
        })
        .collect()
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;
