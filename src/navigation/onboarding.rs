use crate::cpg::CpgNode;
use crate::languages::Language;
use crate::navigation::{module_graph, queries, NavigationSession};
use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

pub const MAX_CONNECTED_MODULES: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectOverview {
    pub schema_version: String,
    pub project: String,
    pub inventory: InventoryOverview,
    pub modules: ModuleOverview,
    pub calls: CallOverview,
    pub warnings: Vec<String>,
    pub next_commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InventoryOverview {
    pub indexed_files: usize,
    pub skipped_files: usize,
    pub functions: usize,
    pub languages: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModuleOverview {
    pub nodes: usize,
    pub edges: usize,
    pub isolated_files: usize,
    pub connected: Vec<ConnectedModule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConnectedModule {
    pub file: String,
    pub dependencies: usize,
    pub dependents: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CallOverview {
    pub total_sites: usize,
    pub exact_edges: usize,
    pub name_only_edges: usize,
    pub demoted_edges: usize,
    pub dropped_multi_owner: usize,
    pub dropped_external_receiver: usize,
    pub dropped_import_external: usize,
    pub unresolved_unknown_name: usize,
}

pub fn build_report(session: &NavigationSession) -> Result<ProjectOverview> {
    // repo_map initializes the same resolved-call-edge index and warm sidecar used
    // by module-deps/caller queries. The report consumes its public projection;
    // it does not add a second resolver or graph interpretation.
    let repo_map = module_graph::repo_map(session);
    let graph = repo_map
        .graph
        .as_ref()
        .context("repo-map did not return its required graph payload")?;

    let modules = summarize_modules(graph)?;
    let stats = queries::call_stats(session.index.call_graph());
    let calls = CallOverview {
        total_sites: required_usize(&stats, "total_call_sites")?,
        exact_edges: sum_counter_map(&stats, "kind_exact")?,
        name_only_edges: sum_counter_map(&stats, "kind_nameonly")?,
        demoted_edges: required_usize(&stats, "demoted_edges")?,
        dropped_multi_owner: required_usize(&stats, "dropped_multi_owner")?,
        dropped_external_receiver: required_usize(&stats, "dropped_external_receiver")?,
        dropped_import_external: required_usize(&stats, "dropped_import_external")?,
        unresolved_unknown_name: required_usize(&stats, "unresolved_unknown_name")?,
    };

    let mut languages = BTreeMap::new();
    for parsed in session.repo.files.values() {
        *languages
            .entry(language_label(parsed.language).to_string())
            .or_default() += 1;
    }
    let functions = session
        .index
        .cpg()
        .graph
        .node_weights()
        .filter(|node| matches!(node, CpgNode::Function { .. }))
        .count();

    let mut warnings: Vec<String> = repo_map
        .warnings
        .iter()
        .map(|warning| warning.message.clone())
        .collect();
    if !session.repo.skipped.is_empty() {
        warnings.push(format!(
            "{} source file(s) were skipped during repository loading",
            session.repo.skipped.len()
        ));
    }
    warnings.sort();
    warnings.dedup();

    let canonical_root = session.repo.root.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize repository root {}",
            session.repo.root.display()
        )
    })?;
    let project = canonical_root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| canonical_root.display().to_string());

    Ok(ProjectOverview {
        schema_version: "1.0".to_string(),
        project,
        inventory: InventoryOverview {
            indexed_files: session.repo.files.len(),
            skipped_files: session.repo.skipped.len(),
            functions,
            languages,
        },
        modules,
        calls,
        warnings,
        next_commands: vec![
            "prism nav repo-map --repo <repo> --format json".to_string(),
            "prism nav module-deps --repo <repo> --file <file> --format json".to_string(),
            "prism nav call-stats --repo <repo>".to_string(),
        ],
    })
}

