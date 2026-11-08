//! #761: with `PRISM_RD_CENSUS_TRACE=1` (the census trace's switch), one stderr line per owner whose
//! parameter list changes what the producer does, so a measurement can attribute every edge or label
//! that moved between a base and a head build to a named mechanism. Silent with the trace off; never
//! changes an output.
//!
//! Line: `rd-census-owner\t<mechanism>\t<file>\t<owner>\t<first line>\t<last line>`: the shape of
//! `guard-deleted:<hazard>` plus the owner's last line (the edge dump carries no owner, so a measurement
//! attributes an edge to the owner whose line range holds both of its endpoints).
//! - `param-alias`: the own parameter list holds an aliasing default (`u = o`, `[u] = o`, `{ a: u } = o`): the
//!   owner mints the alias binding's def (and, where the body's extractor yields one, the alias twin of `o`).
//! - `param-bare`: a formal is read bare only in a default of the own parameter list, so it is mixed
//!   where it used to be member-only (or unused).
use crate::ast::ParsedFile;
use std::collections::BTreeSet;
use std::sync::OnceLock;
use tree_sitter::Node;

fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("PRISM_RD_CENSUS_TRACE").is_ok_and(|v| v == "1"))
}

pub(super) fn trace<'a>(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    owner: &str,
    lines: &BTreeSet<usize>,
    formals: impl Iterator<Item = &'a str>,
) {
    if !enabled() {
        return;
    }
    let (first, last) = parsed.node_line_range(func_node);
    let emit = |mechanism: &str| {
        eprintln!(
            "rd-census-owner\t{mechanism}\t{}\t{owner}\t{first}\t{last}",
            parsed.path
        );
    };
    // Keyed on the minted alias binding defs, not on alias triples: an entry that mints a def but no
    // triple (`{ a: u = 1 } = o`, `{ [k]: u } = o`) still moves edges.
    let mut spans = Vec::new();
    parsed.js_ts_param_default_alias_spans(func_node, lines, &mut spans);
    if !spans.is_empty() {
        emit("param-alias");
    }
    let mut formals: Vec<&str> = formals.collect();
    formals.sort_unstable();
    formals.dedup();
    if formals.into_iter().any(|name| {
        parsed.js_ts_parameter_list_reads_bare(func_node, name)
            && !parsed.has_bare_references(func_node, name)
    }) {
        emit("param-bare");
    }
}
