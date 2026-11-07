//! The writes D3 counts, per CFG node (#660 S1b-2, DESIGN.md 7.1 "Writes that count").
//!
//! One entry per census site that executes eagerly in the owner (recorded or kill-only: the walk
//! lists each target once), for the bases the owner records a def for, with its line and its S0
//! `Position`. Which CFG node a write is in is RD's answer for its line (#696), never this module's:
//! E18's `place` only decides whether the write has a position at all, and orders it inside a node. A write in a deferred region
//! is D2-c's and is not ordered here; a declaration that stores nothing (`var a;`) is not a write.
use super::order::{operand_exception, Position};
use super::walk::{Form, Site};
use crate::access_path::AccessPath;
use std::collections::BTreeSet;

/// A write D3 counts in its node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Counting {
    pub path: AccessPath,
    /// Line of the write's target: RD maps it to the write's CFG node (#696), the only authority.
    pub line: usize,
    pub target_start: usize,
    pub form: Form,
    /// The byte at which the write's binding is looked up (`Counts::push`).
    pub probe: usize,
    pub has_construct: bool,
    pub pos: Position,
}

impl Counting {
    /// This write is the use's own operand read of a `RW` or `delete` construct.
    pub(crate) fn is_operand_of(&self, use_start: usize) -> bool {
        operand_exception(self.form, self.has_construct, self.target_start, use_start)
    }
}

/// Whether a site is a write D3 orders: eager, and storing a value at its position. A declaration
/// counts only as a `let`/`const` declarator (`let a;` stores `undefined`), a `var` declarator with a value, or a `for` head's: a hoisted
/// function, a parameter, an import or a class name is placed by scope, not by source order (S1d).
pub(crate) fn counts(site: &Site) -> bool {
    let r = site.region;
    !(r.nested || r.instance_field || r.own_params)
        && (site.form != Form::Declare || (site.initialized && site.decl == "declarator"))
}

/// The counting writes of one owner, and what places their reads.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Counts {
    pub writes: Vec<Counting>,
    /// The owner's statement spans (the mapping `Position::of_read` uses).
    pub spans: Vec<(usize, usize)>,
    /// Byte ranges of the targets that are no read (plain writes, declarations, `delete`): a read
    /// occurrence of the same spelling at one of them is the write's own target, not a use.
    pub write_only: BTreeSet<(usize, usize)>,
    /// Bases with a counting write whose node the mapping cannot name: every D3 query of the base
    /// is unknown (`d3-unknown`), as an unmapped kill-only def refuses its base.
    pub unplaced: BTreeSet<String>,
}

impl Counts {
    pub(crate) fn is_empty(&self) -> bool {
        self.writes.is_empty() && self.unplaced.is_empty()
    }

    pub(crate) fn note_target(&mut self, site: &Site) {
        if site.form != Form::ReadWrite {
            self.write_only.insert((site.target_start, site.target_end));
        }
    }

    pub(crate) fn push(&mut self, site: &Site, path: AccessPath, probe: usize) {
        self.writes.push(Counting {
            path,
            line: site.line,
            target_start: site.target_start,
            form: site.form,
            probe,
            has_construct: site.construct_kind.is_some(),
            pos: Position::of_site(site),
        });
    }
}
