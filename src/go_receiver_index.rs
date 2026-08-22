//! P11: Go receiver-typing post-merge indices and classification.
//!
//! Lane B (the string receiver-recovery lane; see `resolution.rs`'s
//! `ReceiverClassifier`) recovers a Go call site's receiver type from
//! function-local facts only (`ParsedFile::receiver_type_in_fn`), at
//! call-site EXTRACTION time. Three new recovery forms need REPO-WIDE facts
//! that are not available until the whole program has been parsed and
//! merged:
//!
//! - S1 `go_return_types`: clause-bearing function identity -> declared return type
//!   for free functions/methods whose `result` is a single type or a
//!   `(T, error)` pair. Feeds `d := newDemux(...)` call-RHS recovery.
//! - S3 `go_package_vars`: `(package_dir, var_name) -> declared type` for
//!   package-scope `var` declarations. Feeds a receiver whose qualifier has
//!   no function-local binding at all.
//!
//! (S2's `go_field_types` re-projection lives in `type_providers/go.rs`
//! since it is a straightforward re-projection of data the provider already
//! extracts; S4's embedded-interface routing map lives there too.)
//!
//! Per the spec's binding plumbing rule, these indices — and the
//! recomputation they feed — run in a POST-MERGE rematerialization pass
//! (`CallGraph::rematerialize_go_receiver_keys`), mirroring the shipped
//! Phase-2a Rust precedent (`rematerialize_rust_receiver_keys` /
//! `compute_rust_receiver_updates`), never at per-file extraction time. This
//! avoids the incremental-cache staleness hole a repo-wide-fact-dependent
//! per-file recovery would create (docs/superpowers/pipeline-lessons.md
//! doctrine 5).

use crate::ast::ParsedFile;
use crate::go_receiver_index_visibility::{resolve_go_return_type_call, unique_visible_type};
use crate::languages::Language;
use crate::resolution::{
    dir_of, owner_key, peel_type, resolve_go_owner_identity, ReceiverClassification,
    ReceiverClassifier, ReceiverCtx, ReceiverRecovery, RecoveredReceiver,
};
use crate::type_providers::go::GoTypeProvider;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct GoTypedFact {
    pub ty: String,
    pub defining_file: String,
}

pub type GoReturnTypes = BTreeMap<crate::resolution::GoOwnerIdentity, BTreeSet<GoTypedFact>>;

/// S1: extract a clause-bearing function identity -> declared return type for every
/// Go `function_declaration`/`method_declaration` whose return shape is
/// either a single type or a `(T, error)` pair. Ambiguous keys (two
/// declarations that share one package namespace/function name — e.g. a free
/// function and a same-named method — with DIFFERENT recorded types) drop
/// entirely rather than pick one arbitrarily (favor drop over a guess).
pub fn extract_go_return_types(files: &BTreeMap<String, ParsedFile>) -> GoReturnTypes {
    let mut multi = GoReturnTypes::new();
    for (path, parsed) in files {
        if parsed.language != Language::Go {
            continue;
        }
        let (profile, _) = crate::go_build_profile::extract_go_file_profile(path, parsed);
        if profile.package_clause.trim().is_empty() {
            continue;
        }
        let root = parsed.tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if matches!(child.kind(), "function_declaration" | "method_declaration") {
                if let Some((name, ty)) = extract_one_return_type(&child, parsed) {
                    multi
                        .entry(crate::resolution::GoOwnerIdentity {
                            package_dir: dir_of(path).to_string(),
                            package_clause: profile.package_clause.clone(),
                            name,
                        })
                        .or_default()
                        .insert(GoTypedFact {
                            ty,
                            defining_file: path.clone(),
                        });
                }
            }
        }
    }
    multi
}

