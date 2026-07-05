//! Tier-2 `taint_reaches` query.

use std::collections::BTreeMap;

use petgraph::graph::NodeIndex;

use crate::algorithms::taint::cleansed_categories_for_source;
use crate::cpg::{
    BoundaryKind, CodePropertyGraph, OrderingWarning, Relation, SameLineOrderView, Trace,
};
use crate::data_flow::VarLocation;
use crate::navigation::types::{
    Evidence, EvidenceItem, GraphEdge, GraphNode, GraphPayload, Location, QueryError, Reason,
    Source, SymbolRef, Warning, WarningKind,
};
use crate::navigation::NavigationSession;
use crate::reasoning::order::AstOrderView;
use crate::reasoning::sanitizer_walk::{sanitized_hits_on_chain, SanitizerHit};
use crate::reasoning::scope_honesty;
use crate::reasoning::seeds::{resolve, ResolvedSeed, SeedRole, SeedSpec};
use crate::reasoning::shape::{
    node_to_graph_node, reachability_for_node_from_ordered, window_relations, witness_chain_for,
    witness_graph_for_chain,
};
use crate::reasoning::types::{
    Reachability, ReasoningReason, ReasoningSummary, ReasoningWarning, SanitizerSite, SinkResult,
    SinkSourceResult, SourceWarningKey,
};

#[derive(Clone)]
struct ConcreteSeed {
    node: NodeIndex,
    loc: VarLocation,
    symbol: SymbolRef,
}

pub fn taint_reaches(
    session: &NavigationSession,
    sources: &[SeedSpec],
    sinks: Option<&[SeedSpec]>,
) -> Result<Evidence, QueryError> {
    let source_set = resolve(session, sources, SeedRole::Source)?;
    let source_roots = concrete_seeds(&source_set.seeds);
    if source_roots.is_empty() {
        return Err(QueryError::SymbolNotFound {
            seed: "sources".into(),
        });
    }

    let order = AstOrderView::new(&session.index.cpg, &session.repo.files);
    let trace = session.index.cpg.taint_trace_nodes(
        &source_roots
            .iter()
            .map(|seed| seed.node)
            .collect::<Vec<_>>(),
        Some(&order),
    );

    match sinks {
        Some(sinks) => {
            let sink_set = resolve(session, sinks, SeedRole::Sink)?;
            let sink_nodes = concrete_seeds(&sink_set.seeds);
            witness_mode(
                session,
                &source_set.warnings,
                &source_roots,
                &sink_nodes,
                &trace,
                Some(&order),
            )
        }
        None => Ok(frontier_mode(
            session,
            &source_set.warnings,
            &source_roots,
            &trace,
        )),
    }
}

fn frontier_mode(
    session: &NavigationSession,
    seed_warnings: &[Warning],
    sources: &[ConcreteSeed],
    trace: &Trace,
) -> Evidence {
    let mut warnings = seed_warnings.to_vec();
    warnings.extend(trace_warnings(trace));
    warnings.extend(boundary_warnings(&session.index.cpg, trace));
    warnings.extend(scope_honesty::warnings_for_trace(
        &session.repo.files,
        &session.index.cpg,
        trace,
    ));

    let mut items = Vec::new();
    for node in trace.frontier() {
        let graph_node = node_to_graph_node(&session.index.cpg, node);
        let Some((source, depth)) = nearest_source(trace, sources, node) else {
            continue;
        };
        items.push(EvidenceItem {
            symbol: graph_node.symbol,
            location: graph_node.location,
            score: 1.0 / (1.0 + depth as f32),
            source: Source::PrismCpg,
            fallback: false,
            why: vec![Reason::Reasoning(ReasoningReason::TaintedBy {
                source: source.symbol.clone(),
                sanitizers_present_in_source_fn: sanitizers_for_source(session, &source.loc),
                path_proven: false,
            })],
            snippet: None,
        });
    }
    items.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.location.file.cmp(&b.location.file))
            .then_with(|| a.location.start_line.cmp(&b.location.start_line))
    });

    let frontier_count = items.len();
    Evidence {
        query: "taint_reaches".into(),
        items,
        truncated: false,
        warnings: dedup_warnings(warnings),
        graph: None,
        reasoning: Some(ReasoningSummary {
            reachability: None,
            per_sink: Vec::new(),
            source_count: sources.len(),
            frontier_count,
            sinks_omitted: 0,
        }),
    }
}

