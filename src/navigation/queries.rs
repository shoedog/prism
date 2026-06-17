use crate::call_graph::{CallGraph, FunctionId};
use crate::cpg::{CpgEdge, CpgNode};
use crate::navigation::seed;
use crate::navigation::types::*;
use crate::navigation::NavigationSession;
use crate::resolution::ResolutionConfidence;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub fn call_stats(cg: &CallGraph) -> serde_json::Value {
    use crate::resolution::{DropReason, ResolutionConfidence};

    let mut kinds: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut demoted = 0usize;
    let mut total = 0usize;
    let (mut multi, mut external, mut import_ext, mut unknown) = (0usize, 0usize, 0usize, 0usize);
    for sites in cg.calls.values() {
        for site in sites {
            total += 1;
            let out = cg.resolve_call_site_full(site);
            match out.drop {
                Some(DropReason::MultiOwnerCollision) => multi += 1,
                Some(DropReason::ExternalReceiver) => external += 1,
                Some(DropReason::ImportExternal) => import_ext += 1,
                Some(DropReason::UnknownName) => unknown += 1,
                None => {}
            }
            for c in &out.resolved {
                *kinds.entry(c.kind.as_str()).or_default() += 1;
                if c.confidence == ResolutionConfidence::NameOnly {
                    demoted += 1;
                }
            }
        }
    }
    let mut interface_fanout: BTreeMap<usize, usize> = BTreeMap::new();
    for ids in cg.interface_impls.values() {
        *interface_fanout.entry(ids.len()).or_default() += 1;
    }
    serde_json::json!({
        "total_call_sites": total,
        "kinds": kinds,
        "demoted_edges": demoted,
        "dropped_multi_owner": multi,
        "dropped_external_receiver": external,
        "dropped_import_external": import_ext,
        "unresolved_unknown_name": unknown,
        "embedding_gaps": cg.embedding_gaps,
        "interface_gaps": cg.interface_gaps,
        "interface_overapprox": cg.interface_overapprox,
        "interface_fanout": interface_fanout,
    })
}

