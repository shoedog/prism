//! SCRATCH (measure/mem-census2): call-graph structure census. Never merged.
use crate::call_graph::{CallGraph, CallSite, FunctionId};
use crate::census::{drop_measure, live};
use std::collections::HashMap;

struct Dup<'a> {
    m: HashMap<&'a str, usize>,
    copies: usize,
    cap_bytes: usize,
}
impl<'a> Dup<'a> {
    fn new() -> Self {
        Dup { m: HashMap::new(), copies: 0, cap_bytes: 0 }
    }
    fn add(&mut self, s: &'a String) {
        *self.m.entry(s.as_str()).or_default() += 1;
        self.copies += 1;
        self.cap_bytes += s.capacity();
    }
    fn json(&self, name: &str) -> String {
        let distinct_bytes: usize = self.m.keys().map(|k| k.len()).sum();
        format!(
            "\"{name}\":{{\"copies\":{},\"distinct\":{},\"cap_bytes_all_copies\":{},\"distinct_len_bytes\":{}}}",
            self.copies,
            self.m.len(),
            self.cap_bytes,
            distinct_bytes
        )
    }
}

fn fid<'a>(f: &'a FunctionId, files: &mut Dup<'a>, names: &mut Dup<'a>) {
    files.add(&f.file);
    names.add(&f.name);
}

pub fn census_cg(cg: &CallGraph) {
    if !crate::census::enabled() {
        return;
    }
    let (mut files, mut names, mut callee, mut qual, mut rtype, mut srcname) =
        (Dup::new(), Dup::new(), Dup::new(), Dup::new(), Dup::new(), Dup::new());
    let mut n_calls = 0usize;
    let mut site_inline_heap = 0usize;
    let mut count = |s: &CallSite| {
        site_inline_heap += s.callee_name.capacity()
            + s.qualifier.as_ref().map_or(0, |q| q.capacity())
            + s.receiver_type.as_ref().map_or(0, |q| q.capacity())
            + s.source_callee_name.as_ref().map_or(0, |q| q.capacity());
    };
    // forward edges: key FunctionId + each CallSite (with caller FunctionId inside)
    for (k, set) in &cg.calls {
        let _ = k;
        for s in set {
            n_calls += 1;
            count(s);
        }
    }
    let mut fw_files = Dup::new();
    let mut fw_names = Dup::new();
    for (k, set) in &cg.calls {
        fid(k, &mut fw_files, &mut fw_names);
        for s in set {
            fid(&s.caller, &mut fw_files, &mut fw_names);
            callee.add(&s.callee_name);
            if let Some(q) = &s.qualifier {
                qual.add(q);
            }
            if let Some(q) = &s.receiver_type {
                rtype.add(q);
            }
            if let Some(q) = &s.source_callee_name {
                srcname.add(q);
            }
        }
    }
    let mut rv_files = Dup::new();
    let mut rv_names = Dup::new();
    let mut n_rev = 0usize;
    let mut rev_keys = Dup::new();
    for (k, v) in &cg.callers {
        rev_keys.add(k);
        for s in v {
            n_rev += 1;
            fid(&s.caller, &mut rv_files, &mut rv_names);
        }
    }
    let mut fn_files = Dup::new();
    let mut fn_names = Dup::new();
    let mut fn_keys = Dup::new();
    let mut n_funcs = 0usize;
    for (k, v) in &cg.functions {
        fn_keys.add(k);
        for f in v {
            n_funcs += 1;
            fid(f, &mut fn_files, &mut fn_names);
        }
    }
    let _ = (&mut files, &mut names);
    eprintln!(
        "CENSUS_CG {{\"functions_keys\":{},\"function_ids\":{},\"forward_sources\":{},\"forward_sites\":{},\"reverse_keys\":{},\"reverse_sites\":{},\"size_of_CallSite\":{},\"size_of_FunctionId\":{},\"site_own_string_cap_bytes\":{},{},{},{},{},{},{},{},{},{},{},{}}}",
        cg.functions.len(),
        n_funcs,
        cg.calls.len(),
        n_calls,
        cg.callers.len(),
        n_rev,
        std::mem::size_of::<CallSite>(),
        std::mem::size_of::<FunctionId>(),
        site_inline_heap,
        fw_files.json("fwd_fid_file"),
        fw_names.json("fwd_fid_name"),
        rv_files.json("rev_caller_file"),
        rv_names.json("rev_caller_name"),
        fn_files.json("functions_fid_file"),
        fn_names.json("functions_fid_name"),
        callee.json("callee_name"),
        qual.json("qualifier"),
        rtype.json("receiver_type"),
        srcname.json("source_callee_name"),
        rev_keys.json("reverse_key")
    );
    let _ = live();
}

pub fn drop_cg(cg: &mut CallGraph) {
    macro_rules! d {
        ($f:ident) => {
            drop_measure(stringify!($f), std::mem::take(&mut cg.$f));
        };
    }
    d!(calls);
    d!(callers);
    d!(functions);
    d!(methods);
    d!(method_owners);
    d!(return_types);
    d!(receiver_vars);
    d!(method_arity);
    d!(method_facts);
    d!(imports);
    d!(import_bindings);
    d!(module_bindings);
    d!(js_ts_exports);
    d!(js_ts_function_locals);
    d!(js_ts_return_facts);
    d!(methods_by_scope);
    d!(field_types);
    d!(interface_impls);
    d!(class_bases);
    d!(go_registrations);
    d!(go_file_profiles);
    d!(clean_class_spans);
    d!(method_class_span);
    d!(static_functions);
    d!(indexed_files);
    drop_measure("scope_graph", cg.scope_graph.take());
    let rest = std::mem::replace(cg, CallGraph::empty());
    drop_measure("rest_of_call_graph", rest);
}
