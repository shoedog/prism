//! Behaviour tests for the `call_stats` histogram and its helper classifiers.

use super::*;
use crate::ast::ParsedFile;
use crate::languages::Language;

fn parse(srcs: &[(&str, &str)], lang: Language) -> BTreeMap<String, ParsedFile> {
    srcs.iter()
        .map(|(p, s)| (p.to_string(), ParsedFile::parse(p, s, lang).unwrap()))
        .collect()
}

fn rust_graph(srcs: &[(&str, &str)]) -> CallGraph {
    use crate::call_graph::ScopeGraphBuildInputs;
    use crate::name_resolution::rust_populator::RustCrateConfig;
    let files = parse(srcs, Language::Rust);
    let mut inputs = ScopeGraphBuildInputs::from_files_convention(&files);
    inputs.cfg = RustCrateConfig {
        crate_roots: files.keys().cloned().collect(),
        ..RustCrateConfig::default()
    };
    CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
}

fn site_of(cg: &CallGraph, caller: &str) -> CallSite {
    cg.calls
        .iter()
        .find(|(f, _)| f.name == caller)
        .and_then(|(_, s)| s.iter().next())
        .cloned()
        .unwrap_or_else(|| panic!("no site in {caller}"))
}

/// Two crates-roots each declaring `Foo::make`; `drive` lives next to the second.
fn two_foo_graph() -> CallGraph {
    rust_graph(&[
        ("a.rs", "pub struct Foo;\nimpl Foo { pub fn make() {} }\n"),
        (
            "b.rs",
            "pub struct Foo;\nimpl Foo { pub fn make() {} }\npub fn drive() { Foo::make(); }\n",
        ),
    ])
}

fn with_callee(site: &CallSite, callee: &str) -> CallSite {
    let mut s = site.clone();
    s.callee_name = callee.to_string();
    s
}

#[test]
fn multi_target_shape_names_the_receiver_form_of_each_callee() {
    let cg = rust_graph(&[(
        "m.rs",
        "struct Foo;\nimpl Foo { fn make(&self) {} }\nfn typed(x: Foo) { x.make(); }\nfn bare() { y.make(); }\n",
    )]);
    let typed = site_of(&cg, "typed");
    let qualified = site_of(&cg, "bare");
    assert!(typed.receiver_outcome.is_some());
    assert!(qualified.receiver_outcome.is_none() && qualified.qualifier.is_some());
    let mut bare = qualified.clone();
    bare.qualifier = None;

    let shape = |s: &CallSite| multi_target_shape(s);
    // `T::m`, optionally behind crate/super anchors, is the Option-B target shape.
    assert_eq!(shape(&with_callee(&qualified, "Foo::make")), "type_path");
    assert_eq!(shape(&with_callee(&qualified, "a::Foo::make")), "type_path");
    assert_eq!(
        shape(&with_callee(&qualified, "crate::Foo::make")),
        "type_path"
    );
    assert_eq!(
        shape(&with_callee(&qualified, "super::super::Foo::make")),
        "type_path"
    );
    // `self::m` and any `..::Self::m` are self paths, with anchors stripped first.
    assert_eq!(shape(&with_callee(&qualified, "self::make")), "self_path");
    assert_eq!(
        shape(&with_callee(&qualified, "crate::self::make")),
        "self_path"
    );
    assert_eq!(shape(&with_callee(&qualified, "Self::make")), "self_path");
    assert_eq!(
        shape(&with_callee(&qualified, "a::Self::make")),
        "self_path"
    );
    // Nothing left after the anchors: no owner to shape.
    assert_eq!(shape(&with_callee(&qualified, "crate::make")), "unshaped");
    // Without `::` the receiver outcome wins over the qualifier, then the qualifier, else none.
    assert_eq!(shape(&typed), "receiver_typed");
    assert_eq!(shape(&qualified), "qualifier_field");
    assert_eq!(shape(&bare), "unshaped");
}

