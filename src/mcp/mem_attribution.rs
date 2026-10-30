//! Component counts that attribute an index's memory independently of resident-size deltas
//! (shared-daemon spec §6.1). Counts are exact; byte figures derived from them are estimates and
//! say so in their names. The bytes the retained syntax trees hold are not estimated here: tree
//! nodes a traversal visits do not determine them (tree-sitter also allocates hidden nodes), so
//! the report measures them directly (`mem_tree_alloc`).
use crate::cpg::{CpgNode, StmtKind};
use crate::navigation::NavigationSession;
use std::collections::HashSet;

/// Small-allocation quantum assumed for string heap estimates.
const STRING_HEAP_QUANTUM: u64 = 16;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Attribution {
    pub files: u64,
    pub source_bytes: u64,
    /// Nodes a traversal of each parsed tree visits (`ParsedFile::parse_node_count`). It is not
    /// the number of subtrees tree-sitter allocates, which also includes hidden nodes, and it
    /// is not a size.
    pub tree_visible_nodes: u64,
    pub cpg_nodes: u64,
    pub cpg_edges: u64,
    /// `size_of::<CpgNode>()` times the node count: the inline payload, strings' heap excluded.
    pub cpg_node_inline_bytes: u64,
    pub node_strings: u64,
    pub node_string_bytes: u64,
    pub node_string_heap_bytes_estimate: u64,
    pub node_unique_strings: u64,
    pub node_unique_string_bytes: u64,
    pub node_unique_string_heap_bytes_estimate: u64,
    /// Unique bytes over total bytes; 1.0 means no string is repeated. `None` with no strings.
    pub node_string_unique_ratio: Option<f64>,
}

/// Count the components of a built session. Borrows every string; allocates one hash set of
/// string slices, so call it after the memory samples are taken, not between them.
pub fn attribute(session: &NavigationSession) -> Attribution {
    let files = session.repo.files.len() as u64;
    let source_bytes = session
        .repo
        .files
        .values()
        .map(|file| file.source.len() as u64)
        .sum();
    let tree_visible_nodes: u64 = session
        .repo
        .files
        .values()
        .map(|file| file.parse_node_count as u64)
        .sum();

    let cpg = session.index.cpg();
    let mut tally = StringTally::default();
    for index in cpg.node_indices() {
        tally.add_node(cpg.node(index));
    }
    let cpg_nodes = cpg.node_count() as u64;

    Attribution {
        files,
        source_bytes,
        tree_visible_nodes,
        cpg_nodes,
        cpg_edges: cpg.edge_count() as u64,
        cpg_node_inline_bytes: cpg_nodes * std::mem::size_of::<CpgNode>() as u64,
        node_strings: tally.count,
        node_string_bytes: tally.bytes,
        node_string_heap_bytes_estimate: tally.heap_bytes_estimate,
        node_unique_strings: tally.unique.len() as u64,
        node_unique_string_bytes: tally.unique_bytes(),
        node_unique_string_heap_bytes_estimate: tally.unique_heap_bytes_estimate(),
        node_string_unique_ratio: (tally.bytes > 0)
            .then(|| tally.unique_bytes() as f64 / tally.bytes as f64),
    }
}

fn heap_bytes_estimate(len: usize) -> u64 {
    if len == 0 {
        0
    } else {
        (len as u64).div_ceil(STRING_HEAP_QUANTUM) * STRING_HEAP_QUANTUM
    }
}

/// Totals and the distinct set of every string a CPG node owns.
#[derive(Default)]
struct StringTally<'a> {
    count: u64,
    bytes: u64,
    heap_bytes_estimate: u64,
    unique: HashSet<&'a str>,
}

impl<'a> StringTally<'a> {
    fn add(&mut self, text: &'a str) {
        self.count += 1;
        self.bytes += text.len() as u64;
        self.heap_bytes_estimate += heap_bytes_estimate(text.len());
        self.unique.insert(text);
    }

    fn unique_bytes(&self) -> u64 {
        self.unique.iter().map(|text| text.len() as u64).sum()
    }

    fn unique_heap_bytes_estimate(&self) -> u64 {
        self.unique
            .iter()
            .map(|text| heap_bytes_estimate(text.len()))
            .sum()
    }