/// Phase-IP PR-2 in-scope interface-dispatch manifest (spec §8a, structural — no oracle).
///
/// A call-site is *in-scope* iff its receiver was syntactically recovered
/// (typed_param / constructor_local / type_assertion / var_local) AND the called
/// method appears on some known Go interface (`cg.interface_method_names`). Each
/// in-scope site is keyed by its byte-span (`file:start_byte:end_byte`) and stratified
/// by receiver class. `implementers` (Slice E) is the sorted, deduped set of implementer
/// owner *type names* prism mints for that (interface, method) — the RTA-pruned live set;
/// `fanout` is its cardinality (0 / `[]` for a concrete owner-resolved receiver). The
/// Slice-E gopls oracle (`eval/tools/dispatch_oracle.py`) compares this set per site to
/// gopls's `textDocument/implementation` satisfier set to decide sound vs over-approx.
///
/// The §5 `slice_candidate` (range-element) class is a manifest-only AST scan that is
/// **deferred** (see the PR-2 deferred doc); the recovered classes above do not depend
/// on it. The `corrected_fp` line of the gate report (the Python harness consumes this
/// JSON) is provisional until the Slice-E re-adjudication.
pub fn interface_dispatch_manifest(cg: &CallGraph) -> serde_json::Value {
    use crate::resolution::ReceiverRecovery;
    // receiver_class wire strings (the Rust→JSON→Python contract; pinned by the
    // `interface_manifest_receiver_class_strings` test). NOTE (review MAJOR 5):
    // `SliceElem`/"slice_elem" is the RESERVED variant (Slice F) — the classifier returns
    // None for it, so it never appears on a real site. It is DISTINCT from the spec-§5
    // deferred manifest-only "slice_candidate" range class (a CpgContext AST scan,
    // deferred — see the PR-2 deferred doc); the manifest currently emits only the four
    // recovered classes below.
    let class = |r: ReceiverRecovery| match r {
        ReceiverRecovery::TypedParam => "typed_param",
        ReceiverRecovery::ConstructorLocal => "constructor_local",
        ReceiverRecovery::TypeAssertion => "type_assertion",
        ReceiverRecovery::VarDecl => "var_local",
        ReceiverRecovery::SliceElem => "slice_elem",
    };
    let mut sites = Vec::new();
    for site_set in cg.calls.values() {
        for site in site_set {
            let (Some(recv_ty), Some(recovery)) =
                (site.receiver_type.as_deref(), site.receiver_recovery)
            else {
                continue;
            };
            // Go-caller gate (review MAJOR 4): real interface dispatch is Go-gated in
            // resolution.rs (caller.file is Go), so only count Go-caller sites. A non-Go
            // caller that syntactically recovers a same-named receiver type is not a real
            // interface-dispatch site and would inflate the denominator.
            if crate::languages::Language::from_path(&site.caller.file)
                != Some(crate::languages::Language::Go)
            {
                continue;
            }
            // Denominator predicate (§8a): the called method is on some known interface.
            if !cg.interface_method_names.contains(&site.callee_name) {
                continue;
            }
            // Slice E: emit the minted implementer SET (owner type names), not just the
            // count. The fanned-out FunctionIds are mapped to their owning type via
            // method_owners (fall back to the FunctionId's file stem if absent — a method
            // with no recorded owner is keyed by its file). Deduped + sorted so the wire
            // shape is deterministic and `fanout == implementers.len()`. A concrete
            // (fanout == 0) receiver yields the empty set.
            let impls: &[FunctionId] = crate::resolution::iface_key(recv_ty)
                .and_then(|k| cg.interface_impls.get(&(k, site.callee_name.clone())))
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let implementers: BTreeSet<String> = impls
                .iter()
                .map(|fid| {
                    cg.method_owners
                        .get(fid)
                        .cloned()
                        .unwrap_or_else(|| crate::resolution::file_stem(&fid.file).to_string())
                })
                .collect();
            let implementers: Vec<String> = implementers.into_iter().collect();
            sites.push(serde_json::json!({
                "file": site.caller.file,
                "start_byte": site.start_byte,
                "end_byte": site.end_byte,
                "line": site.line,
                "receiver_class": class(recovery),
                "method": site.callee_name,
                "fanout": implementers.len(),
                "implementers": implementers,
            }));
        }
    }
    // `interface_dispatch_computed` (review MINOR 6): false on a raw build_direct_subset
    // graph (apply_go_interface_dispatch never ran) → an empty `sites` means "not computed",
    // not "no dispatch found". The CLI feeds a full-build graph, so this is true in practice.
    serde_json::json!({
        "sites": sites,
        "interface_dispatch_computed": cg.interface_dispatch_computed,
    })
}

fn confidence_score(c: crate::resolution::ResolutionConfidence) -> f32 {
    match c {
        crate::resolution::ResolutionConfidence::Exact => 1.0,
        crate::resolution::ResolutionConfidence::NameOnly => 0.6,
    }
}

fn resolution_reason(kind: crate::resolution::ResolutionKind) -> Reason {
    Reason::Resolution {
        kind: kind.as_str().to_string(),
    }
}

fn function_bytes(s: &NavigationSession, fid: &FunctionId) -> (usize, usize) {
    s.index
        .cpg
        .function_candidates(&fid.file, &fid.name)
        .into_iter()
        .find_map(|idx| match s.index.cpg.node(idx) {
            CpgNode::Function {
                start_line,
                start_byte,
                end_byte,
                ..
            } if *start_line == fid.start_line => Some((*start_byte, *end_byte)),
            _ => None,
        })
        .unwrap_or((0, 0))
}

fn collision_warning(count: usize) -> Warning {
    Warning {
        kind: WarningKind::Collision,
        message: format!(
            "{count} same-name receiver call site(s) with unknown receiver type across multiple owner types; not attributed as callers"
        ),
        location: None,
    }
}