/// Single function/method declaration -> `(name, raw return-type text)`, or
/// `None` when the declaration doesn't qualify (drop cases below).
fn extract_one_return_type(decl: &Node, parsed: &ParsedFile) -> Option<(String, String)> {
    // Generic decl (type params on the func itself) -> drop (spec: "Generic/
    // type-param returns: drop"). Reuses the provider's signature-scoped
    // generic-syntax gate so this stays consistent with the dispatch
    // provider's own generic exclusion (never re-derive independently).
    if GoTypeProvider::signature_has_generic_syntax(decl) {
        return None;
    }
    let name_node = decl.child_by_field_name("name")?;
    let name = parsed.node_text(&name_node).trim().to_string();
    if name.is_empty() {
        return None;
    }
    let result = decl.child_by_field_name("result")?;
    let ty = match result.kind() {
        "parameter_list" => {
            let elems = expand_go_result_list(&result, parsed);
            if elems.len() != 2 {
                // Multi-return beyond (T, error): out of scope, drop. Known
                // asymmetry (Opus impl-review minor): a single NAMED return
                // like `func f() (d *Demux)` also lands here (parenthesized
                // `parameter_list` of one), producing `elems.len() == 1` and
                // dropping even though it's semantically identical to the
                // unparenthesized `func f() *Demux` the `_ =>` arm below
                // recovers fine. Safe (favor drop over a guess), just a
                // missed opportunity — not attempted here.
                return None;
            }
            let second_bare = elems[1]
                .trim()
                .rsplit('.')
                .next()
                .unwrap_or(elems[1].trim())
                .trim();
            if second_bare != "error" {
                return None;
            }
            elems[0].clone()
        }
        _ => parsed.node_text(&result).trim().to_string(),
    };
    if ty.is_empty() || ty.contains('[') {
        // `contains('[')` rejects generic instantiations (`Foo[T]`) — mirrors
        // `iface_key`'s identical gate. Also conservatively rejects slice/array
        // return types (`[]Foo`); that's a missed opportunity, not a bug (favor
        // drop over a guess, per the safe-failure-direction doctrine).
        return None;
    }
    Some((name, ty))
}

/// Expand a Go result `parameter_list` to one raw type-text entry per return
/// slot (grouped names repeat the type). Any child that is not a plain
/// `parameter_declaration` (an unrecognized shape — returns never carry a
/// variadic slot) makes the whole list unrecognized -> empty (the caller's
/// `len() != 2` check then drops safely).
fn expand_go_result_list(list: &Node, parsed: &ParsedFile) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = list.walk();
    for decl in list.named_children(&mut cursor) {
        if decl.kind() != "parameter_declaration" {
            return Vec::new();
        }
        let Some(ty) = decl.child_by_field_name("type") else {
            return Vec::new();
        };
        let txt = parsed.node_text(&ty).trim().to_string();
        let mut ncur = decl.walk();
        let names = decl
            .children(&mut ncur)
            .filter(|c| c.kind() == "identifier")
            .count()
            .max(1);
        for _ in 0..names {
            out.push(txt.clone());
        }
    }
    out
}

/// S3: extract `(package_dir, var_name) -> declared type` for every
/// package-scope (top-level, `source_file`-scoped) Go `var` declaration that
/// carries an explicit type (`var r Runner`; `var a, b T`). Untyped package
/// vars (`var r = Impl{}`) are out of scope (kept simple, matching the
/// adjudicated fixture shape). Ambiguous keys drop, same policy as S1.
pub fn extract_go_package_vars(
    files: &BTreeMap<String, ParsedFile>,
) -> BTreeMap<(String, String), BTreeSet<GoTypedFact>> {
    let mut multi: BTreeMap<(String, String), BTreeSet<GoTypedFact>> = BTreeMap::new();
    for (path, parsed) in files {
        if parsed.language != Language::Go {
            continue;
        }
        let dir = dir_of(path).to_string();
        let root = parsed.tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() != "var_declaration" {
                continue;
            }
            let mut vcur = child.walk();
            for spec in child.children(&mut vcur) {
                if spec.kind() != "var_spec" {
                    continue;
                }
                let Some(ty) = spec.child_by_field_name("type") else {
                    continue;
                };
                let ty_text = parsed.node_text(&ty).trim().to_string();
                if ty_text.is_empty() {
                    continue;
                }
                let mut ncur = spec.walk();
                for name_node in spec.children_by_field_name("name", &mut ncur) {
                    let name = parsed.node_text(&name_node).trim().to_string();
                    if name.is_empty() || name == "_" {
                        continue;
                    }
                    multi
                        .entry((dir.clone(), name))
                        .or_default()
                        .insert(GoTypedFact {
                            ty: ty_text.clone(),
                            defining_file: path.clone(),
                        });
                }
            }
        }
    }
    multi
}