    /// Every `String` the node owns. The `match` is exhaustive on purpose: a new node variant or
    /// string-bearing statement kind must be counted here before this compiles.
    fn add_node(&mut self, node: &'a CpgNode) {
        match node {
            CpgNode::Function { name, file, .. } => {
                self.add(name);
                self.add(file);
            }
            CpgNode::Statement { file, kind, .. } => {
                self.add(file);
                match kind {
                    StmtKind::Call { callee } => self.add(callee),
                    StmtKind::Goto { target } => self.add(target),
                    StmtKind::Label { name } => self.add(name),
                    StmtKind::Assignment
                    | StmtKind::Return
                    | StmtKind::Branch
                    | StmtKind::Loop
                    | StmtKind::Declaration
                    | StmtKind::Other => {}
                }
            }
            CpgNode::Variable {
                path,
                file,
                function,
                ..
            } => {
                self.add(&path.base);
                self.add(file);
                self.add(function);
                for field in &path.fields {
                    self.add(field);
                }
            }
            CpgNode::ReturnValue { file, function, .. } => {
                self.add(file);
                self.add(function);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access_path::AccessPath;
    use crate::cpg::VarAccess;

    fn function(name: &str, file: &str) -> CpgNode {
        CpgNode::Function {
            name: name.into(),
            file: file.into(),
            start_line: 1,
            end_line: 2,
            start_byte: 0,
            end_byte: 10,
        }
    }

    #[test]
    fn tally_counts_every_string_payload_and_its_duplicates() {
        let nodes = [
            function("f", "a.py"),
            function("g", "a.py"),
            CpgNode::Statement {
                file: "a.py".into(),
                line: 2,
                kind: StmtKind::Call { callee: "f".into() },
                start_byte: 0,
                end_byte: 3,
            },
            CpgNode::Statement {
                file: "a.py".into(),
                line: 3,
                kind: StmtKind::Return,
                start_byte: 0,
                end_byte: 3,
            },
            CpgNode::Variable {
                path: AccessPath {
                    base: "self".into(),
                    fields: vec!["config".into(), "timeout_seconds_value".into()],
                },
                file: "a.py".into(),
                function: "g".into(),
                function_start_line: 1,
                line: 2,
                access: VarAccess::Use,
                start_byte: 0,
                end_byte: 4,
            },
        ];
        let mut tally = StringTally::default();
        for node in &nodes {
            tally.add_node(node);
        }
        // f, a.py | g, a.py | a.py, f | a.py | self, a.py, g, config, timeout_seconds_value
        assert_eq!(tally.count, 12);
        assert_eq!(tally.bytes, 1 + 4 + 1 + 4 + 4 + 1 + 4 + 4 + 4 + 1 + 6 + 21);
        // unique: f, a.py, g, self, config, timeout_seconds_value
        assert_eq!(tally.unique.len(), 6);
        assert_eq!(tally.unique_bytes(), 1 + 4 + 1 + 4 + 6 + 21);
        // every string here is 1..=16 bytes except the 21-byte one, which rounds to 32
        assert_eq!(tally.heap_bytes_estimate, 11 * 16 + 32);
        assert_eq!(tally.unique_heap_bytes_estimate(), 5 * 16 + 32);
    }

    #[test]
    fn goto_and_label_names_are_string_payloads_too() {
        let nodes: Vec<CpgNode> = [
            StmtKind::Goto {
                target: "out".into(),
            },
            StmtKind::Label { name: "out".into() },
        ]
        .into_iter()
        .map(|kind| CpgNode::Statement {
            file: "a.c".into(),
            line: 1,
            kind,
            start_byte: 0,
            end_byte: 1,
        })
        .collect();
        let mut tally = StringTally::default();
        for node in &nodes {
            tally.add_node(node);
        }
        assert_eq!(tally.count, 4);
        assert_eq!(tally.unique.len(), 2);
    }

    #[test]
    fn a_return_value_node_owns_its_file_and_function_strings() {
        let node = CpgNode::ReturnValue {
            file: "lib.rs".into(),
            function: "compute_total".into(),
            function_start_line: 3,
            line: 7,
            return_start_byte: 40,
            return_end_byte: 60,
            child_slot: 0,
            start_byte: 47,
            end_byte: 60,
        };
        let mut tally = StringTally::default();
        tally.add_node(&node);
        tally.add_node(&node);
        assert_eq!(tally.count, 4, "file and function, once per node");
        assert_eq!(
            tally.bytes,
            2 * ("lib.rs".len() + "compute_total".len()) as u64
        );
        assert_eq!(tally.unique.len(), 2);
        assert_eq!(tally.heap_bytes_estimate, 4 * 16);
        assert_eq!(tally.unique_bytes(), (6 + 13) as u64);
    }

    #[test]
    fn attribute_counts_a_real_session() {
        let source = "def f():\n    return 1\n\ndef g():\n    return f()\n";
        let session = crate::mcp::tools::test_support::session(&[("a.py", source)]);
        let got = attribute(&session);

        assert_eq!(got.files, 1);
        assert_eq!(got.source_bytes, source.len() as u64);
        let parsed_nodes = session.repo.files["a.py"].parse_node_count as u64;
        assert!(parsed_nodes > 0);
        assert_eq!(got.tree_visible_nodes, parsed_nodes);
        assert_eq!(got.cpg_nodes, session.index.cpg().node_count() as u64);
        assert_eq!(got.cpg_edges, session.index.cpg().edge_count() as u64);
        assert_eq!(
            got.cpg_node_inline_bytes,
            got.cpg_nodes * std::mem::size_of::<CpgNode>() as u64
        );
        // Two functions in one file: the file name alone repeats.
        assert!(got.node_strings >= 4);
        assert!(got.node_unique_strings < got.node_strings);
        assert!(got.node_unique_string_bytes < got.node_string_bytes);
        let ratio = got.node_string_unique_ratio.expect("strings exist");
        assert!(ratio > 0.0 && ratio < 1.0, "ratio {ratio}");
        assert_eq!(
            ratio,
            got.node_unique_string_bytes as f64 / got.node_string_bytes as f64
        );
    }

    #[test]
    fn an_empty_session_has_no_ratio() {
        let session = crate::mcp::tools::test_support::session(&[]);
        let got = attribute(&session);
        assert_eq!(got.files, 0);
        assert_eq!(got.node_strings, 0);
        assert_eq!(got.node_string_unique_ratio, None);
    }
}
