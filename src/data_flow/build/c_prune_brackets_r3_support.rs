//! #829 chain B (round 3): shared helpers of the pair rows. The rows judge one pair, by owner
//! (`e.from.function`): a nested owner's pair has a duplicate copy in its enclosing owner
//! (`cfg_incomplete`) that survives on base, so a from/to-only check would pass for the wrong
//! reason. Every row asserts REQUIRED behaviour (the live pair is kept, or the control pair is
//! pruned), never a specific `Off::` reason: the rule of DESIGN.md is tentative.
use super::Producer;
use crate::ast::ParsedFile;
use crate::call_graph::c::macro_census::CMacroCensus;
use crate::call_graph::rust::macro_args::RustMacroCensus;
use crate::languages::Language;
use std::collections::BTreeMap;
use std::sync::Arc;

pub(super) fn lang_of(file: &str) -> Language {
    if file.ends_with(".cc") || file.ends_with(".cpp") || file.ends_with(".hpp") {
        Language::Cpp
    } else {
        Language::C
    }
}

/// 1-based number of the first line holding `needle`.
pub(super) fn line_of(src: &str, needle: &str) -> usize {
    src.lines()
        .position(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("no line holds {needle:?}"))
        + 1
}

/// `(line of "int x = 41;", line of the first "return " after it)`: the pair of every guard
/// owner below (`x = 41` is live when `c == 0`).
pub(super) fn guard_pair(src: &str) -> (usize, usize) {
    let from = line_of(src, "int x = 41;");
    let to = src
        .lines()
        .enumerate()
        .skip(from)
        .find(|(_, l)| l.contains("return "))
        .map(|(i, _)| i + 1)
        .expect("a return after the def");
    (from, to)
}

/// The guard owner of the splice and external-macro rows: `OPEN`/`CLOSE` arrive from
/// `defs`. C returns `x`; C++ returns `sink(x + 1)` (no bare call argument).
pub(super) fn guard_source(defs: &str, cpp: bool) -> String {
    let ret = if cpp { "sink(x + 1)" } else { "x" };
    format!(
        "{defs}int sink(int);\nint probe(int c) {{\n    int x = 41;\n    if (c) OPEN;\n    x = 2;\n    CLOSE;\n    return {ret};\n}}\n"
    )
}

/// `(before the rules, after the rules)`: does the pair `path@from -> path@to` exist in
/// `target` (of `files`, census of all of them plus optional census texts stamped on the
/// target), restricted to edges of the owner named `owner` when given?
pub(super) fn pair_in(
    files: &[(&str, &str)],
    target: &str,
    texts: &[(&str, &str)],
    owner: Option<&str>,
    path: &str,
    from: usize,
    to: usize,
) -> (bool, bool) {
    let mut parsed: BTreeMap<String, ParsedFile> = files
        .iter()
        .map(|(n, s)| (n.to_string(), ParsedFile::parse(n, s, lang_of(n)).unwrap()))
        .collect();
    if !texts.is_empty() {
        let t: BTreeMap<String, String> = texts
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        parsed
            .get_mut(target)
            .unwrap()
            .set_c_census_texts(Arc::new(t));
    }
    let census = CMacroCensus::from_files(&parsed);
    let file = target.to_owned();
    let p = &parsed[target];
    let has = |r: &super::FileRefs| {
        r.edges.iter().any(|e| {
            e.from.path.to_string() == path
                && e.from.line == from
                && e.to.line == to
                && owner.is_none_or(|o| e.from.function == o)
        })
    };
    let before = Producer::file_refs_frozen(&file, p);
    let after = super::alias_occurrence::without_policy(|| {
        Producer::file_refs_with_census(&file, p, &RustMacroCensus::default(), &census)
    });
    (has(&before), has(&after))
}

/// Collects the failures of a loop of rows so that every iteration runs: an INADMISSIBLE row
/// (the baseline has no pair, so a removal-only rule could not drop it) panics at once, a
/// dropped live pair or a kept dead pair is recorded and reported together by `finish`.
#[derive(Default)]
pub(super) struct Batch {
    failed: Vec<String>,
}

impl Batch {
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// The live pair `x@from -> x@to` of `owner` must exist before the rules (else the row is
    /// inadmissible) and after them.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn kept(
        &mut self,
        what: &str,
        files: &[(&str, &str)],
        target: &str,
        texts: &[(&str, &str)],
        owner: Option<&str>,
        from: usize,
        to: usize,
    ) {
        let r = pair_in(files, target, texts, owner, "x", from, to);
        self.kept_result(what, r, owner, from, to);
    }

    /// The judgement of `kept` for a `(before, after)` computed elsewhere (the loader rows).
    pub(super) fn kept_result(
        &mut self,
        what: &str,
        (before, after): (bool, bool),
        owner: Option<&str>,
        from: usize,
        to: usize,
    ) {
        assert!(
            before,
            "{what}: inadmissible, the baseline has no x@{from} -> x@{to} (owner {owner:?})"
        );
        if !after {
            self.failed.push(format!(
                "{what}: the live pair x@{from} -> x@{to} (owner {owner:?}) was dropped"
            ));
        }
    }

    /// Control: a pair the compiled program does not use must still be pruned.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn pruned(
        &mut self,
        what: &str,
        files: &[(&str, &str)],
        target: &str,
        texts: &[(&str, &str)],
        owner: Option<&str>,
        from: usize,
        to: usize,
    ) {
        let r = pair_in(files, target, texts, owner, "x", from, to);
        self.pruned_result(what, r, owner, from, to);
    }

    pub(super) fn pruned_result(
        &mut self,
        what: &str,
        (before, after): (bool, bool),
        owner: Option<&str>,
        from: usize,
        to: usize,
    ) {
        assert!(
            before,
            "{what}: inadmissible, the baseline has no x@{from} -> x@{to} (owner {owner:?})"
        );
        if after {
            self.failed.push(format!(
                "{what}: control, the dead pair x@{from} -> x@{to} (owner {owner:?}) must be pruned"
            ));
        }
    }

    /// One file: the pair is the guard pair of `src`.
    pub(super) fn kept_guard(&mut self, what: &str, file: &str, src: &str, owner: Option<&str>) {
        let (from, to) = guard_pair(src);
        self.kept(what, &[(file, src)], file, &[], owner, from, to);
    }

    pub(super) fn pruned_guard(&mut self, what: &str, file: &str, src: &str, owner: Option<&str>) {
        let (from, to) = guard_pair(src);
        self.pruned(what, &[(file, src)], file, &[], owner, from, to);
    }

    pub(super) fn finish(self) {
        assert!(self.failed.is_empty(), "{:#?}", self.failed);
    }
}

pub(super) fn kept_guard(what: &str, file: &str, src: &str, owner: Option<&str>) {
    let mut b = Batch::new();
    b.kept_guard(what, file, src, owner);
    b.finish();
}

pub(super) fn pruned_guard(what: &str, file: &str, src: &str, owner: Option<&str>) {
    let mut b = Batch::new();
    b.pruned_guard(what, file, src, owner);
    b.finish();
}
