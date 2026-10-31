use super::*;
use crate::call_graph::CallGraph;
use std::collections::BTreeSet;

const SOURCE: &str = r#"package p
type T struct{}
func (t *T) M() {}
func makeT() *T { return &T{} }
func reset(x *T) (*T, error) { return x, nil }
func run(x *T, a, b *T) (result *T) {
 x.M()
 x, err := reset(x); x.M()
 x, err := reset(x); x.M()
 x, fresh := 1, 2; x.M()
 err, newer := reset(x); err.M()
 x, other := reset(x), reset(x); x.M()
 x, _ := reset(x); x.M()
 { x := makeT(); x.M(); x, e := reset(x); x.M(); _ = e }
 x.M()
 if x, e := reset(x); e == nil { x.M() } else { x.M() }
 for x, e := reset(x); e != nil; { x.M() }
 for x := range xs { x.M() }
 switch x := makeT(); x { case nil: x.M(); default: x.M() }
 switch x := thing.(type) { case *T: x.M(); default: x.M() }
 select { case x := <-ch: x.M(); default: x.M() }
 f := func(x *T) { x.M(); x, z := reset(x); x.M(); _ = z }
 g := func() { y := makeT(); y.M(); x.M() }
 x.M(); y.M()
 var y *T; y.M()
 var c, d = makeT(), makeT(); c.M(); d.M()
 var z = makeT(); z.M()
 x = nil; x.M()
 result.M()
 _ = func() { x = nil }; x.M()
 _ = a; _ = b; _ = f; _ = g; _ = err; _ = fresh; _ = other; _ = newer
}
"#;

fn parsed(source: &str) -> ParsedFile {
    ParsedFile::parse("p.go", source, Language::Go).unwrap()
}
fn root(p: &ParsedFile) -> Node<'_> {
    p.all_functions()
        .into_iter()
        .find(|n| {
            p.language
                .function_name(n)
                .is_some_and(|name| p.node_text(&name) == "run")
        })
        .unwrap()
}
fn reference<R>(op: impl FnOnce() -> R) -> R {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            BYPASS.with(|flag| flag.set(self.0));
        }
    }
    let _reset = Reset(BYPASS.with(|flag| flag.replace(true)));
    op()
}
fn check(source: &str, every_byte: bool) {
    let p = parsed(source);
    let function = root(&p);
    let _session = Session::new(&p, function);
    let mut bytes = BTreeSet::new();
    fn boundaries(n: Node<'_>, out: &mut BTreeSet<usize>) {
        for byte in [
            n.start_byte(),
            n.start_byte() + 1,
            n.end_byte().saturating_sub(1),
            n.end_byte(),
        ] {
            out.insert(byte);
        }
        let mut c = n.walk();
        for child in n.children(&mut c) {
            boundaries(child, out);
        }
    }
    boundaries(function, &mut bytes);
    if every_byte {
        bytes.extend(function.byte_range());
    }
    let names: BTreeSet<_> = source
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|v| !v.is_empty())
        .chain(["absent", "", "x.y"])
        .collect();
    for byte in bytes {
        let line = p.line_offsets.partition_point(|start| *start <= byte);
        for name in &names {
            for var in [false, true] {
                for reuse in [false, true] {
                    let expected = reference(|| {
                        p.receiver_type_evidence_in_fn_mode(&function, name, line, byte, var, reuse)
                    });
                    assert_eq!(
                        evidence(&p, function, name, line, byte, var, reuse),
                        Some(expected),
                        "evidence {name} @{byte} var={var} reuse={reuse}: {source}"
                    );
                }
            }
            let expected =
                reference(|| p.go_same_scope_short_var_reuse_calls(&function, name, byte));
            assert_eq!(
                reuses(&p, function, name, byte),
                Some(expected),
                "reuse {name} @{byte}: {source}"
            );
            let expected = reference(|| p.go_short_var_call_rhs(&function, name, line, byte));
            assert_eq!(
                rhs(&p, function, name, line, byte),
                Some(expected),
                "rhs {name} @{byte}: {source}"
            );
        }
    }
}

#[test]
fn scope_cutoffs_and_fail_closed_equivalence() {
    check(SOURCE, false);
}

#[test]
fn malformed_and_nested_declarations_equivalence() {
    for body in [
        "x := makeT(); func inner(x *T) { x.M() }; x.M()",
        "x := makeT(); x, err := ; x.M(); x, q := (factory())(); x.M()",
        "x := makeT(); x, err := func() *T { x := makeT(); x.M(); return x }(); x.M()",
        "x := makeT(); x, err := reset(x.M()); x.M()",
        "x := makeT(); x, err := []T{}; x.M(); var x, y T; x.M()",
        "x := makeT(); _ = func(x, x *T) { x.M() }; x.M()",
        "x := makeT(); { x, y := reset(x); x.M() }; x.M()",
    ] {
        check(
            &format!("package p\nfunc run(x, x *T) (result *T) {{ {body} }}"),
            true,
        );
    }
    // No preceding qualifying x declaration may mask the RHS subtree fence.
    check(
        "package p\nfunc run() { x := func() { x := makeT(); x.M() }; x.M() }",
        true,
    );

    // Go recovery syntax need not produce a named declaration inside a body.
    // Exercise the helper's explicit named-function fence with a container root.
    let p = parsed(
        "package p\nfunc run() { x := makeT(); x.M() }\nfunc other() { x, e := reset(x); x.M() }",
    );
    let function = p.tree.root_node();
    let _session = Session::new(&p, function);
    for byte in 0..=p.source.len() {
        let line = p.line_offsets.partition_point(|start| *start <= byte);
        assert_eq!(
            evidence(&p, function, "x", line, byte, true, false),
            Some(reference(|| p.receiver_type_evidence_in_fn_mode(
                &function, "x", line, byte, true, false
            )))
        );
        assert_eq!(
            rhs(&p, function, "x", line, byte),
            Some(reference(
                || p.go_short_var_call_rhs(&function, "x", line, byte)
            ))
        );
        assert_eq!(
            reuses(&p, function, "x", byte),
            Some(reference(
                || p.go_same_scope_short_var_reuse_calls(&function, "x", byte)
            ))
        );
    }
}

