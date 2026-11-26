//! I1 complete predictions (lane 10, #847): which `(W, R)` pairs would survive every refusal.
//!
//! One owner at a time, in the design's order: RD availability, raw work `P` (see `limits.rs`),
//! then per pair the endpoint join, the RD binding and placement proof, the syntax route and the
//! occupancy of the file's final edge, label and supplemental keys; survivors are deduplicated
//! and finally capped by read fan-in and owner size. Reads here are strictly read-only: the RD
//! facts are built from copies of the producer's inputs and never reach the graph, the labels or
//! the statistics.
use super::join::{operand_path, write_path, Buckets, JoinRefusal, OwnerJoin};
use super::limits::{cap, raw_rows, Limits, Rd, RdExit, Work};
use super::route::{self, Refused, Route, Step};
use super::{OwnerScan, Span};
use crate::access_path::ARRAY_FIELD;
use crate::ast::ParsedFile;
use crate::data_flow::{FlowEdge, VarAccessKind, VarLocation};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

use super::super::{deduplicate_definitions, placed_lines, DefSite};

/// What reaching definitions' declaration facts say about a name at a byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Bound {
    Declared(usize),
    Implicit,
    Flat,
}

/// Read-only RD facts of one owner: how RD treated it, the statement each W/R line maps to, and
/// the binding each root occurrence resolves to.
pub(crate) struct RdFacts {
    pub rd: Rd,
    pub(super) placed: BTreeMap<usize, Option<usize>>,
    pub(super) bound: BTreeMap<(String, usize), Bound>,
}

impl RdFacts {
    pub(crate) fn unavailable(rd: Rd) -> Self {
        RdFacts {
            rd,
            placed: BTreeMap::new(),
            bound: BTreeMap::new(),
        }
    }

    /// Facts of an RD-available owner, from the producer's own defs and use edges.
    pub(crate) fn available(
        parsed: &ParsedFile,
        func: &Node<'_>,
        scan: &OwnerScan,
        defs: &[DefSite],
        edges: &[FlowEdge],
    ) -> Self {
        use crate::cpg::binding_facts::{BindingFacts, BindingId};
        let writes = scan.writes.iter().filter(|w| w.verdict.is_ok());
        let reads = scan.operands.iter().filter(|o| o.verdict.is_ok());
        let sites = writes
            .map(|w| (&w.path, w.span))
            .chain(reads.map(|o| (&o.path, o.span)));
        let sites: Vec<_> = sites.collect();
        let lines: BTreeSet<usize> = sites.iter().map(|(_, s)| s.line).collect();
        let lines: Vec<usize> = lines.into_iter().collect();
        let placed = lines
            .iter()
            .copied()
            .zip(placed_lines(parsed, func, &lines));
        let facts = BindingFacts::new(parsed, func, &deduplicate_definitions(defs), edges.iter());
        let mut bound = BTreeMap::new();
        for (path, s) in sites {
            let Some(root) = path.root.as_ref() else {
                continue;
            };
            bound
                .entry((root.clone(), s.start))
                .or_insert(match facts.binding_at(root, s.start) {
                    BindingId::Declaration(i) => Bound::Declared(i),
                    BindingId::ImplicitFunction(_) => Bound::Implicit,
                    BindingId::FlatFallback(_) => Bound::Flat,
                });
        }
        let rd = Rd {
            exit: RdExit::Available,
            early: false,
        };
        let placed = placed.collect();
        RdFacts { rd, placed, bound }
    }

    /// Same declared binding at both bytes (never the flat fallback) and two different mapped
    /// statements.
    fn proves(&self, root: &str, w: Span, r: Span) -> Result<(), Refusal> {
        let at = |s: Span| self.bound.get(&(root.to_string(), s.start)).copied();
        let same = matches!((at(w), at(r)), (Some(a), Some(b)) if a == b && a != Bound::Flat);
        let line = |s: Span| self.placed.get(&s.line).copied().flatten();
        let apart = matches!((line(w), line(r)), (Some(a), Some(b)) if a != b);
        match (same, apart) {
            (false, _) => Err(Refusal::Binding),
            (_, false) => Err(Refusal::Placement),
            _ => Ok(()),
        }
    }
}

