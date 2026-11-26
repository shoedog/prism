//! JS-family argument evidence for source parameter calls. The resolver never
//! reads this index in E1. Each slot is computed from an empty stack and admits
//! at most two holder-caller lookups per path.
//!
//! Empty outcomes store the first proof failure in DESIGN §5 E order: site
//! and Source grounding, holder stability, position and selected value,
//! terminal identity, forwarding and fanout. Each refused contribution is
//! counted even when another path supplies a target for the same site.

use crate::ast::js_ts::param_slot::{self, Parameter, Selection};
use crate::ast::{JsBindingCache, ParsedFile};
use crate::call_graph::{
    CallGraph, CallKind, CallSite, CallSiteOrigin, FunctionId, JsLocalBinding,
};
use crate::resolution::{ResolutionConfidence, ResolutionOutcome};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, MutexGuard, OnceLock};
use tree_sitter::Node;

#[cfg(test)]
mod arg_follow_tests;
pub(crate) mod diagnostic;
mod identity;
mod stability;
mod value;

const MAX_DEPTH: usize = 2;
const MAX_TARGETS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) enum PathStep {
    ObjectKey(String),
    ArrayIndex(usize),
    Computed((usize, usize)),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct ParamSlot {
    pub(crate) holder: FunctionId,
    pub(crate) position: usize,
    pub(crate) path: Vec<PathStep>,
    pub(crate) declaration: (usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ArgOutcome {
    Targets(BTreeSet<FunctionId>),
    Fanout(usize),
    Empty(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct SiteKey(String, usize, usize, String);

impl From<&CallSite> for SiteKey {
    fn from(site: &CallSite) -> Self {
        Self(
            site.caller.file.clone(),
            site.start_byte,
            site.end_byte,
            site.effective_source_callee_name().into(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Fact {
    slot: ParamSlot,
    outcome: ArgOutcome,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ArgIndex {
    sites: BTreeMap<SiteKey, Fact>,
    stats: Stats,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Stats {
    eligible_sites: usize,
    empty: usize,
    target_sites: usize,
    fanout: usize,
    reasons: BTreeMap<String, usize>,
    empty_reasons: BTreeMap<String, usize>,
    parameter_shape: BTreeMap<String, usize>,
    ground_depth: BTreeMap<usize, usize>,
    holder_exported: usize,
    holder_escapes: usize,
}

impl ArgIndex {
    #[allow(dead_code)] // Read by the E2 route.
    pub(crate) fn lookup(&self, site: &CallSite) -> Option<&ArgOutcome> {
        self.sites
            .get(&SiteKey::from(site))
            .map(|fact| &fact.outcome)
    }

    pub(crate) fn stats(&self) -> &Stats {
        &self.stats
    }

    #[cfg(test)]
    fn refusal_count(&self, reason: &str) -> usize {
        self.stats.reasons.get(reason).copied().unwrap_or_default()
    }
}

struct Context<'a> {
    cg: &'a CallGraph,
    files: &'a BTreeMap<String, ParsedFile>,
    bindings: BTreeMap<&'a str, FileBindings<'a>>,
    owners: BTreeMap<FunctionId, Vec<Node<'a>>>,
    callers: BTreeMap<FunctionId, Vec<&'a CallSite>>,
    exports: BTreeSet<FunctionId>,
    escapes: BTreeSet<FunctionId>,
    #[cfg(test)]
    supplied: Option<SuppliedCall<'a>>,
}

struct FileBindings<'a> {
    cache: Mutex<JsBindingCache<'a>>,
    eval_sites: OnceLock<identity::EvalSites<'a>>,
}

impl<'a> FileBindings<'a> {
    fn cache(&self) -> MutexGuard<'_, JsBindingCache<'a>> {
        self.cache.lock().expect("argument-follow binding cache")
    }
}

#[cfg(test)]
struct SuppliedCall<'a> {
    site: &'a CallSite,
    outcome: ResolutionOutcome<'a>,
    arguments: Option<&'a [(Node<'a>, std::ops::Range<usize>)]>,
}

struct Candidate<'a> {
    site: &'a CallSite,
    parameter: Parameter<'a>,
    slot: ParamSlot,
}

#[derive(Default)]
struct Contributions {
    targets: BTreeMap<FunctionId, (usize, usize)>,
    reasons: Vec<&'static str>,
    depths: BTreeMap<usize, usize>,
}

pub(crate) fn compute(cg: &CallGraph, files: &BTreeMap<String, ParsedFile>) -> ArgIndex {
    compute_index(
        cg,
        files,
        #[cfg(test)]
        None,
    )
}

fn compute_index<'a>(
    cg: &'a CallGraph,
    files: &'a BTreeMap<String, ParsedFile>,
    #[cfg(test)] supplied: Option<SuppliedCall<'a>>,
) -> ArgIndex {
    let mut by_file: BTreeMap<_, _> = files
        .iter()
        .filter(|(_, parsed)| parsed.language.is_js_family())
        .map(|(file, parsed)| (file.as_str(), (parsed, Vec::new())))
        .collect();
    if by_file.is_empty() {
        return ArgIndex::default();
    }
    for (order, site) in cg.calls.values().flatten().enumerate() {
        if source(site) && site.qualifier.is_none() {
            if let Some((_, sites)) = by_file.get_mut(site.caller.file.as_str()) {
                sites.push((order, site));
            }
        }
    }

    let discoveries: Vec<_> = by_file
        .into_par_iter()
        .map(|(file, (parsed, sites))| {
            let mut cache = JsBindingCache::for_arg_follow();
            let candidates: Vec<_> = sites
                .into_iter()
                .filter_map(|(order, site)| {
                    discover_candidate(parsed, file, site, &mut cache)
                        .map(|candidate| (order, candidate))
                })
                .collect();
            (file, cache, candidates)
        })
        .collect();
    let mut bindings = BTreeMap::new();
    let mut ordered_candidates = Vec::new();
    for (file, cache, candidates) in discoveries {
        bindings.insert(
            file,
            FileBindings {
                cache: Mutex::new(cache),
                eval_sites: OnceLock::new(),
            },
        );
        ordered_candidates.extend(candidates);
    }
    if ordered_candidates.is_empty() {
        return ArgIndex::default();
    }
    // Preserve which occurrence wins when multiple candidates share a SiteKey.
    ordered_candidates.sort_unstable_by_key(|(order, _)| *order);
    let candidates: Vec<_> = ordered_candidates
        .into_iter()
        .map(|(_, candidate)| candidate)
        .collect();

    let mut context = Context {
        cg,
        files,
        bindings,
        owners: BTreeMap::new(),
        callers: BTreeMap::new(),
        exports: BTreeSet::new(),
        escapes: BTreeSet::new(),
        #[cfg(test)]
        supplied,
    };
    context.index_owners();
    context.index_callers();
    context.index_escapes(&candidates);
    let mut index = ArgIndex::default();
    let output = diagnostic::enabled();
    for candidate in candidates {
        let key = SiteKey::from(candidate.site);
        if index.sites.contains_key(&key) {
            continue;
        }
        let mut result = Contributions::default();
        let diagnostic = diagnostic::Site::new(candidate.site, output.as_ref());
        context.follow(
            &candidate.parameter,
            &candidate.slot,
            &mut Vec::new(),
            &mut result,
            &diagnostic,
            None,
        );
        if result.targets.len() > MAX_TARGETS {
            diagnostic.report(&candidate.parameter, &candidate.slot, None, None, "fanout");
        }
        index.record(key, candidate, &context, result);
    }
    index
}

fn discover_candidate<'a>(
    parsed: &'a ParsedFile,
    file: &str,
    site: &'a CallSite,
    cache: &mut JsBindingCache<'a>,
) -> Option<Candidate<'a>> {
    let callee = param_slot::peel(invocation(call_node(parsed, site)?, site)?)?;
    if site.jsx_element
        && parsed
            .node_text(&callee)
            .chars()
            .next()
            .is_some_and(char::is_lowercase)
    {
        return None;
    }
    let parameter = param_slot::parameter(parsed, callee, cache)?;
    let slot = slot(parsed, file, &parameter)?;
    Some(Candidate {
        site,
        parameter,
        slot,
    })
}

impl ArgIndex {
    fn record(
        &mut self,
        key: SiteKey,
        candidate: Candidate<'_>,
        context: &Context<'_>,
        result: Contributions,
    ) {
        let stats = &mut self.stats;
        stats.eligible_sites += 1;
        stats.holder_exported += usize::from(context.exports.contains(&candidate.slot.holder));
        stats.holder_escapes += usize::from(context.escapes.contains(&candidate.slot.holder));
        let shape = if candidate.parameter.body_read.is_some() {
            "body_destructure"
        } else if !candidate.slot.path.is_empty() {
            "entry_destructure"
        } else if candidate
            .parameter
            .selected
            .iter()
            .any(|step| matches!(step, Selection::Default(_)))
        {
            "default"
        } else {
            "plain"
        };
        *stats.parameter_shape.entry(shape.into()).or_default() += 1;
        for reason in &result.reasons {
            *stats.reasons.entry((*reason).into()).or_default() += 1;
        }
        for (depth, count) in result.depths {
            *stats.ground_depth.entry(depth).or_default() += count;
        }
        let outcome = match result.targets.len() {
            0 => {
                let reason = result
                    .reasons
                    .iter()
                    .min_by_key(|reason| proof_order(reason))
                    .copied()
                    .unwrap_or("no_target");
                stats.empty += 1;
                *stats.empty_reasons.entry(reason.into()).or_default() += 1;
                ArgOutcome::Empty(reason.into())
            }
            count if count > MAX_TARGETS => {
                stats.fanout += 1;
                *stats.reasons.entry("fanout".into()).or_default() += 1;
                ArgOutcome::Fanout(count)
            }
            _ => {
                stats.target_sites += 1;
                ArgOutcome::Targets(result.targets.into_keys().collect())
            }
        };
        self.sites.insert(
            key,
            Fact {
                slot: candidate.slot,
                outcome,
            },
        );
    }
}

fn proof_order(reason: &str) -> u8 {
    match reason {
        "source" | "holder_outcome" | "holder_identity" | "no_caller" => 0,
        "entry_write" | "dynamic_binding" | "mapped_arguments" | "stale_path" => 1,
        "unknown_spread" | "path_step" | "hidden_literal_key" | "jsx_attribute"
        | "undefined_lookup" => 2,
        "opaque_terminal" | "stale_terminal" | "callable_collision" => 3,
        "depth" | "cycle" => 4,
        _ => 5,
    }
}

impl<'a> Context<'a> {
    fn follow(
        &self,
        parameter: &Parameter<'a>,
        slot: &ParamSlot,
        stack: &mut Vec<ParamSlot>,
        result: &mut Contributions,
        diagnostic: &diagnostic::Site<'_>,
        relay: Option<&CallSite>,
    ) {
        let refuse = |result: &mut Contributions, call, reason| {
            result.reasons.push(reason);
            diagnostic.report(parameter, slot, call, None, reason);
        };
        if stack.contains(slot) {
            refuse(result, relay, "cycle");
            return;
        }
        if stack.len() == MAX_DEPTH {
            refuse(result, relay, "depth");
            return;
        }
        let (caller_file, holder_file) = self
            .files
            .get_key_value(&slot.holder.file)
            .expect("indexed holder file");
        if !self.unique_owner(&slot.holder, parameter.owner) {
            refuse(result, relay, "holder_identity");
            return;
        }
        let Some(callers) = self.callers.get(&slot.holder) else {
            refuse(result, relay, "no_caller");
            return;
        };
        stack.push(slot.clone());
        for call in callers {
            let ground = self.ground(slot, parameter, call);
            let (parsed, node) = match ground {
                Ok(ground) => ground,
                Err(reason) => {
                    refuse(result, Some(call), reason);
                    continue;
                }
            };
            if let Some(reason) = self.stability(holder_file, caller_file, parameter) {
                refuse(result, Some(call), reason);
                continue;
            }
            for value in self.values_of(caller_file, parameter, parsed, call, node) {
                match value.and_then(|value| self.terminal(value)) {
                    Ok(identity::Terminal::Callable(target, bytes)) => {
                        diagnostic.report(
                            parameter,
                            slot,
                            Some(call),
                            Some((&target, bytes)),
                            "witnessed_callable",
                        );
                        result.targets.insert(target, bytes);
                        *result.depths.entry(stack.len()).or_default() += 1;
                    }
                    Ok(identity::Terminal::Parameter(file, parameter)) => {
                        let Some(next) = self::slot(&self.files[file], file, &parameter) else {
                            refuse(result, Some(call), "holder_identity");
                            continue;
                        };
                        self.follow(&parameter, &next, stack, result, diagnostic, Some(call));
                    }
                    Err(reason) => refuse(result, Some(call), reason),
                }
            }
        }
        stack.pop();
    }

    fn ground(
        &self,
        slot: &ParamSlot,
        parameter: &Parameter<'a>,
        call: &'a CallSite,
    ) -> Result<(&'a ParsedFile, Node<'a>), &'static str> {
        if !source(call) {
            return Err("source");
        }
        let parsed = &self.files[&call.caller.file];
        let node = call_node(parsed, call).ok_or("source")?;
        let resolved = self.resolve_caller(call);
        let [target] = resolved.resolved.as_slice() else {
            return Err("holder_outcome");
        };
        if resolved.drop.is_some()
            || target.confidence != ResolutionConfidence::Exact
            || target.target != &slot.holder
        {
            return Err("holder_outcome");
        }
        let witness = self.holder_at(node, call)?;
        if witness.0 != slot.holder || witness.1 != bytes(parameter.owner) {
            return Err("holder_identity");
        }
        Ok((parsed, node))
    }

    fn resolve_caller(&self, call: &CallSite) -> ResolutionOutcome<'a> {
        #[cfg(test)]
        if let Some(supplied) = self.supplied_call(call) {
            return supplied.outcome.clone();
        }
        self.cg.resolve_call_site_full(call)
    }

    #[cfg(test)]
    fn supplied_call(&self, call: &CallSite) -> Option<&SuppliedCall<'a>> {
        self.supplied
            .as_ref()
            .filter(|supplied| supplied.site == call)
    }

    fn holder_at(
        &self,
        node: Node<'a>,
        call: &CallSite,
    ) -> Result<(FunctionId, (usize, usize)), &'static str> {
        let callee = caller_implementation(node, call)
            .and_then(param_slot::peel)
            .ok_or("source")?;
        #[cfg(test)]
        if node.kind() == "new_expression"
            && self
                .supplied_call(call)
                .is_some_and(|input| input.arguments.is_some())
        {
            return self
                .supplied_constructor(&call.caller.file, callee)
                .ok_or("holder_identity");
        }
        self.callable_at(&call.caller.file, callee, call.jsx_element)
            .map_err(|_| "holder_identity")
    }
}

