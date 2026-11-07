use super::*;
use crate::languages::Language::Go;
use crate::resolution::{ReceiverRecovery, ResolutionConfidence, ResolutionKind};
use std::collections::BTreeMap;

fn build_go(files: &[(&str, &str)]) -> CallGraph {
    let mut map = BTreeMap::new();
    for (path, src) in files {
        map.insert(path.to_string(), ParsedFile::parse(path, src, Go).unwrap());
    }
    CallGraph::build(&map)
}

fn build_go_module(files: &[(&str, &str)], module: &str) -> CallGraph {
    let mut map = BTreeMap::new();
    for (path, src) in files {
        map.insert(path.to_string(), ParsedFile::parse(path, src, Go).unwrap());
    }
    let root = tempfile::tempdir().expect("temporary Go module");
    std::fs::write(
        root.path().join("go.mod"),
        format!("module {module}\n\ngo 1.24\n"),
    )
    .expect("write go.mod fixture");
    let inputs = crate::repo_loader::scope_graph_build_inputs(root.path(), &map);
    CallGraph::build_with_scope_graph_inputs(&map, Some(&inputs))
}

fn site_in<'a>(cg: &'a CallGraph, caller: &str, callee: &str) -> &'a CallSite {
    cg.calls
        .iter()
        .find(|(fid, _)| fid.name == caller)
        .and_then(|(_, sites)| sites.iter().find(|s| s.callee_name == callee))
        .unwrap_or_else(|| panic!("no call site {caller} -> {callee}"))
}

fn main_owner(name: &str) -> crate::resolution::GoOwnerIdentity {
    crate::resolution::GoOwnerIdentity {
        package_dir: String::new(),
        package_clause: "main".to_string(),
        name: name.to_string(),
    }
}

// ---- S1: call-RHS return-typed recovery ----------------------------

#[test]
fn s1_call_rhs_bare_name_same_file_recovers_return_typed() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Demux struct{}\n\
             func (d *Demux) Init(n int) {}\n\
             func newDemux(a, b int) *Demux { return &Demux{} }\n\
             func run() {\n\td := newDemux(16, 16)\n\td.Init(1)\n}\n",
    )]);
    let site = site_in(&cg, "run", "Init");
    assert_eq!(site.receiver_type.as_deref(), Some("Demux"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::ReturnTyped));
    let out = cg.resolve_call_site_full(site);
    assert!(out.resolved.iter().any(|c| c.target.name == "Init"
        && c.confidence == ResolutionConfidence::Exact
        && c.kind == ResolutionKind::ReturnTyped));
}

#[test]
fn s1_call_rhs_cross_file_same_package_recovers() {
    // The adjudicated etcd shape: the constructor and the receiver
    // methods live in the SAME package (directory) but DIFFERENT files.
    let cg = build_go(&[
        (
            "demux.go",
            "package demux\n\
                 type Demux struct{}\n\
                 func (d *Demux) Init(n int) {}\n\
                 func (d *Demux) Register(w int, id int) {}\n\
                 func newDemux(a, b int) *Demux { return &Demux{} }\n",
        ),
        (
            "demux_test.go",
            "package demux\n\
                 func run() {\n\td := newDemux(16, 16)\n\td.Init(1)\n\td.Register(2, 0)\n}\n",
        ),
    ]);
    let init_site = site_in(&cg, "run", "Init");
    assert_eq!(init_site.receiver_type.as_deref(), Some("Demux"));
    assert_eq!(
        init_site.receiver_recovery,
        Some(ReceiverRecovery::ReturnTyped)
    );
    let reg_site = site_in(&cg, "run", "Register");
    assert_eq!(reg_site.receiver_type.as_deref(), Some("Demux"));
    let out = cg.resolve_call_site_full(reg_site);
    assert!(out
        .resolved
        .iter()
        .any(|c| c.target.name == "Register" && c.confidence == ResolutionConfidence::Exact));
}

#[test]
fn s1_multi_return_beyond_t_error_drops() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Demux struct{}\n\
             func (d *Demux) Init() {}\n\
             func newDemux() (*Demux, int, error) { return nil, 0, nil }\n\
             func run() {\n\td := newDemux()\n\td.Init()\n}\n",
    )]);
    let site = site_in(&cg, "run", "Init");
    assert_eq!(site.receiver_type, None);
}

#[test]
fn s1_paired_error_return_type_recovers() {
    // Opus impl-review Important 1: the mandated `(T, error)` POSITIVE
    // test — a well-formed 2-tuple return with `error` correctly in the
    // second position must still recover T (pins the take-T branch of
    // `extract_one_return_type`'s `parameter_list` match arm; the
    // existing negative tests only cover the 3-tuple-drop and
    // wrong-position-drop shapes, never this one actually recovering).
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Demux struct{}\n\
             func (d *Demux) Init() {}\n\
             func newDemux() (*Demux, error) { return &Demux{}, nil }\n\
             func run() {\n\td, err := newDemux()\n\t_ = err\n\td.Init()\n}\n",
    )]);
    let site = site_in(&cg, "run", "Init");
    assert_eq!(site.receiver_type.as_deref(), Some("Demux"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::ReturnTyped));
}

#[test]
fn s1_non_first_lhs_position_does_not_recover() {
    // Opus impl-review minor: strengthened to ISOLATE the first-LHS-
    // position guard. `newDemux2`'s return is a WELL-FORMED `(*Demux,
    // error)` pair (error correctly in the second position) -- if it
    // were instead the wrong-order `(error, *Demux)` shape (the
    // original fixture), `extract_one_return_type`'s OWN
    // `second_bare != "error"` gate would already reject it, so the
    // test would keep passing even if the first-LHS-position check were
    // weakened/removed (a false sense of coverage). With a
    // well-formed pair, a weakened guard would flip this to
    // `Some("Demux")`, so the assertion is now tied directly to the
    // position check.
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Demux struct{}\n\
             func (d *Demux) Init() {}\n\
             func newDemux2() (*Demux, error) { return nil, nil }\n\
             func run() {\n\terr, d := newDemux2()\n\t_ = err\n\td.Init()\n}\n",
    )]);
    // `d` is bound at LHS position 1 (not first) -> S1 must not recover it,
    // regardless of what the callee returns.
    let site = site_in(&cg, "run", "Init");
    assert_eq!(site.receiver_type, None);
}

#[test]
fn s1_generic_function_return_drops() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Demux struct{}\n\
             func (d *Demux) Init() {}\n\
             func newDemux[T any]() *Demux { return &Demux{} }\n\
             func run() {\n\td := newDemux[int]()\n\td.Init()\n}\n",
    )]);
    let site = site_in(&cg, "run", "Init");
    assert_eq!(site.receiver_type, None);
}

#[test]
fn s1_import_qualified_call_rhs_resolves_via_package_dir() {
    let cg = build_go_module(
        &[
            (
                "factory/factory.go",
                "package factory\n\
                 type Widget struct{}\n\
                 func (w *Widget) Use() {}\n\
                 func New() *Widget { return &Widget{} }\n",
            ),
            (
                "main.go",
                "package main\n\
                 import \"example.com/repo/factory\"\n\
                 func run() {\n\tw := factory.New()\n\tw.Use()\n}\n",
            ),
        ],
        "example.com/repo",
    );
    let site = site_in(&cg, "run", "Use");
    assert_eq!(site.receiver_type.as_deref(), Some("Widget"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::ReturnTyped));
}

