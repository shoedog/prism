//! Preconditions for claims that cannot be inferred from a final missing edge.

use super::props_s2_test_support::Assembly;
use super::{CodePropertyGraph, CpgEdge, CpgNode, VarAccess};
use crate::call_graph::JsLocalBinding;
use crate::resolution::{ResolutionConfidence, ResolutionKind};
use petgraph::{
    graph::{DiGraph, EdgeIndex, NodeIndex},
    visit::EdgeRef,
};
use std::collections::BTreeSet;

pub(super) fn needs_capture(id: &str) -> bool {
    matches!(
        id,
        "d2-literal-selected-read"
            | "d2-jsx-selected-read"
            | "d2-forward-jsx"
            | "d2-guard-bare-arrow-literal"
            | "d2-guard-rest-literal"
            | "d2-guard-bare-arrow-jsx"
            | "d2-guard-rest-jsx"
    )
}

fn resolved(
    graph: &CodePropertyGraph,
    caller: &str,
    callee: &str,
    target_file: &str,
    kind: Option<ResolutionKind>,
) -> Result<String, String> {
    let sites: Vec<_> = graph
        .call_graph
        .calls
        .iter()
        .filter(|(owner, _)| owner.name == caller)
        .flat_map(|(_, sites)| sites)
        .filter(|site| site.callee_name == callee)
        .collect();
    let [site] = sites.as_slice() else {
        return Err(format!("expected one recorded site: {sites:?}"));
    };
    let targets = graph.call_graph.resolve_call_site(site);
    let [target] = targets.as_slice() else {
        return Err(format!("resolver returned {targets:?}"));
    };
    if target.confidence != ResolutionConfidence::Exact
        || target.target.file != target_file
        || target.target.name != callee
        || kind.is_some_and(|kind| target.kind != kind)
    {
        return Err(format!("unexpected target {target:?}"));
    }
    Ok(format!("{target:?}"))
}

fn export_refusal(graph: &CodePropertyGraph, ext: &str) -> Result<String, String> {
    let target = resolved(
        graph,
        "A",
        "send",
        &format!("ctx.{ext}"),
        Some(ResolutionKind::ImportQualified),
    )?;
    let site = graph
        .call_graph
        .calls
        .iter()
        .filter(|(id, _)| id.name == "A")
        .flat_map(|(_, sites)| sites)
        .find(|site| site.callee_name == "send")
        .unwrap();
    if site.qualifier.as_deref() != Some("api")
        || !matches!(site.local_binding, JsLocalBinding::NamespaceImport { .. })
    {
        return Err(format!("value namespace qualifier missing: {site:?}"));
    }
    let candidate = graph.call_graph.resolve_call_site(site)[0].target;
    if graph
        .call_graph
        .js_ts_namespace_export_matches(site, candidate)
    {
        return Err(format!("candidate unexpectedly has export proof: {target}"));
    }
    Ok(target)
}

fn variable(
    graph: &DiGraph<CpgNode, CpgEdge>,
    owner: &str,
    name: &str,
    access: VarAccess,
    bytes: Option<[usize; 2]>,
) -> Result<NodeIndex, String> {
    let nodes: Vec<_> = graph.node_indices().filter(|&idx| {
        matches!(&graph[idx], CpgNode::Variable { function, path, access: actual, start_byte, end_byte, .. }
            if function == owner && path.to_string() == name && *actual == access
                && bytes.is_none_or(|want| want == [*start_byte, *end_byte]))
    }).collect();
    match nodes.as_slice() {
        [node] => Ok(*node),
        _ => Err(format!(
            "expected one {owner}:{name} {access:?} {bytes:?}, found {nodes:?}"
        )),
    }
}

fn has_flow(graph: &DiGraph<CpgNode, CpgEdge>, from: NodeIndex, to: NodeIndex) -> bool {
    graph
        .edges_connecting(from, to)
        .any(|edge| edge.weight().is_data_flow())
}

fn selected_shadow(case: &str, assembly: &Assembly) -> Result<String, String> {
    let bytes = match case {
        "bare-shadow" => [45, 46],
        "member-shadow" => [69, 70],
        "bare-formal" => [26, 27],
        "member-formal" => [50, 51],
        _ => return Err(format!("unknown selected shadow case {case}")),
    };
    let pre = &assembly.pre_binding.as_ref().unwrap().graph;
    let formal = variable(pre, "P", "p", VarAccess::Def, Some([11, 12]))?;
    let selected = variable(pre, "P", "p", VarAccess::Use, Some(bytes))?;
    if !has_flow(pre, formal, selected) {
        return Err(format!(
            "formal does not reach the actual selected Use at {bytes:?}"
        ));
    }
    Ok(format!("pre-Step-5b formal reaches {bytes:?}"))
}

