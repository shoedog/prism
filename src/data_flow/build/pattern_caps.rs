//! Owner-local S1 additions, committed only after selecting the producer pass.

use super::pattern_formals::{PatternFormal, Selected};
use super::Producer;
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::cpg::{DefId, DefSite};
use crate::data_flow::s1_evidence::{
    CapDefinition, CapKind, CapPair, CapWithholding, FileEvidence,
};
use crate::data_flow::VarLocation;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

type Definitions = BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>>;

#[derive(Default)]
pub(super) struct Increment<'a> {
    entries: Vec<(&'a VarLocation, usize)>,
    rd_positions: BTreeSet<usize>,
}

impl<'a> Increment<'a> {
    pub(super) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(super) fn add_entry(
        &mut self,
        loc: &'a VarLocation,
        rd: &mut Vec<DefSite>,
        defs: &Definitions,
    ) {
        self.entries
            .push((loc, defs.get(&key(loc)).map_or(0, Vec::len)));
        self.rd_positions.insert(rd.len());
        Producer::push_rd_def(rd, loc, false);
    }

    /// The solver completes the definition list with census-only defs (and an S1 entry def can make
    /// a write census-eligible): whether this list alone overflows the cap there, which forces every
    /// edge of the owner to `cfg_incomplete`. `applies` is false when the owner is unavailable anyway.
    pub(super) fn census_overflow(
        &self,
        parsed: &ParsedFile,
        owner: &Node<'_>,
        defs: &[DefSite],
        applies: bool,
    ) -> bool {
        applies && !self.is_empty() && crate::cpg::census_def_cap_overflow(parsed, owner, defs)
    }

    pub(super) fn add_rd_only(&mut self, position: usize) {
        self.rd_positions.insert(position);
    }

    pub(super) fn added_rd(&self, position: usize) -> bool {
        self.rd_positions.contains(&position)
    }

    pub(super) fn legacy_rd(&self, rd: &[DefSite]) -> Vec<DefSite> {
        rd.iter()
            .enumerate()
            .filter(|(index, _)| !self.added_rd(*index))
            .enumerate()
            .map(|(index, (_, def))| DefSite {
                id: DefId(index.try_into().expect("existing definition index")),
                ..def.clone()
            })
            .collect()
    }

    pub(super) fn commit(&self, defs: &mut Definitions) {
        for &(loc, offset) in &self.entries {
            defs.entry(key(loc))
                .or_default()
                .insert(offset, loc.clone());
        }
    }

    pub(super) fn report_withheld(
        &self,
        owner: Node<'_>,
        kind: CapKind,
        endpoints: &[(&PatternFormal, Selected<'_>)],
        indices: &[usize],
        legacy_capped: bool,
        evidence: &mut FileEvidence,
    ) {
        let pairs = indices
            .iter()
            .map(|&index| {
                let (formal, selected) = &endpoints[index];
                selected.withhold(formal, owner, evidence);
                CapPair {
                    source: definition(&formal.def),
                    target: definition(&selected.loc),
                }
            })
            .collect();
        if !(legacy_capped && kind == CapKind::Pairs) {
            evidence
                .cap_withholdings
                .push(self.withholding(owner, kind, pairs));
        }
    }

    fn withholding(&self, owner: Node<'_>, kind: CapKind, pairs: Vec<CapPair>) -> CapWithholding {
        CapWithholding {
            owner: [owner.start_byte(), owner.end_byte()],
            kind,
            definitions: self
                .entries
                .iter()
                .map(|(loc, _)| definition(loc))
                .collect(),
            pairs,
        }
    }
}

fn definition(loc: &VarLocation) -> CapDefinition {
    CapDefinition {
        bytes: [loc.start_byte, loc.end_byte],
        path: loc.path.clone(),
    }
}

fn key(loc: &VarLocation) -> (String, String, usize, AccessPath) {
    (
        loc.file.clone(),
        loc.function.clone(),
        loc.function_start_line,
        loc.path.clone(),
    )
}
