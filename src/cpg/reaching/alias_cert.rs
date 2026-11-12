//! #783 option A, stage 2: the certified single-step alias relations `u -> o` of one JS/TS/TSX owner.
//!
//! Stage 1 (`js_write_census::alias_cert`) admits a `const|let|var u = o` declarator syntactically from the
//! census inventory. This stage adds the positive declaration identity, which needs `BindingFacts`: `u`'s
//! recorded def binds an explicit declaration, `o`'s one declaration site has a recorded def that binds an
//! explicit declaration, and `o` at the declarator resolves to that same declaration. `proven_distinct_at ==
//! false` and the flat fallback are not identity: either refuses. Alias twins are never consulted.
//!
//! Inert (slice 4a): the certificates are read by the trace (`rd-alias-cert`) and the unit tests only.
use crate::ast::ParsedFile;
use crate::cpg::binding_facts::{BindingFacts, BindingId};
use crate::cpg::reaching::DefSite;
use crate::js_write_census::alias_cert::{Candidate, Reason};
use std::io::Write;
use std::rc::Rc;
use tree_sitter::Node;

/// One candidate with its final verdict.
#[derive(Debug)]
pub(super) struct Entry {
    pub cand: Candidate,
    pub refusal: Option<Reason>,
}

/// The owner's candidates, admitted or refused.
#[derive(Debug)]
pub(super) struct AliasCerts {
    pub entries: Vec<Entry>,
}

fn def_at(defs: &[DefSite], base: &str, byte: usize) -> Option<usize> {
    defs.iter().position(|d| {
        d.start_byte == byte && d.path.base == base && d.path.fields.is_empty() && !d.alias_derived
    })
}

fn explicit(facts: &BindingFacts, def: usize) -> bool {
    matches!(facts.binding_of_def(def), BindingId::Declaration(_))
}

impl AliasCerts {
    /// The verdicts for `candidates`, `None` when the owner has none. `recorded` are the producer's defs
    /// (the first `n_recorded` of the RD list, the ones `facts` knows as declarations).
    pub(super) fn certify(
        candidates: &[Candidate],
        recorded: &[DefSite],
        facts: &BindingFacts,
    ) -> Option<Rc<Self>> {
        if candidates.is_empty() {
            return None;
        }
        let entries = candidates
            .iter()
            .map(|cand| {
                let refusal = cand.refusal.or_else(|| identity(cand, recorded, facts));
                Entry {
                    cand: cand.clone(),
                    refusal,
                }
            })
            .collect();
        Some(Rc::new(Self { entries }))
    }

    /// `rd-alias-cert\t<admitted|refused>\t<reason|->\t<file>\t<owner>\t<alias def>\t<source decl>\t<target decl>`,
    /// each place `name@line:byte`. The alias def is the declarator's `u`; for a body alias the source
    /// declaration (the declaration the alias spelling binds) is that same name; the target declaration is
    /// `o`'s one declaration (`-` when it has none).
    pub(super) fn trace(&self, out: &mut impl Write, parsed: &ParsedFile, owner: &Node<'_>) {
        let place =
            |name: &str, byte: usize| format!("{name}@{}:{byte}", parsed.line_for_byte(byte));
        let name = parsed.dfg_owner_name(owner).unwrap_or_default();
        for e in &self.entries {
            let c = &e.cand;
            let (verdict, reason) = match e.refusal {
                None => ("admitted", "-"),
                Some(r) => ("refused", r.as_str()),
            };
            let alias = place(&c.alias, c.alias_start);
            let target = c
                .target_start
                .map_or("-".to_string(), |b| place(&c.target, b));
            let _ = writeln!(
                out,
                "rd-alias-cert\t{verdict}\t{reason}\t{}\t{name}\t{alias}\t{alias}\t{target}",
                parsed.path
            );
        }
    }
}

/// Stage 2's refusal of a syntactically admitted candidate.
fn identity(c: &Candidate, recorded: &[DefSite], facts: &BindingFacts) -> Option<Reason> {
    let alias = def_at(recorded, &c.alias, c.alias_start).filter(|&d| explicit(facts, d));
    if alias.is_none() {
        return Some(Reason::AliasIdentity);
    }
    let same = c
        .target_start
        .and_then(|b| def_at(recorded, &c.target, b))
        .filter(|&d| explicit(facts, d))
        .is_some_and(|d| facts.binding_at(&c.target, c.rhs_start) == *facts.binding_of_def(d));
    (!same).then_some(Reason::TargetIdentity)
}