#[test]
fn s1_cross_package_same_name_constructor_pins_own_package_type() {
    // Two UNRELATED packages each declare a bare (same-name, unqualified)
    // `New()` constructor returning their OWN package's type. S1's index
    // is keyed by clause-bearing package/function identity
    // (`extract_go_return_types`),
    // so this must not collide -- each package's `x := New()` recovers
    // to ITS OWN type, never the other's.
    let cg = build_go(&[
        (
            "pkg1/pkg1.go",
            "package pkg1\n\
                 type T1 struct{}\n\
                 func (t *T1) M() {}\n\
                 func New() *T1 { return &T1{} }\n\
                 func run1() {\n\tx := New()\n\tx.M()\n}\n",
        ),
        (
            "pkg2/pkg2.go",
            "package pkg2\n\
                 type T2 struct{}\n\
                 func (t *T2) M() {}\n\
                 func New() *T2 { return &T2{} }\n\
                 func run2() {\n\tx := New()\n\tx.M()\n}\n",
        ),
    ]);
    let site1 = site_in(&cg, "run1", "M");
    assert_eq!(site1.receiver_type.as_deref(), Some("T1"));
    let site2 = site_in(&cg, "run2", "M");
    assert_eq!(site2.receiver_type.as_deref(), Some("T2"));
}

// ---- S2: nested-selector field-typed recovery -----------------------

#[test]
fn s2_one_hop_field_selector_recovers_field_typed() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Inner struct{}\n\
             func (i *Inner) M() {}\n\
             type Outer struct {\n\tField *Inner\n}\n\
             func run(o *Outer) {\n\to.Field.M()\n}\n",
    )]);
    let site = site_in(&cg, "run", "M");
    assert_eq!(site.receiver_type.as_deref(), Some("Inner"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
    let out = cg.resolve_call_site_full(site);
    assert!(out.resolved.iter().any(|c| c.target.name == "M"
        && c.confidence == ResolutionConfidence::Exact
        && c.kind == ResolutionKind::FieldTyped));
}

#[test]
fn s2_two_hop_field_selector_recovers() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Innermost struct{}\n\
             func (i *Innermost) M() {}\n\
             type Middle struct {\n\tLeaf *Innermost\n}\n\
             type Outer struct {\n\tMid *Middle\n}\n\
             func run(o *Outer) {\n\to.Mid.Leaf.M()\n}\n",
    )]);
    let site = site_in(&cg, "run", "M");
    assert_eq!(site.receiver_type.as_deref(), Some("Innermost"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
}

#[test]
fn s2_three_hop_field_selector_drops() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type L4 struct{}\n\
             func (l *L4) M() {}\n\
             type L3 struct {\n\tNext *L4\n}\n\
             type L2 struct {\n\tNext *L3\n}\n\
             type L1 struct {\n\tNext *L2\n}\n\
             func run(o *L1) {\n\to.Next.Next.Next.M()\n}\n",
    )]);
    let site = site_in(&cg, "run", "M");
    assert_eq!(site.receiver_type, None, "3-hop chain must not recover");
}

#[test]
fn s2_embedded_field_selector_recovers() {
    // A TRUE anonymous/embedded field (`Listener`, no field name,
    // non-pointer) — its implicit field name is the bare type name
    // `Listener` (`GoStruct::extract_one_field`), so
    // `w.Listener.Accept()` is the correct accessor shape (mirrors the
    // adjudicated caddy `l.Listener.Accept()` case, but with an IN-REPO
    // `Listener` so this one DOES flip to a FieldTyped Exact edge).
    //
    // Non-pointer embedding deliberately, not `*Listener`: grounding
    // (tree-sitter dump) found `extract_one_field` does not recognize a
    // POINTER-embedded field at all (tree-sitter-go represents it as a
    // bare `*` token sibling, not a `pointer_type` wrapper it expects) —
    // a PRE-EXISTING gap in the already-shipped struct-embedding
    // extraction, untouched by P11 and out of scope here; see the task
    // report.
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Listener struct{}\n\
             func (l *Listener) Accept() {}\n\
             type Wrap struct {\n\tListener\n}\n\
             func run(w *Wrap) {\n\tw.Listener.Accept()\n}\n",
    )]);
    let site = site_in(&cg, "run", "Accept");
    assert_eq!(site.receiver_type.as_deref(), Some("Listener"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
}

#[test]
fn s2_field_miss_at_any_hop_drops_entirely() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Outer struct {\n\tField int\n}\n\
             func run(o *Outer) {\n\to.Missing.M()\n}\n",
    )]);
    // `Missing` is not a field of `Outer` at all -- the field_types
    // lookup at this hop misses, so the whole chain must drop (no
    // partial recovery), not panic.
    let site = site_in(&cg, "run", "M");
    assert_eq!(site.receiver_type, None);
}

#[test]
fn s2_closure_param_shadowing_base_does_not_leak_stale_recovery() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Inner struct{}\n\
             func (i *Inner) M() {}\n\
             type Outer struct {\n\tField *Inner\n}\n\
             func run(o *Outer) {\n\
             \tfn := func(o int) {\n\t\t_ = o\n\t}\n\
             \tfn(1)\n\
             \to.Field.M()\n\
             }\n",
    )]);
    // The closure param `o int` does NOT shadow the OUTER `o.Field.M()`
    // call (it's a sibling statement, not enclosing it) -- must still
    // recover normally. This is the companion positive case to the
    // fail-closed shadow test below (proves the closure fix doesn't
    // over-suppress unrelated recoveries).
    let site = site_in(&cg, "run", "M");
    assert_eq!(site.receiver_type.as_deref(), Some("Inner"));
}

#[test]
fn s2_closure_param_rebinding_base_inside_closure_fails_closed() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Inner struct{}\n\
             func (i *Inner) M() {}\n\
             type Other struct{}\n\
             func (x Other) M() {}\n\
             type Outer struct {\n\tO *Inner\n}\n\
             func run(o *Outer) {\n\
             \tfn := func(o Other) {\n\t\to.M()\n\t}\n\
             \t_ = fn\n\
             }\n",
    )]);
    // The call site `o.M()` is INSIDE the closure, whose OWN parameter
    // `o Other` shadows the outer `o *Outer`. Without the closure-shadow
    // fix, the outer-function walk would see only the OUTER `o` binding
    // (the closure's own param is invisible to `function_node_types()`)
    // and could leak a stale/wrong R1/R2 proof. P17 retains the first type
    // only as the legacy R3 input, and marks that proof shadowed so it
    // cannot mint a direct edge against the outer declaration.
    let site = site_in(&cg, "run", "M");
    assert_eq!(site.receiver_type.as_deref(), Some("Outer"));
    assert!(site.receiver_local_type_shadowed);
    assert!(cg.resolve_call_site(site).is_empty());
}

// ---- B1 (codex impl-review BLOCKER): func_literal lexical-scope fence -

