//! Arm-neutral typed graph reader for the element-recall diagnostics (lane 10).
//!
//! `element_recall_graph <repo> [--out FILE]` builds the same whole-repo index the CLI's
//! `nav --no-cache` builds (`load_repo` + `NavigationIndex::build` on the build pool) and writes
//! sorted JSONL: `meta`, `file` (sha256), `manifest`, `skip`, every `node` (isolated ones included,
//! with a flag), every directed `edge` (endpoints carried as full typed records, duplicates kept)
//! and a terminal `summary`. Identities are full records, never NodeIndex or `CpgNode` equality,
//! and two nodes with one full identity are refused. It reads only public API and never
//! reads `PRISM_EDGES_BYTES`; the output carries no path of the repository root.

use anyhow::{bail, Context, Result};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use prism::access_path::AccessPath;
use prism::cpg::{CpgEdge, CpgNode, FlowConfidence, FlowDoubt, StmtKind, VarAccess};
use prism::navigation::NavigationIndex;
use prism::repo_loader::{load_repo, LoadedRepo};
use prism::resolution::ResolutionConfidence;
use serde::Serialize;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

const SCHEMA: &str = "element_recall_graph/1";

/// Full typed identity of a node: every field, owner and bytes included.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum NodeKey {
    Function {
        name: String,
        file: String,
        start_line: usize,
        end_line: usize,
        start_byte: usize,
        end_byte: usize,
    },
    Statement {
        file: String,
        line: usize,
        stmt_kind: &'static str,
        stmt_detail: Option<String>,
        start_byte: usize,
        end_byte: usize,
    },
    Variable {
        file: String,
        owner: String,
        owner_start_line: usize,
        line: usize,
        access: &'static str,
        path: AccessPath,
        start_byte: usize,
        end_byte: usize,
    },
    ReturnValue {
        file: String,
        owner: String,
        owner_start_line: usize,
        line: usize,
        return_start_byte: usize,
        return_end_byte: usize,
        child_slot: usize,
        start_byte: usize,
        end_byte: usize,
    },
}

fn node_key(node: &CpgNode) -> NodeKey {
    match node {
        CpgNode::Function {
            name,
            file,
            start_line,
            end_line,
            start_byte,
            end_byte,
        } => NodeKey::Function {
            name: name.clone(),
            file: file.clone(),
            start_line: *start_line,
            end_line: *end_line,
            start_byte: *start_byte,
            end_byte: *end_byte,
        },
        CpgNode::Statement {
            file,
            line,
            kind,
            start_byte,
            end_byte,
        } => {
            let (stmt_kind, stmt_detail) = match kind {
                StmtKind::Assignment => ("assignment", None),
                StmtKind::Call { callee } => ("call", Some(callee.clone())),
                StmtKind::Return => ("return", None),
                StmtKind::Branch => ("branch", None),
                StmtKind::Loop => ("loop", None),
                StmtKind::Goto { target } => ("goto", Some(target.clone())),
                StmtKind::Label { name } => ("label", Some(name.clone())),
                StmtKind::Declaration => ("declaration", None),
                StmtKind::Other => ("other", None),
            };
            NodeKey::Statement {
                file: file.clone(),
                line: *line,
                stmt_kind,
                stmt_detail,
                start_byte: *start_byte,
                end_byte: *end_byte,
            }
        }
        CpgNode::Variable {
            path,
            file,
            function,
            function_start_line,
            line,
            access,
            start_byte,
            end_byte,
        } => NodeKey::Variable {
            file: file.clone(),
            owner: function.clone(),
            owner_start_line: *function_start_line,
            line: *line,
            access: match access {
                VarAccess::Def => "def",
                VarAccess::Use => "use",
            },
            path: path.clone(),
            start_byte: *start_byte,
            end_byte: *end_byte,
        },
        CpgNode::ReturnValue {
            file,
            function,
            function_start_line,
            line,
            return_start_byte,
            return_end_byte,
            child_slot,
            start_byte,
            end_byte,
        } => NodeKey::ReturnValue {
            file: file.clone(),
            owner: function.clone(),
            owner_start_line: *function_start_line,
            line: *line,
            return_start_byte: *return_start_byte,
            return_end_byte: *return_end_byte,
            child_slot: *child_slot,
            start_byte: *start_byte,
            end_byte: *end_byte,
        },
    }
}