/// Exact CPG nodes at `file:line` (Function/Variable only, spec §8 R3-M3) plus the
/// innermost enclosing function as `EnclosingFunction` evidence.
pub fn nodes_at(s: &NavigationSession, file: &str, line: usize) -> Evidence {
    let query = format!("nodes-at:{file}:{line}");
    if !s.repo.files.contains_key(file) {
        let message = s
            .repo
            .skipped
            .iter()
            .find(|skipped| skipped.path == file)
            .map(|skipped| format!("file excluded: {:?}: {file}", skipped.reason))
            .unwrap_or_else(|| format!("file not in nav index: {file}"));
        return Evidence {
            query,
            items: vec![],
            truncated: false,
            warnings: vec![Warning {
                kind: WarningKind::SkippedPath,
                message,
                location: Some(Location {
                    file: file.into(),
                    start_line: line,
                    end_line: line,
                    start_byte: 0,
                    end_byte: 0,
                }),
            }],
            graph: None,
            reasoning: None,
        };
    }
    let mut items = Vec::new();
    for idx in s.index.cpg.nodes_at(file, line) {
        match s.index.cpg.node(idx) {
            CpgNode::Function {
                name,
                file: f,
                start_line,
                end_line,
                start_byte,
                end_byte,
                ..
            } => items.push(item_fn(
                f,
                name,
                *start_line,
                *end_line,
                *start_byte,
                *end_byte,
            )),
            CpgNode::Variable {
                path,
                file: f,
                function,
                line: l,
                access,
                start_byte,
                end_byte,
                ..
            } => items.push(EvidenceItem {
                symbol: Some(SymbolRef::Variable {
                    file: f.clone(),
                    function: function.clone(),
                    line: *l,
                    path: format!("{path:?}"),
                    access: format!("{access:?}"),
                    start_byte: *start_byte,
                    end_byte: *end_byte,
                    ordinal: 0,
                }),
                location: Location {
                    file: f.clone(),
                    start_line: *l,
                    end_line: *l,
                    start_byte: *start_byte,
                    end_byte: *end_byte,
                },
                score: 1.0,
                source: Source::PrismCpg,
                fallback: false,
                why: vec![],
                snippet: None,
            }),
            CpgNode::Statement { .. } => {} // statements not first-class in v1 (spec §8 R3-M3)
        }
    }
    // Enclosing function (innermost), as evidence on the line.
    if let Some((eidx, _)) = s.index.enclosing_function(file, line) {
        if let CpgNode::Function {
            name,
            file: f,
            start_line,
            end_line,
            start_byte,
            end_byte,
        } = s.index.cpg.node(eidx)
        {
            let func = SymbolRef::Function {
                file: f.clone(),
                name: name.clone(),
                start_line: *start_line,
                end_line: *end_line,
                start_byte: *start_byte,
                end_byte: *end_byte,
                ordinal: 0,
            };
            items.push(EvidenceItem {
                symbol: Some(func.clone()),
                // The enclosing-function evidence's Location mirrors the function's FULL span so
                // line and byte coordinates are coherent (S2 review MAJOR: queried-line + full-
                // function bytes was incoherent). The queried line is the query input, not the
                // evidence extent; the symbol + location both describe the function.
                location: Location {
                    file: f.clone(),
                    start_line: *start_line,
                    end_line: *end_line,
                    start_byte: *start_byte,
                    end_byte: *end_byte,
                },
                score: 1.0,
                source: Source::PrismCpg,
                fallback: false,
                why: vec![Reason::EnclosingFunction { function: func }],
                snippet: None,
            });
        }
    }
    Evidence {
        query,
        items,
        truncated: false,
        warnings: vec![],
        graph: None,
        reasoning: None,
    }
}

fn fid_of(sym: &SymbolRef) -> FunctionId {
    match sym {
        // `..` — SymbolRef::Function also has `ordinal` (R2-B1).
        SymbolRef::Function {
            name,
            file,
            start_line,
            end_line,
            ..
        } => FunctionId {
            name: name.clone(),
            file: file.clone(),
            start_line: *start_line,
            end_line: *end_line,
        },
        _ => unreachable!("seed resolves to a Function"),
    }
}