/// The legacy keys (line-and-path identity, bytes excluded) of the file's existing `Def -> Use`
/// edges, labels and supplemental exact keys whose source is an indexed store.
#[derive(Default)]
pub(crate) struct Occupied(pub(super) BTreeSet<(VarLocation, VarLocation)>);

impl Occupied {
    pub(crate) fn new<'a>(
        pairs: impl IntoIterator<Item = (&'a VarLocation, &'a VarLocation)>,
    ) -> Self {
        let store = |l: &VarLocation| {
            l.kind == VarAccessKind::Def && l.path.fields.last().is_some_and(|f| f == ARRAY_FIELD)
        };
        let keep = pairs.into_iter().filter(|(from, _)| store(from));
        Occupied(keep.map(|(f, t)| (f.clone(), t.clone())).collect())
    }

    fn has(&self, w: &VarLocation, r: &VarLocation) -> bool {
        self.0.contains(&(w.clone(), r.clone()))
    }
}

/// Why a pair is not predicted. The order of the variants is the order of the checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    Rd(Rd),
    RawCap,
    Join(JoinRefusal),
    Binding,
    Placement,
    Route(Refused),
    Occupied,
    FanIn,
    OwnerCap,
}

impl Refusal {
    pub(crate) fn name(&self) -> String {
        let low = |d: &dyn std::fmt::Debug| format!("{d:?}").to_lowercase();
        match self {
            Refusal::Rd(rd) => format!("rd:{}", rd.name()),
            Refusal::Join(j) => format!("join:{}", low(j)),
            Refusal::Route(r) => format!("route:{}", low(&r.why)),
            other => low(other),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PairPreview {
    pub w: usize,
    pub r: usize,
    pub outcome: Result<Vec<Step>, Refusal>,
}

/// Pairs left after each check, in order (`admitted` is the syntax inventory's).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Attrition {
    pub admitted: usize,
    pub joined: usize,
    pub bound: usize,
    pub routed: usize,
    pub unoccupied: usize,
    pub distinct: usize,
    pub after_fanin: usize,
    pub published: usize,
}

pub(crate) struct OwnerPreview {
    pub rd: Rd,
    /// `None` when RD is unavailable: `P` is unknown.
    pub work: Option<Work>,
    pub raw_capped: bool,
    pub pairs: Vec<PairPreview>,
    pub attrition: Attrition,
    pub fanin_reads: usize,
    pub fanin_pairs: usize,
    pub over_edge_cap: bool,
    pub cap_pairs: usize,
}

pub(crate) struct Inputs<'a> {
    pub file: &'a str,
    pub parsed: &'a ParsedFile,
    pub scan: &'a OwnerScan,
    pub join: &'a OwnerJoin,
    pub rd: Option<&'a RdFacts>,
    pub defs: &'a Buckets,
    pub uses: &'a Buckets,
    pub occupied: &'a Occupied,
    pub limits: Limits,
}

fn blank() -> crate::access_path::AccessPath {
    crate::access_path::AccessPath::simple("")
}

fn location(
    i: &Inputs,
    kind: VarAccessKind,
    path: crate::access_path::AccessPath,
    s: Span,
) -> VarLocation {
    VarLocation {
        file: i.file.into(),
        function: i.scan.name.clone(),
        function_start_line: i.scan.span.line,
        line: s.line,
        path,
        start_byte: s.start,
        end_byte: s.end,
        kind,
    }
}

/// Predict one owner.
pub(crate) fn preview(i: &Inputs) -> OwnerPreview {
    let scan = i.scan;
    let pairs = scan.pairs();
    let rd = i.rd.map_or(
        Rd {
            exit: RdExit::Unseen,
            early: false,
        },
        |f| f.rd,
    );
    let mut out = OwnerPreview {
        rd,
        work: None,
        raw_capped: false,
        pairs: Vec::new(),
        attrition: Attrition {
            admitted: pairs.len(),
            ..Attrition::default()
        },
        fanin_reads: 0,
        fanin_pairs: 0,
        over_edge_cap: false,
        cap_pairs: 0,
    };
    let refuse = |why: Refusal| {
        let all = pairs.iter().map(|&(w, r)| PairPreview {
            w,
            r,
            outcome: Err(why.clone()),
        });
        all.collect::<Vec<_>>()
    };
    let facts = match i.rd {
        Some(f) if rd.exit == RdExit::Available => f,
        _ => {
            out.pairs = refuse(Refusal::Rd(rd));
            return out;
        }
    };
    let owner = (i.file, scan.name.as_str(), scan.span.line);
    let mut buckets: BTreeMap<usize, (u64, BTreeSet<usize>)> = BTreeMap::new();
    for &(w, r) in &pairs {
        let (ws, os) = (&scan.writes[w], &scan.operands[r]);
        let (wp, op) = (write_path(&ws.path), operand_path(&os.path));
        let rows = |b, p: Option<_>, kind, s| p.map_or(0, |p| raw_rows(b, owner, &p, kind, s));
        let (mult, reads) = (
            rows(i.defs, wp, VarAccessKind::Def, ws.span),
            rows(i.uses, op, VarAccessKind::Use, os.span),
        );
        let entry = buckets.entry(w).or_insert((mult, BTreeSet::new()));
        if mult > 0 && reads > 0 {
            entry.1.insert(r);
        }
    }
    let work = Work::count(buckets.values().map(|(m, rs)| (*m, rs.len() as u64)));
    out.work = Some(work);
    if work.exceeds(i.limits.raw_pairs) {
        out.raw_capped = true;
        out.pairs = refuse(Refusal::RawCap);
        return out;
    }
    let routes = route::routes(i.parsed, scan);
    let mut survivors: BTreeMap<(Span, Span), Vec<Step>> = BTreeMap::new();
    let a = &mut out.attrition;
    for (k, &(w, r)) in pairs.iter().enumerate() {
        let (ws, os) = (&scan.writes[w], &scan.operands[r]);
        let root = ws.path.root.as_deref().unwrap_or("");
        let free = |a: &mut Attrition| -> Result<Vec<Step>, Refusal> {
            i.join.pairs[k].1.map_err(Refusal::Join)?;
            a.joined += 1;
            facts.proves(root, ws.span, os.span)?;
            a.bound += 1;
            let steps = match &routes[k].1 {
                Route::Reached(steps) => steps.clone(),
                Route::Refused(why) => return Err(Refusal::Route(why.clone())),
            };
            a.routed += 1;
            let wl = location(
                i,
                VarAccessKind::Def,
                write_path(&ws.path).unwrap_or_else(blank),
                ws.span,
            );
            let rl = location(
                i,
                VarAccessKind::Use,
                operand_path(&os.path).unwrap_or_else(blank),
                os.span,
            );
            if i.occupied.has(&wl, &rl) {
                return Err(Refusal::Occupied);
            }
            a.unoccupied += 1;
            Ok(steps)
        };
        let outcome = free(a);
        if let Ok(steps) = &outcome {
            survivors
                .entry((ws.span, os.span))
                .or_insert_with(|| steps.clone());
        }
        out.pairs.push(PairPreview { w, r, outcome });
    }
    let keys: BTreeSet<_> = survivors.keys().copied().collect();
    let capped = cap(&i.limits, &keys);
    out.attrition.distinct = keys.len();
    out.attrition.after_fanin = capped.kept.len() + capped.owner_pairs;
    out.attrition.published = capped.kept.len();
    out.fanin_reads = capped.fanin_reads.len();
    out.fanin_pairs = capped.fanin_pairs;
    out.over_edge_cap = capped.owner_over;
    out.cap_pairs = capped.owner_pairs;
    for p in out.pairs.iter_mut().filter(|p| p.outcome.is_ok()) {
        let key = (scan.writes[p.w].span, scan.operands[p.r].span);
        if !capped.kept.contains(&key) {
            let fan = capped.fanin_reads.contains(&key.1);
            p.outcome = Err(if fan {
                Refusal::FanIn
            } else {
                Refusal::OwnerCap
            });
        }
    }
    out
}