/// Edge variant and complete payload (confidence, doubt, kill line, shortcut bit).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "variant", rename_all = "snake_case")]
enum EdgeRec {
    DataFlow {
        confidence: &'static str,
        doubt: Option<&'static str>,
        kill_line: Option<u32>,
    },
    ControlFlow,
    Call {
        confidence: &'static str,
    },
    Return {
        confidence: &'static str,
    },
    ReturnFlow {
        suppress_shortcut: bool,
    },
    ReturnInput,
    Contains,
    FieldOf,
}

fn resolution(confidence: ResolutionConfidence) -> &'static str {
    match confidence {
        ResolutionConfidence::Exact => "exact",
        ResolutionConfidence::NameOnly => "nameonly",
    }
}

fn edge_rec(edge: &CpgEdge) -> EdgeRec {
    match edge {
        CpgEdge::DataFlow(FlowConfidence::Exact) => EdgeRec::DataFlow {
            confidence: "exact",
            doubt: None,
            kill_line: None,
        },
        CpgEdge::DataFlow(FlowConfidence::NameOnly(doubt)) => {
            let (doubt, kill_line) = match doubt {
                FlowDoubt::Killed { kill_line } => ("killed", Some(*kill_line)),
                FlowDoubt::SameLine => ("sameline", None),
                FlowDoubt::CfgIncomplete => ("cfg_incomplete", None),
                FlowDoubt::AliasUnstable => ("alias_unstable", None),
                FlowDoubt::CallNameOnly => ("call_nameonly", None),
            };
            EdgeRec::DataFlow {
                confidence: "nameonly",
                doubt: Some(doubt),
                kill_line,
            }
        }
        CpgEdge::ControlFlow => EdgeRec::ControlFlow,
        CpgEdge::Call(c) => EdgeRec::Call {
            confidence: resolution(*c),
        },
        CpgEdge::Return(c) => EdgeRec::Return {
            confidence: resolution(*c),
        },
        CpgEdge::ReturnFlow { suppress_shortcut } => EdgeRec::ReturnFlow {
            suppress_shortcut: *suppress_shortcut,
        },
        CpgEdge::ReturnInput => EdgeRec::ReturnInput,
        CpgEdge::Contains => EdgeRec::Contains,
        CpgEdge::FieldOf => EdgeRec::FieldOf,
    }
}

#[derive(Serialize)]
struct NodeLine<'a> {
    kind: &'static str,
    #[serde(flatten)]
    key: &'a NodeKey,
    isolated: bool,
}

#[derive(Serialize)]
struct EdgeLine<'a> {
    kind: &'static str,
    from: &'a NodeKey,
    to: &'a NodeKey,
    edge: &'a EdgeRec,
}

#[derive(Serialize)]
struct Meta {
    kind: &'static str,
    schema: &'static str,
    prism_version: &'static str,
    /// The sha the CLI prints inside `slicing <semver> (<sha>)`, of the library linked here.
    build_identity: &'static str,
}

#[derive(Serialize)]
struct Hashed<'a> {
    kind: &'static str,
    key: &'a str,
    sha256: &'a str,
}

#[derive(Serialize)]
struct Skip<'a> {
    kind: &'static str,
    path: &'a str,
    reason: &'a prism::repo_loader::SkipReason,
}

#[derive(Serialize)]
struct Summary {
    kind: &'static str,
    nodes: usize,
    isolated_nodes: usize,
    edges: usize,
    edges_by_variant: BTreeMap<String, usize>,
}

fn line<W: Write>(out: &mut W, record: &impl Serialize) -> Result<()> {
    serde_json::to_writer(&mut *out, record)?;
    out.write_all(b"\n")?;
    Ok(())
}