#[test]
fn b1_s1_sibling_closure_short_var_does_not_leak_across_lexical_scope() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Demux struct{}\n\
             func (d *Demux) Init(n int) {}\n\
             func newDemux(a, b int) *Demux { return &Demux{} }\n\
             func run() {\n\
             \tfn := func() {\n\t\td := newDemux(16, 16)\n\t\t_ = d\n\t}\n\
             \t_ = fn\n\
             \td.Init(1)\n\
             }\n",
    )]);
    // `d` in `run`'s OWN scope has no binding at all -- only the SIBLING
    // closure's `d := newDemux(...)` does, and that closure does not
    // contain this call. Without the lexical-scope fence, the walk
    // descends into the sibling closure anyway and finds `d`'s binding
    // there, minting a false `ReturnTyped` recovery for an unrelated
    // (undefined-here) `d`. Must NOT recover.
    let site = site_in(&cg, "run", "Init");
    assert_eq!(site.receiver_type, None);
}

#[test]
fn b1_s1_call_inside_closure_recovers_from_its_own_binding() {
    // Positive control: the call IS inside the closure, so the closure's
    // OWN `d := newDemux(...)` binding is genuinely in scope and must
    // still recover (proves the fence doesn't over-suppress).
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Demux struct{}\n\
             func (d *Demux) Init(n int) {}\n\
             func newDemux(a, b int) *Demux { return &Demux{} }\n\
             func run() {\n\
             \tfn := func() {\n\t\td := newDemux(16, 16)\n\t\td.Init(1)\n\t}\n\
             \tfn()\n\
             }\n",
    )]);
    let site = site_in(&cg, "run", "Init");
    assert_eq!(site.receiver_type.as_deref(), Some("Demux"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::ReturnTyped));
}

#[test]
fn b1_s2_sibling_closure_base_short_var_does_not_leak_across_lexical_scope() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Inner struct{}\n\
             func (i *Inner) M() {}\n\
             type Outer struct {\n\tField *Inner\n}\n\
             func newOuter() *Outer { return &Outer{} }\n\
             func run() {\n\
             \tfn := func() {\n\t\to := newOuter()\n\t\t_ = o\n\t}\n\
             \t_ = fn\n\
             \to.Field.M()\n\
             }\n",
    )]);
    // Same lexical-scope leak, via S2's base recovery (which recurses
    // through the SAME S1 machinery for the base identifier `o`).
    let site = site_in(&cg, "run", "M");
    assert_eq!(site.receiver_type, None);
}

#[test]
fn b1_real_outer_binding_survives_sibling_closure_same_name_binding() {
    // Pins the 903->939 anomaly mechanism (codex re-review MINOR): pre-fix,
    // `walk_receiver_bindings`'s unfenced `func_literal` arm counted the
    // sibling closure's OWN `d := other()` binding as a SECOND binding of
    // `d` in `run`'s scope (on top of the genuine outer `d :=
    // newDemux(...)`), inflating `bindings` from the correct 1 to 2.
    // `call_graph/go/receiver_index.rs`'s partition-aware classifier then bails
    // at its `if bindings > 1 { return baseline; }` gate (~line 389)
    // BEFORE ever attempting the S1 call-RHS retry -- so the outer,
    // wholly unambiguous `d.Init(1)` lost its recovery entirely, not just
    // a false one. Post-fix (B1 lexical-scope fence), the sibling
    // closure's subtree is skipped outright, `bindings` is correctly 1,
    // and the S1 call-RHS retry recovers `Demux`/`ReturnTyped` as it
    // should.
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Demux struct{}\n\
             func (d *Demux) Init(n int) {}\n\
             func newDemux(a, b int) *Demux { return &Demux{} }\n\
             func other() *Demux { return &Demux{} }\n\
             func run() {\n\
             \td := newDemux(16, 16)\n\
             \tgo func() {\n\t\td := other()\n\t\t_ = d\n\t}()\n\
             \td.Init(1)\n\
             }\n",
    )]);
    let site = site_in(&cg, "run", "Init");
    assert_eq!(
        site.receiver_type.as_deref(),
        Some("Demux"),
        "the real outer `d := newDemux(...)` binding must recover \
             despite the sibling closure's own same-name `d` binding: {:?}",
        site
    );
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::ReturnTyped));
}

// ---- S3: package-level var receivers ---------------------------------

#[test]
fn s3_package_level_var_recovers_var_decl() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Runner interface { Go() }\n\
             type Impl struct{}\n\
             func (i Impl) Go() {}\n\
             var r Runner\n\
             func run() {\n\tr.Go()\n}\n",
    )]);
    let site = site_in(&cg, "run", "Go");
    assert_eq!(site.receiver_type.as_deref(), Some("Runner"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::VarDecl));
    let out = cg.resolve_call_site_full(site);
    assert!(out
        .resolved
        .iter()
        .any(|c| c.target.name == "Go" && c.confidence == ResolutionConfidence::Exact));
}

#[test]
fn s3_function_local_binding_shadows_package_var() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Runner interface { Go() }\n\
             type Impl struct{}\n\
             func (i Impl) Go() {}\n\
             type Other struct{}\n\
             var r Runner\n\
             func run() {\n\tr := Other{}\n\t_ = r\n}\n\
             func run2() {\n\tr.Go()\n}\n",
    )]);
    // `run2` has no local `r` -- package var recovers.
    let site2 = site_in(&cg, "run2", "Go");
    assert_eq!(site2.receiver_type.as_deref(), Some("Runner"));
}

#[test]
fn s3_same_function_local_binding_shadows_package_var() {
    // Minor (both reviews): the test above isn't a REAL shadow test --
    // `run2` has no local `r` binding AT ALL, so it only proves S3
    // recovers in the ABSENCE of a local, not that a local correctly
    // shadows the package var. This is the real same-function case: `r`
    // is bound locally IN THE SAME function that calls `r.Go()` -- the
    // local (`Other`) must win, never falling back to the package var's
    // `Runner` type.
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Runner interface { Go() }\n\
             type Impl struct{}\n\
             func (i Impl) Go() {}\n\
             type Other struct{}\n\
             func (o Other) Go() {}\n\
             var r Runner\n\
             func run() {\n\tr := Other{}\n\tr.Go()\n}\n",
    )]);
    let site = site_in(&cg, "run", "Go");
    assert_eq!(
        site.receiver_type.as_deref(),
        Some("Other"),
        "local `r := Other{{}}` must shadow the package `var r Runner`"
    );
}

#[test]
fn s3_package_var_cross_file_in_same_package() {
    let cg = build_go(&[
        (
            "vars.go",
            "package main\n\
                 type Runner interface { Go() }\n\
                 var r Runner\n",
        ),
        (
            "impl.go",
            "package main\n\
                 type Impl struct{}\n\
                 func (i Impl) Go() {}\n\
                 func run() {\n\tr.Go()\n}\n",
        ),
    ]);
    let site = site_in(&cg, "run", "Go");
    assert_eq!(site.receiver_type.as_deref(), Some("Runner"));
}

// ---- S4: embedded-interface satisfaction/routing ---------------------