/// Decompose a Go selector-chain receiver expression into its base identifier
/// node plus the ordered list of field segments, AST-shaped only (never
/// text-split): `identifier(.selector_expression){1,2}`. Any index/slice/map
/// segment, call segment, or non-`field_identifier` field rejects (`None`).
/// Depth is NOT capped here (a 3+-hop chain decomposes fine); the caller caps
/// `segments.len()` to 1..=2 per the spec's field-chain-depth guard.
fn decompose_go_selector_chain<'a>(
    node: Node<'a>,
    parsed: &ParsedFile,
) -> Option<(Node<'a>, Vec<String>)> {
    if node.kind() != "selector_expression" {
        return None;
    }
    let mut segments = Vec::new();
    let mut cur = node;
    loop {
        if cur.kind() != "selector_expression" {
            return None;
        }
        let field = cur.child_by_field_name("field")?;
        if field.kind() != "field_identifier" {
            return None;
        }
        segments.push(parsed.node_text(&field).trim().to_string());
        let operand = cur.child_by_field_name("operand")?;
        match operand.kind() {
            "identifier" => {
                segments.reverse();
                return Some((operand, segments));
            }
            "selector_expression" => cur = operand,
            _ => return None,
        }
    }
}

fn is_simple_ident_text(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Bundled repo-wide indices the post-merge Go receiver pass consults.
/// Borrowed for the lifetime of one rematerialization pass.
pub struct GoReceiverFacts<'a> {
    pub return_types: &'a GoReturnTypes,
    pub package_vars: &'a BTreeMap<(String, String), BTreeSet<GoTypedFact>>,
    pub field_types: &'a crate::go_owner_partition::GoStructDeclarations,
    pub package_basenames: &'a BTreeMap<String, std::collections::BTreeSet<String>>,
    pub imports: &'a BTreeMap<String, BTreeMap<String, String>>,
    pub go_file_profiles: &'a BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
}

/// Per-call-site inputs for the post-merge Go classification. Mirrors
/// `ReceiverCtx` (resolution.rs) but always carries a live `receiver_expr` —
/// the post-merge pass only ever re-derives sites that HAD a qualifier at
/// extraction time (an unqualified call has no receiver to retype).
pub struct GoReceiverCtx<'a> {
    pub parsed: &'a ParsedFile,
    pub fn_node: Node<'a>,
    pub qualifier: &'a str,
    pub receiver_expr: Node<'a>,
    pub call_line: usize,
    pub call_start_byte: usize,
    pub recv_var: Option<&'a str>,
    pub file_imports: Option<&'a BTreeMap<String, String>>,
    pub caller_file: &'a str,
}

/// Recompute a Go call site's receiver classification with the full S1/S2/S3
/// repo-wide facts available. Strict superset of the extraction-time
/// classifier: reproduces its result byte-for-byte whenever the new forms
/// don't apply (ambiguous/shadowed bindings, an import-qualified or
/// keyword/receiver-var qualifier, or a genuinely unrecoverable local
/// binding), and only diverges to a NEW recovery when:
/// - the qualifier is a dotted 1-2 hop field-selector chain (S2), or
/// - the qualifier has zero function-local bindings (S3 package var), or
/// - the qualifier has exactly one local binding whose RHS is a single
///   first-position call the extraction-time constructor/`New`-prefix
///   heuristics couldn't type (S1 call-RHS).
///
/// Run fresh for EVERY Go call site on every full/incremental build (not
/// conditionally skipped when already `Some`) so a type-defining file edited
/// elsewhere in the package can never leave a stale recovery behind —
/// required for the bidirectional incremental-parity guarantee.
///
/// Classifier-seam note (Opus impl-review, controller-adjudicated INTENDED,
/// no behavior change): S1 (call-RHS `ReturnTyped`) and S2 (nested-selector
/// `FieldTyped`) below fire regardless of `base_classifier`/
/// `ReceiverRecoveryConfig.mode` — even under `Legacy` — while S3 (package
/// `var`) and the base-identifier `var r T` recovery respect the caller's
/// `var_local` flag explicitly. This mirrors the EXISTING un-gated
/// `ConstructorLocal` precedent: S1/S2 are grounded, AST-shape-derived
/// recoveries (a call's own declared return type; a struct's own declared
/// field type) with no heuristic ambiguity for a config toggle to gate,
/// whereas `var_local`/`type_assertion` gate recoveries that trade off
/// precision more heuristically. `Legacy` mode is a parity/fall-back mode
/// for THOSE forms, not a request to disable grounded ones.
pub fn classify_go_receiver_expanded(
    ctx: &GoReceiverCtx<'_>,
    base_classifier: &dyn ReceiverClassifier,
    facts: &GoReceiverFacts<'_>,
    var_local: bool,
) -> ReceiverClassification {
    classify_go_receiver_expanded_with_partition(ctx, base_classifier, facts, var_local).0
}

