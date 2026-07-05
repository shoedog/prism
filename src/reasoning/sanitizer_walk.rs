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
use crate::cpg::{CodePropertyGraph, Relation, Trace};
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
///
/// `window_relations[i]` is the `Relation` labeling the edge `(chain[i], chain[i+1])` — recovered
/// from `parents_by_root` by the caller (`shape::window_relations`, the SAME recovery the
/// witness-graph edge-kind labeling uses; see `reasoning::taint_reaches::witness_mode`). A window
/// whose relation is `Relation::CallDescent` is SKIPPED unconditionally: a descent window is an
/// arg->param hop by construction (P14 §S4) and can never be a same-assignment `x = sanitizer(y)`
/// transition — the byte-span reconstruction below has no cross-function knowledge and must not be
/// left to accidentally resolve one. This is the authoritative replacement for Stage A's interim
/// same-function-identity guard (do not keep both — the relation is the sole signal here).
pub fn sanitized_hits_on_chain(
    files: &BTreeMap<String, ParsedFile>,
    cpg: &CodePropertyGraph,
    chain: &[NodeIndex],
    window_relations: &[Option<Relation>],
) -> Vec<SanitizerHit> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for (window, relation) in chain.windows(2).zip(window_relations.iter().copied()) {
        if relation == Some(Relation::CallDescent) {
            continue;
        }
        let (use_idx, def_idx) = (window[0], window[1]);
        // `sanitizer_transition` re-derives and validates each `VarLocation` (Use/Def kind) itself,
        // so no separate same-file/same-function pre-check is needed here — the `CallDescent` skip
        // above is the sole cross-function guard (Stage A's interim function-identity check is gone).
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

/// P14 fix-wave BLOCKER fix (bypass-proven `Sanitized`): every hop `(parent, node)` in `trace`'s
/// first-parent tree rooted at `root` — NOT just the ones lying on the single first-enqueue-wins
/// witness chain to one particular sink — that is itself a genuine, provable `x = sanitizer(y)`
/// transition (the same per-window proof [`sanitized_hits_on_chain`] uses).
///
/// Why tree-scoped and not chain-scoped: the witness chain to a given sink is one linear path picked
/// by first-enqueue-wins dedup. Two independently-sanitized branches can converge on a shared
/// confluence node — typically a callee parameter reached from two call sites under interprocedural
/// descent (`g(safe1); g(safe2)`, both separately sanitized), but also reachable intra-function via
/// same-line `AssignmentPropagation` fan-in to a shared target — and only ONE of the two sanitizer
/// transitions can ever appear on any single witness chain; the other lives on a sibling branch of
/// the SAME first-parent tree that never got a chance to become "the" chain to this sink. Scanning
/// the whole tree (still bounded — `parents_by_root` holds at most one entry per node reachable from
/// `root`) finds every one, so the caller's exclude-then-rewalk bypass check
/// (`reasoning::taint_reaches::witness_mode`) can prove ALL of them together disconnect the sink,
/// not just whichever one happened to win the enqueue race.
///
/// `CallDescent`-relation hops are excluded up front for the same reason [`sanitized_hits_on_chain`]
/// skips them: an arg->param hop can never itself be a same-assignment transition, and the byte-span
/// reconstruction below has no cross-function knowledge to rule that out on its own.
pub fn sanitizer_bypass_exclusions(
    files: &BTreeMap<String, ParsedFile>,
    cpg: &CodePropertyGraph,
    trace: &Trace,
    root: NodeIndex,
) -> BTreeSet<(NodeIndex, NodeIndex)> {
    let mut out = BTreeSet::new();
    for (&(hop_root, node), &(parent, relation)) in trace.parents_by_root.iter() {
        if hop_root != root || relation == Relation::CallDescent {
            continue;
        }
        if sanitizer_transition(files, cpg, parent, node).is_some() {
            out.insert((parent, node));
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
    // `use_loc` must sit inside the call's data-argument span specifically, never the callee span
    // and never a non-data argument.
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
    //
    // F3 BLOCKER: JS/TS has no keyword-argument call syntax, so for those languages the FIRST
    // named child of the argument list is always the positional data argument — unchanged. Python
    // can pass the data value BY NAME instead, e.g. `html.escape(s=user)`, and the argument list's
    // first named child can then be a `keyword_argument` node spanning `name=value` as a whole.
    // `collect_identifier_path_spans` (ast.rs) recurses into BOTH sides of a `keyword_argument`, so
    // a Use can sit in the NAME (label) identifier — which can coincidentally share text with a
    // real tainted variable, see `python_keyword_label_use_is_not_a_sanitizer_transition` — or in a
    // DIFFERENT, non-data keyword argument's value, see
    // `python_tainted_non_data_kwarg_is_not_a_sanitizer_transition`. Accepting "somewhere in the
    // first named child" for a keyword form would launder either case into a false `Sanitized`.
    // `use_in_data_argument` scans every argument and only accepts (a) a non-keyword first
    // argument (unchanged positional behavior) or (b) the VALUE span of the `keyword_argument`
    // whose NAME matches the matched recognizer's `data_param` — never the label, never an
    // unrelated kwarg. A keyword form with `data_param == None` always rejects (conservative:
    // missed-`Sanitized` is the safe direction).
    let args = parsed.language.call_arguments(&rhs)?;
    if !use_in_data_argument(parsed, &args, call.data_param, use_loc) {
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

/// F3: does `use_loc` sit inside the call's DATA-ARGUMENT span? `args` is the call's argument-list
/// node. Scans every named argument in order:
///   - a `keyword_argument` node (Python `name=value`) is accepted ONLY via its VALUE span, and
///     ONLY when its NAME text equals `data_param` — the label identifier and any other keyword
///     argument are never a match, regardless of position;
///   - any other (positional) argument is accepted only at position 0 — this is the pre-F3
///     behavior, unchanged, and is the only form JS/TS (no keyword-argument syntax) ever produces.
/// `data_param == None` means the matched recognizer has no verified data-parameter name, so every
/// keyword form is rejected for it (conservative: stays `Reached`, never a false `Sanitized`).
fn use_in_data_argument(
    parsed: &ParsedFile,
    args: &Node<'_>,
    data_param: Option<&'static str>,
    use_loc: &VarLocation,
) -> bool {
    let mut cursor = args.walk();
    for (index, arg) in args.named_children(&mut cursor).enumerate() {
        if arg.kind() == "keyword_argument" {
            let Some(data_param) = data_param else {
                continue;
            };
            let name_matches = arg
                .child_by_field_name("name")
                .is_some_and(|n| parsed.node_text(&n) == data_param);
            if !name_matches {
                continue;
            }
            let value = arg
                .child_by_field_name("value")
                .or_else(|| arg.named_child(1));
            if let Some(value) = value {
                if value.start_byte() <= use_loc.start_byte && use_loc.end_byte <= value.end_byte()
                {
                    return true;
                }
            }
            continue;
        }
        if index == 0
            && arg.start_byte() <= use_loc.start_byte
            && use_loc.end_byte <= arg.end_byte()
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_flow::VarAccessKind;
    use crate::languages::Language;
    use crate::reasoning::shape::{window_relations, witness_chain_for};

    fn build_python_cpg(src: &str) -> (CodePropertyGraph, BTreeMap<String, ParsedFile>) {
        let parsed = ParsedFile::parse("test.py", src, Language::Python).unwrap();
        let mut files = BTreeMap::new();
        files.insert("test.py".to_string(), parsed);
        let cpg = CodePropertyGraph::build(&files);
        (cpg, files)
    }

    fn node_at(cpg: &CodePropertyGraph, line: usize, path: &str, kind: VarAccessKind) -> NodeIndex {
        cpg.nodes_at("test.py", line)
            .into_iter()
            .find(|&n| {
                cpg.to_var_location(n)
                    .is_some_and(|l| l.path.to_string() == path && l.kind == kind)
            })
            .unwrap_or_else(|| panic!("{path} ({kind:?}) not found on line {line}"))
    }

    /// P14 T8: a `CallDescent`-labeled window must NEVER be evaluated as a sanitizer transition, even
    /// when its byte spans WOULD otherwise structurally satisfy `sanitizer_site_for_assignment` (the
    /// "same assignment" reconstruction has no cross-function knowledge and cannot tell a genuine
    /// intra-function transition from a coincidental byte-span collision on its own). This directly
    /// exercises the relation-skip mechanism, independent of whether any particular fixture's descent
    /// hop happens to collide with an enclosing assignment.
    #[test]
    fn call_descent_relation_window_is_never_evaluated_as_a_sanitizer_transition() {
        let (cpg, files) = build_python_cpg(
            "def f():\n    user = input()\n    safe = html.escape(user)\n    sink(safe)\n",
        );
        let trace = cpg.taint_trace(&[("test.py".to_string(), 2)]);
        let root = node_at(&cpg, 2, "user", VarAccessKind::Def);
        let sink_use = node_at(&cpg, 4, "safe", VarAccessKind::Use);
        let chain = witness_chain_for(&trace, root, sink_use).expect("chain exists");

        // Baseline: the REAL (all intra-function) relations recovered from the trace DO produce a
        // hit — the window (`user` use inside `html.escape(user)` -> `safe` def) genuinely satisfies
        // the same-assignment sanitizer-transition proof.
        let real_relations = window_relations(&trace, Some(root), &chain);
        let baseline_hits = sanitized_hits_on_chain(&files, &cpg, &chain, &real_relations);
        assert_eq!(
            baseline_hits.len(),
            1,
            "expected the genuine sanitizer transition to be provable at baseline: {:?}",
            chain
        );

        // Now force the SAME structurally-matching window's relation to `CallDescent` (as it would be
        // if `safe` had instead been an arg->param descent target) and confirm it is skipped outright
        // — proving the skip is doing real work, not merely absent because our fixtures never collide.
        let safe_def_pos = chain
            .iter()
            .position(|&n| {
                cpg.to_var_location(n).is_some_and(|l| {
                    l.path.to_string() == "safe" && matches!(l.kind, VarAccessKind::Def)
                })
            })
            .expect("safe def in chain");
        let mut forced_relations = real_relations.clone();
        forced_relations[safe_def_pos - 1] = Some(Relation::CallDescent);
        let forced_hits = sanitized_hits_on_chain(&files, &cpg, &chain, &forced_relations);
        assert!(
            forced_hits.is_empty(),
            "a CallDescent-labeled window must never be evaluated as a sanitizer transition: {:?}",
            forced_hits.iter().map(|h| h.use_node).collect::<Vec<_>>()
        );
    }
}