#[test]
fn s4_struct_embeds_interface_routes_to_concrete_implementer() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Doer interface { Do() }\n\
             type Concrete struct{}\n\
             func (c Concrete) Do() {}\n\
             type Holder struct {\n\tDoer\n}\n\
             func run(h Holder) {\n\th.Do()\n}\n",
    )]);
    let site = site_in(&cg, "run", "Do");
    let out = cg.resolve_call_site_full(site);
    assert!(
        out.resolved.iter().any(|c| c.target.name == "Do"
            && c.confidence == ResolutionConfidence::Exact
            && c.kind == ResolutionKind::InterfaceDispatch),
        "expected Exact InterfaceDispatch via embedded interface, got {:?}",
        out
    );
    assert_eq!(
        cg.go_embedded_interface_route("Holder", None, "Do", "main.go")
            .value
            .as_deref(),
        Some("Doer")
    );
}

#[test]
fn s2_pointer_embedded_field_recovers_nested_selector_receiver() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Listener struct{}\n\
             func (l *Listener) Serve() {}\n\
             type Holder struct { *Listener }\n\
             func run(h Holder) { h.Listener.Serve() }\n",
    )]);
    let site = site_in(&cg, "run", "Serve");
    assert_eq!(site.receiver_type.as_deref(), Some("Listener"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
    let out = cg.resolve_call_site_full(site);
    assert!(out.resolved.iter().any(|candidate| {
        candidate.target.name == "Serve"
            && candidate.confidence == ResolutionConfidence::Exact
            && candidate.kind == ResolutionKind::FieldTyped
    }));
}

#[test]
fn s2_pointer_embedded_field_routes_only_to_its_proven_package() {
    let cg = build_go(&[
        (
            "a/types.go",
            "package a\n\
                 type Listener struct{}\n\
                 func (*Listener) Serve() {}\n\
                 type Holder struct { *Listener }\n\
                 func run(h Holder) { h.Listener.Serve() }\n",
        ),
        (
            "b/types.go",
            "package b\n\
                 type Listener struct{}\n\
                 func (*Listener) Serve() {}\n",
        ),
    ]);
    let site = site_in(&cg, "run", "Serve");
    assert_eq!(site.receiver_type.as_deref(), Some("Listener"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
    assert!(
        site.receiver_outcome.is_some(),
        "target proof was not carried"
    );
    let round_tripped: CallSite = bincode::deserialize(&bincode::serialize(site).unwrap()).unwrap();
    assert_eq!(round_tripped.receiver_outcome, site.receiver_outcome);

    let out = cg.resolve_call_site_full(&round_tripped);
    assert_eq!(out.resolved.len(), 1, "package decoy leaked: {out:?}");
    assert_eq!(out.resolved[0].target.file, "a/types.go");
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out.resolved[0].kind, ResolutionKind::FieldTyped);
}

#[test]
fn s2_value_embedded_field_routes_only_to_its_proven_package() {
    let cg = build_go(&[
        (
            "a/types.go",
            "package a\n\
                 type Listener struct{}\n\
                 func (Listener) Serve() {}\n\
                 type Holder struct { Listener }\n\
                 func run(h Holder) { h.Listener.Serve() }\n",
        ),
        (
            "b/types.go",
            "package b\n\
                 type Listener struct{}\n\
                 func (Listener) Serve() {}\n",
        ),
    ]);
    let site = site_in(&cg, "run", "Serve");
    assert_eq!(site.receiver_type.as_deref(), Some("Listener"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));

    let out = cg.resolve_call_site_full(site);
    assert_eq!(out.resolved.len(), 1, "package decoy leaked: {out:?}");
    assert_eq!(out.resolved[0].target.file, "a/types.go");
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out.resolved[0].kind, ResolutionKind::FieldTyped);
}

#[test]
fn s2_embedded_field_drops_incompatible_target_method_profile() {
    let cg = build_go(&[
        (
            "a/types_linux.go",
            "package a\n\
                 type Listener struct{}\n\
                 type Holder struct { *Listener }\n\
                 func run(h Holder) { h.Listener.Serve() }\n",
        ),
        (
            "a/method_darwin.go",
            "package a\nfunc (*Listener) Serve() {}\n",
        ),
    ]);
    let site = site_in(&cg, "run", "Serve");
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
    let out = cg.resolve_call_site_full(site);
    assert!(
        out.resolved.is_empty(),
        "darwin-only method leaked into linux target route: {out:?}"
    );
}

#[test]
fn s2_embedded_field_drops_external_test_package_method_candidate() {
    let cg = build_go(&[
        (
            "p/types.go",
            "package p\n\
                 type Listener struct{}\n\
                 type Holder struct { *Listener }\n\
                 func run(h Holder) { h.Listener.Serve() }\n",
        ),
        (
            "p/method_test.go",
            "package p_test\nfunc (*Listener) Serve() {}\n",
        ),
    ]);
    let site = site_in(&cg, "run", "Serve");
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
    let out = cg.resolve_call_site_full(site);
    assert!(
        out.resolved.is_empty(),
        "p_test method leaked into package p target route: {out:?}"
    );
}

#[test]
fn s2_embedded_field_drops_conflicting_compatible_target_methods() {
    let cg = build_go(&[
        (
            "a/types.go",
            "package a\n\
                 type Listener struct{}\n\
                 type Holder struct { Listener }\n\
                 func run(h Holder) { h.Listener.Serve() }\n",
        ),
        ("a/method_one.go", "package a\nfunc (Listener) Serve() {}\n"),
        ("a/method_two.go", "package a\nfunc (Listener) Serve() {}\n"),
    ]);
    let site = site_in(&cg, "run", "Serve");
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
    let out = cg.resolve_call_site_full(site);
    assert!(
        out.resolved.is_empty(),
        "conflicting compatible methods must fail closed: {out:?}"
    );
}

#[test]
fn s2_pointer_embed_struct_in_external_test_package_never_recovers() {
    let cg = build_go(&[
        (
            "p/holder.go",
            "package p\n\
                 type Holder struct { *Listener }\n\
                 func run(h Holder) { h.Listener.Do() }\n",
        ),
        (
            "p/listener_test.go",
            "package p_test\n\
                 type Listener struct{}\n\
                 func (*Listener) Do() {}\n",
        ),
    ]);
    let site = site_in(&cg, "run", "Do");
    assert_eq!(site.receiver_type, None);
    assert_eq!(site.receiver_recovery, None);
    let out = cg.resolve_call_site_full(site);
    assert!(
        out.resolved
            .iter()
            .all(|candidate| candidate.confidence != ResolutionConfidence::Exact),
        "a p_test struct must not prove p.Holder's pointer embed or emit Exact: {out:?}"
    );
}

#[test]
fn s2_pointer_embed_struct_in_incompatible_build_profile_never_recovers() {
    let cg = build_go(&[
        (
            "p/holder_linux.go",
            "package p\n\
                 type Holder struct { *Listener }\n\
                 func run(h Holder) { h.Listener.Do() }\n",
        ),
        (
            "p/listener_darwin.go",
            "package p\n\
                 type Listener struct{}\n\
                 func (*Listener) Do() {}\n",
        ),
    ]);
    let site = site_in(&cg, "run", "Do");
    assert_eq!(site.receiver_type, None);
    assert_eq!(site.receiver_recovery, None);
    let out = cg.resolve_call_site_full(site);
    assert!(
        out.resolved
            .iter()
            .all(|candidate| candidate.confidence != ResolutionConfidence::Exact),
        "a darwin-only struct must not prove a linux holder's pointer embed: {out:?}"
    );
}

#[test]
fn s2_pointer_embed_struct_with_uncertain_embedding_profile_never_recovers() {
    let cg = build_go(&[
        (
            "p/holder.go",
            "//go:build linux &&\n\n\
                 package p\n\
                 type Holder struct { *Listener }\n\
                 func run(h Holder) { h.Listener.Do() }\n",
        ),
        (
            "p/listener.go",
            "package p\n\
                 type Listener struct{}\n\
                 func (*Listener) Do() {}\n",
        ),
    ]);
    let site = site_in(&cg, "run", "Do");
    assert_eq!(site.receiver_type, None);
    assert_eq!(site.receiver_recovery, None);
    assert!(
        cg.resolve_call_site_full(site)
            .resolved
            .iter()
            .all(|candidate| candidate.confidence != ResolutionConfidence::Exact),
        "an unparsed embedding profile cannot prove pointer-target visibility"
    );
}

#[test]
fn s2_pointer_embed_struct_in_same_build_profile_still_recovers() {
    let cg = build_go(&[
        (
            "p/holder_linux.go",
            "package p\n\
                 type Holder struct { *Listener }\n\
                 func run(h Holder) { h.Listener.Do() }\n",
        ),
        (
            "p/listener_linux.go",
            "package p\n\
                 type Listener struct{}\n\
                 func (*Listener) Do() {}\n",
        ),
    ]);
    let site = site_in(&cg, "run", "Do");
    assert_eq!(site.receiver_type.as_deref(), Some("Listener"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
    assert!(cg
        .resolve_call_site_full(site)
        .resolved
        .iter()
        .any(|candidate| {
            candidate.target.name == "Do"
                && candidate.confidence == ResolutionConfidence::Exact
                && candidate.kind == ResolutionKind::FieldTyped
        }));
}

#[test]
fn s2_pointer_embed_unconstrained_cross_file_struct_still_recovers() {
    let cg = build_go(&[
        (
            "p/holder.go",
            "package p\n\
                 type Holder struct { *Listener }\n\
                 func run(h Holder) { h.Listener.Do() }\n",
        ),
        (
            "p/listener.go",
            "package p\n\
                 type Listener struct{}\n\
                 func (*Listener) Do() {}\n",
        ),
    ]);
    let site = site_in(&cg, "run", "Do");
    assert_eq!(site.receiver_type.as_deref(), Some("Listener"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
    assert!(cg
        .resolve_call_site_full(site)
        .resolved
        .iter()
        .any(|candidate| {
            candidate.target.name == "Do"
                && candidate.confidence == ResolutionConfidence::Exact
                && candidate.kind == ResolutionKind::FieldTyped
        }));
}

#[test]
fn s2_pointer_embed_external_test_clause_does_not_conflict_with_owner() {
    let cg = build_go(&[
        (
            "p/holder.go",
            "package p\n\
                 type Holder struct { *Listener }\n\
                 func run(h Holder) { h.Listener.Do() }\n",
        ),
        (
            "p/listener.go",
            "package p\n\
                 type Listener struct{}\n\
                 func (*Listener) Do() {}\n",
        ),
        (
            "p/listener_test.go",
            "package p_test\n\
                 type Listener interface { Do() }\n",
        ),
    ]);
    let site = site_in(&cg, "run", "Do");
    assert_eq!(site.receiver_type.as_deref(), Some("Listener"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
    let outcome = cg.resolve_call_site_full(site);
    assert!(
        outcome.resolved.iter().any(|candidate| {
            candidate.target.file == "p/listener.go"
                && candidate.target.name == "Do"
                && candidate.confidence == ResolutionConfidence::Exact
                && candidate.kind == ResolutionKind::FieldTyped
        }),
        "the p_test clause must not suppress the proven p.Listener target: {outcome:?}"
    );
    assert!(
        outcome
            .resolved
            .iter()
            .all(|candidate| candidate.target.file != "p/listener_test.go"),
        "the external-test clause must never donate the pointer-embed target: {outcome:?}"
    );
}

#[test]
fn s2_pointer_embed_interface_alias_collision_never_recovers() {
    let cg = build_go(&[
        (
            "a/a.go",
            "package a\n\
                 type Alias = interface { Do() }\n\
                 type Holder struct { *Alias }\n\
                 func run(h Holder) { h.Alias.Do() }\n",
        ),
        (
            "z/z.go",
            "package z\n\
                 type Alias interface { Do() }\n\
                 type Impl struct{}\n\
                 func (Impl) Do() {}\n",
        ),
    ]);
    let site = site_in(&cg, "run", "Do");
    assert_eq!(site.receiver_type, None);
    assert_eq!(site.receiver_recovery, None);
    let out = cg.resolve_call_site_full(site);
    assert!(
        out.resolved
            .iter()
            .all(|candidate| candidate.confidence != ResolutionConfidence::Exact),
        "an invalid pointer-to-interface alias must not emit Exact z.Impl.Do: {out:?}"
    );
}

#[test]
fn s2_pointer_embed_struct_alias_fails_closed() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Listener struct{}\n\
             func (*Listener) Serve() {}\n\
             type Alias = Listener\n\
             type Holder struct { *Alias }\n\
             func run(h Holder) { h.Alias.Serve() }\n",
    )]);
    let site = site_in(&cg, "run", "Serve");
    assert_eq!(site.receiver_type, None);
    assert_eq!(site.receiver_recovery, None);
    assert!(
        cg.resolve_call_site_full(site)
            .resolved
            .iter()
            .all(|candidate| candidate.confidence != ResolutionConfidence::Exact),
        "struct aliases intentionally fail closed for S2"
    );
}

#[test]
fn s2_pointer_embed_defined_interface_collision_never_recovers() {
    let cg = build_go(&[
        (
            "a/a.go",
            "package a\n\
                 type I interface { Do() }\n\
                 type I2 I\n\
                 type Holder struct { *I2 }\n\
                 func run(h Holder) { h.I2.Do() }\n",
        ),
        (
            "z/z.go",
            "package z\n\
                 type I2 interface { Do() }\n\
                 type Impl struct{}\n\
                 func (Impl) Do() {}\n",
        ),
    ]);
    let site = site_in(&cg, "run", "Do");
    assert_eq!(site.receiver_type, None);
    assert_eq!(site.receiver_recovery, None);
    let out = cg.resolve_call_site_full(site);
    assert!(
        out.resolved
            .iter()
            .all(|candidate| candidate.confidence != ResolutionConfidence::Exact),
        "a defined interface type must not emit an Exact edge through z.I2: {out:?}"
    );
}

#[test]
fn cross_package_unqualified_embed_never_mints_embedded_promotion() {
    let cg = build_go(&[
        (
            "a/types.go",
            "package a\n\
                 type Base struct{}\n\
                 type S struct { *Base }\n",
        ),
        (
            "a/run.go",
            "package a\n\
                 func run(s S) { s.Serve() }\n",
        ),
        (
            "b/types.go",
            "package b\n\
                 type Base struct{}\n\
                 func (b *Base) Serve() {}\n",
        ),
    ]);
    assert!(
        cg.resolve_call_site_full(site_in(&cg, "run", "Serve"))
            .resolved
            .iter()
            .all(|candidate| candidate.kind != ResolutionKind::EmbeddedPromotion),
        "a.S must not promote b.(*Base).Serve"
    );
}

#[test]
fn s2_unrelated_package_interface_does_not_remove_local_pointer_embed() {
    let cg = build_go(&[
        (
            "a/types.go",
            "package a\n\
                 type Base struct{}\n\
                 func (b *Base) Serve() {}\n\
                 type S struct { *Base }\n",
        ),
        (
            "a/run.go",
            "package a\n\
                 func run(s S) { s.Base.Serve() }\n",
        ),
        ("z/types.go", "package z\ntype Base interface { Serve() }\n"),
    ]);
    let site = site_in(&cg, "run", "Serve");
    assert_eq!(site.receiver_type.as_deref(), Some("Base"));
    assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
    assert!(cg
        .resolve_call_site_full(site)
        .resolved
        .iter()
        .any(|candidate| {
            candidate.target.name == "Serve"
                && candidate.confidence == ResolutionConfidence::Exact
                && candidate.kind == ResolutionKind::FieldTyped
        }));
}

#[test]
fn qualified_pointer_embed_field_shadows_concrete_promotion() {
    let cg = build_go(&[
        ("ext/types.go", "package ext\ntype Listener struct{}\n"),
        (
            "main.go",
            "package main\n\
                 import \"example.com/ext\"\n\
                 type D struct{}\n\
                 func (d D) Listener() {}\n\
                 type S struct { *ext.Listener; D }\n\
                 func run(s S) { s.Listener() }\n",
        ),
    ]);
    let out = cg.resolve_call_site_full(site_in(&cg, "run", "Listener"));
    assert!(
        out.resolved.is_empty(),
        "the depth-0 ext.Listener field must shadow promoted D.Listener: {out:?}"
    );
    assert!(!cg
        .promoted_aliases
        .contains_key(&("S".to_string(), "Listener".to_string())));
}

#[test]
fn duplicate_outer_struct_name_never_cross_routes_promoted_alias() {
    let cg = build_go(&[
        (
            "a/types.go",
            "package a\n\
                 type I interface { Serve(x int) }\n\
                 type Concrete struct{}\n\
                 func (c Concrete) Serve(x int) {}\n\
                 type S struct { I }\n",
        ),
        ("a/run.go", "package a\nfunc run(s S) { s.Serve(1) }\n"),
        (
            "z/types.go",
            "package z\n\
                 type Base struct{}\n\
                 func (b *Base) Serve(x string) {}\n\
                 type S struct { *Base }\n",
        ),
    ]);
    let out = cg.resolve_call_site_full(site_in(&cg, "run", "Serve"));
    assert!(
        out.resolved.is_empty()
            || out.resolved.iter().all(|candidate| {
                candidate.target.file == "a/types.go"
                    && candidate.target.name == "Serve"
                    && candidate.kind == ResolutionKind::InterfaceDispatch
            }),
        "a.S may route through its own I or fail closed, but must never reach z.Base.Serve: \
             {out:?}"
    );
}

#[test]
fn s4_pointer_embedded_interface_never_routes_or_satisfies() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type I interface { Do() }\n\
             type Concrete struct{}\n\
             func (c Concrete) Do() {}\n\
             type Holder struct { *I }\n\
             func direct(h Holder) { h.Do() }\n\
             func nested(h Holder) { h.I.Do() }\n",
    )]);
    assert!(
        cg.go_embedded_interface_methods
            .get(&main_owner("Holder"))
            .and_then(|methods| methods.get("Do"))
            .is_none(),
        "invalid *I embed must not mint an S4 InterfaceDispatch route"
    );
    assert!(
        cg.resolve_call_site_full(site_in(&cg, "direct", "Do"))
            .resolved
            .is_empty(),
        "invalid *I embed must not satisfy I or resolve h.Do()"
    );
    let nested = site_in(&cg, "nested", "Do");
    assert_eq!(nested.receiver_type, None);
    assert_eq!(nested.receiver_recovery, None);
    let nested_outcome = cg.resolve_call_site_full(nested);
    assert!(
        nested_outcome
            .resolved
            .iter()
            .all(|candidate| candidate.kind != ResolutionKind::InterfaceDispatch),
        "invalid *I embed must not recover h.I as I and dispatch it: \
             site={nested:?}, outcome={nested_outcome:?}"
    );
}