/// Writes the sorted node, edge and summary records of one graph. Errors on an ambiguous identity.
fn write_graph<W: Write>(graph: &DiGraph<CpgNode, CpgEdge>, out: &mut W) -> Result<()> {
    let mut nodes: Vec<(NodeKey, NodeIndex)> = graph
        .node_indices()
        .map(|i| (node_key(&graph[i]), i))
        .collect();
    nodes.sort();
    if let Some(pair) = nodes.windows(2).find(|w| w[0].0 == w[1].0) {
        bail!(
            "ambiguous node identity (two nodes share one full record): {}",
            serde_json::to_string(&pair[0].0)?
        );
    }
    let mut rank = vec![0usize; graph.node_count()];
    for (r, (_, idx)) in nodes.iter().enumerate() {
        rank[idx.index()] = r;
    }
    let mut isolated = 0usize;
    for (key, idx) in &nodes {
        let alone = graph
            .edges_directed(*idx, Direction::Outgoing)
            .next()
            .is_none()
            && graph
                .edges_directed(*idx, Direction::Incoming)
                .next()
                .is_none();
        isolated += usize::from(alone);
        line(
            out,
            &NodeLine {
                kind: "node",
                key,
                isolated: alone,
            },
        )?;
    }
    let mut edges: Vec<(usize, usize, EdgeRec)> = graph
        .edge_references()
        .map(|e| {
            (
                rank[e.source().index()],
                rank[e.target().index()],
                edge_rec(e.weight()),
            )
        })
        .collect();
    edges.sort();
    let mut by_variant: BTreeMap<String, usize> = BTreeMap::new();
    for (from, to, rec) in &edges {
        let variant = serde_json::to_value(rec)?["variant"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        *by_variant.entry(variant).or_default() += 1;
        line(
            out,
            &EdgeLine {
                kind: "edge",
                from: &nodes[*from].0,
                to: &nodes[*to].0,
                edge: rec,
            },
        )?;
    }
    let summary = Summary {
        kind: "summary",
        nodes: nodes.len(),
        isolated_nodes: isolated,
        edges: edges.len(),
        edges_by_variant: by_variant,
    };
    line(out, &summary)
}

fn write_repo_records<W: Write>(repo: &LoadedRepo, out: &mut W) -> Result<()> {
    line(
        out,
        &Meta {
            kind: "meta",
            schema: SCHEMA,
            prism_version: env!("CARGO_PKG_VERSION"),
            build_identity: prism::api::build_info().git_sha,
        },
    )?;
    for (key, sha256) in &repo.file_hashes {
        line(
            out,
            &Hashed {
                kind: "file",
                key,
                sha256,
            },
        )?;
    }
    for (key, sha256) in &repo.manifest_hashes {
        line(
            out,
            &Hashed {
                kind: "manifest",
                key,
                sha256,
            },
        )?;
    }
    let mut skips: Vec<_> = repo.skipped.iter().collect();
    skips.sort_by_cached_key(|s| {
        (
            s.path.clone(),
            serde_json::to_string(&s.reason).unwrap_or_default(),
        )
    });
    for s in skips {
        line(
            out,
            &Skip {
                kind: "skip",
                path: &s.path,
                reason: &s.reason,
            },
        )?;
    }
    Ok(())
}

/// The CLI's cold `nav --no-cache` build, then the typed records.
fn read_repo<W: Write>(root: &Path, out: &mut W) -> Result<()> {
    let (repo, index) = prism::api::with_build_pool(|| -> Result<_> {
        let repo = load_repo(root).with_context(|| format!("load {}", root.display()))?;
        let index = NavigationIndex::build(&repo);
        Ok((repo, index))
    })?;
    write_repo_records(&repo, out)?;
    write_graph(&index.cpg().graph, out)
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (repo, out_path) = match args.as_slice() {
        [repo] => (repo, None),
        [repo, flag, path] if flag == "--out" => (repo, Some(path)),
        _ => bail!("usage: element_recall_graph <repo> [--out FILE]"),
    };
    match out_path {
        Some(path) => {
            let mut out = std::io::BufWriter::new(
                std::fs::File::create(path).with_context(|| format!("create {path}"))?,
            );
            read_repo(Path::new(repo), &mut out)?;
            out.flush()?;
        }
        None => {
            let mut out = std::io::BufWriter::new(std::io::stdout().lock());
            read_repo(Path::new(repo), &mut out)?;
            out.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "element_recall_graph/tests.rs"]
mod tests;
