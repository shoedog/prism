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
    let (_, _, qualifier, start_byte, _, receiver_expr, _, _) = parsed
        .function_calls_with_qualifier_and_spans_on_lines(&func_node, &all_lines)
        .into_iter()
        .find(|(name, _, _, _, _, _, _, _)| name == callee)?;
    super::RustReceiverTyper::new(cg).type_of_receiver(super::ReceiverTypeCtx {
        parsed,
        caller: &caller,
        fn_node: func_node,
        receiver_expr,
        qualifier: qualifier.as_deref(),
        call_start_byte: start_byte,
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
