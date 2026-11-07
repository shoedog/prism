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
use crate::js_write_census::completion::{completion, Completion};
use std::collections::BTreeSet;
use std::rc::Rc;
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
    completion: Option<Completion>,
    /// A refusal the census decided on availability grounds (`owner-refuse:one-node|def-cap`).
    forced: Option<&'static str>,
    /// Bases refused at solve time: a kill-only def whose line is no CFG statement.
    unmapped: BTreeSet<String>,
    /// #660 S1b-2: the same-node order refusal; shared by clones.
    pub(super) d3: Option<Rc<super::d3::D3>>,
}

impl CensusFacts {
    fn inert(n_recorded: usize) -> Self {
        Self {
            n_recorded,
            ..Self::default()
        }
    }

    /// The census changes nothing for this owner (no refusal, no kill-only def, no refused base, no
    /// D3 state to refuse an edge with).
    pub(super) fn is_inert(&self) -> bool {
        self.d3.is_none()
            && self.unmapped.is_empty()
            && self.forced.is_none()
            && self.completion.as_ref().is_none_or(|c| {
                c.refusal.is_none() && c.kill_only.is_empty() && c.refused_bases.is_empty()
            })
    }

    /// The def cap on the recorded list only: a completed list that overflows it is cut back to the
    /// recorded defs and the owner refused by label (`owner-refuse:def-cap`).
    pub(super) fn cap_defs(&mut self, defs: &mut Vec<DefSite>, cap: usize) {
        if defs.len() > cap {
            // An owner already over the cap on its recorded defs reports the recorded count, as main.
            defs.truncate(self.n_recorded);
            if self.n_recorded <= cap {
                self.forced = Some("owner-refuse:def-cap");
            }
        }
    }

    /// The one-node distinct-bases belt on the recorded bases, as main runs it (`false`: refuse
    /// availability). When only the completed bases fail it, the list is cut back to the recorded defs
    /// and the owner refused by label (`owner-refuse:one-node`).
    pub(super) fn one_node_admits(
        &mut self,
        defs: &mut Vec<DefSite>,
        admits: impl Fn(&[&str]) -> bool,
    ) -> bool {
        fn bases(d: &[DefSite]) -> Vec<&str> {
            d.iter().map(|x| x.path.base.as_str()).collect()
        }
        if !admits(&bases(&defs[..self.n_recorded])) {
            return false;
        }
        if defs.len() > self.n_recorded && !admits(&bases(defs)) {
            defs.truncate(self.n_recorded);
            self.forced = Some("owner-refuse:one-node");
        }
        true
    }

    /// A kill-only def at a line that is no CFG statement kills nowhere: refuse its base (E18).
    pub(super) fn refuse_unmapped(&mut self, defs: &[DefSite], mapped: &[Option<usize>]) {
        for (def, m) in defs[self.n_recorded..]
            .iter()
            .zip(&mapped[self.n_recorded..])
        {
            if m.is_none() {
                self.unmapped.insert(def.path.base.clone());
            }
        }
    }

    /// Owner refusal, then refused base (DESIGN.md 8, `classify_edge` steps 1-2).
    pub(super) fn refuses(&self, edge: &FlowEdge) -> Option<Refused> {
        let c = self.completion.as_ref();
        if let Some(l) = self.forced {
            return Some(Refused::Owner(l.to_string()));
        }
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
        if label == FlowConfidence::NameOnly(FlowDoubt::SameLine) {
            if let Some(m) = self.d3.as_ref().and_then(|d| d.logged(edge)) {
                return m.to_string();
            }
        }
        let Some(c) = self.completion.as_ref() else {
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
    log: bool,
    parsed: &ParsedFile,
    owner: &Node<'_>,
    recorded: Vec<DefSite>,
) -> (Vec<DefSite>, CensusFacts) {
    if enabled {
        complete_with(parsed, owner, recorded, log)
    } else {
        let n = recorded.len();
        (recorded, CensusFacts::inert(n))
    }
}

#[cfg(test)]
pub(super) fn complete(
    parsed: &ParsedFile,
    owner: &Node<'_>,
    recorded: Vec<DefSite>,
) -> (Vec<DefSite>, CensusFacts) {
    complete_with(parsed, owner, recorded, false)
}

/// `complete`, with D3 recording its refusals' mechanisms when `log` (the trace).
pub(super) fn complete_with(
    parsed: &ParsedFile,
    owner: &Node<'_>,
    mut recorded: Vec<DefSite>,
    log: bool,
) -> (Vec<DefSite>, CensusFacts) {
    let n_recorded = recorded.len();
    if n_recorded == 0 || !parsed.language.is_js_family() {
        return (recorded, CensusFacts::inert(n_recorded));
    }
    let pairs: Vec<_> = recorded
        .iter()
        .map(|d| (&d.path, d.start_byte, d.alias_derived))
        .collect();
    let mut done = completion(parsed, *owner, &pairs);
    drop(pairs);
    let d3 = super::d3::D3::new(std::mem::take(&mut done.counting), owner, log).map(Rc::new);
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
        forced: None,
        unmapped: BTreeSet::new(),
        d3,
    };
    (recorded, facts)
}