pub(crate) fn classify_go_receiver_expanded_with_partition(
    ctx: &GoReceiverCtx<'_>,
    base_classifier: &dyn ReceiverClassifier,
    facts: &GoReceiverFacts<'_>,
    var_local: bool,
) -> (
    ReceiverClassification,
    crate::go_owner_partition::GoPartitionEvidence,
) {
    let rctx = ReceiverCtx {
        receiver_expr: Some(ctx.receiver_expr),
        qualifier: Some(ctx.qualifier),
        fn_node: ctx.fn_node,
        call_line: ctx.call_line,
        call_start_byte: ctx.call_start_byte,
        parsed: ctx.parsed,
        recv_var: ctx.recv_var,
        file_imports: ctx.file_imports,
    };
    let baseline = base_classifier.classify(rctx);
    if baseline.recovered.is_some() {
        return (baseline, Default::default());
    }

    if ctx.qualifier.contains('.') {
        let (recovered, evidence) =
            classify_nested_selector(ctx, base_classifier, facts, var_local);
        if let Some(rec) = recovered {
            return (
                ReceiverClassification {
                    recovered: Some(rec),
                    materialized: true,
                },
                evidence,
            );
        }
        return (baseline, evidence);
    }

    // Same suppression gate `classify_simple_ident` applies (resolution.rs):
    // a keyword receiver, the enclosing method's OWN receiver variable, or an
    // import-qualified identifier are never candidates for local-binding
    // receiver recovery — replicated here (not re-derived from a different
    // rule) so a same-named package var/return-typed local can't shadow-steal
    // what those rungs already own.
    let is_kw = matches!(ctx.qualifier, "self" | "this" | "cls");
    let is_recv = ctx.recv_var == Some(ctx.qualifier);
    let is_import = ctx
        .file_imports
        .map(|m| m.contains_key(ctx.qualifier))
        .unwrap_or(false);
    if !is_simple_ident_text(ctx.qualifier) || is_kw || is_recv || is_import {
        return (baseline, Default::default());
    }

    // `var_local` (not hardcoded `true`): must match the SAME flag
    // `base_classifier` was built with, or a Legacy-mode / `var_local: false`
    // build's intentionally-disabled `var r T` recovery would be silently
    // re-enabled here (found via a DIFFERENT recover_var value than the
    // config specifies) even though `baseline` just correctly refused it.
    let (found, bindings) = ctx.parsed.receiver_type_in_fn(
        &ctx.fn_node,
        ctx.qualifier,
        ctx.call_line,
        ctx.call_start_byte,
        var_local,
    );
    if bindings > 1 {
        return (baseline, Default::default()); // ambiguous/shadowed — unchanged.
    }
    if let Some((ty, how)) = found {
        let static_type = owner_key(&peel_type(&ty));
        return (
            ReceiverClassification {
                recovered: Some(RecoveredReceiver {
                    static_type,
                    recovery: how,
                }),
                materialized: true,
            },
            Default::default(),
        );
    }
    if bindings == 0 {
        // S3 is the package-scope generalization of the SAME `var`-decl
        // recovery form `var_local` gates function-locally — respect the
        // same config flag (a Legacy/`var_local: false` build should not
        // trust a package-level `var` any more than a function-local one).
        if var_local {
            let key = (
                dir_of(ctx.caller_file).to_string(),
                ctx.qualifier.to_string(),
            );
            if let Some(ty) = facts.package_vars.get(&key).and_then(|entries| {
                unique_visible_type(ctx.caller_file, entries, facts.go_file_profiles)
            }) {
                let static_type = owner_key(&peel_type(&ty));
                return (
                    ReceiverClassification {
                        recovered: Some(RecoveredReceiver {
                            static_type,
                            recovery: ReceiverRecovery::VarDecl,
                        }),
                        materialized: true,
                    },
                    Default::default(),
                );
            }
        }
        return (baseline, Default::default());
    }
    // bindings == 1 && found.is_none(): exactly one qualifying local
    // statement bound this name but its type wasn't recoverable via the
    // composite-literal/`New`-prefix heuristics — try the call-RHS retry.
    if let Some(callee_text) = ctx.parsed.go_short_var_call_rhs(
        &ctx.fn_node,
        ctx.qualifier,
        ctx.call_line,
        ctx.call_start_byte,
    ) {
        if let Some(ty) = resolve_go_return_type_call(
            &callee_text,
            ctx.caller_file,
            facts.imports,
            facts.package_basenames,
            facts.return_types,
            facts.go_file_profiles,
        ) {
            let static_type = owner_key(&peel_type(&ty));
            return (
                ReceiverClassification {
                    recovered: Some(RecoveredReceiver {
                        static_type,
                        recovery: ReceiverRecovery::ReturnTyped,
                    }),
                    materialized: true,
                },
                Default::default(),
            );
        }
    }
    (baseline, Default::default())
}