#[test]
fn s2_qualified_pointer_embedded_interface_never_recovers() {
    let cg = build_go(&[
        (
            "ext/types.go",
            "package ext\n\
                 type I interface { Do() }\n\
                 type Concrete struct{}\n\
                 func (c Concrete) Do() {}\n",
        ),
        (
            "main.go",
            "package main\n\
                 import \"github.com/x/y/ext\"\n\
                 type Holder struct { *ext.I }\n\
                 func nested(h Holder) { h.I.Do() }\n",
        ),
    ]);
    let nested = site_in(&cg, "nested", "Do");
    assert_eq!(nested.receiver_type, None);
    assert_eq!(nested.receiver_recovery, None);
    assert!(
        cg.resolve_call_site_full(nested)
            .resolved
            .iter()
            .all(|candidate| candidate.kind != ResolutionKind::InterfaceDispatch),
        "invalid *ext.I embed must not recover h.I for interface dispatch"
    );
}

#[test]
fn qualified_embedded_struct_does_not_promote_unrelated_local_bare_target() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Listener struct{}\n\
             func (l Listener) Serve() {}\n\
             type S struct { ext.Listener }\n\
             func run(s S) { s.Serve() }\n",
    )]);
    assert!(
        cg.resolve_call_site_full(site_in(&cg, "run", "Serve"))
            .resolved
            .is_empty(),
        "qualified ext.Listener must not promote an unrelated local Listener.Serve"
    );
}

