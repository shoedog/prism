//! `PRISM_ELEMENT_RECALL_TRACE=1`: versioned stderr records of the I0 inventory and its physical
//! joins, Python only. Unset or any other value, no analysis runs and nothing is written. The
//! trace never changes the graph, the statistics or the review output.
//!
//! Line: `element-recall\tv1\t<json>`. `kind` is `owner` (one per owner holding a candidate store),
//! `write` (every indexed store candidate, refused ones included), `node` (every occurrence of a
//! store's root) or `prediction` (one per admitted `(W, R)` pair: `predicted` or the first
//! refusal, never a published edge). `join` is the endpoint verdict (`joined` or the refusal),
//! null for a candidate the syntax refused. `binding` is the root name. The `owner` record also
//! carries the RD exit, the raw work `P` (null when RD is unavailable), the sequential
//! attrition and the limit outcomes. An owner refused as a whole with more admitted pairs than
//! the raw limit lists a bounded sample of `prediction` and `pairs` rows; `attrition.admitted` is
//! its exact pair count and `omitted_pair_records` the exact number of rows left out. Predictions are finalized once the file's edges, labels and
//! supplemental exact keys are known.
use super::super::{DefSite, RdUnavailable};
use super::join::{join_owner, Buckets, Endpoint, Originals};
use super::limits::{Limits, Rd, RdExit};
use super::preview::{preview, Inputs, Occupied, OwnerPreview, RdFacts};
use super::*;
use crate::ast::{ParsedFile, PathSpan};
use crate::data_flow::{ExactFlowEdge, FlowEdge, VarLocation};
use crate::languages::Language;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::Write;
use std::sync::OnceLock;

pub(crate) const PREFIX: &str = "element-recall\tv1\t";

/// Whether the environment asked for the trace (read once per process).
fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("PRISM_ELEMENT_RECALL_TRACE").is_ok_and(|v| v == "1"))
}

/// Unit tests collect the records of the files they register instead of reading the environment.
#[cfg(test)]
pub(super) static CAPTURE: std::sync::Mutex<
    Option<std::collections::BTreeMap<String, Vec<String>>>,
> = std::sync::Mutex::new(None);

/// Unit tests inject smaller limits for the file they register; production uses [`Limits::PRODUCTION`].
#[cfg(test)]
pub(super) static LIMITS: std::sync::Mutex<BTreeMap<String, Limits>> =
    std::sync::Mutex::new(BTreeMap::new());

fn limits(_file: &str) -> Limits {
    #[cfg(test)]
    if let Some(l) = LIMITS.lock().unwrap().get(_file) {
        return *l;
    }
    Limits::PRODUCTION
}

#[cfg(test)]
fn captured(file: &str) -> bool {
    CAPTURE
        .lock()
        .unwrap()
        .as_ref()
        .is_some_and(|m| m.contains_key(file))
}
#[cfg(not(test))]
fn captured(_: &str) -> bool {
    false
}

/// The ordinary occurrences of every owner pass of one file, kept until the buckets are final,
/// with the RD facts the producer reported per owner.
pub(crate) struct FileTrace {
    owners: Vec<Originals>,
    scans: Vec<OwnerScan>,
    rd: BTreeMap<(String, usize), RdFacts>,
}

/// The file-wide results the predictions are finalized against.
pub(crate) struct Final<'a> {
    pub defs: &'a Buckets,
    pub uses: &'a Buckets,
    pub edges: &'a [FlowEdge],
    pub labels: &'a BTreeMap<(VarLocation, VarLocation), crate::cpg::FlowConfidence>,
    pub exact: &'a BTreeMap<ExactFlowEdge, crate::cpg::FlowConfidence>,
}

fn occurrences(spans: &[PathSpan]) -> Vec<join::Occurrence> {
    let row = |s: &PathSpan| (s.path.clone(), s.line, s.start_byte, s.end_byte);
    spans.iter().map(row).collect()
}

fn verdict(e: Option<Endpoint>) -> Value {
    match e {
        None => Value::Null,
        Some(Ok(())) => json!("joined"),
        Some(Err(why)) => json!(format!("{why:?}").to_lowercase()),
    }
}

/// `Assign` for an admitted candidate, `refused:Slice` for a refused one.
fn tag<T: std::fmt::Debug, E: std::fmt::Debug>(r: Result<T, E>) -> String {
    match r {
        Ok(v) => format!("{v:?}"),
        Err(e) => format!("refused:{e:?}"),
    }
}