/// S2: base + up to 2 field hops, AST-shaped, any miss (unresolved base,
/// unknown owner identity, or a field-types miss at any hop) drops entirely
/// — no partial recovery.
fn classify_nested_selector(
    ctx: &GoReceiverCtx<'_>,
    base_classifier: &dyn ReceiverClassifier,
    facts: &GoReceiverFacts<'_>,
    var_local: bool,
) -> (
    Option<RecoveredReceiver>,
    crate::go_owner_partition::GoPartitionEvidence,
) {
    let mut evidence = crate::go_owner_partition::GoPartitionEvidence::default();
    let Some((base_node, segments)) = decompose_go_selector_chain(ctx.receiver_expr, ctx.parsed)
    else {
        return (None, evidence);
    };
    if segments.is_empty() || segments.len() > 2 {
        return (None, evidence); // 3+-hop chain — depth guard, no recovery.
    }
    let base_text = ctx.parsed.node_text(&base_node).trim();
    if !is_simple_ident_text(base_text) {
        return (None, evidence);
    }
    let base_ctx = GoReceiverCtx {
        parsed: ctx.parsed,
        fn_node: ctx.fn_node,
        qualifier: base_text,
        receiver_expr: base_node,
        call_line: ctx.call_line,
        call_start_byte: ctx.call_start_byte,
        recv_var: ctx.recv_var,
        file_imports: ctx.file_imports,
        caller_file: ctx.caller_file,
    };
    // Recurse through the SAME simple-ident + S1/S3 machinery for the base —
    // terminates in one level since `base_text` is never dotted.
    let Some(base_recovered) =
        classify_go_receiver_expanded(&base_ctx, base_classifier, facts, var_local).recovered
    else {
        return (None, evidence);
    };

    let mut current = base_recovered.static_type;
    for seg in segments {
        let Some(owner) = resolve_go_owner_identity(
            &current,
            ctx.caller_file,
            facts.imports,
            facts.package_basenames,
            facts.go_file_profiles,
        ) else {
            return (None, evidence);
        };
        let Some(declarations) = facts.field_types.get(&owner) else {
            return (None, evidence);
        };
        let selection = crate::go_owner_partition::select_struct_field(
            &owner,
            ctx.caller_file,
            &current,
            &seg,
            declarations,
            facts.go_file_profiles,
        );
        evidence.merge(selection.evidence);
        let Some(field_ty) = selection.value else {
            return (None, evidence);
        };
        current = owner_key(&peel_type(&field_ty));
    }
    (
        Some(RecoveredReceiver {
            static_type: current,
            recovery: ReceiverRecovery::FieldTyped,
        }),
        evidence,
    )
}