/// Direct callers of `target` (qualifier-aware identity filter): callers index is keyed by name,
/// so each candidate CallSite is resolved from its caller's file and kept only if it reaches THIS target.
fn direct_callers<'a>(
    s: &'a NavigationSession,
    target: &FunctionId,
) -> Vec<(FunctionId, crate::navigation::call_resolve::NavCallEdge<'a>)> {
    let mut out = Vec::new();
    let cg = &s.index.cpg.call_graph;
    for site in crate::navigation::call_resolve::scoped_caller_sites(cg, &target.name) {
        let resolved = crate::navigation::call_resolve::resolve_site_nav(cg, site);
        for edge in resolved {
            if *edge.target == *target {
                out.push((site.caller.clone(), edge));
            }
        }
    }
    out
}

pub fn callers(
    s: &NavigationSession,
    symbol: Option<&str>,
    file: Option<&str>,
    location: Option<&str>,
    depth: usize,
) -> Result<Evidence, QueryError> {
    callers_with_confidence(s, symbol, file, location, depth, false)
}

pub fn callers_with_confidence(
    s: &NavigationSession,
    symbol: Option<&str>,
    file: Option<&str>,
    location: Option<&str>,
    depth: usize,
    exact_only: bool,
) -> Result<Evidence, QueryError> {
    let resolved = seed::resolve_fn(s, symbol, file, location)?;
    let target = fid_of(&resolved.symbol);
    let target_for_warning = target.clone();
    let query = format!("callers:{}@{}", target.name, target.file); // @file identity (R3-M4)
    let mut items = Vec::new();
    let mut visited: std::collections::BTreeSet<FunctionId> = std::collections::BTreeSet::new();
    visited.insert(target.clone());
    let mut frontier = vec![target];
    for hop in 0..depth {
        // hop 0 = direct hit → score 1.0 (R3-M2); depth=2 → hops 0,1
        let mut next = Vec::new();
        for fid in &frontier {
            for (caller, edge) in direct_callers(s, fid) {
                if exact_only && edge.confidence != ResolutionConfidence::Exact {
                    continue;
                }
                let (start_byte, end_byte) = function_bytes(s, &caller);
                // One item PER CALL SITE (m7 symmetry with callees); `visited` only gates BFS recursion.
                items.push(EvidenceItem {
                    symbol: Some(SymbolRef::Function {
                        file: caller.file.clone(),
                        name: caller.name.clone(),
                        start_line: caller.start_line,
                        end_line: caller.end_line,
                        start_byte,
                        end_byte,
                        ordinal: 0,
                    }),
                    location: Location {
                        file: caller.file.clone(),
                        start_line: caller.start_line,
                        end_line: caller.end_line,
                        start_byte,
                        end_byte,
                    },
                    score: confidence_score(edge.confidence) / (1.0 + hop as f32),
                    source: Source::PrismCpg,
                    fallback: false,
                    why: vec![Reason::CalledBy {
                        caller: caller.name.clone(),
                        call_site_line: edge.call_site_line,
                    }]
                    .into_iter()
                    .chain(std::iter::once(resolution_reason(edge.kind)))
                    .collect(),
                    snippet: None,
                });
                if visited.insert(caller.clone()) {
                    next.push(caller);
                }
            }
        }
        frontier = next;
    }
    items.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap()
            .then(a.location.file.cmp(&b.location.file))
            .then(a.location.start_line.cmp(&b.location.start_line))
    });
    let dropped = crate::navigation::call_resolve::collision_dropped_sites(
        &s.index.cpg.call_graph,
        &target_for_warning.name,
    );
    let warnings = if dropped > 0 {
        vec![collision_warning(dropped)]
    } else {
        vec![]
    };
    Ok(Evidence {
        query,
        items,
        truncated: false,
        warnings,
        graph: None,
        reasoning: None,
    })
}

