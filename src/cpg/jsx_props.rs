//! Step 5b binding for recorded JSX openings against the immutable caller graph.

use super::{
    object_literal_args,
    props_binding::{
        self, BindingSite, Leaf, PropsEntry, PropsTarget, SiteReadRefusals, Step5bOutput,
    },
    *,
};
use crate::ast::js_ts::component_route::{js_ts_component_route, Route, SiteKind};
use crate::resolution::go::satisfiers::CpgCallResolver;
use std::ops::Range;
use tree_sitter::Node;

#[path = "jsx_props/read.rs"]
mod read;
#[path = "jsx_props/source.rs"]
mod source;
#[path = "jsx_props/syntax.rs"]
mod syntax;

pub(in crate::cpg::build) use read::{
    jsx_plain_read, FormalReadContext, PlainRead, ReachedRead, ReadRefusal,
};
pub(in crate::cpg::build) use source::SourceRefusal;
use syntax::{attributes, entries, opening_at};

pub(super) struct JsxSite<'a> {
    call: &'a CallSite,
    caller: &'a ParsedFile,
    graph: &'a DiGraph<CpgNode, CpgEdge>,
    lookup: &'a VarIndex,
    #[cfg(test)]
    fall_through_on_full_conflict: bool,
}

impl<'a> JsxSite<'a> {
    pub(super) fn new(
        call: &'a CallSite,
        caller: &'a ParsedFile,
        graph: &'a DiGraph<CpgNode, CpgEdge>,
        lookup: &'a VarIndex,
    ) -> Self {
        Self {
            call,
            caller,
            graph,
            lookup,
            #[cfg(test)]
            fall_through_on_full_conflict: false,
        }
    }

    fn span(&self) -> Range<usize> {
        self.call.start_byte..self.call.end_byte
    }
}

pub(super) enum Patterns<'a, 'files> {
    Cached(&'a callee_params::ParamCache<'files>),
    #[cfg(test)]
    Direct(
        &'files BTreeMap<String, ParsedFile>,
        &'a mut BTreeMap<FunctionId, Option<callee_params::PatternFacts<NodeIndex>>>,
    ),
}

pub(super) fn collect_site(
    output: &mut Step5bOutput,
    site: JsxSite<'_>,
    resolver: &impl CpgCallResolver,
    resolved: &[ResolvedCallee<'_>],
    exact: &ExactVarIndex,
    patterns: Patterns<'_, '_>,
) -> bool {
    if !site.call.jsx_element {
        return false;
    }
    if !site.caller.language.is_js_family()
        || crate::anon_owner::is_module_owner_name(&site.call.caller.name)
        || (site.call.qualifier.is_none()
            && (site
                .call
                .callee_name
                .starts_with(|c: char| c.is_ascii_lowercase())
                || site.call.callee_name.contains(['-', ':'])))
    {
        return true;
    }
    let Some(opening) = opening_at(site.caller, site.span()) else {
        return true;
    };
    if attributes(opening).is_empty()
        || syntax::execution_owner(site.caller, opening).is_none_or(|owner| {
            crate::anon_owner::identity(site.caller, &owner, &site.call.caller.file).as_ref()
                != Some(&site.call.caller)
        })
    {
        return true;
    }
    output.props.add("sites_jsx", 1);
    let files = match &patterns {
        Patterns::Cached(cache) => cache.files,
        #[cfg(test)]
        Patterns::Direct(files, _) => *files,
    };
    let selected = object_literal_args::exact_singleton(resolved)
        .then(|| resolved[0].target)
        .and_then(|target| {
            let file = files.get(&target.file)?;
            file.language.is_js_family().then_some((target, file))
        });
    let (exact, result) = if let Some((callee, file)) = selected {
        match patterns {
            Patterns::Cached(cache) => {
                let cell = cache.patterns(callee, file, exact, site.graph);
                let facts = cell.get().and_then(Option::as_ref);
                (
                    facts.is_some(),
                    admit_and_collect(
                        output,
                        &site,
                        opening,
                        resolver,
                        callee,
                        file,
                        facts,
                        |target| target.resolve(exact, site.graph),
                    ),
                )
            }
            #[cfg(test)]
            Patterns::Direct(_, cache) => {
                let facts = cache
                    .entry(callee.clone())
                    .or_insert_with(|| {
                        callee_params::compute_pattern_formal_nodes(file, callee, exact, site.graph)
                    })
                    .as_ref();
                (
                    facts.is_some(),
                    admit_and_collect(
                        output,
                        &site,
                        opening,
                        resolver,
                        callee,
                        file,
                        facts,
                        |node| Some(*node),
                    ),
                )
            }
        }
    } else {
        (false, Err("refused_not_exact_singleton"))
    };
    if let Err(reason) = result {
        output.props.add(reason, 1);
    }
    #[cfg(test)]
    output
        .props
        .attempts
        .push(super::super::props_s2_test_support::LiteralAttempt {
            caller: site.call.caller.clone(),
            bytes: (site.call.start_byte, site.call.end_byte),
            kind: SiteKind::Jsx,
            exact,
            refusal: result.err(),
        });
    #[cfg(not(test))]
    let _ = exact;
    true
}

fn admit_and_collect<T>(
    output: &mut Step5bOutput,
    site: &JsxSite<'_>,
    opening: Node<'_>,
    resolver: &impl CpgCallResolver,
    callee: &FunctionId,
    file: &ParsedFile,
    facts: Option<&callee_params::PatternFacts<T>>,
    resolve: impl Fn(&T) -> Option<NodeIndex>,
) -> Result<(), &'static str> {
    let facts = facts.ok_or("refused_not_exact_singleton")?;
    let callable = syntax::exact_node(file, facts.callable.0..facts.callable.1)
        .ok_or("refused_not_exact_singleton")?;
    let tag = opening
        .child_by_field_name("name")
        .ok_or("refused_target_identity")?;
    let qualified = tag.kind() == "member_expression" || site.call.qualifier.is_some();
    if qualified {
        let simple = tag.kind() == "member_expression"
            && tag.child_by_field_name("object").is_some_and(|object| {
                object.kind() == "identifier"
                    && Some(site.caller.node_text(&object)) == site.call.qualifier.as_deref()
            })
            && tag.child_by_field_name("property").is_some_and(|property| {
                property.kind() == "property_identifier"
                    && site.caller.node_text(&property) == site.call.callee_name
            })
            && matches!(
                site.call.local_binding,
                crate::call_graph::JsLocalBinding::NamespaceImport { .. }
            );
        if !simple {
            return Err("refused_member_qualifier");
        }
    }
    let export = resolver.js_ts_props_export_matches(site.call, callee, file);
    if qualified && !export {
        return Err("refused_member_not_export");
    }
    match js_ts_component_route(SiteKind::Jsx, site.caller, opening, file, callable, export) {
        Route::Direct | Route::JsxRenderOperand => {}
        Route::UnknownWrapper | Route::LocalHookOperand => return Err("refused_unknown_wrapper"),
        Route::PreInitializerUnproved => return Err("refused_pre_initializer_unproved"),
        Route::UnprovedIdentity => return Err("refused_target_identity"),
    }
    let target = if let Some(pattern) = facts
        .positions
        .iter()
        .find(|pattern| pattern.syntax.position == 0)
    {
        if pattern.syntax.alias_skipped {
            return Err("refused_alias_pattern");
        }
        PropsTarget::Pattern(pattern)
    } else {
        let def =
            callee_params::formal_def_nodes(file, &callable, callee, site.lookup, site.graph, true)
                .and_then(|slots| slots.first().copied().flatten())
                .ok_or("refused_no_props_formal")?;
        PropsTarget::Plain {
            def,
            bare_literal: false,
        }
    };
    if let Some(writes) = entries(site.caller, site.span()) {
        collect(output, site, file, target, &writes, resolve);
    }
    Ok(())
}