#[test]
fn shadow_narrow_reports_each_pool_outcome_of_a_type_path_site() {
    let mut cg = rust_graph(&[(
        "a.rs",
        "mod a { pub struct Foo; impl Foo { pub fn make() {} } }\npub fn drive() { a::Foo::make(); }\n",
    )]);
    let site = site_of(&cg, "drive");
    assert_eq!(site.callee_name, "a::Foo::make");
    // The type path resolves to one in-repo scope holding exactly one `make`.
    assert_eq!(shadow_narrow_type_path(&cg, &site), "singleton");
    // A crate anchor is dropped before the owner path is resolved.
    assert_eq!(
        shadow_narrow_type_path(&cg, &with_callee(&site, "crate::a::Foo::make")),
        "singleton"
    );
    // The resolved type has no such method.
    assert_eq!(
        shadow_narrow_type_path(&cg, &with_callee(&site, "a::Foo::absent")),
        "failopen_no_method"
    );
    // The owner path names no in-repo type.
    assert_eq!(
        shadow_narrow_type_path(&cg, &with_callee(&site, "zzz::Foo::make")),
        "failopen_type_unresolved"
    );
    // Two candidates under the resolved scope would still demote.
    let key = cg
        .methods_by_scope
        .keys()
        .find(|(_, name)| name == "make")
        .cloned()
        .expect("make is indexed under its type scope");
    let dup = cg.methods_by_scope[&key][0].clone();
    cg.methods_by_scope.get_mut(&key).unwrap().push(dup);
    assert_eq!(shadow_narrow_type_path(&cg, &site), "multiple");
    // No scope graph at all.
    cg.scope_graph = None;
    assert_eq!(shadow_narrow_type_path(&cg, &site), "failopen_no_graph");
}

#[test]
fn recovery_classifier_reports_what_the_disproof_prune_decided() {
    let mut cg = two_foo_graph();
    let site = site_of(&cg, "drive");
    assert_eq!(site.callee_name, "Foo::make");
    // The caller's own `Foo` disproves the other file's candidate: one survivor.
    assert_eq!(classify_recovery_typepath(&cg, &site), "singleton");
    // A crate anchor is stripped before the owner key is read.
    assert_eq!(
        classify_recovery_typepath(&cg, &with_callee(&site, "crate::Foo::make")),
        "singleton"
    );
    // The prune disproved one candidate but two copies of the survivor remain.
    let key = ("Foo".to_string(), "make".to_string());
    let survivor = cg.methods[&key]
        .iter()
        .find(|id| id.file == "b.rs")
        .cloned()
        .unwrap();
    cg.methods.get_mut(&key).unwrap().push(survivor);
    assert_eq!(classify_recovery_typepath(&cg, &site), "pruned_multiple");

    // A caller that sees neither `Foo` proves nothing: the bare pool of two demotes.
    let cg = rust_graph(&[
        ("a.rs", "pub struct Foo;\nimpl Foo { pub fn make() {} }\n"),
        ("b.rs", "pub struct Foo;\nimpl Foo { pub fn make() {} }\n"),
        ("c.rs", "pub fn drive() { Foo::make(); }\n"),
    ]);
    assert_eq!(
        classify_recovery_typepath(&cg, &site_of(&cg, "drive")),
        "failopen_demote"
    );
    // The same with a single `Foo`: nothing to prune, nothing colliding.
    let cg = rust_graph(&[(
        "a.rs",
        "pub struct Foo;\nimpl Foo { pub fn make() {} }\npub fn drive() { Foo::make(); }\n",
    )]);
    assert_eq!(
        classify_recovery_typepath(&cg, &site_of(&cg, "drive")),
        "failopen_singleton"
    );
}

#[test]
fn recovery_classifier_leaves_non_owner_method_sites_out() {
    let base = two_foo_graph();
    let site = site_of(&base, "drive");
    let out = |cg: &CallGraph, s: &CallSite| classify_recovery_typepath(cg, s);
    // No owner left after the anchors, a `self`/`Self` head, or an owner with no method pool.
    assert_eq!(
        out(&base, &with_callee(&site, "crate::make")),
        "not_owner_method"
    );
    assert_eq!(
        out(&base, &with_callee(&site, "self::make")),
        "not_owner_method"
    );
    assert_eq!(
        out(&base, &with_callee(&site, "Self::make")),
        "not_owner_method"
    );
    assert_eq!(
        out(&base, &with_callee(&site, "Zed::make")),
        "not_owner_method"
    );
    assert_eq!(
        out(&base, &with_callee(&site, "Foo::absent")),
        "not_owner_method"
    );
    // The site never reached the prune: unknown file, unmapped byte, incomplete or missing graph.
    let mut unknown_file = site.clone();
    unknown_file.caller.file = "nope.rs".to_string();
    assert_eq!(out(&base, &unknown_file), "not_owner_method");
    let mut unmapped = site.clone();
    unmapped.start_byte = usize::MAX;
    assert_eq!(out(&base, &unmapped), "not_owner_method");
    let mut incomplete = two_foo_graph();
    incomplete.scope_graph.as_mut().unwrap().complete = false;
    assert_eq!(out(&incomplete, &site), "not_owner_method");
    let mut none = two_foo_graph();
    none.scope_graph = None;
    assert_eq!(out(&none, &site), "not_owner_method");
}