fn guard_seed(
    graph: &CodePropertyGraph,
    ext: &str,
    case: &str,
    assembly: &Assembly,
) -> Result<String, String> {
    let foreign = case == "foreign";
    let kind = if foreign {
        ResolutionKind::FreeSingle
    } else {
        ResolutionKind::ImportMember
    };
    let target = resolved(graph, "D", "Ghost", &format!("other.{ext}"), Some(kind))?;
    let before = assembly.guard_input.as_ref().unwrap();
    let use_node = variable(&before.graph, "D", "a", VarAccess::Use, None)?;
    let ghost = variable(&before.graph, "Ghost", "g", VarAccess::Def, None)?;
    let pending = assembly
        .pending
        .iter()
        .any(|((from, to, edge), seed, bare)| {
            *from == use_node && *to == ghost && edge.is_data_flow() && *seed == foreign && !*bare
        });
    if !pending || before.seeds.contains(&use_node) != foreign {
        return Err(format!(
            "wrong guard seed flags, pending route {pending}, seeds {:?}",
            before.seeds
        ));
    }
    let app_def = variable(&before.graph, "App", "x", VarAccess::Def, None)?;
    let app_use = variable(&before.graph, "App", "x", VarAccess::Use, None)?;
    if !has_flow(&before.graph, app_def, app_use) {
        return Err("App formal fails to reach the props source before the guard".into());
    }
    Ok(target)
}

fn reaches_seed(
    graph: &DiGraph<CpgNode, CpgEdge>,
    root: NodeIndex,
    seeds: &[NodeIndex],
    skip: Option<EdgeIndex>,
) -> bool {
    let mut seen = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if !seen.insert(node) {
            continue;
        }
        if seeds.contains(&node) {
            return true;
        }
        pending.extend(
            graph
                .edges(node)
                .filter(|edge| Some(edge.id()) != skip && edge.weight().is_data_flow())
                .map(|edge| edge.target()),
        );
    }
    false
}

pub(super) fn guard_counterfactual(
    id: &str,
    case: &str,
    graph: &CodePropertyGraph,
    observed: Option<&Assembly>,
) -> Option<Result<(), String>> {
    if !matches!(
        id,
        "d2-guard-bare-arrow-literal"
            | "d2-guard-rest-literal"
            | "d2-guard-bare-arrow-jsx"
            | "d2-guard-rest-jsx"
    ) {
        return None;
    }
    Some((|| {
        let before = observed.unwrap().guard_input.as_ref().unwrap();
        let app_def = variable(&before.graph, "App", "x", VarAccess::Def, None)?;
        let app_use = variable(&before.graph, "App", "x", VarAccess::Use, None)?;
        let dest = variable(&before.graph, "D", "a", VarAccess::Def, None)?;
        let hops: Vec<_> = before
            .graph
            .edges_connecting(app_use, dest)
            .filter(|edge| edge.weight().is_data_flow())
            .map(|edge| edge.id())
            .collect();
        let [hop] = hops.as_slice() else {
            return Err(format!("expected one pending props hop, found {hops:?}"));
        };
        let capture = observed.unwrap();
        let pending: Vec<_> = capture
            .pending
            .iter()
            .filter(|((from, to, edge), _, _)| {
                *from == app_use && *to == dest && edge.is_data_flow()
            })
            .collect();
        let [((_, _, edge), false, false)] = pending.as_slice() else {
            return Err(format!("unexpected pending props hop flags: {pending:?}"));
        };
        if edge != &before.graph[*hop] {
            return Err("pending hop and pre-guard graph disagree".into());
        }
        let foreign = case == "foreign";
        if reaches_seed(&before.graph, app_def, &before.seeds, None) != foreign
            || reaches_seed(&before.graph, app_def, &before.seeds, Some(*hop))
            || has_flow(&graph.graph, app_def, app_use) == foreign
            || !has_flow(&graph.graph, app_use, dest)
        {
            return Err("seed path, single-hop counterfactual or final removal differs".into());
        }
        if before.counters.get("edges_to_def") != Some(&1)
            || before.counters != graph.dfg_label_stats.dfg_props
        {
            return Err("guard must retain the pending props counts".into());
        }
        Ok(())
    })())
}

pub(super) fn check(
    id: &str,
    case: &str,
    ext: &str,
    graph: &CodePropertyGraph,
    observed: Option<&Assembly>,
) -> Option<Result<String, String>> {
    let result = match id {
        "l22" => export_refusal(graph, ext),
        "d1-rewritten-argument"
        | "d1-debounce"
        | "d1-pass-through"
        | "d1-local-usecallback"
        | "d1-shadowed-usecallback" => resolved(
            graph,
            "App",
            "D",
            &format!("m.{ext}"),
            Some(ResolutionKind::LocalDef),
        ),
        "d1-unbound-singleton" if case == "foreign" => resolved(
            graph,
            "App",
            "D",
            &format!("other.{ext}"),
            Some(ResolutionKind::FreeSingle),
        ),
        "d2-literal-selected-read" | "d2-jsx-selected-read" => {
            selected_shadow(case, observed.unwrap())
        }
        "d2-forward-jsx" => (|| {
            let pre = &observed.unwrap().pre_binding.as_ref().unwrap().graph;
            let from = variable(pre, "Page", "x", VarAccess::Def, None)?;
            let source = variable(pre, "Page", "x", VarAccess::Use, None)?;
            variable(pre, "Header", "a", VarAccess::Def, None)?;
            variable(pre, "Header", "a", VarAccess::Use, None)?;
            if !has_flow(pre, from, source) {
                return Err("Page formal does not reach the JSX source".into());
            }
            resolved(graph, "Page", "Header", &format!("m.{ext}"), None)
        })(),
        "d2-guard-bare-arrow-literal"
        | "d2-guard-rest-literal"
        | "d2-guard-bare-arrow-jsx"
        | "d2-guard-rest-jsx" => guard_seed(graph, ext, case, observed.unwrap()),
        _ => return None,
    };
    Some(result)
}
