use super::*;
use std::cell::Cell;

fn leaf(name: &str) -> JsExportFacts {
    let target = ResolvedJsExport {
        file: format!("{name}.ts"),
        local_name: "real".into(),
        span: Some((1, 1)),
        wrapped: false,
        via_unresolved_star: false,
    };
    JsExportFacts {
        namespace_proof_complete: true,
        namespace_callable_locals: BTreeMap::from([("real".into(), vec![target])]),
        forwardable_function_locals: BTreeSet::from(["real".into()]),
        named: BTreeMap::from([(
            "real".into(),
            JsExportTarget::VerifiedLocal {
                local: "real".into(),
                start_line: 1,
                end_line: 1,
            },
        )]),
        ..Default::default()
    }
}
fn stars(specs: &[&str]) -> JsExportFacts {
    JsExportFacts {
        namespace_proof_complete: true,
        star_reexports: specs.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    }
}
fn module(_: &str, spec: &str) -> Option<String> {
    (!spec.contains("missing")).then(|| format!("{}.ts", spec.trim_start_matches("./")))
}
fn compare(raw: &BTreeMap<String, JsExportFacts>) {
    for projection in [ExportProjection::Legacy, ExportProjection::CallerPaths] {
        assert_eq!(
            resolve(raw, &module, raw.keys(), projection),
            oracle::with_uncached(|| resolve(raw, &module, raw.keys(), projection))
        );
        // Repeated roots preserve counter multiplicity, and empty roots stay empty.
        let roots = raw.keys().chain(raw.keys());
        assert_eq!(
            resolve(raw, &module, roots, projection),
            oracle::with_uncached(|| resolve(
                raw,
                &module,
                raw.keys().chain(raw.keys()),
                projection
            ))
        );
        assert_eq!(
            resolve(raw, &module, std::iter::empty(), projection),
            JsExportResolution::default()
        );
    }
    assert_eq!(
        namespace(raw, &module),
        oracle::with_uncached(|| namespace(raw, &module))
    );
}

#[test]
fn repeated_roots_preserve_maps_and_double_telemetry() {
    let raw = BTreeMap::from([
        ("a.ts".into(), leaf("a")),
        ("b.ts".into(), leaf("b")),
        ("barrel.ts".into(), stars(&["./a", "./missing"])),
        ("conflict.ts".into(), stars(&["./a", "./b"])),
        ("cycle.ts".into(), stars(&["./cycle", "./a"])),
    ]);
    for projection in [ExportProjection::Legacy, ExportProjection::CallerPaths] {
        let single = resolve(&raw, &module, raw.keys(), projection);
        assert!(single.chain_unresolved > 0);
        assert!(single.barrel_conflicts > 0);
        let repeated = resolve(&raw, &module, raw.keys().chain(raw.keys()), projection);
        assert_eq!(repeated.resolved, single.resolved);
        assert_eq!(repeated.chain_unresolved, single.chain_unresolved * 2);
        assert_eq!(repeated.barrel_conflicts, single.barrel_conflicts * 2);
    }
}

#[test]
fn class_hits_are_excluded_from_callable_resolution() {
    let mut facts = leaf("a");
    facts
        .named
        .insert("Ctor".into(), JsExportTarget::Class("Ctor".into()));
    let raw = BTreeMap::from([
        ("a.ts".into(), facts),
        ("barrel.ts".into(), stars(&["./a"])),
    ]);
    for projection in [ExportProjection::Legacy, ExportProjection::CallerPaths] {
        let out = resolve(&raw, &module, raw.keys(), projection);
        for file in ["a.ts", "barrel.ts"] {
            assert_eq!(out.resolved[file]["real"].file, "a.ts");
            assert!(
                !out.resolved[file].contains_key("Ctor"),
                "class hit must not be projected as callable in {file}"
            );
        }
    }
}