#[test]
fn semantic_boundaries() {
    check("package p\nfunc run(x *T) { x, e := reset(x.M()); x.M(); x, e := reset(x); x.M(); { x := makeT(); x.M() }; x.M(); _ = func(y *T) { y.M(); x, z := reset(x); x.M() }; y.M(); x, k := 1, 2; x.M(); k, x, q := reset(x); x.M() }", true);
    check("package p\nfunc (r *T) run(x *T) (result *T) { result, e := reset(x); result.M(); var y *T; y.M(); x = nil; x.M() }", true);
}

#[test]
fn auxiliary_walks_keep_their_distinct_fences() {
    let source = "package p\nfunc run() { type T struct{}; x.M(); _ = func(x *T) { type U = T; x.M() }; x.M(); func inner() { type V = T; _ = func(y *V) { y.M() } } }";
    let p = parsed(source);
    let function = root(&p);
    let _session = Session::new(&p, function);
    for byte in function.start_byte()..=function.end_byte() {
        for name in ["T", "U", "V", "x", "y", "absent"] {
            assert_eq!(
                auxiliary(&p, function, name, byte, false),
                Some(reference(|| p.go_local_type_shadows(&function, name, byte))),
                "type {name} @{byte}"
            );
            assert_eq!(
                auxiliary(&p, function, name, byte, true),
                Some(reference(|| p.go_func_literal_parameter_binds_receiver(
                    &function, name, byte
                ))),
                "literal {name} @{byte}"
            );
        }
    }
}

#[test]
fn one_scan_and_phase_custody() {
    let p = parsed(SOURCE);
    let function = root(&p);
    let byte = p.source.find("x.M()").unwrap();
    let line = p.line_offsets.partition_point(|start| *start <= byte);
    SCANS.with(|n| n.set(0));
    assert!(evidence(&p, function, "x", line, byte, true, false).is_none());
    {
        let _session = Session::new(&p, function);
        for _ in 0..200 {
            p.receiver_type_evidence_in_fn(&function, "x", line, byte, true);
            p.go_same_scope_short_var_reuse_calls(&function, "x", byte)
                .unwrap();
            p.go_short_var_call_rhs(&function, "x", line, byte);
        }
        assert_eq!(
            SCANS.with(std::cell::Cell::get),
            1,
            "production helpers must use one precomputation"
        );
        let other = parsed("package p\nfunc run() { x.M() }");
        assert!(evidence(&other, root(&other), "x", 2, 25, true, false).is_none());
        assert!(evidence(&p, function, "x", line + 1, byte, true, false).is_none());
    }
    assert!(ACTIVE.with(|v| v.borrow().is_empty()));
    assert!(reuses(&p, function, "x", byte).is_none());
    let _next = Session::new(&p, function);
    p.go_same_scope_short_var_reuse_calls(&function, "x", byte)
        .unwrap();
    assert_eq!(SCANS.with(std::cell::Cell::get), 2);
}

#[test]
fn graph_edges_and_call_stats_equivalence() {
    const CHILD: &str = "PRISM_TEST_GO_WALK_SNAPSHOT";
    if let Some(path) = std::env::var_os(CHILD) {
        let files = BTreeMap::from([
            ("p.go".to_owned(), parsed(SOURCE)),
            (
                "other.go".to_owned(),
                parsed("package p\nfunc other(x *T) { x.M(); x, e := reset(x); x.M(); _ = e }"),
            ),
        ]);
        let graph = CallGraph::build(&files);
        let scans = TOTAL_SCANS.load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(
            scans > 0,
            std::env::var_os("PRISM_TEST_GO_WALK_REFERENCE").is_none(),
            "graph build must exercise the selected path"
        );
        let mut bytes = bincode::serialize(&graph).unwrap();
        bytes.extend(serde_json::to_vec(&crate::navigation::queries::call_stats(&graph)).unwrap());
        std::fs::write(path, bytes).unwrap();
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mut snapshots = Vec::new();
    for threads in ["1", "4"] {
        for old in [false, true] {
            let path = dir.path().join(format!("{threads}-{old}"));
            let mut command = std::process::Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "--exact",
                    "ast::go_receiver_walk::tests::graph_edges_and_call_stats_equivalence",
                    "--nocapture",
                ])
                .env("RAYON_NUM_THREADS", threads)
                .env(CHILD, &path)
                .env_remove("PRISM_TEST_GO_WALK_REFERENCE");
            if old {
                command.env("PRISM_TEST_GO_WALK_REFERENCE", "1");
            }
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let bytes = std::fs::read(path).unwrap();
            assert!(!bytes.is_empty());
            snapshots.push(bytes);
        }
    }
    for snapshot in &snapshots[1..] {
        assert_eq!(snapshot, &snapshots[0]);
    }
}