fn path(p: &StaticPath, brackets: bool) -> String {
    let fields: String = p.fields.iter().map(|f| format!(".{f}")).collect();
    let root = p.root.as_deref().unwrap_or("");
    let tail = if brackets {
        "[]".repeat(p.brackets)
    } else {
        String::new()
    };
    format!("{root}{fields}{tail}")
}

fn bytes(s: Span) -> Value {
    json!([s.start, s.end])
}

/// The owner-level ledger: RD exit, raw work, sequential attrition and the limit outcomes.
fn ledger(rec: &mut Value, pv: &OwnerPreview) {
    let a = pv.attrition;
    rec["rd"] = json!(pv.rd.name());
    rec["rd_early"] = json!(pv.rd.early);
    rec["raw_pairs"] = json!(pv.work.map(|w| w.total));
    rec["raw_overflow"] = json!(pv.work.is_some_and(|w| w.overflow));
    rec["raw_capped"] = json!(pv.raw_capped);
    rec["attrition"] = json!({"admitted": a.admitted, "joined": a.joined, "bound": a.bound,
        "routed": a.routed, "unoccupied": a.unoccupied, "distinct": a.distinct,
        "after_fanin": a.after_fanin, "published": a.published});
    rec["reads_over_fanin"] = json!(pv.fanin_reads);
    rec["omitted_fanin_pairs"] = json!(pv.fanin_pairs);
    rec["over_edge_cap"] = json!(pv.over_edge_cap);
    rec["omitted_cap_pairs"] = json!(pv.cap_pairs);
    rec["omitted_pair_records"] = json!(pv.omitted_pairs);
}

impl FileTrace {
    /// `Some` only for a Python file while the trace is on.
    pub(crate) fn begin(file: &str, parsed: &ParsedFile) -> Option<Self> {
        let on = parsed.language == Language::Python && (enabled() || captured(file));
        on.then(|| FileTrace {
            owners: Vec::new(),
            scans: python::scan_file(parsed),
            rd: BTreeMap::new(),
        })
    }

    /// Record how RD treated one owner (`exit`: the unavailability and whether the owner was
    /// skipped before the RD call, `None` when available). Read-only: nothing flows back.
    pub(crate) fn rd(
        &mut self,
        parsed: &ParsedFile,
        func: &tree_sitter::Node<'_>,
        defs: &[DefSite],
        edges: &[FlowEdge],
        exit: Option<(RdUnavailable, bool)>,
    ) {
        let (Some(name), line) = (parsed.dfg_owner_name(func), parsed.node_line_range(func).0)
        else {
            return;
        };
        let scan = self
            .scans
            .iter()
            .find(|s| s.name == name && s.span.line == line);
        let facts = match (exit, scan) {
            (_, None) => return,
            (None, Some(scan)) => RdFacts::available(parsed, func, scan, defs, edges),
            (Some((why, early)), _) => {
                let exit = match why {
                    RdUnavailable::DefinitionsCapExceeded { .. } => RdExit::DefCap,
                    RdUnavailable::StatementLinesCapExceeded { .. } => RdExit::LineCap,
                    RdUnavailable::NoCfgEdges => RdExit::NoCfg,
                };
                RdFacts::unavailable(Rd { exit, early })
            }
        };
        self.rd.entry((name, line)).or_insert(facts);
    }

    /// Record the ordinary occurrences one owner pass produced, before materialization.
    pub(crate) fn owner(
        &mut self,
        name: &str,
        start_line: usize,
        defs: &[PathSpan],
        uses: &[PathSpan],
    ) {
        self.owners.push(Originals {
            name: name.to_string(),
            start_line,
            defs: occurrences(defs),
            uses: occurrences(uses),
        });
    }

    /// Join the inventory to the final buckets and write the records.
    pub(crate) fn finish(self, file: &str, parsed: &ParsedFile, done: Final) {
        let lines = self.records(file, parsed, &done);
        #[cfg(test)]
        if let Some(sink) = CAPTURE
            .lock()
            .unwrap()
            .as_mut()
            .and_then(|m| m.get_mut(file))
        {
            sink.extend(lines);
            return;
        }
        if lines.is_empty() {
            return;
        }
        let mut out = lines.join("\n");
        out.push('\n');
        let _ = std::io::stderr().lock().write_all(out.as_bytes());
    }