fn witness_mode(
    session: &NavigationSession,
    seed_warnings: &[Warning],
    sources: &[ConcreteSeed],
    sinks: &[ConcreteSeed],
    trace: &Trace,
    order: Option<&dyn SameLineOrderView>,
) -> Result<Evidence, QueryError> {
    let mut warnings = seed_warnings.to_vec();
    warnings.extend(trace_warnings(trace));
    warnings.extend(boundary_warnings(&session.index.cpg, trace));
    warnings.extend(scope_honesty::warnings_for_trace(
        &session.repo.files,
        &session.index.cpg,
        trace,
    ));

    let mut union = GraphPayload {
        nodes: Vec::new(),
        edges: Vec::new(),
    };
    let mut node_map = BTreeMap::new();
    let mut per_sink = Vec::new();

    for sink in sinks {
        let mut source_results = Vec::new();
        for source in sources {
            let mut ordering_warnings = Vec::new();
            let raw_reachability = reachability_for_node_from_ordered(
                &session.index.cpg,
                trace,
                source.node,
                sink.node,
                order,
                &mut ordering_warnings,
            );
            warnings.extend(ordering_warnings.iter().map(ordering_warning));

            // P10: derive the ordered chain first, run the sanitizer-window check and settle the
            // FINAL verdict BEFORE the witness-graph gate below — the graph (for Reached|Sanitized)
            // then carries the sanitizer step for whichever verdict actually won.
            let mut reachability = raw_reachability;
            let mut graph_node = None;
            let mut sanitized_by: Vec<SanitizerSite> = Vec::new();
            let mut descent_depth: usize = 0;
            if raw_reachability == Reachability::Reached {
                if let Some(chain) = witness_chain_for(trace, source.node, sink.node) {
                    // P14 S4: recover each window's `Relation` the SAME way the witness-graph edge-kind
                    // labeling does (`shape::window_relations`) and thread it into the sanitizer walk so
                    // a `CallDescent` window (arg->param by construction) is never evaluated as an
                    // `x = sanitizer(y)` transition — this REPLACES Stage A's interim function-identity
                    // guard in `sanitized_hits_on_chain`.
                    let relations = window_relations(trace, Some(source.node), &chain);
                    descent_depth = relations
                        .iter()
                        .filter(|relation| matches!(relation, Some(Relation::CallDescent)))
                        .count();
                    let hits = sanitized_hits_on_chain(
                        &session.repo.files,
                        &session.index.cpg,
                        &chain,
                        &relations,
                    );
                    if !hits.is_empty() {
                        reachability = Reachability::Sanitized;
                    }
                    sanitized_by = hits.iter().map(|hit| hit.site.clone()).collect();

                    let (mut witness, idx_of) =
                        witness_graph_for_chain(&session.index.cpg, trace, source.node, &chain);
                    if !hits.is_empty() {
                        attach_sanitizer_steps(&mut witness, &idx_of, &hits);
                    }
                    let sink_key =
                        graph_node_key(&node_to_graph_node(&session.index.cpg, sink.node));
                    graph_node = merge_witness(&mut union, &mut node_map, witness, &sink_key);
                }
            }
            let sanitizers = sanitizers_for_source(session, &source.loc);
            if !sanitizers.is_empty() {
                let source_key = SourceWarningKey::from_var_location(&source.loc);
                // Path-proven cases name the site (the discriminant stays `Cleansed` — pinned by
                // eval fixtures); function-body-presence-only cases keep the original message.
                let message = match sanitized_by.first() {
                    Some(site) => format!(
                        "{} sanitizes this path via {} at {}:{}",
                        source_key.function, site.callee_text, site.file, site.line
                    ),
                    None => format!(
                        "{} contains sanitizer categories: {}",
                        source_key.function,
                        sanitizers.join(", ")
                    ),
                };
                warnings.push(Warning {
                    kind: WarningKind::Reasoning(ReasoningWarning::Cleansed {
                        source_function: source_key.function.clone(),
                    }),
                    message,
                    location: Some(source_key.location()),
                });
            }
            source_results.push(SinkSourceResult {
                source: source.symbol.clone(),
                reachability,
                graph_node,
                sanitizers_present_in_source_fn: sanitizers,
                sanitized_by,
                descent_depth,
            });
        }
        let reachability =
            Reachability::aggregate(source_results.iter().map(|source| source.reachability))
                .unwrap_or(Reachability::NotReached);
        per_sink.push(SinkResult {
            sink: sink.symbol.clone(),
            reachability,
            sources: source_results,
            sources_omitted: 0,
        });
    }

    let reachability = ReasoningSummary::aggregate(&per_sink, 0);
    Ok(Evidence {
        query: "taint_reaches".into(),
        items: Vec::new(),
        truncated: false,
        warnings: dedup_warnings(warnings),
        graph: (!union.nodes.is_empty()).then_some(union),
        reasoning: Some(ReasoningSummary {
            reachability,
            per_sink,
            source_count: sources.len(),
            frontier_count: trace.frontier().len(),
            sinks_omitted: 0,
        }),
    })
}

