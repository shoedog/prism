//! Lane A2 S1: guards for anonymous call-graph nodes. The nodes are not minted
//! on main yet, so these tests inject `<anon@L:C>` Function and
//! Variable nodes straight into a built graph, then check that position lookups
//! skip them by default and the opt-in variants find them.

use super::{CodePropertyGraph, CpgEdge, CpgNode, VarAccess};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::languages::Language;
use petgraph::graph::NodeIndex;
use std::collections::BTreeMap;

pub(crate) const FILE: &str = "a.js";
/// `outer` spans lines 1-3; line 2 holds a one-line arrow callback.
pub(crate) const SRC: &str = "function outer(a) {\n  return [1].map((x) => x + a);\n}\n";

pub(crate) fn parse_files(src: &str) -> BTreeMap<String, ParsedFile> {
    let parsed = ParsedFile::parse(FILE, src, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(FILE.to_string(), parsed);
    files
}

pub(crate) fn build() -> CodePropertyGraph {
    CodePropertyGraph::build(&parse_files(SRC))
}

/// Add an anonymous Function node on `line` (single-line), registered in the
/// same indexes the builder fills for named functions.
pub(crate) fn add_anonymous_function(
    cpg: &mut CodePropertyGraph,
    name: &str,
    line: usize,
) -> NodeIndex {
    let idx = cpg.graph.add_node(CpgNode::Function {
        name: name.to_string(),
        file: FILE.to_string(),
        start_line: line,
        end_line: line,
        start_byte: 0,
        end_byte: 0,
    });
    cpg.func_index
        .insert((FILE.to_string(), name.to_string(), line), idx);
    cpg.location_index
        .entry((FILE.to_string(), line))
        .or_default()
        .push(idx);
    idx
}

pub(crate) fn function_idx(cpg: &CodePropertyGraph, name: &str) -> NodeIndex {
    cpg.node_indices()
        .find(|&i| matches!(cpg.node(i), CpgNode::Function { name: n, .. } if n == name))
        .unwrap_or_else(|| panic!("no function {name}"))
}

/// Add a Variable owned by `owner` and a Contains edge from `container`.
pub(crate) fn add_variable(
    cpg: &mut CodePropertyGraph,
    container: NodeIndex,
    owner: &str,
    owner_start_line: usize,
    line: usize,
) -> NodeIndex {
    let idx = cpg.graph.add_node(CpgNode::Variable {
        path: AccessPath::simple("x"),
        file: FILE.to_string(),
        function: owner.to_string(),
        function_start_line: owner_start_line,
        line,
        access: VarAccess::Def,
        start_byte: 0,
        end_byte: 0,
    });
    cpg.location_index
        .entry((FILE.to_string(), line))
        .or_default()
        .push(idx);
    cpg.graph.add_edge(container, idx, CpgEdge::Contains);
    idx
}

fn id_name(cpg: &CodePropertyGraph, file: &str, line: usize, anon: bool) -> Option<String> {
    let found = if anon {
        cpg.function_at_including_anonymous(file, line)
    } else {
        cpg.function_at(file, line)
    };
    found.map(|(_, id)| id.name)
}

#[test]
fn function_at_skips_anonymous_nodes_by_default() {
    for anon_name in ["<anon@2:21>", "<anon@2:21>"] {
        let mut cpg = build();
        add_anonymous_function(&mut cpg, anon_name, 2);
        // Without the guard the one-line anonymous node wins the smallest-range tie.
        assert_eq!(id_name(&cpg, FILE, 2, false).as_deref(), Some("outer"));
        assert_eq!(id_name(&cpg, FILE, 1, false).as_deref(), Some("outer"));
    }
}

#[test]
fn function_at_including_anonymous_finds_the_anonymous_node() {
    let cpg = build();
    assert_eq!(id_name(&cpg, FILE, 2, true).as_deref(), Some("<anon@2:18>"));
    // Outside the anonymous node's range the opt-in variant still answers with the named fn.
    assert_eq!(id_name(&cpg, FILE, 3, true).as_deref(), Some("outer"));
}

#[test]
fn function_at_named_control_is_unchanged_without_anonymous_nodes() {
    let cpg = CodePropertyGraph::build(&parse_files("function outer(a) {\n  return a;\n}\n"));
    for line in 1..=3 {
        assert_eq!(id_name(&cpg, FILE, line, false).as_deref(), Some("outer"));
        assert_eq!(id_name(&cpg, FILE, line, true).as_deref(), Some("outer"));
    }
    assert_eq!(id_name(&cpg, FILE, 9, false), None);
    assert_eq!(id_name(&cpg, "other.js", 2, false), None);
}

#[test]
fn function_starts_at_ignores_anonymous_nodes_by_default() {
    let mut cpg = build();
    add_anonymous_function(&mut cpg, "<anon@2:21>", 2);
    assert!(!cpg.function_starts_at(FILE, 2));
    assert!(cpg.function_starts_at_including_anonymous(FILE, 2));
    // Named control: `outer` starts on line 1 under both variants.
    assert!(cpg.function_starts_at(FILE, 1));
    assert!(cpg.function_starts_at_including_anonymous(FILE, 1));
    assert!(!cpg.function_starts_at(FILE, 3));
}