#[test]
fn call_stats_counts_recovered_owner_site_and_skips_non_owner_sites() {
    let stats = call_stats(&two_foo_graph());
    assert_eq!(
        stats["recovery_typepath"],
        serde_json::json!({"singleton": 1})
    );
    assert_eq!(stats["kinds"], serde_json::json!({"qualified_owner": 1}));
    assert_eq!(
        stats["kind_exact"],
        serde_json::json!({"qualified_owner": 1})
    );
    assert_eq!(stats["demoted_edges"], 0);
    assert_eq!(stats["multi_target_exact_sites"], 0);
    // A `::` site whose owner has no method pool is not a recovery outcome.
    let stats = call_stats(&rust_graph(&[(
        "m.rs",
        "fn drive() { Missing::make(); }\n",
    )]));
    assert_eq!(stats["recovery_typepath"], serde_json::json!({}));
    assert_eq!(stats["total_call_sites"], 1);
}

#[test]
fn call_stats_stratifies_r6_single_owner_demotes_by_recovery_and_method_kind() {
    let stats = call_stats(&rust_graph(&[(
        "m.rs",
        "struct A;\nimpl A { fn poll(&self) {} }\nfn drive() { x.poll(); }\n",
    )]));
    assert_eq!(stats["kinds"], serde_json::json!({"r6_single_owner": 1}));
    assert_eq!(
        stats["kind_nameonly"],
        serde_json::json!({"r6_single_owner": 1})
    );
    assert_eq!(stats["kind_exact"], serde_json::json!({}));
    assert_eq!(stats["demoted_edges"], 1);
    assert_eq!(stats["r6_single_owner_rust"], 1);
    assert_eq!(
        stats["nameonly_by_recovery_methodkind"],
        serde_json::json!({"none/inherent": 1})
    );
    assert_eq!(stats["wrapper_peel_clone_demotes"], 0);
    // Same shape in Python: counted as R6 but not as a Rust R6.
    let files = parse(
        &[(
            "m.py",
            "class A:\n    def poll(self):\n        pass\n\ndef drive(x):\n    x.poll()\n",
        )],
        Language::Python,
    );
    let stats = call_stats(&CallGraph::build(&files));
    assert_eq!(stats["kinds"], serde_json::json!({"r6_single_owner": 1}));
    assert_eq!(stats["r6_single_owner_rust"], 0);
}

#[test]
fn call_stats_counts_std_wrapper_peel_clone_demote() {
    let stats = call_stats(&rust_graph(&[(
        "m.rs",
        "use std::rc::Rc;\nstruct Foo;\nimpl Clone for Foo { fn clone(&self) -> Foo { Foo } }\nfn drive(x: Rc<Foo>) { x.clone(); }\n",
    )]));
    assert_eq!(
        stats["kind_nameonly"],
        serde_json::json!({"typed_param": 1})
    );
    assert_eq!(
        stats["nameonly_by_recovery_methodkind"],
        serde_json::json!({"StdWrapperPeel/trait": 1})
    );
    assert_eq!(stats["wrapper_peel_clone_demotes"], 1);
    assert_eq!(stats["demoted_edges"], 1);
}

#[test]
fn call_stats_counts_two_owner_collision_as_a_multi_owner_drop() {
    let stats = call_stats(&rust_graph(&[
        ("a.rs", "struct A;\nimpl A { fn poll(&self) {} }\n"),
        ("b.rs", "struct B;\nimpl B { fn poll(&self) {} }\n"),
        ("m.rs", "fn drive() { x.poll(); }\n"),
    ]));
    assert_eq!(stats["dropped_multi_owner"], 1);
    assert_eq!(stats["unresolved_unknown_name"], 0);
    assert_eq!(stats["kinds"], serde_json::json!({}));
    assert_eq!(stats["demoted_edges"], 0);
}