#[test]
fn qualified_embedded_interface_does_not_route_local_bare_interface() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type I interface { Do() }\n\
             type Concrete struct{}\n\
             func (c Concrete) Do() {}\n\
             type Holder struct { ext.I }\n\
             func run(h Holder) { h.Do() }\n",
    )]);
    assert!(
        cg.go_embedded_interface_methods
            .get(&main_owner("Holder"))
            .and_then(|methods| methods.get("Do"))
            .is_none(),
        "qualified ext.I must not mint an S4 route through unrelated local I"
    );
    assert!(
        cg.resolve_call_site_full(site_in(&cg, "run", "Do"))
            .resolved
            .is_empty(),
        "qualified ext.I must not dispatch h.Do() to local I implementers"
    );
}

#[test]
fn s4_own_method_shadows_embedded_interface_promotion() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Doer interface { Do() }\n\
             type Concrete struct{}\n\
             func (c Concrete) Do() {}\n\
             type Holder struct {\n\tDoer\n}\n\
             func (h Holder) Do() {}\n\
             func run(h Holder) {\n\th.Do()\n}\n",
    )]);
    // Holder has its OWN direct `Do` -- must NOT appear in the
    // embedded-interface promotion map (own method wins, ordinary
    // owner_lookup already resolves it).
    assert!(cg
        .go_embedded_interface_route("Holder", None, "Do", "main.go")
        .value
        .is_none());
    let site = site_in(&cg, "run", "Do");
    let out = cg.resolve_call_site_full(site);
    assert!(out.resolved.iter().any(|c| c.target.file == "main.go"
        && c.confidence == ResolutionConfidence::Exact
        && c.kind != ResolutionKind::InterfaceDispatch));
}

