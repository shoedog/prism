//! Field-selective binding for direct JS/TS/TSX object-literal arguments.
//!
//! Inventory precedes target iteration. An Exact singleton with an admitted
//! plain formal keeps #594's projection from literal leaves to reached reads,
//! after proving the selected Use. An admitted object pattern binds surviving
//! leaves to its concrete local Defs. The pairing and writer fold live in
//! `props_binding.rs`. Both routes run before positional and rest collection.
//!
//! Callee reads. The targets are the `Use` nodes one DataFlow hop from the formal's `Def`
//! whose path base is the formal: the DFG has already decided which reads that definition
//! reaches (member-only projections, reaching definitions, kills by reassignment or by a
//! member write), and the new edge inherits that decision. A member-only formal's reads
//! carry their field path (`p.a.b`); a mixed formal's member reads are base `p` Uses, so
//! the static member chain is read back from the syntax (`p.a.b` -> `[a, b]`; any other
//! use of `p`, including a `p[k]` read, retains an empty legacy chain).
//!
//! Pairing. A leaf at key path `L` (a chain of static keys) feeds a read with chain `R`
//! when one is a prefix of the other: `L = [a]` feeds `p.a`, `p.a.x` and bare `p`, never
//! `p.b`. A nested literal contributes its leaf paths (`{ a: { b: x } }` is `L = [a, b]`),
//! so `p.a` (the container) sees `x` while `p.a.c` does not.
//!
//! Member writes. A member-only formal's reads already honour the callee's member writes
//! (the DFG drops a suppressed path). A mixed formal's reads are base `p` Uses that
//! assignment to `p.a` does not redefine, so the pass applies the member-only lane's own
//! flow-insensitive suppression itself (`ParsedFile::formal_member_facts_at`): a read whose
//! path is written, has a written prefix or has a deeper write below it is not fed, and a
//! leaf at or below a written path is not fed to any read (the write replaced that key).
//! Cost: a read that would have run before the write loses its edge. A computed-member
//! write to the formal itself (`p[k] = v`, `p["a"] = v`, `delete p[k]`, a pattern or
//! `for-in/of` target, wrappers and nested callables included) may replace any key, so it
//! is a wildcard write: no leaf of the literal feeds any read of that formal (fail closed;
//! `FormalMemberFacts::has_computed_write`, read by this pass only). A write below a key
//! (`p.a[k] = v`) and aliases of `p` are not modelled, as in the member-only lane.
//!
//! Label. The shortcut replaces the two hops `Use(v) -> Def(p) -> Use(p.<read>)`, so its
//! label is the worse of the call's label (`Exact` here) and the definition edge's own
//! label (for example `CfgIncomplete` in a one-statement callee). A `NameOnly(Killed)`
//! label keeps the definition edge's `kill_line`, which is a line of the *callee*, the
//! function the edge ends in; the argument's own function is the edge's `from`.
//!
//! Key decisions (each pinned by a test in `object_literal_arg_tests.rs`):
//! * A leaf is an identifier, a member path or a shorthand property, parenthesised or
//!   type-asserted; calls, arithmetic, arrays and function values carry no binding (the
//!   rule plain arguments already follow).
//! * Spread or a computed key (property or method) is a wildcard writer: it may overwrite
//!   any key written before it, so those keys are refused; keys after the last wildcard
//!   are definitive. The spread's own source is not bound (unknown keys).
//! * Duplicate keys: the last property for a key wins; if it is an accessor or method the
//!   key is refused. `__proto__` is never an own property and is refused.
//! * A spread argument before the literal makes the positional slot ambiguous: refused.
//! * A defaulted formal is bound exactly when Step 5b binds it for a plain argument; a
//!   literal is never `undefined`, so the default cannot replace it.

use super::{
    props_binding::{self, Counters, Leaf, PropsEntry, PropsValue, PropsWriter},
    *,
};
use crate::ast::js_ts::component_route::{js_ts_component_route, Route, SiteKind};
pub(super) use crate::ast::js_ts::props_keys::is_transparent_wrapper;
use crate::ast::js_ts::props_keys::{definite_key, strip_transparent_wrappers};
use crate::resolution::go::satisfiers::CpgCallResolver;
use tree_sitter::Node;