    fn records(&self, file: &str, parsed: &ParsedFile, done: &Final) -> Vec<String> {
        let (scans, Final { defs, uses, .. }) = (&self.scans, done);
        let pairs = done.labels.keys().map(|(f, t)| (f, t));
        let exact: Vec<_> = done
            .exact
            .keys()
            .map(|e| (e.from.as_legacy(), e.to.as_legacy()))
            .collect();
        let legacy = done.edges.iter().map(|e| (&e.from, &e.to));
        let occupied = Occupied::new(legacy.chain(pairs).chain(exact.iter().map(|(f, t)| (f, t))));
        let mut out = Vec::new();
        for scan in scans.iter().filter(|s| !s.writes.is_empty()) {
            let key = |o: &Originals| o.name == scan.name && o.start_line == scan.span.line;
            let count = scans
                .iter()
                .filter(|s| s.name == scan.name && s.span.line == scan.span.line);
            let joined = join_owner(
                file,
                scan,
                count.count(),
                self.owners.iter().find(|o| key(o)),
                defs,
                uses,
            );
            let pv = preview(&Inputs {
                file,
                parsed,
                scan,
                join: &joined,
                rd: self.rd.get(&(scan.name.clone(), scan.span.line)),
                defs,
                uses,
                occupied: &occupied,
                limits: limits(file),
            });
            let at = |kind: &str| json!({"kind": kind, "file": file, "owner": scan.name, "owner_line": scan.span.line});
            let mut rec = at("owner");
            ledger(&mut rec, &pv);
            rec["span"] = bytes(scan.span);
            rec["writes"] = json!(scan.writes.len());
            rec["nodes"] = json!(scan.operands.len());
            rec["pairs"] = pv
                .pairs
                .iter()
                .map(|p| {
                    let (w, r) = (&scan.writes[p.w], &scan.operands[p.r]);
                    json!({"w": bytes(w.span), "r": bytes(r.span), "role": tag(r.verdict), "join": verdict(Some(joined.pair(p.w, p.r)))})
                })
                .collect();
            out.push(rec);
            for (w, j) in scan.writes.iter().zip(&joined.writes) {
                let mut rec = at("write");
                rec["bytes"] = bytes(w.span);
                rec["line"] = json!(w.span.line);
                rec["path"] = json!(path(&w.path, true));
                rec["binding"] = json!(w.path.root);
                rec["write"] = json!(tag(w.verdict));
                rec["join"] = verdict(*j);
                out.push(rec);
            }
            for (o, j) in scan.operands.iter().zip(&joined.operands) {
                let mut rec = at("node");
                rec["bytes"] = bytes(o.span);
                rec["outer"] = bytes(o.outer);
                rec["line"] = json!(o.span.line);
                rec["path"] = json!(path(&o.path, false));
                rec["binding"] = json!(o.path.root);
                rec["role"] = json!(tag(o.verdict));
                rec["join"] = verdict(*j);
                out.push(rec);
            }
            for p in &pv.pairs {
                let (w, r) = (&scan.writes[p.w], &scan.operands[p.r]);
                let mut rec = at("prediction");
                rec["w"] = bytes(w.span);
                rec["r"] = bytes(r.span);
                rec["role"] = json!(tag(r.verdict));
                rec["binding"] = json!(w.path.root);
                rec["w_path"] = json!(path(&w.path, true));
                rec["r_path"] = json!(path(&r.path, false));
                match &p.outcome {
                    Ok(steps) => {
                        let phases: Vec<_> =
                            steps.iter().map(|s| format!("{:?}", s.phase)).collect();
                        rec["outcome"] = json!("predicted");
                        rec["route"] = json!(phases.join(">"));
                    }
                    Err(why) => {
                        rec["outcome"] = json!(format!("refused:{}", why.name()));
                        if let super::preview::Refusal::Route(f) = why {
                            let unproven: Vec<_> =
                                f.unproven.iter().map(|(k, _)| format!("{k:?}")).collect();
                            rec["flags"] = json!({"reentry": f.reentry_cut, "killed": f.killed, "unproven": unproven});
                        }
                    }
                }
                out.push(rec);
            }
        }
        out.into_iter().map(|r| format!("{PREFIX}{r}")).collect()
    }
}
