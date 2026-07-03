//! P10: path-proven sanitizer transition walk.
//!
//! `taint_reaches`'s raw reachability (`Reachability::Reached`) means the CPG's taint trace
//! connects source to sink SOMEWHERE — it says nothing about whether a recognized sanitizer sits ON
//! that specific path. Function-body presence (`cleansed_categories_for_source` / the `Cleansed`
//! warning) stays advisory-only for exactly that reason: node-presence in the trace's witness chain
//! is NOT proof. `cpg/trace.rs`'s `AssignmentPropagation` edges connect a Use to EVERY same-line Def
//! in the enclosing function (see `taint_neighbors`'s Use arm), not just the Def the Use's own
//! enclosing assignment actually targets — so "some chain node sits inside a sanitizer call's line"
//! does not show the transform runs on THIS path.
//!
//! This module proves the narrower, sound claim instead: chain window `[node[i], node[i+1]]` is a
//! genuine `x = sanitizer(y)` transition IFF
//!   (a) `node[i]`'s byte span sits inside the RHS/value of an assignment or declaration whose
//!       value IS a recognizer-matched, non-`paired_check` sanitizer call,
//!   (b) `node[i+1]`'s byte span is EXACTLY that SAME assignment's target/declared-name span, AND
//!   (c) `node[i]`'s byte span sits inside the call's FIRST (data) argument specifically — NOT
//!       merely somewhere in the call expression. `ast.rs::rvalue_identifier_spans_on_lines` records
//!       BOTH the callee/function-name identifier and EVERY call argument as a Use, so (a) alone
//!       would also accept the callee span or a non-data argument at index > 0 (e.g. `escape(other,
//!       user)` with `user` untouched at arg[1], or a tainted local shadowing the callee name in
//!       `escape = input(); safe = escape(other)`). See `sanitizer_site_for_assignment`'s doc
//!       comment for which recognizers this first-argument assumption is verified against.
//!
//! Reconstructing "same assignment" from byte spans (via `descendant_for_byte_range` + a parent
//! walk to the nearest enclosing assignment/declaration node), rather than trusting the `Relation`
//! label on the edge, is what makes this sound against the over-approximating
//! `AssignmentPropagation` edges. Paired-check recognizers (Go's `Clean`→`HasPrefix` path family,
//! `src/sanitizers/path.rs`) are excluded by `taint::sanitizer_call_site` itself — see that
//! function's doc comment for the any-category-cut, paired-check-exclusion, and
//! language-applicability divergences from the CWE engine.

use std::collections::{BTreeMap, BTreeSet};

use petgraph::graph::NodeIndex;
use tree_sitter::Node;

use crate::algorithms::taint::{sanitizer_call_site, sanitizer_category_str};
use crate::ast::ParsedFile;
use crate::cpg::CodePropertyGraph;
use crate::data_flow::{VarAccessKind, VarLocation};
use crate::reasoning::types::SanitizerSite;

/// One proven sanitizer transition on a witness chain.
pub struct SanitizerHit {
    /// The chain node (position `i`) whose value flows into the sanitizer call — the graph
    /// attachment point for the `"SanitizedBy"` witness step.
    pub use_node: NodeIndex,
    pub call_start_byte: usize,
    pub call_end_byte: usize,
    /// The wire-shape discriminating fact.
    pub site: SanitizerSite,
}

/// Walk every contiguous window of `chain` (an ordered root(source)->sink witness chain, e.g. from
/// `shape::witness_chain_for`) looking for a proven sanitizer transition. Returns every distinct hit
/// in chain order, deduped by `(file, line, callee_text)`. A chain's verdict is `Sanitized` iff this
/// is non-empty.
pub fn sanitized_hits_on_chain(
    files: &BTreeMap<String, ParsedFile>,
    cpg: &CodePropertyGraph,
    chain: &[NodeIndex],
) -> Vec<SanitizerHit> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for window in chain.windows(2) {
        let (use_idx, def_idx) = (window[0], window[1]);
        if let Some((site, call_start_byte, call_end_byte)) =
            sanitizer_transition(files, cpg, use_idx, def_idx)
        {
            let key = (site.file.clone(), site.line, site.callee_text.clone());
            if seen.insert(key) {
                out.push(SanitizerHit {
                    use_node: use_idx,
                    call_start_byte,
                    call_end_byte,
                    site,
                });
            }
        }
    }
    out
}