fn direct_callees<'a>(
    s: &'a NavigationSession,
    caller: &FunctionId,
) -> Vec<(
    Option<crate::navigation::call_resolve::NavCallEdge<'a>>,
    String,
    usize,
    Option<String>,
)> {
    let mut out = Vec::new();
    if let Some(sites) = s.index.cpg.call_graph.calls.get(caller) {
        for site in sites {
            let resolved =
                crate::navigation::call_resolve::resolve_site_nav(&s.index.cpg.call_graph, site);
            if resolved.is_empty() {
                out.push((
                    None,
                    site.callee_name.clone(),
                    site.line,
                    site.qualifier.clone(),
                ));
            } else {
                for edge in resolved {
                    out.push((
                        Some(edge),
                        site.callee_name.clone(),
                        site.line,
                        site.qualifier.clone(),
                    ));
                }
            }
        }
    }
    out
}

pub fn callees(
    s: &NavigationSession,
    symbol: Option<&str>,
    file: Option<&str>,
    location: Option<&str>,
    depth: usize,
) -> Result<Evidence, QueryError> {
    callees_with_confidence(s, symbol, file, location, depth, false)
}

pub fn callees_with_confidence(
    s: &NavigationSession,
    symbol: Option<&str>,
    file: Option<&str>,
    location: Option<&str>,
    depth: usize,
    exact_only: bool,
) -> Result<Evidence, QueryError> {
    let resolved = seed::resolve_fn(s, symbol, file, location)?;
    let seed_fid = fid_of(&resolved.symbol);
    let query = format!("callees:{}@{}", seed_fid.name, seed_fid.file); // @file identity (R3-M4)
    let mut items = Vec::new();
    let mut visited: std::collections::BTreeSet<FunctionId> = std::collections::BTreeSet::new();
    visited.insert(seed_fid.clone());
    let mut frontier = vec![seed_fid];
    for hop in 0..depth {
        // hop 0 = direct hit → score 1.0 (R3-M2); depth=2 → hops 0,1
        let mut next = Vec::new();
        for fid in &frontier {
            for (edge, callee_name, line, qualifier) in direct_callees(s, fid) {
                if exact_only
                    && !edge
                        .as_ref()
                        .map_or(false, |e| e.confidence == ResolutionConfidence::Exact)
                {
                    continue;
                }
                let def = edge.as_ref().map(|e| e.target);
                let (sym, loc) = match &def {
                    Some(d) => {
                        let (start_byte, end_byte) = function_bytes(s, d);
                        (
                            Some(SymbolRef::Function {
                                file: d.file.clone(),
                                name: d.name.clone(),
                                start_line: d.start_line,
                                end_line: d.end_line,
                                start_byte,
                                end_byte,
                                ordinal: 0,
                            }),
                            Location {
                                file: d.file.clone(),
                                start_line: d.start_line,
                                end_line: d.end_line,
                                start_byte,
                                end_byte,
                            },
                        )
                    }
                    None => (
                        None,
                        Location {
                            file: fid.file.clone(),
                            start_line: line,
                            end_line: line,
                            start_byte: 0,
                            end_byte: 0,
                        },
                    ),
                };
                items.push(EvidenceItem {
                    symbol: sym,
                    location: loc,
                    score: edge
                        .as_ref()
                        .map(|e| confidence_score(e.confidence))
                        .unwrap_or(1.0)
                        / (1.0 + hop as f32),
                    source: Source::PrismCpg,
                    fallback: false,
                    why: {
                        let mut why = vec![Reason::Calls {
                            callee: callee_name,
                            call_site_line: line,
                            qualifier,
                        }];
                        if let Some(edge) = &edge {
                            why.push(resolution_reason(edge.kind));
                        }
                        why
                    },
                    snippet: None,
                });
                if let Some(d) = def {
                    if visited.insert((*d).clone()) {
                        next.push((*d).clone());
                    }
                }
            }
        }
        frontier = next;
    }
    items.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap()
            .then(a.location.file.cmp(&b.location.file))
            .then(a.location.start_line.cmp(&b.location.start_line))
    });
    Ok(Evidence {
        query,
        items,
        truncated: false,
        warnings: vec![],
        graph: None,
        reasoning: None,
    })
}

