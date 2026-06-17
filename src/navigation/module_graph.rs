use crate::name_resolution::consumer::{graph_module_dep_edge, GraphImport, ResolvedImport};
use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::rust_policy::EK_GLOB;
use crate::name_resolution::types::{BindTarget, FileId};
use crate::navigation::types::*;
use crate::navigation::NavigationSession;
use crate::resolution::ResolutionConfidence;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

/// Stable secondary sort rank when score/file/line tie.
fn source_rank(s: &Source) -> u8 {
    match s {
        Source::PrismCpg => 0,
        Source::HeuristicImport => 1,
        Source::ExternalIndex { .. } => 2,
    }
}

fn sort_items(items: &mut [EvidenceItem]) {
    items.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal) // NaN-safe (scores are 1.0 today)
            .then(a.location.file.cmp(&b.location.file))
            .then(a.location.start_line.cmp(&b.location.start_line))
            .then(source_rank(&a.source).cmp(&source_rank(&b.source)))
    });
}

fn confidence_score(c: ResolutionConfidence) -> f32 {
    match c {
        ResolutionConfidence::Exact => 1.0,
        ResolutionConfidence::NameOnly => 0.6,
    }
}

fn resolution_kind_reason(kind: &str) -> Reason {
    Reason::Resolution {
        kind: kind.to_string(),
    }
}

/// The call sites establishing one cross-file dependency.
/// `call_site_line` is a line in the SOURCE (caller) file, not the target.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleCallReason {
    pub callee: String,
    pub call_site_line: usize,
    pub qualifier: Option<String>,
    pub score: f32,
    pub kind: String,
}

impl Eq for ModuleCallReason {}

impl PartialOrd for ModuleCallReason {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ModuleCallReason {
    fn cmp(&self, other: &Self) -> Ordering {
        self.callee
            .cmp(&other.callee)
            .then(self.call_site_line.cmp(&other.call_site_line))
            .then(self.qualifier.cmp(&other.qualifier))
            .then(self.score.to_bits().cmp(&other.score.to_bits()))
            .then(self.kind.cmp(&other.kind))
    }
}

#[derive(Debug, Default)]
struct EdgeReasons {
    max_score: f32,
    reasons: BTreeSet<ModuleCallReason>,
}

#[derive(Debug, Default)]
struct GraphImportDeps {
    files: BTreeMap<String, BTreeSet<String>>,
    external: BTreeSet<String>,
}

/// Derive every distinct call-derived cross-file edge once, keyed
/// `(source_file, target_file) -> reasons` (self-file edges excluded). Shared by
/// `module_deps` (projected to one source file) and `repo_map` (all edges) so the
/// two can never diverge (holistic-review MAJOR).
///
/// NOTE: nav-local call resolution uses documented same-stem behavior for
/// qualified/scoped module hints, so a `utils` qualifier can yield edges to both
/// lib/utils.py and src/utils.py. Deterministic over-reporting is acceptable for
/// the v1 map.
fn collect_module_edges(s: &NavigationSession) -> BTreeMap<(String, String), EdgeReasons> {
    let cg = &s.index.cpg.call_graph;
    let mut edges: BTreeMap<(String, String), EdgeReasons> = BTreeMap::new();
    for (caller, sites) in &cg.calls {
        for site in sites {
            let resolved = crate::navigation::call_resolve::resolve_site_nav(cg, site);
            for edge in resolved {
                let def = edge.target;
                if def.file != caller.file {
                    let score = confidence_score(edge.confidence);
                    let entry = edges
                        .entry((caller.file.clone(), def.file.clone()))
                        .or_default();
                    entry.max_score = entry.max_score.max(score);
                    entry.reasons.insert(ModuleCallReason {
                        callee: site.callee_name.clone(),
                        call_site_line: site.line,
                        qualifier: edge.qualifier,
                        score,
                        kind: edge.kind.as_str().to_string(),
                    });
                }
            }
        }
    }
    edges
}

fn authoritative_rust_scope_graph<'a>(
    s: &'a NavigationSession,
    file: &str,
) -> Option<&'a ScopeGraph> {
    let parsed = s.repo.files.get(file)?;
    if !matches!(parsed.language, crate::languages::Language::Rust) {
        return None;
    }
    let graph = s.index.cpg.call_graph.scope_graph.as_ref()?;
    if !graph.complete || !graph.file_paths.contains_key(file) {
        return None;
    }
    Some(graph)
}