fn concrete_seeds(seeds: &[ResolvedSeed]) -> Vec<ConcreteSeed> {
    let mut out = Vec::new();
    for seed in seeds {
        for (&node, loc) in seed.nodes.iter().zip(seed.locations.iter()) {
            let symbol = symbol_for_var_location(loc);
            out.push(ConcreteSeed {
                node,
                loc: loc.clone(),
                symbol,
            });
        }
    }
    out
}

fn nearest_source<'a>(
    trace: &Trace,
    sources: &'a [ConcreteSeed],
    node: NodeIndex,
) -> Option<(&'a ConcreteSeed, usize)> {
    sources
        .iter()
        .filter(|source| {
            trace
                .frontier_by_root
                .get(&source.node)
                .is_some_and(|frontier| frontier.contains(&node))
        })
        .filter_map(|source| depth(trace, source.node, node).map(|depth| (source, depth)))
        .min_by_key(|(_, depth)| *depth)
}

fn depth(trace: &Trace, root: NodeIndex, node: NodeIndex) -> Option<usize> {
    if root == node {
        return Some(0);
    }
    let mut cur = node;
    let mut depth = 0usize;
    let mut seen = std::collections::BTreeSet::new();
    while let Some((parent, _)) = trace.parents_by_root.get(&(root, cur)) {
        if !seen.insert(cur) {
            return None;
        }
        depth += 1;
        if *parent == root {
            return Some(depth);
        }
        cur = *parent;
    }
    None
}

/// P10: append a `"SanitizedBy"` witness step for each proven sanitizer hit — a new `GraphNode` for
/// the sanitizer call site, wired from the chain node whose value flows into it. Attached to the
/// per-source witness graph BEFORE `merge_witness` folds it into the union (so the union's node/edge
/// dedup handles the synthetic node identically to any other witness node).
fn attach_sanitizer_steps(
    witness: &mut GraphPayload,
    idx_of: &BTreeMap<NodeIndex, usize>,
    hits: &[SanitizerHit],
) {
    for hit in hits {
        let Some(&from_idx) = idx_of.get(&hit.use_node) else {
            continue;
        };
        let new_idx = witness.nodes.len();
        witness.nodes.push(sanitizer_graph_node(hit));
        witness.edges.push(GraphEdge {
            from: from_idx,
            to: new_idx,
            kind: "SanitizedBy".to_string(),
        });
    }
}

fn sanitizer_graph_node(hit: &SanitizerHit) -> GraphNode {
    GraphNode {
        symbol: Some(SymbolRef::Statement {
            file: hit.site.file.clone(),
            line: hit.site.line,
            kind: "SanitizerCall".into(),
            start_byte: hit.call_start_byte,
            end_byte: hit.call_end_byte,
            ordinal: 0,
        }),
        location: Location {
            file: hit.site.file.clone(),
            start_line: hit.site.line,
            end_line: hit.site.line,
            start_byte: hit.call_start_byte,
            end_byte: hit.call_end_byte,
        },
    }
}