/// Everything one resolved call site contributes to the pass.
pub(super) struct SiteInputs<'a> {
    #[cfg(test)]
    pub call_bytes: (usize, usize),
    pub caller_id: &'a FunctionId,
    pub caller_parsed: &'a ParsedFile,
    pub callee_parsed: &'a ParsedFile,
    pub args: &'a [(String, std::ops::Range<usize>)],
    pub var_index: &'a VarIndex,
    pub exact_var_index: &'a ExactVarIndex,
    pub graph: &'a DiGraph<CpgNode, CpgEdge>,
    pub cross_file_free_single_exact: bool,
}

fn plain_identifier(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || matches!(b, b'_' | b'$'))
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'$'))
}

fn operand_path(parsed: &ParsedFile, node: Node<'_>) -> Option<AccessPath> {
    let node = strip_transparent_wrappers(node)?;
    match node.kind() {
        "identifier" | "shorthand_property_identifier" | "this" | "super" => {
            Some(AccessPath::simple(parsed.node_text(&node)))
        }
        "member_expression" => {
            let property = node.child_by_field_name("property").filter(|property| {
                matches!(
                    property.kind(),
                    "property_identifier" | "private_property_identifier"
                )
            })?;
            let mut path = operand_path(parsed, node.child_by_field_name("object")?)?;
            path.fields.push(parsed.node_text(&property).to_owned());
            Some(path)
        }
        _ => None,
    }
}

fn entries(parsed: &ParsedFile, object: Node<'_>) -> Option<Vec<PropsEntry>> {
    if object.has_error() {
        return None;
    }
    let mut entries = Vec::new();
    let mut cursor = object.walk();
    for child in object.named_children(&mut cursor) {
        let (key, value) = match child.kind() {
            "comment" => continue,
            "spread_element" => {
                entries.push(PropsEntry::Spread);
                continue;
            }
            "shorthand_property_identifier" => (child, Some(child)),
            "pair" => (
                child.child_by_field_name("key")?,
                child.child_by_field_name("value"),
            ),
            "method_definition" => (child.child_by_field_name("name")?, None),
            _ => return None,
        };
        let name = definite_key(parsed, key);
        if key.kind() == "number" {
            entries.push(match name {
                Some(key) => PropsEntry::Named(PropsWriter {
                    key,
                    legacy: false,
                    value: PropsValue::Numeric,
                }),
                None => PropsEntry::NumericWildcard,
            });
            continue;
        }
        let Some(name) = name else {
            entries.push(PropsEntry::Wildcard);
            continue;
        };
        let legacy = key.kind() != "string" || plain_identifier(&name);
        let value = if name == "__proto__" {
            PropsValue::Opaque
        } else if let Some(value) = value.and_then(strip_transparent_wrappers) {
            match value.kind() {
                "object" => PropsValue::Object(self::entries(parsed, value)?),
                "shorthand_property_identifier" | "identifier" | "member_expression" => {
                    PropsValue::Operand {
                        span: value.byte_range(),
                        path: operand_path(parsed, value),
                    }
                }
                _ => PropsValue::Opaque,
            }
        } else {
            PropsValue::Opaque
        };
        entries.push(PropsEntry::Named(PropsWriter {
            key: name,
            legacy,
            value,
        }));
    }
    Some(entries)
}

pub(super) struct Literal<'tree> {
    pub(super) position: usize,
    pub(super) safe: bool,
    object: Node<'tree>,
}

pub(super) fn literal_arguments<'tree>(
    caller: &'tree ParsedFile,
    site: &CallSite,
    args: &[spread_args::Argument],
) -> Vec<Literal<'tree>> {
    if !caller.language.is_js_family()
        || site.jsx_element
        || site.kind != crate::call_graph::CallKind::Call
    {
        return Vec::new();
    }
    let root = caller.tree.root_node();
    let mut safe = true;
    let mut literals = Vec::new();
    for (position, (text, span)) in args.iter().enumerate() {
        if !text.starts_with(['{', '(', '<', '.']) {
            continue;
        }
        let Some(node) = root.descendant_for_byte_range(span.start, span.end) else {
            continue;
        };
        if node.byte_range() != *span {
            continue;
        }
        if node.kind() == "spread_element" {
            safe = false;
        } else if let Some(object) =
            strip_transparent_wrappers(node).filter(|node| node.kind() == "object")
        {
            literals.push(Literal {
                position,
                safe,
                object,
            });
        }
    }
    literals
}

impl Literal<'_> {
    pub(super) fn entries(&self, parsed: &ParsedFile) -> Option<Vec<PropsEntry>> {
        entries(parsed, self.object)
    }
}

