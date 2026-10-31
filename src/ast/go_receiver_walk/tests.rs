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
fn range_assignment_preserves_binding() {
    check("package p\nfunc run() { x := makeT(); for x = range xs { x.M() }; x.M(); for x := range xs { x.M() }; x.M() }", true);
}

fn duplicate_literal_source(count: usize, outer: bool) -> String {
    let params = ["x *T", "x *U", "x *V"][..count].join(", ");
    let binding = if outer { "x := &T{}; " } else { "" };
    let extra = if count == 3 {
        "type V struct{}\nfunc (v *V) M() {}\n"
    } else {
        ""
    };
    format!("package p\ntype T struct{{}}\ntype U struct{{}}\nfunc (t *T) M() {{}}\nfunc (u *U) M() {{}}\n{extra}func run() {{ {binding}_ = func({params}) {{ x.M() }} }}")
}

#[test]
fn duplicate_literal_parameter_transactions() {
    for count in [2, 3] {
        for outer in [false, true] {
            check(&duplicate_literal_source(count, outer), true);
        }
    }
    // Grouped duplicates count once per declaration; separate declarations
    // count individually, including when nested lists enter and leave scope.
    check("package p\nfunc run() { _ = func(x, x *T, x *U) { _ = func(x *V, x *T) { x.M() }; x.M() }; x.M() }", true);
}

#[test]
fn grouped_parameter_payload_is_shared_before_collection() {
    let ty = format!("interface {{ M(); /*{}*/ }}", " ".repeat(256));
    for count in [1, 4] {
        let names = (0..count)
            .map(|i| format!("x{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let p = parsed(&format!("package p\nfunc run({names} {ty}) {{}}"));
        for plain in [false, true] {
            let parameters = collect::parameters(&p, root(&p), plain);
            assert_eq!(parameters.len(), count);
            let pointers: BTreeSet<_> = parameters
                .iter()
                .map(|(_, (text, _))| text.as_ptr())
                .collect();
            assert_eq!(
                pointers.len(),
                1,
                "grouped names must share the temporary parameter payload"
            );
            assert!(parameters.iter().all(|(_, (text, _))| {
                let text: &str = text;
                text == ty.as_str()
            }));
        }
    }
}

#[test]
fn retained_payload_bytes_follow_distinct_types() {
    let ty = format!("interface {{ M(); /*{}*/ }}", " ".repeat(4096));
    let mut retained = Vec::new();
    for assignments in [8, 128] {
        let source = format!("package p\nfunc run(x {ty}, t *T) {{ {} {{ var y, z {ty}; y.M(); z.M() }} _ = func(y {ty}, z {ty}) {{ y.M(); z.M() }}; {{ q := &T{{}}; q.M() }}; x.M() }}", "x = nil; ".repeat(assignments));
        let p = parsed(&source);
        let function = root(&p);
        let _session = Session::new(&p, function);
        // Include scope activation/deactivation and every lazy mode.
        check(&source, false);
        let byte = source.rfind("x.M()").unwrap();
        let line = p.line_offsets.partition_point(|start| *start <= byte);
        for var in [false, true] {
            for reuse in [false, true] {
                assert_eq!(
                    evidence(&p, function, "x", line, byte, var, reuse),
                    Some((None, assignments + 1, None))
                );
            }
        }
        let bytes = with_facts(&p, function, |facts| {
            let mut payloads = BTreeMap::new();
            for row in facts.names.values() {
                row.seed.record_payloads(&mut payloads);
                for event in &row.events {
                    event.value.record_payloads(&mut payloads);
                }
                for timeline in row.timelines.iter().filter_map(OnceCell::get) {
                    timeline.record_payloads(&mut payloads);
                }
            }
            assert!(payloads.contains_key(&ty));
            assert!(payloads.len() >= 2, "exercise distinct types");
            let allocated: usize = payloads
                .iter()
                .map(|(text, pointers)| text.len() * pointers.len())
                .sum();
            let distinct: usize = payloads.keys().map(String::len).sum();
            assert_eq!(
                allocated, distinct,
                "retained recovery payload allocations must be shared: {assignments} assignments"
            );
            println!(
                "assignments={assignments} distinct_types={} retained_payload_bytes={allocated}",
                payloads.len()
            );
            allocated
        })
        .unwrap();
        retained.push(bytes);
    }
    assert_eq!(
        retained[0], retained[1],
        "assignment count must not multiply retained payload bytes"
    );
}

#[test]
fn duplicate_literal_graph_equivalence() {
    const CHILD: &str = "PRISM_TEST_GO_DUPLICATE_SNAPSHOT";
    if let Some(path) = std::env::var_os(CHILD) {
        let graphs: Vec<_> = [2, 3]
            .into_iter()
            .flat_map(|count| {
                [false, true].into_iter().map(move |outer| {
                    let files = BTreeMap::from([(
                        "p.go".to_owned(),
                        parsed(&duplicate_literal_source(count, outer)),
                    )]);
                    let graph = CallGraph::build(&files);
                    let sites: Vec<_> = graph.calls.iter().collect();
                    (
                        serde_json::to_value(&sites).unwrap(),
                        bincode::serialize(&graph).unwrap(),
                        crate::navigation::queries::call_stats(&graph),
                    )
                })
            })
            .collect();
        std::fs::write(path, serde_json::to_vec(&graphs).unwrap()).unwrap();
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
                    "ast::go_receiver_walk::tests::duplicate_literal_graph_equivalence",
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
            snapshots.push(std::fs::read(path).unwrap());
        }
    }
    for snapshot in &snapshots[1..] {
        assert!(snapshot == &snapshots[0], "duplicate literal graph differs");
    }
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