fn summarize_modules(graph: &crate::navigation::types::GraphPayload) -> Result<ModuleOverview> {
    let mut outgoing = vec![0usize; graph.nodes.len()];
    let mut incoming = vec![0usize; graph.nodes.len()];
    for edge in &graph.edges {
        let Some(from) = outgoing.get_mut(edge.from) else {
            return Err(anyhow!(
                "repo-map edge source {} exceeds {} nodes",
                edge.from,
                graph.nodes.len()
            ));
        };
        *from += 1;
        let Some(to) = incoming.get_mut(edge.to) else {
            return Err(anyhow!(
                "repo-map edge target {} exceeds {} nodes",
                edge.to,
                graph.nodes.len()
            ));
        };
        *to += 1;
    }

    let isolated_files = outgoing
        .iter()
        .zip(&incoming)
        .filter(|(out, incoming)| **out == 0 && **incoming == 0)
        .count();
    let mut connected: Vec<ConnectedModule> = graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(index, _)| outgoing[*index] + incoming[*index] > 0)
        .map(|(index, node)| ConnectedModule {
            file: node.location.file.clone(),
            dependencies: outgoing[index],
            dependents: incoming[index],
        })
        .collect();
    connected.sort_by(|left, right| {
        let left_degree = left.dependencies + left.dependents;
        let right_degree = right.dependencies + right.dependents;
        right_degree
            .cmp(&left_degree)
            .then_with(|| left.file.cmp(&right.file))
    });
    connected.truncate(MAX_CONNECTED_MODULES);

    Ok(ModuleOverview {
        nodes: graph.nodes.len(),
        edges: graph.edges.len(),
        isolated_files,
        connected,
    })
}

fn required_usize(stats: &Value, key: &str) -> Result<usize> {
    stats
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| anyhow!("call-stats field {key:?} is missing or not a usize"))
}

fn sum_counter_map(stats: &Value, key: &str) -> Result<usize> {
    let counters = stats
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("call-stats field {key:?} is missing or not an object"))?;
    counters.values().try_fold(0usize, |total, value| {
        let count = value
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| anyhow!("call-stats field {key:?} contains a non-usize counter"))?;
        total
            .checked_add(count)
            .ok_or_else(|| anyhow!("call-stats field {key:?} overflows usize"))
    })
}

fn language_label(language: Language) -> &'static str {
    match language {
        Language::Python => "python",
        Language::JavaScript => "javascript",
        Language::TypeScript => "typescript",
        Language::Go => "go",
        Language::Java => "java",
        Language::C => "c",
        Language::Cpp => "cpp",
        Language::Rust => "rust",
        Language::Lua => "lua",
        Language::Terraform => "terraform",
        Language::Tsx => "tsx",
        Language::Bash => "bash",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn telemetry_extractors_reject_missing_wrong_and_overflowing_values() {
        let stats = json!({"present": 2, "map": {"a": 1, "b": 2}});
        assert_eq!(required_usize(&stats, "present").unwrap(), 2);
        assert_eq!(sum_counter_map(&stats, "map").unwrap(), 3);
        assert!(required_usize(&stats, "missing").is_err());
        assert!(required_usize(&json!({"bad": "2"}), "bad").is_err());
        assert!(sum_counter_map(&json!({"bad": {"a": "1"}}), "bad").is_err());
        assert!(sum_counter_map(&json!({"bad": []}), "bad").is_err());
        assert!(sum_counter_map(&json!({"bad": {"a": u64::MAX, "b": 1}}), "bad").is_err());
    }

    #[test]
    fn malformed_graph_edge_is_rejected() {
        let graph = crate::navigation::types::GraphPayload {
            nodes: vec![],
            edges: vec![crate::navigation::types::GraphEdge {
                from: 0,
                to: 0,
                kind: "ModuleDep".to_string(),
            }],
        };
        assert!(summarize_modules(&graph).is_err());

        let graph = crate::navigation::types::GraphPayload {
            nodes: vec![crate::navigation::types::GraphNode {
                symbol: None,
                location: crate::navigation::types::Location {
                    file: "only.rs".to_string(),
                    start_line: 1,
                    end_line: 1,
                    start_byte: 0,
                    end_byte: 0,
                },
            }],
            edges: vec![crate::navigation::types::GraphEdge {
                from: 0,
                to: 1,
                kind: "ModuleDep".to_string(),
            }],
        };
        assert!(summarize_modules(&graph).is_err());
    }
}