fn edge_kind(e: &CpgEdge) -> &'static str {
    match e {
        CpgEdge::DataFlow => "DataFlow",
        CpgEdge::ControlFlow => "ControlFlow",
        CpgEdge::Call(_) => "Call",
        CpgEdge::Return(_) => "Return",
        CpgEdge::Contains => "Contains",
        CpgEdge::FieldOf => "FieldOf",
    }
}

fn parse_ego_edges(edges: &[&str]) -> Result<BTreeSet<&'static str>, QueryError> {
    let mut parsed = BTreeSet::new();
    for edge in edges {
        let edge = edge.trim();
        let kind = match edge {
            "DataFlow" => "DataFlow",
            "ControlFlow" => "ControlFlow",
            "Call" => "Call",
            "Return" => "Return",
            "Contains" => "Contains",
            "FieldOf" => "FieldOf",
            _ => {
                return Err(QueryError::UnknownEdge {
                    edge: edge.to_string(),
                });
            }
        };
        parsed.insert(kind);
    }
    Ok(parsed)
}

fn node_symbol_loc(s: &NavigationSession, ni: NodeIndex) -> (SymbolRef, Location) {
    match s.index.cpg.node(ni) {
        CpgNode::Function {
            name,
            file,
            start_line,
            end_line,
            start_byte,
            end_byte,
        } => (
            SymbolRef::Function {
                file: file.clone(),
                name: name.clone(),
                start_line: *start_line,
                end_line: *end_line,
                start_byte: *start_byte,
                end_byte: *end_byte,
                ordinal: 0,
            },
            Location {
                file: file.clone(),
                start_line: *start_line,
                end_line: *end_line,
                start_byte: *start_byte,
                end_byte: *end_byte,
            },
        ),
        CpgNode::Variable {
            path,
            file,
            function,
            line,
            access,
            start_byte,
            end_byte,
            ..
        } => (
            SymbolRef::Variable {
                file: file.clone(),
                function: function.clone(),
                line: *line,
                path: format!("{path:?}"),
                access: format!("{access:?}"),
                start_byte: *start_byte,
                end_byte: *end_byte,
                ordinal: 0,
            },
            Location {
                file: file.clone(),
                start_line: *line,
                end_line: *line,
                start_byte: *start_byte,
                end_byte: *end_byte,
            },
        ),
        CpgNode::Statement {
            file,
            line,
            kind,
            start_byte,
            end_byte,
            ..
        } => (
            SymbolRef::Statement {
                file: file.clone(),
                line: *line,
                kind: format!("{kind:?}"),
                start_byte: *start_byte,
                end_byte: *end_byte,
                ordinal: 0,
            },
            Location {
                file: file.clone(),
                start_line: *line,
                end_line: *line,
                start_byte: *start_byte,
                end_byte: *end_byte,
            },
        ),
    }
}

struct EgoSeed {
    query: String,
    nodes: Vec<NodeIndex>,
}

fn parse_location(location: &str) -> Result<(String, usize), QueryError> {
    location
        .rsplit_once(':')
        .and_then(|(f, l)| l.parse::<usize>().ok().map(|n| (f.to_string(), n)))
        .ok_or_else(|| QueryError::SymbolNotFound {
            seed: format!("loc:{location}"),
        })
}

fn resolve_ego_seed(
    s: &NavigationSession,
    symbol: Option<&str>,
    file: Option<&str>,
    location: Option<&str>,
) -> Result<EgoSeed, QueryError> {
    if let Some(loc) = location {
        let (f, line) = parse_location(loc)?;
        let mut nodes = s.index.cpg.nodes_at(&f, line);
        nodes.sort_by_key(|i| i.index());
        nodes.dedup();
        if nodes.is_empty() {
            let (idx, _) =
                s.index
                    .enclosing_function(&f, line)
                    .ok_or(QueryError::LocationOutOfRange {
                        file: f.clone(),
                        line,
                    })?;
            nodes.push(idx);
        }
        return Ok(EgoSeed {
            query: format!("ego:{f}:{line}"),
            nodes,
        });
    }

    let resolved = seed::resolve_fn(s, symbol, file, None)?;
    let ego_fid = fid_of(&resolved.symbol);
    Ok(EgoSeed {
        query: format!("ego:{}@{}", ego_fid.name, ego_fid.file),
        nodes: vec![resolved.idx],
    })
}