/// A call site binds object literals only when it resolves to one Exact callee.
pub(super) fn exact_singleton(resolved: &[ResolvedCallee<'_>]) -> bool {
    resolved.len() == 1 && resolved[0].confidence == ResolutionConfidence::Exact
}

pub(super) fn inventory<'tree>(
    caller: &'tree ParsedFile,
    site: &CallSite,
    call_args: &mut Option<(Vec<spread_args::Argument>, spread_args::Offsets)>,
    read_arguments: impl FnOnce(
        &ParsedFile,
        &CallSite,
    ) -> (Vec<spread_args::Argument>, spread_args::Offsets),
    resolved: &[ResolvedCallee<'_>],
    files: &BTreeMap<String, ParsedFile>,
    counters: &mut Counters,
) -> Vec<Literal<'tree>> {
    if !caller.language.is_js_family() || site.jsx_element {
        return Vec::new();
    }
    let (args, _) = call_args.get_or_insert_with(|| read_arguments(caller, site));
    let mut literals = literal_arguments(caller, site, args);
    if literals.is_empty() {
        return literals;
    }
    counters.add("sites_literal", 1);
    literals.retain(|literal| !literal.object.has_error());
    let exact = exact_singleton(resolved)
        && files
            .get(&resolved[0].target.file)
            .is_some_and(|file| file.language.is_js_family());
    let refusal = (!exact && !literals.is_empty()).then_some("refused_not_exact_singleton");
    #[cfg(test)]
    counters
        .attempts
        .push(super::super::props_s2_test_support::LiteralAttempt {
            caller: site.caller.clone(),
            bytes: (site.start_byte, site.end_byte),
            kind: SiteKind::Literal,
            exact,
            refusal,
        });
    if let Some(reason) = refusal {
        counters.add(reason, 1);
        return Vec::new();
    }
    literals
}

impl SiteInputs<'_> {
    fn literal_sources(&self, leaf: &Leaf) -> Vec<NodeIndex> {
        let Some(path) = &leaf.path else {
            return CodePropertyGraph::argument_nodes(
                self.caller_id,
                self.caller_parsed,
                self.caller_parsed
                    .source
                    .get(leaf.span.clone())
                    .unwrap_or("")
                    .trim(),
                &leaf.span,
                self.var_index,
                self.exact_var_index,
                self.graph,
            );
        };
        let base = AccessPath::simple(&path.base);
        std::iter::once(path)
            .chain((base != *path).then_some(&base))
            .filter_map(|path| {
                CodePropertyGraph::argument_var_node_in_span_with_exact(
                    self.caller_id,
                    self.caller_parsed,
                    path,
                    &leaf.span,
                    self.var_index,
                    self.exact_var_index,
                    self.graph,
                )
            })
            .collect()
    }
}

fn pattern_admission(
    inputs: &SiteInputs<'_>,
    call: &CallSite,
    resolver: &impl CpgCallResolver,
    callee: &FunctionId,
    callable: (usize, usize),
) -> Result<Route, (u8, &'static str)> {
    let lexical = inputs
        .caller_parsed
        .tree
        .root_node()
        .descendant_for_byte_range(call.start_byte, call.end_byte)
        .filter(|node| {
            node.byte_range() == (call.start_byte..call.end_byte)
                && node.kind() == "call_expression"
        })
        .ok_or((3, "refused_target_identity"))?;
    let value = lexical
        .child_by_field_name("function")
        .and_then(strip_transparent_wrappers)
        .ok_or((3, "refused_target_identity"))?;
    let qualified = matches!(value.kind(), "member_expression" | "subscript_expression")
        || call.qualifier.is_some();
    let export = resolver.js_ts_props_export_matches(call, callee, inputs.callee_parsed);
    if qualified {
        let simple_namespace = value.kind() == "member_expression"
            && value.child_by_field_name("object").is_some_and(|node| {
                node.kind() == "identifier"
                    && Some(inputs.caller_parsed.node_text(&node)) == call.qualifier.as_deref()
            })
            && value.child_by_field_name("property").is_some_and(|node| {
                node.kind() == "property_identifier"
                    && inputs.caller_parsed.node_text(&node) == call.callee_name
            })
            && matches!(
                call.local_binding,
                crate::call_graph::JsLocalBinding::NamespaceImport { .. }
            );
        if !simple_namespace {
            return Err((2, "refused_member_qualifier"));
        }
        if !export {
            return Err((2, "refused_member_not_export"));
        }
    }
    let selected = inputs
        .callee_parsed
        .tree
        .root_node()
        .descendant_for_byte_range(callable.0, callable.1)
        .filter(|node| node.byte_range() == (callable.0..callable.1))
        .ok_or((3, "refused_target_identity"))?;
    let route = js_ts_component_route(
        SiteKind::Literal,
        inputs.caller_parsed,
        lexical,
        inputs.callee_parsed,
        selected,
        export,
    );
    match route {
        Route::Direct | Route::LocalHookOperand => Ok(route),
        Route::UnknownWrapper | Route::JsxRenderOperand => Err((3, "refused_unknown_wrapper")),
        Route::PreInitializerUnproved => Err((3, "refused_pre_initializer_unproved")),
        Route::UnprovedIdentity => Err((3, "refused_target_identity")),
    }
}

