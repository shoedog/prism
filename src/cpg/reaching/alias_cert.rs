//! #783 option A, stage 2: the certified single-step alias relations `u -> o` of one JS/TS/TSX owner.
//!
//! Stage 1 (`js_write_census::alias_cert`) admits a `const|let|var u = o` declarator syntactically from the
//! census inventory. This stage adds the positive declaration identity, which needs `BindingFacts`: `u`'s
//! recorded def binds an explicit declaration, `o`'s one declaration site has a recorded def that binds an
//! explicit declaration, and `o` at the declarator resolves to that same declaration. `proven_distinct_at ==
//! false` and the flat fallback are not identity: either refuses. Alias twins are never consulted.
//!
//! Slice 4a read them through the trace (`rd-alias-cert`) only; slice 4b's refinement (`alias_kill`) consumes
//! the admitted ones.
//!
//! **A certificate is a fact about the relation, not about a write.** A consumer (slice 4b's refinement) must
//! still check, for each write W and each candidate def it classifies:
//! - the base written at W binds `alias_decl` (`BindingFacts::binding_at` at W's probe byte);
//! - the alias's initialisation `alias_def` reaches W, and W is not in the declarator's statement or CFG node;
//! - W is an eager, precisely identified census write (a counting write, not a deferred or own-parameter one);
//! - the candidate def binds `target_decl`.
use crate::ast::ParsedFile;
use crate::cpg::binding_facts::{BindingFacts, BindingId};
use crate::cpg::reaching::DefSite;
use crate::js_write_census::alias_cert::{Candidate, Reason};
use std::io::Write;
use std::rc::Rc;
use tree_sitter::Node;

/// The verdict on one candidate: the identities stage 2 proved, or the reason it was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Verdict {
    Admitted {
        /// RD def index of `u`'s declarator and the explicit declaration it binds.
        alias_def: usize,
        alias_decl: BindingId,
        /// RD def index of `o`'s one declaration and the explicit declaration `o` at the alias resolves to.
        target_def: usize,
        target_decl: BindingId,
    },
    Refused(Reason),
}

/// One candidate with its verdict.
#[derive(Debug)]
pub(super) struct Entry {
    pub cand: Candidate,
    pub verdict: Verdict,
}

/// The owner's candidates, admitted or refused.
#[derive(Debug)]
pub(super) struct AliasCerts {
    pub entries: Vec<Entry>,
}

/// `name@line:byte`, the trace's place.
pub(super) fn at(parsed: &ParsedFile, name: &str, byte: usize) -> String {
    format!("{name}@{}:{byte}", parsed.line_for_byte(byte))
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
                let verdict = match cand.refusal {
                    Some(r) => Verdict::Refused(r),
                    None => identity(cand, recorded, facts),
                };
                Entry {
                    cand: cand.clone(),
                    verdict,
                }
            })
            .collect();
        Some(Rc::new(Self { entries }))
    }

    /// The admitted relation for the alias spelled `name` at `probe`: the occurrence follows the declarator
    /// and binds the alias's own declaration (an outer or shadowing `name` does not). Returns its entry index.
    pub(super) fn alias_at(&self, facts: &BindingFacts, name: &str, probe: usize) -> Option<usize> {
        self.entries.iter().position(|e| {
            matches!(&e.verdict, Verdict::Admitted { alias_decl, .. }
                if e.cand.alias == name
                    && probe >= e.cand.alias_end
                    && facts.binding_at(name, probe) == *alias_decl)
        })
    }

    pub(super) fn any_admitted(&self) -> bool {
        self.entries
            .iter()
            .any(|e| matches!(e.verdict, Verdict::Admitted { .. }))
    }

    /// `rd-alias-cert\t<admitted|refused>\t<reason|->\t<file>\t<owner>\t<alias def>\t<source decl>\t<target decl>`,
    /// each place `name@line:byte`. The alias def is the declarator's `u`; for a body alias the source
    /// declaration (the declaration the alias spelling binds) is that same name; the target declaration is
    /// `o`'s one declaration (`-` when it has none).
    pub(super) fn trace(&self, out: &mut impl Write, parsed: &ParsedFile, owner: &Node<'_>) {
        let place = |name: &str, byte: usize| at(parsed, name, byte);
        let name = parsed.dfg_owner_name(owner).unwrap_or_default();
        for e in &self.entries {
            let c = &e.cand;
            let (verdict, reason) = match e.verdict {
                Verdict::Admitted { .. } => ("admitted", "-"),
                Verdict::Refused(r) => ("refused", r.as_str()),
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

/// Stage 2: the identities of a syntactically admitted candidate, or the refusal.
fn identity(c: &Candidate, recorded: &[DefSite], facts: &BindingFacts) -> Verdict {
    let Some(alias_def) = def_at(recorded, &c.alias, c.alias_start).filter(|&d| explicit(facts, d))
    else {
        return Verdict::Refused(Reason::AliasIdentity);
    };
    let target = c
        .target_start
        .and_then(|b| def_at(recorded, &c.target, b))
        .filter(|&d| explicit(facts, d))
        .map(|d| (d, facts.binding_of_def(d).clone()))
        .filter(|(_, decl)| facts.binding_at(&c.target, c.rhs_start) == *decl);
    match target {
        Some((target_def, target_decl)) => Verdict::Admitted {
            alias_def,
            alias_decl: facts.binding_of_def(alias_def).clone(),
            target_def,
            target_decl,
        },
        None => Verdict::Refused(Reason::TargetIdentity),
    }
}