pub fn ego_graph(
    s: &NavigationSession,
    symbol: Option<&str>,
    file: Option<&str>,
    location: Option<&str>,
    hops: usize,
    edges: &[&str],
) -> Result<Evidence, QueryError> {
    let seed = resolve_ego_seed(s, symbol, file, location)?;
    let edge_filter = parse_ego_edges(edges)?;
    let g = &s.index.cpg.graph;
    let mut order: BTreeMap<NodeIndex, usize> = BTreeMap::new();
    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut ego_edges: Vec<GraphEdge> = Vec::new();
    // returns (index, is_new) so we only enqueue freshly-discovered nodes (m9).
    let intern = |s: &NavigationSession,
                  ni: NodeIndex,
                  order: &mut BTreeMap<NodeIndex, usize>,
                  nodes: &mut Vec<GraphNode>|
     -> (usize, bool) {
        if let Some(&i) = order.get(&ni) {
            return (i, false);
        }
        let i = nodes.len();
        let (symbol, location) = node_symbol_loc(s, ni);
        nodes.push(GraphNode {
            symbol: Some(symbol),
            location,
        });
        order.insert(ni, i);
        (i, true)
    };
    let mut q = VecDeque::new();
    for seed_node in &seed.nodes {
        intern(s, *seed_node, &mut order, &mut nodes);
        q.push_back((*seed_node, 0usize));
    }
    while let Some((ni, d)) = q.pop_front() {
        if d >= hops {
            continue;
        }
        for dir in [Direction::Outgoing, Direction::Incoming] {
            for er in g.edges_directed(ni, dir) {
                if !edge_filter.contains(edge_kind(er.weight())) {
                    continue;
                }
                let other = if er.source() == ni {
                    er.target()
                } else {
                    er.source()
                };
                let (from, _) = intern(s, ni, &mut order, &mut nodes);
                let (to, is_new) = intern(s, other, &mut order, &mut nodes);
                let (a, b) = if dir == Direction::Outgoing {
                    (from, to)
                } else {
                    (to, from)
                };
                ego_edges.push(GraphEdge {
                    from: a,
                    to: b,
                    kind: edge_kind(er.weight()).into(),
                });
                if is_new {
                    q.push_back((other, d + 1));
                }
            }
        }
    }
    ego_edges.sort_by(|x, y| (x.from, x.to, &x.kind).cmp(&(y.from, y.to, &y.kind)));
    ego_edges.dedup();
    let dropped: usize = if hops > 0 && edge_filter.contains("Call") {
        seed.nodes
            .iter()
            .filter_map(|ni| match s.index.cpg.node(*ni) {
                CpgNode::Function { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .map(|name| {
                crate::navigation::call_resolve::collision_dropped_sites(
                    &s.index.cpg.call_graph,
                    name,
                )
            })
            .sum()
    } else {
        0
    };
    let warnings = if dropped > 0 {
        vec![collision_warning(dropped)]
    } else {
        vec![]
    };
    Ok(Evidence {
        query: seed.query,
        items: vec![],
        truncated: false,
        warnings,
        graph: Some(GraphPayload {
            nodes,
            edges: ego_edges,
        }),
        reasoning: None,
    })
}

fn item_fn(
    file: &str,
    name: &str,
    start_line: usize,
    end_line: usize,
    start_byte: usize,
    end_byte: usize,
) -> EvidenceItem {
    let sym = SymbolRef::Function {
        file: file.into(),
        name: name.into(),
        start_line,
        end_line,
        start_byte,
        end_byte,
        ordinal: 0,
    };
    EvidenceItem {
        symbol: Some(sym),
        location: Location {
            file: file.into(),
            start_line,
            end_line,
            start_byte,
            end_byte,
        },
        score: 1.0,
        source: Source::PrismCpg,
        fallback: false,
        why: vec![],
        snippet: None,
    }
}