fn merge_witness(
    union: &mut GraphPayload,
    node_map: &mut BTreeMap<String, usize>,
    witness: GraphPayload,
    sink_key: &str,
) -> Option<usize> {
    let mut remap = Vec::new();
    let mut sink_node = None;
    for node in witness.nodes {
        let key = graph_node_key(&node);
        let is_sink = key == sink_key;
        let idx = *node_map.entry(key).or_insert_with(|| {
            union.nodes.push(node);
            union.nodes.len() - 1
        });
        if is_sink {
            sink_node = Some(idx);
        }
        remap.push(idx);
    }
    for edge in witness.edges {
        let Some(&from) = remap.get(edge.from) else {
            continue;
        };
        let Some(&to) = remap.get(edge.to) else {
            continue;
        };
        if from == to {
            continue;
        }
        let merged = GraphEdge {
            from,
            to,
            kind: edge.kind,
        };
        if !union.edges.contains(&merged) {
            union.edges.push(merged);
        }
    }
    debug_assert!(
        sink_node.is_some(),
        "witness graph omitted its requested sink endpoint"
    );
    sink_node
}

fn graph_node_key(node: &GraphNode) -> String {
    format!(
        "{:?}:{}:{}:{}:{}",
        node.symbol,
        node.location.file,
        node.location.start_line,
        node.location.start_byte,
        node.location.end_byte
    )
}

fn trace_warnings(trace: &Trace) -> Vec<Warning> {
    let mut warnings = Vec::new();
    for message in &trace.warnings {
        warnings.push(Warning {
            kind: WarningKind::SkippedPath,
            message: message.clone(),
            location: None,
        });
    }
    for warning in &trace.ordering_warnings {
        warnings.push(ordering_warning(warning));
    }
    warnings
}

fn ordering_warning(warning: &OrderingWarning) -> Warning {
    Warning {
        kind: WarningKind::Reasoning(ReasoningWarning::OrderingUnavailable {
            file: warning.file.clone(),
            line: warning.line,
            path: warning.path.clone(),
            reason: warning.reason,
        }),
        message: format!(
            "{}:{}:{} has ambiguous same-line ordering; witness edge was admitted conservatively",
            warning.file, warning.line, warning.path
        ),
        location: Some(Location {
            file: warning.file.clone(),
            start_line: warning.line,
            end_line: warning.line,
            start_byte: 0,
            end_byte: 0,
        }),
    }
}

fn boundary_warnings(cpg: &CodePropertyGraph, trace: &Trace) -> Vec<Warning> {
    trace
        .boundary
        .iter()
        .filter_map(|boundary| {
            let loc = cpg.to_var_location(boundary.to)?;
            let kind = match boundary.kind {
                BoundaryKind::CrossFunction => "cross-function",
                BoundaryKind::SelfFunctionParam => "self-function parameter",
            };
            Some(Warning {
                kind: WarningKind::Reasoning(ReasoningWarning::InterproceduralBoundary {
                    sink: loc.path.to_string(),
                }),
                message: format!("taint reached a {kind} boundary at {}", loc.path),
                location: Some(location_for_var(&loc)),
            })
        })
        .collect()
}

fn sanitizers_for_source(session: &NavigationSession, source: &VarLocation) -> Vec<String> {
    cleansed_categories_for_source(&session.repo.files, source)
}

fn symbol_for_var_location(loc: &VarLocation) -> SymbolRef {
    SymbolRef::Variable {
        file: loc.file.clone(),
        function: loc.function.clone(),
        line: loc.line,
        path: loc.path.to_string(),
        access: match loc.kind {
            crate::data_flow::VarAccessKind::Def => "def",
            crate::data_flow::VarAccessKind::Use => "use",
        }
        .into(),
        start_byte: loc.start_byte,
        end_byte: loc.end_byte,
        ordinal: 0,
    }
}

fn location_for_var(loc: &VarLocation) -> Location {
    Location {
        file: loc.file.clone(),
        start_line: loc.line,
        end_line: loc.line,
        start_byte: loc.start_byte,
        end_byte: loc.end_byte,
    }
}

fn dedup_warnings(warnings: Vec<Warning>) -> Vec<Warning> {
    let mut by_key = BTreeMap::new();
    for warning in warnings {
        by_key
            .entry(format!(
                "{:?}:{}:{}:{}:{}",
                warning.kind,
                warning.message,
                warning
                    .location
                    .as_ref()
                    .map(|location| location.file.clone())
                    .unwrap_or_default(),
                warning
                    .location
                    .as_ref()
                    .map(|location| location.start_byte)
                    .unwrap_or_default(),
                warning
                    .location
                    .as_ref()
                    .map(|location| location.end_byte)
                    .unwrap_or_default()
            ))
            .or_insert(warning);
    }
    by_key.into_values().collect()
}