#[test]
fn call_stats_counts_go_registrations_as_nameonly_callback_edges() {
    let files = parse(
        &[(
            "main.go",
            "package main\ntype Command struct {\n\tRun func()\n}\nfunc helper() {}\nfunc main() {\n\tc := Command{Run: helper}\n\t_ = c\n}\n",
        )],
        Language::Go,
    );
    let stats = call_stats(&CallGraph::build(&files));
    assert_eq!(stats["callback_registrations_recorded"], 1);
    assert_eq!(
        stats["kinds"],
        serde_json::json!({"callback_registration": 1})
    );
    assert_eq!(
        stats["kind_nameonly"],
        serde_json::json!({"callback_registration": 1})
    );
    assert_eq!(stats["demoted_edges"], 1);
}

#[test]
fn call_stats_counts_python_property_accesses_and_cached_property_getters() {
    let cached = parse(
        &[(
            "r.py",
            "import functools\nclass R:\n    @functools.cached_property\n    def text(self):\n        return 1\n    def dump(self):\n        return self.text\n",
        )],
        Language::Python,
    );
    let stats = call_stats(&CallGraph::build(&cached));
    assert_eq!(stats["property_accesses_recorded"], 1);
    assert_eq!(stats["property_access_cached_property_recorded"], 1);
    assert_eq!(stats["kinds"], serde_json::json!({"property_access": 1}));
    assert_eq!(
        stats["kind_nameonly"],
        serde_json::json!({"property_access": 1})
    );
    assert_eq!(stats["demoted_edges"], 1);
    // A plain `@property` is an access but not a cached one.
    let plain = parse(
        &[(
            "r.py",
            "class R:\n    @property\n    def text(self):\n        return 1\n    def dump(self):\n        return self.text\n",
        )],
        Language::Python,
    );
    let stats = call_stats(&CallGraph::build(&plain));
    assert_eq!(stats["property_accesses_recorded"], 1);
    assert_eq!(stats["property_access_cached_property_recorded"], 0);
    assert_eq!(stats["kinds"], serde_json::json!({"property_access": 1}));
}

#[test]
fn call_stats_counts_func_value_field_fan_out_beyond_the_cap_as_a_drop() {
    let files = parse(
        &[(
            "main.go",
            "package main\ntype Command struct {\n\tRun func()\n}\nfunc h1() {}\nfunc h2() {}\nfunc h3() {}\nfunc h4() {}\n\
func register_a() *Command { return &Command{Run: h1} }\nfunc register_b() *Command { return &Command{Run: h2} }\n\
func register_c() *Command { return &Command{Run: h3} }\nfunc register_d() *Command { return &Command{Run: h4} }\n\
func invoke(cmd *Command) {\n\tcmd.Run()\n}\n",
        )],
        Language::Go,
    );
    let stats = call_stats(&CallGraph::build(&files));
    assert_eq!(stats["dropped_func_value_fanout"], 1);
    assert_eq!(stats["callback_registrations_recorded"], 4);
    // Only the four registrations are edges; the diffuse invocation resolves to none.
    assert_eq!(
        stats["kinds"],
        serde_json::json!({"callback_registration": 4})
    );
    assert_eq!(stats["dropped_external_receiver"], 0);
}

#[test]
fn call_stats_counts_go_package_clause_filtered_candidates_as_a_drop() {
    let files = parse(
        &[
            ("a/blackbox_test.go", "package a_test\nfunc helper() {}\n"),
            ("a/use_test.go", "package a\nfunc TestIt() { helper() }\n"),
        ],
        Language::Go,
    );
    let stats = call_stats(&CallGraph::build(&files));
    assert_eq!(stats["go_same_pkg_all_filtered_drop"], 1);
    assert_eq!(stats["unresolved_unknown_name"], 0);
    assert_eq!(stats["kinds"], serde_json::json!({}));
}

#[test]
fn call_stats_counts_wrapped_export_reached_by_a_non_jsx_call() {
    let mut files = parse(
        &[(
            "lib.tsx",
            "import { memo } from 'react';\nexport const Island = memo((p: any) => null);\nexport const x = 1;\n",
        )],
        Language::Tsx,
    );
    files.extend(parse(
        &[(
            "app.tsx",
            "import { Island } from './lib';\nexport function App() {\n  Island({});\n  return <Island/>;\n}\n",
        )],
        Language::Tsx,
    ));
    let stats = call_stats(&CallGraph::build(&files));
    assert_eq!(stats["dropped_wrapped_export_non_jsx"], 1);
    assert_eq!(stats["js_export_spanned_admitted"], 1);
}
