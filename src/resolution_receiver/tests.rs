use crate::ast::ParsedFile;
use crate::call_graph::{CallGraph, ScopeGraphBuildInputs};
use crate::languages::Language::Rust;
use crate::name_resolution::rust_populator::RustCrateConfig;
use crate::resolution::ReceiverRecovery;
use crate::resolution_identity::{ReceiverOutcome, ReceiverTypeKey};
use std::collections::{BTreeMap, BTreeSet};

fn files(srcs: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    let mut files = BTreeMap::new();
    for (path, source) in srcs {
        files.insert(
            path.to_string(),
            ParsedFile::parse(path, source, Rust).unwrap(),
        );
    }
    files
}

fn graph(files: &BTreeMap<String, ParsedFile>) -> CallGraph {
    let mut inputs = ScopeGraphBuildInputs::from_files_convention(files);
    inputs.cfg = RustCrateConfig {
        crate_roots: files.keys().cloned().collect(),
        ..RustCrateConfig::default()
    };
    CallGraph::build_with_scope_graph_inputs(files, Some(&inputs))
}

fn ty(
    files: &BTreeMap<String, ParsedFile>,
    cg: &CallGraph,
    function: &str,
    callee: &str,
) -> Option<ReceiverOutcome> {
    let caller = cg
        .functions
        .values()
        .flatten()
        .find(|f| f.name == function)
        .cloned()
        .unwrap();
    let parsed = files.get(&caller.file).expect("parsed file");
    let func_node = parsed.all_functions().into_iter().find(|node| {
        parsed
            .language
            .function_name(node)
            .is_some_and(|name| parsed.node_text(&name) == function)
    })?;
    let all_lines: BTreeSet<usize> = (caller.start_line..=caller.end_line).collect();
    let (metas, _facts) = parsed.function_calls_with_qualifier_and_spans_on_lines(
        &func_node,
        &all_lines,
        &BTreeSet::new(),
    );
    let meta = metas.into_iter().find(|meta| meta.callee_name == callee)?;
    super::RustReceiverTyper::new(cg).type_of_receiver(super::ReceiverTypeCtx {
        parsed,
        caller: &caller,
        fn_node: func_node,
        receiver_expr: meta.receiver_node,
        qualifier: meta.qualifier.as_deref(),
        call_start_byte: meta.start_byte,
    })
}

fn assert_ty(
    files: &BTreeMap<String, ParsedFile>,
    cg: &CallGraph,
    function: &str,
    callee: &str,
    recovery: ReceiverRecovery,
    bare: &str,
) {
    let got = ty(files, cg, function, callee).unwrap();
    assert_eq!(got.recovery, recovery);
    assert_eq!(got.bare, bare);
}

fn type_scope(cg: &CallGraph, name: &str) -> crate::name_resolution::types::ScopeId {
    let graph = cg.scope_graph.as_ref().unwrap();
    graph
        .bindings
        .iter()
        .find_map(|binding| {
            (binding.name == name)
                .then(|| match &binding.target {
                    crate::name_resolution::types::BindTarget::Resolved(
                        crate::name_resolution::types::Target::Item {
                            owns: Some(scope), ..
                        },
                    ) => Some(*scope),
                    _ => None,
                })
                .flatten()
        })
        .unwrap_or_else(|| panic!("missing type scope for {name}"))
}