#[test]
fn s4_external_embedded_interface_drops_without_wrong_edge() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             import \"net\"\n\
             type Wrap struct {\n\tnet.Listener\n}\n\
             func run(w Wrap) {\n\tw.Accept()\n}\n",
    )]);
    // `net.Listener` is external (not in `data.interfaces`) -- no in-repo
    // satisfier set. Must drop cleanly: no panic, no wrong edge, no
    // Exact flip.
    let site = site_in(&cg, "run", "Accept");
    let out = cg.resolve_call_site_full(site);
    assert!(out.resolved.is_empty());
    assert!(cg
        .go_embedded_interface_route("Wrap", None, "Accept", "main.go")
        .value
        .is_none());
}

// ---- B2/M1 (codex impl-review): S4 package scoping + gate-failure drop

#[test]
fn b2_embedded_interface_route_is_package_scoped_not_bare_name() {
    // Two packages each define `Holder`; only the SECOND-inserted one
    // (by path order -- `BTreeMap<String, ParsedFile>` iterates
    // lexicographically, and `zzz_embed/...` sorts after
    // `aaa_plain/...`) embeds `Doer`. Pre-fix, the bare-keyed
    // `data.structs["Holder"]` extraction collapses to whichever file's
    // struct is processed LAST, so `zzz_embed`'s embedding data
    // overwrites `aaa_plain`'s (non-embedding) entry entirely --
    // `aaa_plain`'s `Holder.Do()` call site then incorrectly inherits
    // `zzz_embed`'s `Doer` donation and routes to `Concrete.Do`.
    let cg = build_go(&[
        (
            "zzz_embed/holder.go",
            "package zzzembed\n\
                 type Doer interface { Do() }\n\
                 type Concrete struct{}\n\
                 func (c Concrete) Do() {}\n\
                 type Holder struct {\n\tDoer\n}\n",
        ),
        (
            "aaa_plain/holder.go",
            "package aaaplain\n\
                 type Holder struct{}\n\
                 func run(h Holder) {\n\th.Do()\n}\n",
        ),
    ]);
    let site = site_in(&cg, "run", "Do");
    let out = cg.resolve_call_site_full(site);
    assert!(
        out.resolved.is_empty(),
        "aaa_plain's non-embedding Holder must not route via zzz_embed's \
             Doer donation, got {:?}",
        out
    );
}

#[test]
fn m1_s4_gate_failure_drops_not_falls_through_to_alternate_route() {
    let cg = build_go(&[
        (
            "holder/holder.go",
            "package holder\n\
                 type Doer interface { Do(x int) }\n\
                 type Concrete struct{}\n\
                 func (c Concrete) Do(x int) {}\n\
                 type Holder struct {\n\tDoer\n}\n\
                 func run(h Holder) {\n\th.Do()\n}\n",
        ),
        (
            "other/other.go",
            "package other\n\
                 type Holder interface { Do() }\n\
                 type OtherImpl struct{}\n\
                 func (o OtherImpl) Do() {}\n",
        ),
    ]);
    let site = site_in(&cg, "run", "Do");
    let out = cg.resolve_call_site_full(site);
    // S4 routes `h.Do()` to the embedded `Doer.Do(x int)`, but the call
    // has 0 args -- arity-rejected. Without the M1 fix this falls
    // through to the bare `iface_key("Holder")` ladder and picks up the
    // UNRELATED `other.Holder` interface's `OtherImpl.Do` (a bare-name
    // collision with the STRUCT `holder.Holder`) -- must drop instead.
    assert!(
        out.resolved.is_empty(),
        "arity-rejected S4 route must drop, not mint an alternate-route edge: {:?}",
        out
    );
}

#[test]
fn s4_ambiguous_two_embedded_interfaces_supplying_same_method_drops() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type A interface { M() }\n\
             type B interface { M() }\n\
             type ImplA struct{}\n\
             func (c ImplA) M() {}\n\
             type Holder struct {\n\tA\n\tB\n}\n\
             func run(h Holder) {\n\th.M()\n}\n",
    )]);
    let route = cg.go_embedded_interface_route("Holder", None, "M", "main.go");
    assert!(route.value.is_none());
    assert!(route.evidence.conflict);
}

// ---- S5: telemetry ----------------------------------------------------

#[test]
fn s5_resolution_kind_as_str_field_and_return_typed() {
    assert_eq!(ResolutionKind::FieldTyped.as_str(), "field_typed");
    assert_eq!(ResolutionKind::ReturnTyped.as_str(), "return_typed");
}

#[test]
fn s5_call_stats_reports_dropped_go_receiver_bucket() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
             type Outer struct {\n\tField int\n}\n\
             func run(o *Outer) {\n\to.Field.M()\n}\n",
    )]);
    let stats = crate::navigation::queries::call_stats(&cg);
    let dropped = stats
        .get("dropped_go_receiver")
        .and_then(|v| v.as_object())
        .expect("dropped_go_receiver present");
    let total: u64 = dropped.values().filter_map(|v| v.as_u64()).sum();
    assert!(
        total > 0,
        "expected at least one attributed Go drop: {stats}"
    );
}

// ---- incremental parity (bidirectional) -------------------------------

#[test]
fn incremental_parity_edit_type_defining_file_updates_consumer() {
    use crate::cpg::CodePropertyGraph;
    use crate::data_flow::DataFlowGraph;
    use std::collections::BTreeSet;

    let mut files: BTreeMap<String, ParsedFile> = BTreeMap::new();
    files.insert(
        "demux.go".to_string(),
        ParsedFile::parse(
            "demux.go",
            "package demux\n\
                 type Demux struct{}\n\
                 func (d *Demux) Init() {}\n\
                 func newDemux() *Demux { return &Demux{} }\n",
            Go,
        )
        .unwrap(),
    );
    files.insert(
        "consumer.go".to_string(),
        ParsedFile::parse(
            "consumer.go",
            "package demux\n\
                 func run() {\n\td := newDemux()\n\td.Init()\n}\n",
            Go,
        )
        .unwrap(),
    );
    let cg0 = CallGraph::build(&files);
    let site0 = site_in(&cg0, "run", "Init");
    assert_eq!(site0.receiver_type.as_deref(), Some("Demux"));

    // Edit ONLY the type-defining file: `newDemux` now returns a
    // DIFFERENT type. `consumer.go` is untouched.
    files.insert(
        "demux.go".to_string(),
        ParsedFile::parse(
            "demux.go",
            "package demux\n\
                 type Demux struct{}\n\
                 func (d *Demux) Init() {}\n\
                 type Other struct{}\n\
                 func newDemux() *Other { return &Other{} }\n",
            Go,
        )
        .unwrap(),
    );
    let changed: BTreeSet<String> = ["demux.go".to_string()].into_iter().collect();
    let cpg = CodePropertyGraph::build_incremental(
        cg0,
        DataFlowGraph::build(&BTreeMap::new()),
        &changed,
        &files,
        None,
    );
    let cg1 = &cpg.call_graph;
    let site1 = site_in(cg1, "run", "Init");
    // `consumer.go` never changed, but its recovery MUST update: `Demux`
    // no longer has an `Init` method reachable from `d`'s new type.
    assert_ne!(
        site1.receiver_type.as_deref(),
        Some("Demux"),
        "consumer.go's retained call site kept a stale recovery after \
             the type-defining file changed"
    );
}

