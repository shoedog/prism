//! E2 payload into a bare-arrow target in another file (lane 9, review round 1 finding F1).
use super::*;
use crate::ast::ParsedFile;
use crate::languages::Language;

fn build(arrow: &str) -> CodePropertyGraph {
    let holder = "export function run(cb) {\n  const input = read();\n  cb(input);\n}\n";
    let user = format!(
        "import {{ run }} from './holder.js';\nexport function main() {{\n  run({arrow});\n}}\n"
    );
    let files = BTreeMap::from([
        (
            "holder.js".to_string(),
            ParsedFile::parse("holder.js", holder, Language::JavaScript).unwrap(),
        ),
        (
            "b.js".to_string(),
            ParsedFile::parse("b.js", &user, Language::JavaScript).unwrap(),
        ),
    ]);
    CodePropertyGraph::build(&files)
}

/// Does a DataFlow edge run from the `input` Use in holder.js to a `w` Def in b.js?
fn payload_edge(cpg: &CodePropertyGraph) -> bool {
    cpg.graph.edge_indices().any(|e| {
        let CpgEdge::DataFlow(_) = cpg.graph[e] else { return false };
        let Some((from, to)) = cpg.graph.edge_endpoints(e) else { return false };
        matches!(
            (cpg.node(from), cpg.node(to)),
            (
                CpgNode::Variable { file: ff, path: fp, access: VarAccess::Use, .. },
                CpgNode::Variable { file: tf, path: tp, access: VarAccess::Def, .. },
            ) if ff == "holder.js" && tf == "b.js" && fp.to_string() == "input" && tp.to_string() == "w"
        )
    })
}

#[test]
fn a_parenthesized_cross_file_arrow_gets_the_payload_edge() {
    assert!(payload_edge(&build("(w) => use(w)")));
}

#[test]
fn a_bare_cross_file_arrow_gets_the_payload_edge_when_argument_follow_credits_it() {
    assert!(payload_edge(&build("w => use(w)")));
}
