use super::{
    normalize_repo_dir, GoModuleGraph, ModuleBoundary, ParsedGoWork, Replacement, ReplacementRhs,
};
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct LhsKey {
    path: String,
    version: Option<String>,
}

#[derive(Debug, Clone)]
struct Candidate {
    replacement: Replacement,
    source_dir: String,
}

#[derive(Debug, PartialEq, Eq)]
enum SemanticRhs {
    InRepo(String),
    UnprovenLocal { source_dir: String, raw: String },
    Module { path: String, version: String },
}

pub(super) fn apply(graph: &mut GoModuleGraph, repo_root: &Path, work: Option<&ParsedGoWork>) {
    let active_modules = graph
        .active
        .iter()
        .map(|dir| {
            let ModuleBoundary::Valid(module) = &graph.boundaries[dir] else {
                unreachable!("active module must remain valid")
            };
            (dir.clone(), module.clone())
        })
        .collect::<Vec<_>>();
    let active_paths = active_modules
        .iter()
        .map(|(_, module)| module.path.clone())
        .collect::<BTreeSet<_>>();
    graph.telemetry.replaces_parsed = work.map_or(0, |work| work.replaces.len())
        + active_modules
            .iter()
            .map(|(_, module)| module.replaces.len())
            .sum::<usize>();

    let mut workspace = BTreeMap::new();
    if let Some(work) = work {
        for replacement in &work.replaces {
            let key = lhs_key(replacement);
            if workspace
                .insert(
                    key,
                    Candidate {
                        replacement: replacement.clone(),
                        source_dir: String::new(),
                    },
                )
                .is_some()
            {
                graph.invalidate_workspace();
                return;
            }
        }
    }

    let mut module_union: BTreeMap<LhsKey, Candidate> = BTreeMap::new();
    for (source_dir, module) in &active_modules {
        let mut per_module = BTreeSet::new();
        for replacement in &module.replaces {
            let key = lhs_key(replacement);
            if !per_module.insert(key.clone()) {
                graph.invalidate_workspace();
                return;
            }
            if workspace.contains_key(&key) {
                continue;
            }
            if active_paths.contains(&key.path) {
                continue;
            }
            let candidate = Candidate {
                replacement: replacement.clone(),
                source_dir: source_dir.clone(),
            };
            match module_union.entry(key) {
                Entry::Vacant(entry) => {
                    entry.insert(candidate);
                }
                Entry::Occupied(entry) => {
                    if semantic_rhs(repo_root, entry.get()) != semantic_rhs(repo_root, &candidate) {
                        graph.invalidate_workspace();
                        return;
                    }
                }
            }
        }
    }

    module_union.extend(workspace);
    let required = active_modules
        .iter()
        .flat_map(|(_, module)| module.requires.iter().cloned())
        .collect::<BTreeSet<_>>();
    let mut by_path: BTreeMap<String, Vec<Candidate>> = BTreeMap::new();
    for (key, candidate) in module_union {
        if active_paths.contains(&key.path) || !required.contains(&key.path) {
            continue;
        }
        by_path.entry(key.path).or_default().push(candidate);
    }

    let mut effective_paths = graph.providers.values().cloned().collect::<BTreeSet<_>>();
    for (lhs_path, candidates) in by_path {
        if candidates
            .iter()
            .any(|candidate| candidate.replacement.lhs_version.is_some())
        {
            mark_unproven(graph, repo_root, &lhs_path, &candidates);
            continue;
        }
        let candidate = &candidates[0];
        let ReplacementRhs::Local(raw) = &candidate.replacement.rhs else {
            graph.replace_unproven.insert(lhs_path);
            continue;
        };
        let Some(target_dir) = normalize_repo_dir(repo_root, &candidate.source_dir, raw) else {
            graph.replace_unproven.insert(lhs_path);
            continue;
        };
        if !matches!(
            graph.boundaries.get(&target_dir),
            Some(ModuleBoundary::Valid(_))
        ) {
            graph.replace_unproven.insert(lhs_path);
            graph.replace_unproven_dirs.insert(target_dir);
            continue;
        }
        if graph.providers.contains_key(&target_dir) || !effective_paths.insert(lhs_path.clone()) {
            graph.invalidate_workspace();
            return;
        }
        graph.providers.insert(target_dir, lhs_path);
        graph.telemetry.replaces_applied += 1;
    }
}

fn lhs_key(replacement: &Replacement) -> LhsKey {
    LhsKey {
        path: replacement.lhs_path.clone(),
        version: replacement.lhs_version.clone(),
    }
}

fn semantic_rhs(repo_root: &Path, candidate: &Candidate) -> SemanticRhs {
    match &candidate.replacement.rhs {
        ReplacementRhs::Local(raw) => normalize_repo_dir(repo_root, &candidate.source_dir, raw)
            .map_or_else(
                || SemanticRhs::UnprovenLocal {
                    source_dir: candidate.source_dir.clone(),
                    raw: raw.clone(),
                },
                SemanticRhs::InRepo,
            ),
        ReplacementRhs::Module { path, version } => SemanticRhs::Module {
            path: path.clone(),
            version: version.clone(),
        },
    }
}

fn mark_unproven(
    graph: &mut GoModuleGraph,
    repo_root: &Path,
    lhs_path: &str,
    candidates: &[Candidate],
) {
    graph.replace_unproven.insert(lhs_path.to_string());
    for candidate in candidates {
        if let ReplacementRhs::Local(raw) = &candidate.replacement.rhs {
            if let Some(dir) = normalize_repo_dir(repo_root, &candidate.source_dir, raw) {
                graph.replace_unproven_dirs.insert(dir);
            }
        }
    }
}