#[test]
fn incremental_parity_edit_pointer_embed_defining_file_updates_retained_consumer() {
    use crate::cpg::CodePropertyGraph;
    use crate::data_flow::DataFlowGraph;
    use std::collections::BTreeSet;

    let mut files = BTreeMap::new();
    files.insert(
        "listener.go".to_string(),
        ParsedFile::parse(
            "listener.go",
            "package p\ntype Listener struct{}\nfunc (l *Listener) Serve() {}\n",
            Go,
        )
        .unwrap(),
    );
    files.insert(
        "embed.go".to_string(),
        ParsedFile::parse("embed.go", "package p\ntype S struct{}\n", Go).unwrap(),
    );
    files.insert(
        "consumer.go".to_string(),
        ParsedFile::parse(
            "consumer.go",
            "package p\nfunc run(s S) { s.Listener.Serve() }\n",
            Go,
        )
        .unwrap(),
    );
    files.insert(
        "b/listener.go".to_string(),
        ParsedFile::parse(
            "b/listener.go",
            "package b\ntype Listener struct{}\nfunc (*Listener) Serve() {}\n",
            Go,
        )
        .unwrap(),
    );
    let cg0 = CallGraph::build(&files);
    let site0 = site_in(&cg0, "run", "Serve");
    assert_eq!(site0.receiver_type, None);
    assert_eq!(site0.receiver_recovery, None);
    assert_eq!(site0.receiver_outcome, None);

    files.insert(
        "embed.go".to_string(),
        ParsedFile::parse("embed.go", "package p\ntype S struct { *Listener }\n", Go).unwrap(),
    );
    let full = CallGraph::build(&files);
    let full_out = full.resolve_call_site_full(site_in(&full, "run", "Serve"));
    assert_eq!(
        full_out.resolved.len(),
        1,
        "full-build decoy leaked: {full_out:?}"
    );
    assert_eq!(full_out.resolved[0].target.file, "listener.go");
    assert_eq!(full_out.resolved[0].confidence, ResolutionConfidence::Exact);

    let changed = BTreeSet::from(["embed.go".to_string()]);
    let incremental = CodePropertyGraph::build_incremental(
        cg0,
        DataFlowGraph::build(&BTreeMap::new()),
        &changed,
        &files,
        None,
    );
    let incremental_out = incremental.call_graph.resolve_call_site_full(site_in(
        &incremental.call_graph,
        "run",
        "Serve",
    ));
    assert_eq!(
        incremental_out.resolved.len(),
        1,
        "incremental decoy leaked: {incremental_out:?}"
    );
    assert_eq!(incremental_out.resolved[0].target.file, "listener.go");
    assert_eq!(
        incremental_out.resolved[0].confidence,
        ResolutionConfidence::Exact
    );
}

#[test]
fn incremental_parity_edit_consuming_file_recomputes() {
    use crate::cpg::CodePropertyGraph;
    use crate::data_flow::DataFlowGraph;
    use std::collections::BTreeSet;

    let mut files: BTreeMap<String, ParsedFile> = BTreeMap::new();
    files.insert(
        "demux.go".to_string(),
        ParsedFile::parse(
            "demux.go",
            "package demux\n\
                 type Demux struct{}\n\
                 func (d *Demux) Init() {}\n\
                 func newDemux() *Demux { return &Demux{} }\n",
            Go,
        )
        .unwrap(),
    );
    files.insert(
        "consumer.go".to_string(),
        ParsedFile::parse("consumer.go", "package demux\nfunc run() {}\n", Go).unwrap(),
    );
    let cg0 = CallGraph::build(&files);
    assert!(cg0
        .calls
        .iter()
        .all(|(fid, sites)| fid.name != "run" || sites.is_empty()));

    // Edit ONLY the consuming file: add the call-RHS receiver usage.
    files.insert(
        "consumer.go".to_string(),
        ParsedFile::parse(
            "consumer.go",
            "package demux\n\
                 func run() {\n\td := newDemux()\n\td.Init()\n}\n",
            Go,
        )
        .unwrap(),
    );
    let changed: BTreeSet<String> = ["consumer.go".to_string()].into_iter().collect();
    let cpg = CodePropertyGraph::build_incremental(
        cg0,
        DataFlowGraph::build(&BTreeMap::new()),
        &changed,
        &files,
        None,
    );
    let cg1 = &cpg.call_graph;
    let site1 = site_in(cg1, "run", "Init");
    assert_eq!(site1.receiver_type.as_deref(), Some("Demux"));
    assert_eq!(site1.receiver_recovery, Some(ReceiverRecovery::ReturnTyped));
}

#[test]
fn incremental_parity_edit_unrelated_file_survives() {
    use crate::cpg::CodePropertyGraph;
    use crate::data_flow::DataFlowGraph;
    use std::collections::BTreeSet;

    let mut files: BTreeMap<String, ParsedFile> = BTreeMap::new();
    files.insert(
        "demux.go".to_string(),
        ParsedFile::parse(
            "demux.go",
            "package demux\n\
                 type Demux struct{}\n\
                 func (d *Demux) Init() {}\n\
                 func newDemux() *Demux { return &Demux{} }\n",
            Go,
        )
        .unwrap(),
    );
    files.insert(
        "consumer.go".to_string(),
        ParsedFile::parse(
            "consumer.go",
            "package demux\n\
                 func run() {\n\td := newDemux()\n\td.Init()\n}\n",
            Go,
        )
        .unwrap(),
    );
    files.insert(
        "unrelated.go".to_string(),
        ParsedFile::parse("unrelated.go", "package other\nfunc noop() {}\n", Go).unwrap(),
    );
    let cg0 = CallGraph::build(&files);
    let site0 = site_in(&cg0, "run", "Init");
    assert_eq!(site0.receiver_type.as_deref(), Some("Demux"));

    files.insert(
        "unrelated.go".to_string(),
        ParsedFile::parse(
            "unrelated.go",
            "package other\nfunc noop() {}\nfunc noop2() {}\n",
            Go,
        )
        .unwrap(),
    );
    let changed: BTreeSet<String> = ["unrelated.go".to_string()].into_iter().collect();
    let cpg = CodePropertyGraph::build_incremental(
        cg0,
        DataFlowGraph::build(&BTreeMap::new()),
        &changed,
        &files,
        None,
    );
    let cg1 = &cpg.call_graph;
    let site1 = site_in(cg1, "run", "Init");
    assert_eq!(site1.receiver_type.as_deref(), Some("Demux"));
    assert_eq!(site1.receiver_recovery, Some(ReceiverRecovery::ReturnTyped));
}
