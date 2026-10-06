// Review-only identity dumper. Copy to examples/r1_byte_dump.rs on each pinned
// archive and build --release --example r1_byte_dump --offline --locked.
use petgraph::visit::EdgeRef;
use prism::cpg::{CodePropertyGraph, CpgEdge, CpgNode, FlowConfidence, FlowDoubt, VarAccess};
use serde_json::{json, Value};
fn main() -> anyhow::Result<()> {
    let root = std::env::args().nth(1).expect("repository root");
    let repo = prism::repo_loader::load_repo(std::path::Path::new(&root))?;
    let cpg = CodePropertyGraph::build_enriched_with_scope_graph_inputs(
        &repo.files, repo.type_db.as_ref(), repo.scope_graph_inputs.as_ref());
    let mut owner_index = std::collections::BTreeMap::new();
    for (file, parsed) in &repo.files {
        for function in parsed.all_functions() {
            let Some(name) = parsed.language.function_name(&function) else { continue; };
            let key = (file.clone(), parsed.node_text(&name).to_string(), parsed.node_line_range(&function).0);
            let entry = owner_index.entry(key).or_insert_with(|| (Vec::new(), std::collections::BTreeSet::new()));
            entry.0.push((function.start_byte(), function.end_byte()));
            entry.1.extend(parsed.function_parameter_occurrences(&function).into_iter().map(|(_,start,end)| (start,end)));
        }
    }
    let endpoint = |node: &CpgNode| -> Option<Value> {
        let CpgNode::Variable { file, function, function_start_line, line, path, access, start_byte, end_byte } = node else { return None; };
        let empty = (Vec::new(), std::collections::BTreeSet::new());
        let (owners,parameters) = owner_index.get(&(file.clone(),function.clone(),*function_start_line)).unwrap_or(&empty);
        let owner = if owners.len() == 1 { json!({"name":function,"start_line":function_start_line,
            "start_byte":owners[0].0,"end_byte":owners[0].1}) }
            else { json!({"name":function,"start_line":function_start_line,"ambiguous":owners.len()}) };
        Some(json!({"file":file,"line":line,"path":path,"access":match access { VarAccess::Def=>"def",VarAccess::Use=>"use" },
            "start_byte":start_byte,"end_byte":end_byte,"owner":owner,
            "parameter":parameters.contains(&(*start_byte,*end_byte))}))
    };
    let mut rows = Vec::new();
    for edge in cpg.graph.edge_references() {
        let CpgEdge::DataFlow(label) = *edge.weight() else { continue; };
        let (Some(from),Some(to)) = (endpoint(cpg.node(edge.source())),endpoint(cpg.node(edge.target()))) else { continue; };
        let (doubt, kill_line) = match label {
            FlowConfidence::Exact => (None,None),
            FlowConfidence::NameOnly(FlowDoubt::Killed {kill_line}) => (Some("killed"),Some(kill_line)),
            FlowConfidence::NameOnly(FlowDoubt::SameLine) => (Some("sameline"),None),
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete) => (Some("cfg_incomplete"),None),
            FlowConfidence::NameOnly(FlowDoubt::AliasUnstable) => (Some("alias_unstable"),None),
            FlowConfidence::NameOnly(FlowDoubt::CallNameOnly) => (Some("call_nameonly"),None),
        };
        rows.push(json!({"from":from,"to":to,"confidence":label.level(),"doubt":doubt,"kill_line":kill_line}).to_string());
    }
    rows.sort();
    use std::io::Write;
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    for row in rows { writeln!(out,"{row}")?; }
    Ok(())
}
