//! Optional, occurrence-bound guard proofs for retained receiver selections.
use super::{
    children, field, identifier, is_loop, is_owner, is_scope,
    name_reads::{Analysis, Origin},
    operator, pattern_parts, peel, AstSpan, GuardForm, GuardState,
};
use crate::ast::ParsedFile;
use std::collections::BTreeSet;
use tree_sitter::Node;

mod bindings;
mod finite;
mod flow;
mod names;
mod natives;
mod own;
mod receiver;
mod sequences;
use bindings::Bindings;
pub(super) use natives::NativeWrites;

#[cfg(test)]
#[path = "guard_binding_tests.rs"]
mod guard_binding_tests;

#[derive(Clone, Copy)]
pub(super) struct Selection<'a> {
    pub(super) object: Node<'a>,
    pub(super) key: Node<'a>,
    pub(super) effect: Node<'a>,
    pub(super) nested: bool,
    pub(super) shared_link: bool,
}

pub(super) struct GuardInput<'a> {
    pub(super) owner: Node<'a>,
    pub(super) target: Node<'a>,
    pub(super) store: Node<'a>,
    pub(super) receiver: Node<'a>,
    pub(super) steps: Vec<Selection<'a>>,
}

pub(super) struct GuardDecision {
    pub(super) state: GuardState,
    pub(super) forms: Vec<GuardForm>,
    pub(super) refusals: BTreeSet<(usize, &'static str)>,
}

struct Proof<'a, 'b, F> {
    file: &'a ParsedFile,
    bindings: Bindings<'a, 'b>,
    natives: &'b NativeWrites,
    charge: F,
    exhausted: bool,
    refusals: BTreeSet<(usize, &'static str)>,
}

pub(super) fn classify<'a>(
    file: &'a ParsedFile,
    analysis: &Analysis<'a>,
    input: &GuardInput<'a>,
    natives: &NativeWrites,
    charge: impl FnMut(usize) -> bool,
) -> GuardDecision {
    let mut proof = Proof {
        file,
        bindings: Bindings::new(analysis),
        natives,
        charge,
        exhausted: false,
        refusals: BTreeSet::new(),
    };
    let (mut state, name_steps) = proof.name_guard(input);
    let mut forms = Vec::new();
    if state == GuardState::Sufficient {
        forms.push(GuardForm::DenyNames);
    }
    if state != GuardState::Sufficient {
        let (finite, admitted) = proof.finite_guard(input, &name_steps);
        state = state.max(finite);
        forms.extend(admitted);
    }
    if state != GuardState::Sufficient {
        let (own, admitted) = proof.own_guard(input);
        state = state.max(own);
        forms.extend(admitted);
    }
    if state != GuardState::Sufficient {
        let (receiver, admitted) = proof.receiver_guard(input);
        state = state.max(receiver);
        forms.extend(admitted);
    }
    if state == GuardState::None && !proof.exhausted {
        proof.report_misplaced_checks(input);
    }
    let (state, forms) = if proof.exhausted {
        proof.refuse(input.target, "budget");
        (GuardState::None, Vec::new())
    } else {
        (state, forms)
    };
    GuardDecision {
        state,
        forms,
        refusals: proof.refusals,
    }
}

impl<'a, F: FnMut(usize) -> bool> Proof<'a, '_, F> {
    fn spend(&mut self, cost: usize) -> bool {
        if !self.exhausted {
            self.exhausted = !(self.charge)(cost);
        }
        !self.exhausted
    }

    fn refuse(&mut self, candidate: Node<'a>, reason: &'static str) {
        self.refusals.insert((candidate.id(), reason));
    }

    fn text(&self, node: Node<'a>) -> &'a str {
        self.file.node_text(&node)
    }

    fn string(&self, node: Node<'a>) -> Option<&'a str> {
        let node = peel(node);
        let text = self.text(node);
        (node.kind() == "string" && !text.contains('\\'))
            .then(|| text.get(1..text.len().saturating_sub(1)))
            .flatten()
    }

    fn native(&mut self, site: Node<'a>, binding: Node<'a>, path: &[&str]) -> bool {
        let name = self.bindings.canonical(self.text(binding));
        if name != path[0] {
            self.refuse(site, "unproved_native_identity");
            return false;
        }
        if self.bindings.is_shadowed(binding) {
            self.refuse(site, "intrinsic_rebound");
            return false;
        }
        self.unwritten(site, path)
    }

    fn unwritten(&mut self, site: Node<'a>, path: &[&str]) -> bool {
        if self.natives.exhausted {
            self.exhausted = true;
            return false;
        }
        if self.natives.written(path) {
            self.refuse(site, "intrinsic_rebound");
            return false;
        }
        true
    }
}