fn source(site: &CallSite) -> bool {
    site.origin == CallSiteOrigin::Source
        && site.kind == CallKind::Call
        && site.pre_resolved_target.is_none()
}

fn invocation<'a>(node: Node<'a>, site: &CallSite) -> Option<Node<'a>> {
    match node.kind() {
        "call_expression"
            if node
                .child_by_field_name("arguments")
                .is_some_and(|args| args.kind() == "arguments") =>
        {
            node.child_by_field_name("function")
        }
        "jsx_opening_element" | "jsx_self_closing_element" if site.jsx_element => {
            node.child_by_field_name("name")
        }
        _ => None,
    }
}

fn caller_implementation<'a>(node: Node<'a>, site: &CallSite) -> Option<Node<'a>> {
    if node.kind() == "new_expression" {
        node.child_by_field_name("constructor")
    } else {
        invocation(node, site)
    }
}

fn call_node<'a>(parsed: &'a ParsedFile, site: &CallSite) -> Option<Node<'a>> {
    let mut node = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(site.start_byte, site.end_byte)?;
    while bytes(node) == (site.start_byte, site.end_byte) {
        if matches!(
            node.kind(),
            "call_expression"
                | "new_expression"
                | "jsx_opening_element"
                | "jsx_self_closing_element"
        ) {
            return Some(node);
        }
        node = node.parent()?;
    }
    None
}

fn slot(parsed: &ParsedFile, file: &str, parameter: &Parameter<'_>) -> Option<ParamSlot> {
    let holder = crate::anon_owner::identity(parsed, &parameter.owner, file)?;
    let path = parameter
        .selected
        .iter()
        .filter_map(|step| match step {
            Selection::Object(key) => Some(PathStep::ObjectKey(key.clone())),
            Selection::Array(index) => Some(PathStep::ArrayIndex(*index)),
            Selection::Computed(node) => Some(PathStep::Computed(bytes(*node))),
            Selection::Default(_) | Selection::Rest => None,
        })
        .collect();
    Some(ParamSlot {
        holder,
        position: parameter.position,
        path,
        declaration: bytes(parameter.binding),
    })
}

fn bytes(node: Node<'_>) -> (usize, usize) {
    (node.start_byte(), node.end_byte())
}