#[test]
fn generated_graphs_preserve_all_fields_and_namespace() {
    let mut raw = BTreeMap::from([("a.ts".into(), leaf("a")), ("b.ts".into(), leaf("b"))]);
    raw.insert("barrel.ts".into(), stars(&["./a", "./missing"]));
    raw.insert("conflict.ts".into(), stars(&["./a", "./b"]));
    raw.insert("empty.ts".into(), stars(&["./missing"]));
    let mut duplicate = leaf("dup");
    duplicate.insert_named("real".into(), JsExportTarget::Local("real".into()));
    raw.insert("dup.ts".into(), duplicate);
    raw.insert("cycle.ts".into(), stars(&["./cycle", "./a"]));
    // Both named and star chains at and well beyond the depth boundary.
    for i in 0..MAX_REEXPORT_DEPTH + 5 {
        let next = if i == 0 {
            "./a".into()
        } else {
            format!("./chain{}", i - 1)
        };
        raw.insert(
            format!("chain{i}.ts"),
            JsExportFacts {
                namespace_proof_complete: true,
                named: BTreeMap::from([(
                    "real".into(),
                    JsExportTarget::ReExport {
                        module_path: next.clone(),
                        imported: "real".into(),
                    },
                )]),
                ..Default::default()
            },
        );
        let next = if i == 0 {
            "./a".into()
        } else {
            format!("./star{}", i - 1)
        };
        raw.insert(format!("star{i}.ts"), stars(&[&next]));
    }
    for i in 0..48 {
        raw.insert(
            format!("root{i}.ts"),
            stars(&["./barrel", "./empty", "./conflict", "./dup"]),
        );
    }
    compare(&raw);
    let out = resolve(&raw, &module, raw.keys(), ExportProjection::Legacy);
    assert!(out.chain_unresolved > 0);
    assert!(out.barrel_conflicts > 0);
    assert!(out.resolved["barrel.ts"]["real"].via_unresolved_star);
    assert!(out.resolved[&format!("chain{}.ts", MAX_REEXPORT_DEPTH - 1)].contains_key("real"));
    assert!(!out
        .resolved
        .contains_key(&format!("chain{}.ts", MAX_REEXPORT_DEPTH)));
    let namespaces = namespace(&raw, &module);
    assert_eq!(namespaces["a.ts"]["real"].file, "a.ts");
    assert!(!namespaces.contains_key("barrel.ts"));
    // Deterministically generated topologies include diamonds, self/mutual cycles,
    // renamed/default/import-forward claims, classes and opaque namespace nodes.
    let mut seed = 19u64;
    for _ in 0..64 {
        let mut graph = BTreeMap::new();
        for i in 0..9 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let a = (seed >> 32) % 9;
            let b = (seed >> 40) % 9;
            let mut f = leaf(&format!("n{i}"));
            f.namespace_proof_complete = seed & 8 != 0;
            match seed % 7 {
                0 => f = stars(&[&format!("./n{a}"), &format!("./n{b}"), "./missing"]),
                1 | 2 => {
                    f.named.insert(
                        "real".into(),
                        JsExportTarget::ReExport {
                            module_path: format!("./n{a}"),
                            imported: if seed & 16 == 0 { "real" } else { "default" }.into(),
                        },
                    );
                }
                3 => {
                    f.named.insert(
                        "real".into(),
                        JsExportTarget::ImportForward {
                            module_path: format!("n{a}"),
                            imported: "real".into(),
                        },
                    );
                }
                4 => {
                    f.insert_named("real".into(), JsExportTarget::Local("real".into()));
                }
                5 => {
                    f.named
                        .insert("real".into(), JsExportTarget::Class("real".into()));
                }
                _ => {}
            }
            graph.insert(format!("n{i}.ts"), f);
        }
        compare(&graph);
    }
}

#[test]
fn repeated_module_work_is_bounded_by_distinct_pairs() {
    let mut raw = BTreeMap::from([
        ("a.ts".into(), leaf("a")),
        ("barrel.ts".into(), stars(&["./a", "./missing"])),
    ]);
    for i in 0..64 {
        raw.insert(format!("root{i}.ts"), stars(&["./barrel"]));
    }
    let calls = Cell::new(0);
    let counted = |from: &str, spec: &str| {
        calls.set(calls.get() + 1);
        module(from, spec)
    };
    let old =
        oracle::with_uncached(|| resolve(&raw, &counted, raw.keys(), ExportProjection::Legacy));
    let old_calls = calls.replace(0);
    let new = resolve(&raw, &counted, raw.keys(), ExportProjection::Legacy);
    assert_eq!(old, new);
    assert_eq!(
        calls.get(),
        66,
        "one closure call per exact pair, including misses"
    );
    assert!(
        old_calls > calls.get() * 4,
        "witness must exercise repeated work"
    );
    eprintln!(
        "legacy module calls: main={old_calls}, memoized={}",
        calls.get()
    );
}

#[test]
fn namespace_module_work_is_bounded_by_distinct_pairs() {
    let mut raw = BTreeMap::from([
        ("a.ts".into(), leaf("a")),
        ("barrel.ts".into(), stars(&["./a", "./missing"])),
    ]);
    for i in 0..64 {
        raw.insert(format!("root{i}.ts"), stars(&["./barrel"]));
    }
    let calls = Cell::new(0);
    let counted = |from: &str, spec: &str| {
        calls.set(calls.get() + 1);
        module(from, spec)
    };
    let old = oracle::with_uncached(|| namespace(&raw, &counted));
    let old_calls = calls.replace(0);
    assert_eq!(namespace(&raw, &counted), old);
    assert_eq!(calls.get(), 66);
    assert!(old_calls > calls.get() * 4);
    eprintln!(
        "namespace module calls: main={old_calls}, memoized={}",
        calls.get()
    );
}

#[test]
fn cache_keys_include_from_and_exact_spec_and_cache_misses() {
    let memo = Modules::default();
    let calls = Cell::new(0);
    let f = |from: &str, spec: &str| {
        calls.set(calls.get() + 1);
        (spec != "missing").then(|| format!("{from}:{spec}"))
    };
    for _ in 0..3 {
        assert_eq!(memo.get(&f, "a", "./x"), Some("a:./x".into()));
        assert_eq!(memo.get(&f, "b", "./x"), Some("b:./x".into()));
        assert_eq!(memo.get(&f, "a", " ./x"), Some("a: ./x".into()));
        assert_eq!(memo.get(&f, "a", "missing"), None);
    }
    assert_eq!(calls.get(), 4);
}

#[test]
fn invocations_do_not_share_project_or_snapshot_results() {
    let raw = BTreeMap::from([
        ("root.ts".into(), stars(&["@leaf"])),
        ("a.ts".into(), leaf("a")),
        ("b.ts".into(), leaf("b")),
    ]);
    for target in ["a.ts", "b.ts", "a.ts"] {
        let module = |_: &str, _: &str| Some(target.into());
        let out = resolve(&raw, &module, raw.keys(), ExportProjection::CallerPaths);
        assert_eq!(out.resolved["root.ts"]["real"].file, target);
        assert_eq!(namespace(&raw, &module)["root.ts"]["real"].file, target);
    }
}