/// Emit literal edges immediately before ordinary positional binding.
pub(super) fn collect<T, P, F>(
    output: &mut props_binding::Step5bOutput,
    literals: &[Literal<'_>],
    site: &SiteInputs<'_>,
    call: &CallSite,
    resolver: &impl CpgCallResolver,
    callee: &FunctionId,
    params: &[Option<T>],
    admission: &bare_param_admission::Admission,
    bare_slot: impl Fn(&T) -> Option<callee_params::BareSlot>,
    resolve: impl Fn(&T) -> Option<NodeIndex>,
    patterns: impl FnOnce() -> F,
    as_patterns: impl Fn(&F) -> Option<&callee_params::PatternFacts<P>>,
    resolve_pattern: impl Fn(&P) -> Option<NodeIndex>,
) {
    use props_binding::{PropsTarget, SiteReadRefusals};

    if literals.is_empty() {
        return;
    }
    let mut get_patterns = Some(patterns);
    let mut facts = None;
    let mut route = None;
    let mut refusals = Vec::new();
    let mut read_refusals = SiteReadRefusals::default();
    for literal in literals {
        if !literal.safe {
            output.props.add("refused_argument_after_spread", 1);
            continue;
        }
        let position = literal.position;
        let plain = params.get(position).and_then(Option::as_ref);
        let target = if let Some(param) = plain {
            let bare = bare_slot(param);
            if !admission.literal(bare) {
                // The incoming bare-formal admission skips this position; the
                // site still counts one reason (check 3).
                if bare.is_some() {
                    refusals.push((position, true, 3, "refused_bare_formal_skipped"));
                }
                continue;
            }
            let Some(def) = resolve(param) else {
                continue;
            };
            PropsTarget::Plain {
                def,
                bare_literal: bare.is_some(),
            }
        } else {
            if let Some(get) = get_patterns.take() {
                facts = Some(get());
            }
            let Some(facts) = facts.as_ref().and_then(&as_patterns) else {
                refusals.push((position, true, 3, "refused_target_identity"));
                continue;
            };
            let Some(pattern) = facts
                .positions
                .iter()
                .find(|pattern| pattern.syntax.position == position)
            else {
                refusals.push((position, true, 4, "refused_no_props_formal"));
                continue;
            };
            if let Err((check, refusal)) = *route.get_or_insert_with(|| {
                pattern_admission(site, call, resolver, callee, facts.callable)
            }) {
                refusals.push((position, true, check, refusal));
                continue;
            }
            if pattern.syntax.alias_skipped {
                refusals.push((position, true, 4, "refused_alias_pattern"));
                continue;
            }
            PropsTarget::Pattern(pattern)
        };
        let Some(writes) = literal.entries(site.caller_parsed) else {
            continue;
        };
        let rich = props_binding::rich_leaves(&writes, &mut output.props);
        let legacy;
        let leaves = match target {
            PropsTarget::Plain { .. } => {
                legacy = props_binding::legacy_leaves(&writes);
                &legacy
            }
            PropsTarget::Pattern(_) => &rich,
        };
        props_binding::bind(
            output,
            &mut read_refusals,
            site,
            target,
            leaves,
            &rich,
            |leaf| site.literal_sources(leaf),
            &resolve_pattern,
        );
    }
    if let Some(refusal) = props_binding::literal_site_refusal(&refusals) {
        output.props.add(refusal, 1);
        #[cfg(test)]
        if let Some(attempt) = output.props.attempts.last_mut() {
            attempt.refusal = Some(refusal);
        }
    }
    read_refusals.record(&mut output.props);
}

#[cfg(test)]
#[path = "object_literal_parity_tests.rs"]
mod parity_tests;

#[cfg(test)]
#[path = "object_literal_args_coverage_tests.rs"]
mod coverage_tests;