fn scope_in_file(
    graph: &ScopeGraph,
    scope: crate::name_resolution::types::ScopeId,
    file: FileId,
) -> bool {
    graph
        .scope(scope)
        .map(|s| s.extents.iter().any(|e| e.file == file))
        .unwrap_or(false)
}

fn collect_graph_import_deps(graph: &ScopeGraph, file: &str) -> GraphImportDeps {
    let Some(&file_id) = graph.file_paths.get(file) else {
        return GraphImportDeps::default();
    };
    let mut deps = GraphImportDeps::default();
    for binding in &graph.bindings {
        if !scope_in_file(graph, binding.scope, file_id) {
            continue;
        }
        if !matches!(binding.target, BindTarget::Pending(_, _)) {
            continue;
        }
        match graph_module_dep_edge(graph, GraphImport::Named(binding)) {
            ResolvedImport::File(target) if target != file => {
                deps.files
                    .entry(target)
                    .or_default()
                    .insert(binding.name.clone());
            }
            ResolvedImport::External => {
                deps.external.insert(binding.name.clone());
            }
            ResolvedImport::File(_) | ResolvedImport::Unresolved => {}
        }
    }
    for edge in &graph.edges {
        if edge.kind != EK_GLOB || !scope_in_file(graph, edge.from, file_id) {
            continue;
        }
        match graph_module_dep_edge(graph, GraphImport::Glob(edge)) {
            ResolvedImport::File(target) if target != file => {
                deps.files
                    .entry(target)
                    .or_default()
                    .insert("*".to_string());
            }
            ResolvedImport::External => {
                deps.external.insert("*".to_string());
            }
            ResolvedImport::File(_) | ResolvedImport::Unresolved => {}
        }
    }
    deps
}

/// Outbound module dependencies of `file`: distinct target files reached by a
/// resolved cross-file call (`source: PrismCpg`), plus extracted-but-unresolved
/// imports labeled `UnresolvedImport` (`source: HeuristicImport`). Spec §10.
///
/// Mirrors `nodes_at`'s §5 contract: a skipped or unknown file returns empty
/// `items` + a `SkippedPath` warning - never a hard error.
pub fn module_deps(s: &NavigationSession, file: &str) -> Evidence {
    let query = format!("module-deps:{file}");
    if !s.repo.files.contains_key(file) {
        let message = s
            .repo
            .skipped
            .iter()
            .find(|sk| sk.path == file)
            .map(|sk| format!("file excluded: {:?}: {file}", sk.reason))
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
                    start_line: 1,
                    end_line: 1,
                    start_byte: 0,
                    end_byte: 0,
                }),
            }],
            graph: None,
            reasoning: None,
        };
    }

    let cg = &s.index.cpg.call_graph;
    // Project the shared edge collector to edges whose SOURCE is the queried file;
    // each distinct target file becomes one PrismCpg item carrying its Calls reasons.
    let mut items = Vec::new();
    for ((source, target), reasons) in collect_module_edges(s) {
        if source != file {
            continue;
        }
        let why = reasons
            .reasons
            .iter()
            .flat_map(|reason| {
                std::iter::once(Reason::Calls {
                    callee: reason.callee.clone(),
                    call_site_line: reason.call_site_line,
                    qualifier: reason.qualifier.clone(),
                })
                .chain(std::iter::once(resolution_kind_reason(&reason.kind)))
            })
            .collect();
        items.push(EvidenceItem {
            symbol: None,
            location: Location {
                file: target.clone(),
                start_line: 1,
                end_line: 1,
                start_byte: 0,
                end_byte: 0,
            },
            score: reasons.max_score,
            source: Source::PrismCpg,
            fallback: false,
            why,
            snippet: None,
        });
    }

    let mut warnings = Vec::new();
    if let Some(graph) = authoritative_rust_scope_graph(s, file) {
        let deps = collect_graph_import_deps(graph, file);
        for (target, modules) in deps.files {
            items.push(EvidenceItem {
                symbol: None,
                location: Location {
                    file: target.clone(),
                    start_line: 1,
                    end_line: 1,
                    start_byte: 0,
                    end_byte: 0,
                },
                score: 1.0,
                source: Source::PrismCpg,
                fallback: false,
                why: modules
                    .into_iter()
                    .map(|module| Reason::ResolvedImport {
                        module,
                        target_file: target.clone(),
                    })
                    .collect(),
                snippet: None,
            });
        }
        for module in &deps.external {
            items.push(EvidenceItem {
                symbol: None,
                location: Location {
                    file: file.into(),
                    start_line: 1,
                    end_line: 1,
                    start_byte: 0,
                    end_byte: 0,
                },
                score: 1.0,
                source: Source::HeuristicImport,
                fallback: false,
                why: vec![Reason::UnresolvedImport {
                    module: module.clone(),
                }],
                snippet: None,
            });
        }
        if !deps.external.is_empty() {
            warnings.push(Warning {
                kind: WarningKind::UnresolvedModule,
                message: format!(
                    "{} module import(s) not filesystem-resolved (v1)",
                    deps.external.len()
                ),
                location: None,
            });
        }
    } else if let Some(imports) = cg.imports.get(file) {
        // Import labeling: Python/JS/TS/TSX/Go extract imports. Rust reaches
        // this branch only when the scope graph is not authoritative, preserving
        // the existing fallback behavior for that file.
        let modules: BTreeSet<&String> = imports.values().collect();
        for module in &modules {
            items.push(EvidenceItem {
                symbol: None,
                location: Location {
                    file: file.into(),
                    start_line: 1,
                    end_line: 1,
                    start_byte: 0,
                    end_byte: 0,
                },
                score: 1.0,
                source: Source::HeuristicImport,
                fallback: false,
                why: vec![Reason::UnresolvedImport {
                    module: (*module).clone(),
                }],
                snippet: None,
            });
        }
        if !modules.is_empty() {
            warnings.push(Warning {
                kind: WarningKind::UnresolvedModule,
                message: format!(
                    "{} module import(s) not filesystem-resolved (v1)",
                    modules.len()
                ),
                location: None,
            });
        }
    }

    sort_items(&mut items);
    Evidence {
        query,
        items,
        truncated: false,
        warnings,
        graph: None,
        reasoning: None,
    }
}