fn collect<T>(
    output: &mut Step5bOutput,
    site: &JsxSite<'_>,
    callee: &ParsedFile,
    target: PropsTarget<'_, T>,
    entries: &[PropsEntry],
    resolve: impl Fn(&T) -> Option<NodeIndex>,
) {
    let binding = BindingSite {
        kind: SiteKind::Jsx,
        #[cfg(test)]
        bytes: (site.call.start_byte, site.call.end_byte),
        graph: site.graph,
        callee,
        // A JSX edge needs positive identity, which excludes cross-file spelling fallback.
        cross_file_free_single_exact: false,
    };
    let leaves = props_binding::rich_leaves(entries, &mut output.props);
    let mut refusals = SiteReadRefusals::default();
    let mut sources = source::Sources::new(site);
    props_binding::bind(
        output,
        &mut refusals,
        &binding,
        target,
        &leaves,
        &leaves,
        |leaf: &Leaf| sources.source_node(leaf).map(|node| vec![node]),
        resolve,
    );
    refusals.record(&mut output.props);
}

#[cfg(test)]
impl CodePropertyGraph {
    pub(in crate::cpg) fn replay_jsx_props(
        &self,
        file: &ParsedFile,
        opening: Range<usize>,
        graph: &DiGraph<CpgNode, CpgEdge>,
        lookup: &VarIndex,
        target: NodeIndex,
        fall_through_on_full_conflict: bool,
    ) -> Vec<PendingEdge> {
        let Some(call) = self.call_graph.calls.values().flatten().find(|call| {
            call.caller.file == file.path
                && (call.start_byte..call.end_byte) == opening
                && call.jsx_element
        }) else {
            return Vec::new();
        };
        let mut site = JsxSite::new(call, file, graph, lookup);
        site.fall_through_on_full_conflict = fall_through_on_full_conflict;
        let Some(entries) = entries(file, opening) else {
            return Vec::new();
        };
        let resolved = self.call_graph.resolve_call_site(call);
        let [resolved] = resolved.as_slice() else {
            return Vec::new();
        };
        let exact = CodePropertyGraph::exact_var_index_from_graph(graph);
        let facts =
            callee_params::compute_pattern_formal_nodes(file, resolved.target, &exact, graph)
                .unwrap();
        let pattern = facts
            .positions
            .iter()
            .find(|position| position.syntax.position == 0)
            .unwrap();
        let mut output = Step5bOutput::default();
        collect(
            &mut output,
            &site,
            file,
            PropsTarget::Pattern(pattern),
            &entries,
            |node| (*node == target).then_some(*node),
        );
        output.flows.into_iter().map(|flow| flow.edge).collect()
    }
}
