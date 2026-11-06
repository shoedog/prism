//! #660 S1b-1: the JS/TS/TSX write census as reaching-definitions input (DESIGN.md 5.2, 5.3, 8).
//!
//! `complete` appends one kill-only def per census write the producer did not record (A7: no recorded
//! def with the site's target start byte and stored path), for bases the owner defines (E16), at the
//! statement node E18 maps the write to. A kill-only def gens and kills (`affects`, A2) but is never
//! an edge's source, never killed and never part of a collapse group (E1). An owner whose census is
//! refused (escape, parse, a refused target kind) labels every edge `NameOnly(CfgIncomplete)` and
//! gets no kill-only defs, so a refusal never moves availability; a base with an unmappable site is
//! refused the same way for its own edges. Every other language is returned unchanged.
use super::{DefId, DefSite, FlowConfidence, FlowDoubt};
use crate::ast::ParsedFile;
use crate::data_flow::FlowEdge;
use crate::js_write_census::completion::{completion, fingerprint, Completion};
use std::collections::BTreeSet;
use std::sync::Arc;
use tree_sitter::Node;

/// Why an edge's label was decided by the census.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Refused {
    /// The whole owner (`owner-refuse:escape|parse|target:<kind>`).
    Owner(String),
    /// The edge's base has an eager site that maps to no CFG node.
    UnmappedSite,
}

/// What the census decided for one owner: where the recorded defs end and what it refuses.
#[derive(Clone, Default)]
pub(super) struct CensusFacts {
    /// Defs `..n_recorded` are the producer's; the rest are kill-only.
    pub(super) n_recorded: usize,
    completion: Option<Arc<Completion>>,
    /// Bases refused at solve time: a kill-only def whose line is no CFG statement.
    unmapped: BTreeSet<String>,
}

impl CensusFacts {
    fn inert(n_recorded: usize) -> Self {
        Self {
            n_recorded,
            ..Self::default()
        }
    }

    /// The census changes nothing for this owner (no refusal, no kill-only def, no refused base).
    pub(super) fn is_inert(&self) -> bool {
        self.unmapped.is_empty()
            && self.completion.as_ref().is_none_or(|c| {
                c.refusal.is_none() && c.kill_only.is_empty() && c.refused_bases.is_empty()
            })
    }

    /// Record that the kill-only def at `index` maps to no statement node: its base is refused.
    pub(super) fn refuse_unmapped(&mut self, base: &str) {
        self.unmapped.insert(base.to_string());
    }

    /// Owner refusal, then refused base (DESIGN.md 8, `classify_edge` steps 1-2).
    pub(super) fn refuses(&self, edge: &FlowEdge) -> Option<Refused> {
        let c = self.completion.as_deref();
        if let Some(r) = c.and_then(|c| c.refusal.as_ref()) {
            return Some(Refused::Owner(r.label()));
        }
        let hit = |base: &str| {
            self.unmapped.contains(base) || c.is_some_and(|c| c.refused_bases.contains(base))
        };
        (hit(&edge.from.path.base) || hit(&edge.to.path.base)).then_some(Refused::UnmappedSite)
    }

    /// The label a refusal gives every edge it covers.
    pub(super) fn refused_label() -> FlowConfidence {
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)
    }

    /// Trace mechanism of the edge's moved label: a refusal, else `kill:<form>` of the kill-only def
    /// that kills at the label's kill line (any kill-only def affecting the edge's path otherwise).
    pub(super) fn mechanism(&self, edge: &FlowEdge, label: FlowConfidence) -> String {
        match self.refuses(edge) {
            Some(Refused::Owner(m)) => return m,
            Some(Refused::UnmappedSite) => return "site-unmapped".to_string(),
            None => {}
        }
        let Some(c) = self.completion.as_deref() else {
            return "kill:other".to_string();
        };
        let kill_line = match label {
            FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line }) => Some(kill_line as usize),
            _ => None,
        };
        let affecting = |k: &&crate::js_write_census::completion::KillOnly| {
            super::def_buckets::affects(&k.path, &edge.from.path)
        };
        let form = c
            .kill_only
            .iter()
            .filter(affecting)
            .find(|k| Some(k.line) == kill_line)
            .or_else(|| c.kill_only.iter().find(affecting))
            .map_or("other", |k| k.form);
        format!("kill:{form}")
    }
}

/// The recorded defs followed by the owner's kill-only defs, and the census facts that name them.
/// `recorded` is the deduplicated producer list; non-JS owners and owners with no recorded def
/// (no edge to label) come back unchanged.
pub(super) fn complete_for(
    enabled: bool,
    parsed: &ParsedFile,
    owner: &Node<'_>,
    recorded: Vec<DefSite>,
) -> (Vec<DefSite>, CensusFacts) {
    if enabled {
        complete(parsed, owner, recorded)
    } else {
        let n = recorded.len();
        (recorded, CensusFacts::inert(n))
    }
}

pub(super) fn complete(
    parsed: &ParsedFile,
    owner: &Node<'_>,
    mut recorded: Vec<DefSite>,
) -> (Vec<DefSite>, CensusFacts) {
    let n_recorded = recorded.len();
    if n_recorded == 0 || !parsed.language.is_js_family() {
        return (recorded, CensusFacts::inert(n_recorded));
    }
    let pairs: Vec<_> = recorded.iter().map(|d| (&d.path, d.start_byte)).collect();
    let print = fingerprint(pairs.iter().copied());
    let key = (owner.start_byte(), owner.end_byte());
    let done = parsed
        .js_census
        .get_or_compute(key, print, || completion(parsed, *owner, &pairs, print));
    drop(pairs);
    let mut next = recorded.iter().map(|d| d.id.0).max().map_or(0, |m| m + 1);
    for k in &done.kill_only {
        recorded.push(DefSite {
            id: DefId(next),
            path: k.path.clone(),
            line: k.line,
            start_byte: k.start_byte,
            alias_derived: false,
        });
        next += 1;
    }
    let facts = CensusFacts {
        n_recorded,
        completion: Some(done),
        unmapped: BTreeSet::new(),
    };
    (recorded, facts)
}