/// Whole-repo module graph: one file node per indexed file (isolated files
/// included) + distinct call-derived `ModuleDep` file->file edges. Spec §10.
pub fn repo_map(s: &NavigationSession) -> Evidence {
    let cg = &s.index.cpg.call_graph;
    // Shared edge collector: distinct (source_file, target_file) keys are the edges.
    let mut edges_map = collect_module_edges(s);
    let mut graph_external_modules = BTreeSet::new();
    for file in s.repo.files.keys() {
        let Some(graph) = authoritative_rust_scope_graph(s, file) else {
            continue;
        };
        let deps = collect_graph_import_deps(graph, file);
        for target in deps.files.keys() {
            edges_map.entry((file.clone(), target.clone())).or_default();
        }
        graph_external_modules.extend(deps.external);
    }

    // Whole-repo node set: every indexed file (isolated files included).
    let files: BTreeSet<&String> = s.repo.files.keys().collect();
    let order: BTreeMap<&String, usize> = files.iter().enumerate().map(|(i, f)| (*f, i)).collect();
    let nodes = files
        .iter()
        .map(|f| GraphNode {
            symbol: None,
            location: Location {
                file: (*f).clone(),
                start_line: 1,
                end_line: 1,
                start_byte: 0,
                end_byte: 0,
            },
        })
        .collect();
    // Defensive: only emit an edge when both endpoints are indexed file nodes.
    let edges = edges_map
        .keys()
        .filter_map(|(a, b)| match (order.get(a), order.get(b)) {
            (Some(&from), Some(&to)) => Some(GraphEdge {
                from,
                to,
                kind: "ModuleDep".into(),
            }),
            _ => None,
        })
        .collect();

    // Distinct modules across the WHOLE repo: collect every module into ONE BTreeSet,
    // so a module imported from N files counts once (matches the "distinct module" unit;
    // summing per-file distinct counts would double-count - round-3 MAJOR fix).
    let mut import_modules: BTreeSet<String> = cg
        .imports
        .values()
        .flat_map(|m| m.values())
        .cloned()
        .collect();
    import_modules.extend(graph_external_modules);
    let mut warnings = Vec::new();
    if !import_modules.is_empty() {
        warnings.push(Warning {
            kind: WarningKind::UnresolvedModule,
            message: format!(
                "{} module import(s) not filesystem-resolved (v1)",
                import_modules.len()
            ),
            location: None,
        });
    }

    Evidence {
        query: "repo-map".into(),
        items: vec![],
        truncated: false,
        warnings,
        graph: Some(GraphPayload { nodes, edges }),
        reasoning: None,
    }
}
