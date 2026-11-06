//! SCRATCH (measure/mem-census). Census of CPG assembly structures.
use super::*;
use petgraph::graph::{Edge, Node};
use serde_json::json;
use std::collections::HashMap;
use std::mem::size_of;

struct Dup<'a> {
    m: HashMap<&'a str, usize>,
    copies: usize,
    bytes_copies: usize, // sum of capacity of all copies
}
impl<'a> Dup<'a> {
    fn new() -> Self {
        Dup {
            m: HashMap::new(),
            copies: 0,
            bytes_copies: 0,
        }
    }
    fn add(&mut self, s: &'a String) {
        *self.m.entry(s.as_str()).or_default() += 1;
        self.copies += 1;
        self.bytes_copies += s.capacity();
    }
    fn report(&self) -> serde_json::Value {
        let distinct_bytes: usize = self.m.keys().map(|k| k.len()).sum();
        json!({"copies": self.copies, "distinct": self.m.len(),
               "heap_cap_bytes_all_copies": self.bytes_copies,
               "distinct_len_bytes": distinct_bytes})
    }
}

pub(super) fn run(
    phase: &str,
    graph: &DiGraph<CpgNode, CpgEdge>,
    func_index: &BTreeMap<(String, String, usize), NodeIndex>,
    name_index: &BTreeMap<(String, String), Vec<NodeIndex>>,
    var_index: &BTreeMap<LegacyVarKey, NodeIndex>,
    exact_var_index: &ExactVarIndex,
    location_index: &BTreeMap<(String, usize), Vec<NodeIndex>>,
) {
    if !crate::census::enabled() {
        return;
    }
    let (mut nf, mut ns, mut nvd, mut nvu, mut nr) = (0usize, 0, 0, 0, 0);
    let mut heap_nodes = 0usize;
    let mut heap_by_kind = [0usize; 4];
    let mut files = Dup::new();
    let mut funcs = Dup::new();
    let mut bases = Dup::new();
    let mut paths: HashMap<(&str, &[String]), usize> = HashMap::new();
    let mut field_strs = Dup::new();
    let mut fields_vec_heap = 0usize;
    let mut stmt_call_heap = 0usize;
    let mut simple_paths = 0usize;
    for n in graph.node_weights() {
        match n {
            CpgNode::Function { name, file, .. } => {
                nf += 1;
                heap_by_kind[0] += name.capacity() + file.capacity();
                files.add(file);
                funcs.add(name);
            }
            CpgNode::Statement { file, kind, .. } => {
                ns += 1;
                let mut h = file.capacity();
                match kind {
                    StmtKind::Call { callee } => h += callee.capacity(),
                    StmtKind::Goto { target } => h += target.capacity(),
                    StmtKind::Label { name } => h += name.capacity(),
                    _ => {}
                }
                stmt_call_heap += h - file.capacity();
                heap_by_kind[1] += h;
                files.add(file);
            }
            CpgNode::Variable {
                path,
                file,
                function,
                access,
                ..
            } => {
                if *access == VarAccess::Def {
                    nvd += 1
                } else {
                    nvu += 1
                }
                let fv: usize = path.fields.iter().map(|s| s.capacity()).sum::<usize>()
                    + path.fields.capacity() * size_of::<String>();
                fields_vec_heap += fv;
                let h = path.base.capacity() + fv + file.capacity() + function.capacity();
                heap_by_kind[2] += h;
                files.add(file);
                funcs.add(function);
                bases.add(&path.base);
                for f in &path.fields {
                    field_strs.add(f);
                }
                if path.fields.is_empty() {
                    simple_paths += 1;
                }
                *paths
                    .entry((path.base.as_str(), path.fields.as_slice()))
                    .or_default() += 1;
            }
            CpgNode::ReturnValue { file, function, .. } => {
                nr += 1;
                let h = file.capacity() + function.capacity();
                heap_by_kind[3] += h;
                files.add(file);
                funcs.add(function);
            }
        }
    }
    heap_nodes += heap_by_kind.iter().sum::<usize>();
    // distinct AccessPath values
    let ap_distinct = paths.len();
    let ap_copies: usize = paths.values().sum();

    let mut ek: BTreeMap<&str, usize> = BTreeMap::new();
    let mut df_by_label: BTreeMap<&str, usize> = BTreeMap::new();
    for e in graph.edge_references() {
        let k = match e.weight() {
            CpgEdge::DataFlow(c) => {
                let l = match c {
                    FlowConfidence::Exact => "exact",
                    FlowConfidence::NameOnly(FlowDoubt::Killed { .. }) => "killed",
                    FlowConfidence::NameOnly(FlowDoubt::SameLine) => "sameline",
                    FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete) => "cfg_incomplete",
                    FlowConfidence::NameOnly(FlowDoubt::AliasUnstable) => "alias_unstable",
                    FlowConfidence::NameOnly(FlowDoubt::CallNameOnly) => "call_nameonly",
                };
                *df_by_label.entry(l).or_default() += 1;
                "DataFlow"
            }
            CpgEdge::ControlFlow => "ControlFlow",
            CpgEdge::Call(_) => "Call",
            CpgEdge::Return(_) => "Return",
            CpgEdge::ReturnFlow { .. } => "ReturnFlow",
            CpgEdge::ReturnInput => "ReturnInput",
            CpgEdge::Contains => "Contains",
            CpgEdge::FieldOf => "FieldOf",
        };
        *ek.entry(k).or_default() += 1;
    }
    let (ncap, ecap) = graph.capacity();
    let node_slot = size_of::<Node<CpgNode>>();
    let edge_slot = size_of::<Edge<CpgEdge>>();

    // index sizes
    let key_sz = size_of::<LegacyVarKey>();
    let mut vi_key_heap = 0usize;
    let mut vi_file = Dup::new();
    for k in var_index.keys() {
        vi_key_heap += k.0.capacity()
            + k.1.capacity()
            + k.4.base.capacity()
            + k.4.fields.iter().map(|s| s.capacity()).sum::<usize>()
            + k.4.fields.capacity() * size_of::<String>();
        vi_file.add(&k.0);
    }
    let mut ev_key_heap = 0usize;
    let mut ev_spans = 0usize;
    let mut ev_vec_heap = 0usize;
    for (k, spans) in exact_var_index {
        ev_key_heap += k.0.capacity()
            + k.1.capacity()
            + k.4.base.capacity()
            + k.4.fields.iter().map(|s| s.capacity()).sum::<usize>()
            + k.4.fields.capacity() * size_of::<String>();
        ev_spans += spans.len();
        for v in spans.values() {
            ev_vec_heap += v.capacity() * size_of::<NodeIndex>();
        }
    }
    let mut li_key_heap = 0usize;
    let mut li_vec_heap = 0usize;
    let mut li_nodes = 0usize;
    for (k, v) in location_index {
        li_key_heap += k.0.capacity();
        li_vec_heap += v.capacity() * size_of::<NodeIndex>();
        li_nodes += v.len();
    }
    let fi_heap: usize = func_index
        .keys()
        .map(|k| k.0.capacity() + k.1.capacity())
        .sum();
    let ni_heap: usize = name_index
        .iter()
        .map(|(k, v)| k.0.capacity() + k.1.capacity() + v.capacity() * size_of::<NodeIndex>())
        .sum();
    let vi_clone = crate::census::clone_bytes(var_index);
    let ev_clone = crate::census::clone_bytes(exact_var_index);
    let li_clone = crate::census::clone_bytes(location_index);
    let fi_clone = crate::census::clone_bytes(func_index);
    let ni_clone = crate::census::clone_bytes(name_index);

    let nodes_total = graph.node_count();
    let edges_total = graph.edge_count();
    let out = json!({
        "phase": phase,
        "live_bytes": crate::census::live(),
        "nodes": {"total": nodes_total, "function": nf, "statement": ns,
                  "variable_def": nvd, "variable_use": nvu, "return_value": nr,
                  "capacity": ncap},
        "edges": {"total": edges_total, "by_kind": ek, "dataflow_by_label": df_by_label,
                  "capacity": ecap},
        "sizes": {"size_of_CpgNode": size_of::<CpgNode>(), "size_of_CpgEdge": size_of::<CpgEdge>(),
                  "petgraph_node_slot": node_slot, "petgraph_edge_slot": edge_slot,
                  "size_of_AccessPath": size_of::<crate::access_path::AccessPath>(),
                  "size_of_LegacyVarKey": key_sz, "size_of_NodeIndex": size_of::<NodeIndex>()},
        "graph_bytes": {"node_slots_cap": ncap * node_slot, "edge_slots_cap": ecap * edge_slot,
                        "node_owned_heap_cap": heap_nodes,
                        "node_heap_by_kind[fn,stmt,var,retval]": heap_by_kind,
                        "var_fields_vec_heap": fields_vec_heap,
                        "stmt_call_label_heap": stmt_call_heap},
        "dup": {"file_strings_in_nodes": files.report(), "function_strings_in_nodes": funcs.report(),
                "var_base_strings": bases.report(), "var_field_strings": field_strs.report(),
                "access_paths": {"copies": ap_copies, "distinct": ap_distinct,
                                 "simple_paths": simple_paths},
                "var_index_key_file_strings": vi_file.report()},
        "indexes": {
            "var_index": {"entries": var_index.len(), "slot_bytes_entries_x_(K+V)": var_index.len()*(key_sz+size_of::<NodeIndex>()),
                          "key_heap_cap": vi_key_heap, "clone_measured_total": vi_clone},
            "exact_var_index": {"keys": exact_var_index.len(), "spans": ev_spans,
                                "key_heap_cap": ev_key_heap, "vec_heap": ev_vec_heap,
                                "clone_measured_total": ev_clone},
            "location_index": {"keys": location_index.len(), "node_refs": li_nodes,
                               "key_heap_cap": li_key_heap, "vec_heap": li_vec_heap,
                               "clone_measured_total": li_clone},
            "func_index": {"entries": func_index.len(), "key_heap": fi_heap, "clone_measured_total": fi_clone},
            "name_index": {"entries": name_index.len(), "heap": ni_heap, "clone_measured_total": ni_clone},
        },
    });
    eprintln!("CENSUS_JSON {}", out);
}
