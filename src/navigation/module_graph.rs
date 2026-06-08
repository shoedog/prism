use crate::navigation::types::*;
use crate::navigation::NavigationSession;
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

/// The call sites establishing one cross-file dependency: `(callee, call_site_line, qualifier)`.
/// `call_site_line` is a line in the SOURCE (caller) file, not the target.
type EdgeReasons = BTreeSet<(String, usize, Option<String>)>;

/// Derive every distinct call-derived cross-file edge once, keyed
/// `(source_file, target_file) -> reasons` (self-file edges excluded). Shared by
/// `module_deps` (projected to one source file) and `repo_map` (all edges) so the
/// two can never diverge (holistic-review MAJOR).
///
/// NOTE: `resolve_callees_qualified` matches any file whose stem == the module
/// qualifier (documented same-stem behavior, call_graph.rs), so a `utils` qualifier
/// can yield edges to both lib/utils.py and src/utils.py. Deterministic
/// over-reporting, acceptable for the v1 map.
fn collect_module_edges(s: &NavigationSession) -> BTreeMap<(String, String), EdgeReasons> {
    let cg = &s.index.cpg.call_graph;
    let mut edges: BTreeMap<(String, String), EdgeReasons> = BTreeMap::new();
    for (caller, sites) in &cg.calls {
        for site in sites {
            let resolved = cg.resolve_callees_qualified(
                &site.callee_name,
                &site.caller.file,
                site.qualifier.as_deref(),
            );
            for def in resolved {
                if def.file != caller.file {
                    edges
                        .entry((caller.file.clone(), def.file.clone()))
                        .or_default()
                        .insert((site.callee_name.clone(), site.line, site.qualifier.clone()));
                }
            }
        }
    }
    edges
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
                }),
            }],
            graph: None,
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
            .iter()
            .map(|(callee, line, qualifier)| Reason::Calls {
                callee: callee.clone(),
                call_site_line: *line,
                qualifier: qualifier.clone(),
            })
            .collect();
        items.push(EvidenceItem {
            symbol: None,
            location: Location {
                file: target.clone(),
                start_line: 1,
                end_line: 1,
            },
            score: 1.0,
            source: Source::PrismCpg,
            fallback: false,
            why,
            snippet: None,
        });
    }

    // Import labeling: Python/JS/TS/TSX/Go extract imports; Rust/Java/C/C++ do not.
    // NOTE: labeling is unconditional on call resolution, so a module that is BOTH
    // imported and call-resolved (e.g. `import util` + `util.helper()`) appears twice -
    // once as a PrismCpg call edge and once as a HeuristicImport item. Intentional in
    // v1 (filesystem import resolution is deferred, Design-decision #4).
    let mut warnings = Vec::new();
    if let Some(imports) = cg.imports.get(file) {
        let modules: BTreeSet<&String> = imports.values().collect();
        for module in &modules {
            items.push(EvidenceItem {
                symbol: None,
                location: Location {
                    file: file.into(),
                    start_line: 1,
                    end_line: 1,
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
    }
}

/// Whole-repo module graph: one file node per indexed file (isolated files
/// included) + distinct call-derived `ModuleDep` file->file edges. Spec §10.
pub fn repo_map(s: &NavigationSession) -> Evidence {
    let cg = &s.index.cpg.call_graph;
    // Shared edge collector: distinct (source_file, target_file) keys are the edges.
    let edges_map = collect_module_edges(s);

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
    let import_modules: usize = cg
        .imports
        .values()
        .flat_map(|m| m.values())
        .collect::<BTreeSet<_>>()
        .len();
    let mut warnings = Vec::new();
    if import_modules > 0 {
        warnings.push(Warning {
            kind: WarningKind::UnresolvedModule,
            message: format!("{import_modules} module import(s) not filesystem-resolved (v1)"),
            location: None,
        });
    }

    Evidence {
        query: "repo-map".into(),
        items: vec![],
        truncated: false,
        warnings,
        graph: Some(GraphPayload { nodes, edges }),
    }
}