fn type_scope_in_module(
    cg: &CallGraph,
    module: &str,
    name: &str,
) -> crate::name_resolution::types::ScopeId {
    let graph = cg.scope_graph.as_ref().unwrap();
    let module_scope = graph
        .bindings
        .iter()
        .find_map(|binding| match &binding.target {
            crate::name_resolution::types::BindTarget::Resolved(
                crate::name_resolution::types::Target::Scope(scope),
            ) if binding.name == module => Some(*scope),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing module scope for {module}"));
    graph
        .bindings
        .iter()
        .find_map(|binding| match &binding.target {
            crate::name_resolution::types::BindTarget::Resolved(
                crate::name_resolution::types::Target::Item {
                    owns: Some(scope), ..
                },
            ) if binding.scope == module_scope && binding.name == name => Some(*scope),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing type scope for {module}::{name}"))
}

#[test]
fn method_call_parts_decomposes_nested_arg_chain_from_ast() {
    let files = files(&[(
        "main.rs",
        "pub struct A;\n\
         pub fn drive(a: A) { a.b(1).c(2, 3).d(); }\n",
    )]);
    let parsed = files.get("main.rs").expect("parsed file");
    let func_node = parsed
        .all_functions()
        .into_iter()
        .find(|node| {
            parsed
                .language
                .function_name(node)
                .is_some_and(|name| parsed.node_text(&name) == "drive")
        })
        .expect("drive function");
    let all_lines: BTreeSet<usize> = (1..=3).collect();
    let (metas, _facts) = parsed.function_calls_with_qualifier_and_spans_on_lines(
        &func_node,
        &all_lines,
        &BTreeSet::new(),
    );
    let meta = metas
        .into_iter()
        .find(|meta| meta.callee_name == "d")
        .expect("d call");
    let c_call = meta.receiver_node.expect("receiver for d is the c call");
    assert_eq!(c_call.kind(), "call_expression");

    let c = super::method_call_parts(parsed, c_call).expect("c method call");
    assert_eq!(c.method, "c");
    assert_eq!(c.arg_count, 2);
    assert_eq!(parsed.node_text(&c.receiver), "a.b(1)");

    let b = super::method_call_parts(parsed, c.receiver).expect("b method call");
    assert_eq!(b.method, "b");
    assert_eq!(b.arg_count, 1);
    assert_eq!(parsed.node_text(&b.receiver), "a");
}

#[test]
fn dispatch_method_single_exact_filters_kind_arity_self_and_wrapper_peel() {
    let files = files(&[(
        "lib.rs",
        "pub struct Inner;\n\
         impl Inner { pub fn step(&self, n: u8) -> Inner { Inner } pub fn assoc() -> Inner { Inner } }\n\
         pub struct Outer;\n\
         trait T { fn m(&self); }\n\
         impl Outer { fn m(&self) {} }\n\
         impl T for Outer { fn m(&self) {} }\n",
    )]);
    let cg = graph(&files);
    let inner = type_scope(&cg, "Inner");
    let outer = type_scope(&cg, "Outer");

    assert!(super::dispatch_method_single_exact(
        &cg,
        inner,
        "step",
        ReceiverRecovery::TypedParam,
        Some(1),
        false,
    )
    .is_some());
    assert_eq!(
        super::dispatch_method_single_exact(
            &cg,
            inner,
            "step",
            ReceiverRecovery::TypedParam,
            Some(0),
            false,
        ),
        None
    );
    assert_eq!(
        super::dispatch_method_single_exact(
            &cg,
            inner,
            "assoc",
            ReceiverRecovery::TypedParam,
            Some(0),
            false,
        ),
        None
    );
    assert_eq!(
        super::dispatch_method_single_exact(
            &cg,
            outer,
            "m",
            ReceiverRecovery::TypedParam,
            Some(0),
            false,
        ),
        None
    );
    assert_eq!(
        super::dispatch_method_single_exact(
            &cg,
            inner,
            "step",
            ReceiverRecovery::StdWrapperPeel,
            Some(1),
            false,
        ),
        None
    );
}

#[test]
fn rust_receiver_typer_recovers_param_field_return_wrapper_and_path() {
    let files = files(&[(
        "main.rs",
        "pub mod a { pub struct Foo; impl Foo { pub fn m(&self){} } }\
         pub struct W; impl W { pub fn go(&self){} }\
         pub struct Inner; impl Inner { pub fn poke(&self){} }\
         pub struct Outer { pub inner: Inner }\
         pub struct R; impl R { pub fn run(&self){} } pub fn make() -> R { R }\
         pub fn typed_param(o: Outer) { o.take(); }\
         pub fn fielded(o: Outer) { let x = o.inner; x.poke(); }\
         pub fn returned() { let x = make(); x.run(); }\
         pub fn wrapped(x: Box<W>) { x.go(); }\
         pub fn pathy(x: crate::a::Foo) { x.m(); }",
    )]);
    let cg = graph(&files);
    assert_ty(
        &files,
        &cg,
        "typed_param",
        "take",
        ReceiverRecovery::TypedParam,
        "Outer",
    );
    assert_ty(
        &files,
        &cg,
        "fielded",
        "poke",
        ReceiverRecovery::FieldTyped,
        "Inner",
    );
    assert_ty(
        &files,
        &cg,
        "returned",
        "run",
        ReceiverRecovery::ReturnTyped,
        "R",
    );
    assert_ty(
        &files,
        &cg,
        "wrapped",
        "go",
        ReceiverRecovery::StdWrapperPeel,
        "W",
    );
    assert_ty(
        &files,
        &cg,
        "pathy",
        "m",
        ReceiverRecovery::TypedParam,
        "Foo",
    );
}

#[test]
fn rust_receiver_typer_preserves_cross_module_identity_for_field_return_and_self() {
    let files = files(&[(
        "main.rs",
        "pub mod a {
             pub struct Inner;
             impl Inner { pub fn m(&self){} }
             pub struct Outer { pub inner: Inner }
             pub fn make() -> Inner { Inner }
             impl Outer {
                 pub fn fielded(&self) { let x = self.inner; x.m(); }
                 pub fn direct_self(&self) { self.take(); }
             }
         }
         pub mod b {
             pub struct Inner;
             impl Inner { pub fn m(&self){} }
             pub fn returned() { let x = crate::a::make(); x.m(); }
         }",
    )]);
    let cg = graph(&files);
    let a_inner = type_scope_in_module(&cg, "a", "Inner");
    let b_inner = type_scope_in_module(&cg, "b", "Inner");
    let a_outer = type_scope_in_module(&cg, "a", "Outer");
    assert_ne!(a_inner, b_inner);

    let fielded = ty(&files, &cg, "fielded", "m").unwrap();
    assert_eq!(fielded.recovery, ReceiverRecovery::FieldTyped);
    assert_eq!(fielded.bare, "Inner");
    assert_eq!(fielded.key, ReceiverTypeKey::InRepo(a_inner));

    let returned = ty(&files, &cg, "returned", "m").unwrap();
    assert_eq!(returned.recovery, ReceiverRecovery::ReturnTyped);
    assert_eq!(returned.bare, "Inner");
    assert_eq!(returned.key, ReceiverTypeKey::InRepo(a_inner));

    let self_ty = ty(&files, &cg, "direct_self", "take").unwrap();
    assert_eq!(self_ty.recovery, ReceiverRecovery::TypedParam);
    assert_eq!(self_ty.bare, "Outer");
    assert_eq!(self_ty.key, ReceiverTypeKey::InRepo(a_outer));
}

#[test]
fn rust_receiver_typer_recovers_direct_call_with_args_and_typed_let() {
    let files = files(&[(
        "main.rs",
        "pub struct R; impl R { pub fn run(&self){} } pub fn make(_: i32) -> R { R }
         pub struct T; impl T { pub fn go(&self){} }
         pub fn returned() { let x = make(1); x.run(); }
         pub fn typed_let() { let x: T = T; x.go(); }",
    )]);
    let cg = graph(&files);

    let returned = ty(&files, &cg, "returned", "run").unwrap();
    assert_eq!(returned.recovery, ReceiverRecovery::ReturnTyped);
    assert_eq!(returned.bare, "R");
    assert_eq!(returned.key, ReceiverTypeKey::InRepo(type_scope(&cg, "R")));

    let typed_let = ty(&files, &cg, "typed_let", "go").unwrap();
    assert_eq!(typed_let.recovery, ReceiverRecovery::TypedLet);
    assert_eq!(typed_let.bare, "T");
    assert_eq!(typed_let.key, ReceiverTypeKey::InRepo(type_scope(&cg, "T")));
}

#[test]
fn rust_receiver_typer_falls_through_generic_and_unresolved() {
    let files = files(&[(
        "main.rs",
        "pub fn generic<T>(x: T) { x.go(); } pub fn unresolved() { mystery().go(); }",
    )]);
    let cg = graph(&files);

    assert!(ty(&files, &cg, "generic", "go").is_none());
    assert!(ty(&files, &cg, "unresolved", "go").is_none());
}

#[test]
fn rust_receiver_chain_depth_cap_fails_closed() {
    let files = files(&[(
        "main.rs",
        "pub struct B;\n\
         impl B { pub fn a(&self) -> B { B } pub fn run(&self) {} }\n\
         pub fn drive(b: B) { b.a().a().a().a().a().run(); }\n",
    )]);
    let cg = graph(&files);
    assert_eq!(super::MAX_RECEIVER_TYPE_DEPTH, 4);
    assert!(
        ty(&files, &cg, "drive", "run").is_none(),
        "chain deeper than MAX_RECEIVER_TYPE_DEPTH must fail closed"
    );
}

#[test]
fn rust_receiver_local_fact_cycle_fails_closed() {
    let files = files(&[(
        "main.rs",
        "pub struct Inner; impl Inner { pub fn run(&self) {} }\n\
         pub struct Holder { pub inner: Inner }\n\
         pub fn drive() { let a = a.inner; a.run(); }\n",
    )]);
    let cg = graph(&files);
    assert!(
        ty(&files, &cg, "drive", "run").is_none(),
        "TypeVisit locals cycle guard must fail closed"
    );
}