fn sanitizer_transition(
    files: &BTreeMap<String, ParsedFile>,
    cpg: &CodePropertyGraph,
    use_idx: NodeIndex,
    def_idx: NodeIndex,
) -> Option<(SanitizerSite, usize, usize)> {
    let use_loc = cpg.to_var_location(use_idx)?;
    if !matches!(use_loc.kind, VarAccessKind::Use) {
        return None;
    }
    let def_loc = cpg.to_var_location(def_idx)?;
    if !matches!(def_loc.kind, VarAccessKind::Def) {
        return None;
    }
    let parsed = files.get(&use_loc.file)?;
    if !crate::sanitizers::sanitizer_supported(parsed.language) {
        return None;
    }

    let leaf = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(use_loc.start_byte, use_loc.end_byte)?;

    // Walk up from the Use to the NEAREST enclosing assignment/declaration node. This is the
    // reconstruction step: "same assignment" is decided by byte-span containment, not by the
    // `Relation` the trace happened to label the edge with.
    let mut cur = Some(leaf);
    while let Some(node) = cur {
        if parsed.language.is_assignment_node(node.kind())
            || parsed.language.is_declaration_node(node.kind())
        {
            return sanitizer_site_for_assignment(parsed, &node, &use_loc, &def_loc);
        }
        cur = node.parent();
    }
    None
}

fn sanitizer_site_for_assignment(
    parsed: &ParsedFile,
    node: &Node<'_>,
    use_loc: &VarLocation,
    def_loc: &VarLocation,
) -> Option<(SanitizerSite, usize, usize)> {
    let rhs = parsed
        .language
        .assignment_value(node)
        .or_else(|| parsed.language.declaration_value(node))?;
    // (a): node[i] (the Use) must sit inside THIS assignment's RHS/value span.
    if !(rhs.start_byte() <= use_loc.start_byte && use_loc.end_byte <= rhs.end_byte()) {
        return None;
    }
    let target = parsed
        .language
        .assignment_target(node)
        .or_else(|| parsed.language.declaration_name(node))?;
    // (b): node[i+1] (the Def) must be EXACTLY this SAME assignment's target — not merely another
    // same-line Def the over-approximating AssignmentPropagation edge happened to connect to.
    if target.start_byte() != def_loc.start_byte || target.end_byte() != def_loc.end_byte {
        return None;
    }
    // The RHS must itself be the sanitizer call (direct form only — `safe = escape(user)` /
    // `const safe = escape(user)`; a wrapped/nested call is out of scope this slice).
    if !parsed.language.is_call_node(rhs.kind()) {
        return None;
    }
    let call = sanitizer_call_site(parsed, &rhs)?;
    // F1 BLOCKER: `ast.rs::rvalue_identifier_spans_on_lines` records BOTH the callee/function-name
    // identifier AND every call argument as a Use — so "Use sits inside the call's overall span" is
    // not enough; the sanitizer transform must actually run ON `use_loc`'s value. That means
    // `use_loc` must sit inside the call's FIRST (data) argument specifically, never the callee span
    // and never an argument at index > 0.
    //
    // VERIFIED against the recognizer tables this feeds (`sanitizer_call_site` already excludes
    // `paired_check` recognizers, i.e. `PATH_RECOGNIZERS`; `SHELL_RECOGNIZERS` is empty): every
    // remaining active recognizer — `JS_TS_RECOGNIZERS` (`DOMPurify.sanitize`, `escapeHtml`,
    // `escape`) and `PYTHON_RECOGNIZERS` (`html.escape`, `markupsafe.escape`, `escape`,
    // `bleach.clean`, `bleach.linkify`) — is a plain value-transform call that takes the tainted
    // data as its first positional argument, e.g. `escape(data)` / `html.escape(data)`. NONE are
    // receiver-style (`data.transform()`, where `data` itself is the call's receiver rather than an
    // argument). If a future recognizer IS receiver-style, this check needs a per-recognizer
    // exception that accepts the receiver span (via `call_function_qualifier`) instead of arg[0] for
    // that recognizer specifically — see `src/sanitizers/{js_ts.rs,python.rs}`.
    let args = parsed.language.call_arguments(&rhs)?;
    let mut arg_cursor = args.walk();
    let first_arg = args.named_children(&mut arg_cursor).next()?;
    if !(first_arg.start_byte() <= use_loc.start_byte && use_loc.end_byte <= first_arg.end_byte()) {
        return None;
    }
    Some((
        SanitizerSite {
            category: sanitizer_category_str(call.category).to_string(),
            callee_text: call.callee_text,
            file: call.file,
            line: call.line,
        },
        rhs.start_byte(),
        rhs.end_byte(),
    ))
}
