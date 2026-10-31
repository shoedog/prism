use super::*;
use crate::call_graph::{CallGraph, ScopeGraphBuildInputs};
use crate::go_type_alias::import_memo::{measured, reference::signature_imports_reference};
use crate::go_type_alias::SignatureImportMemo;
use sha2::{Digest, Sha256};

fn fixture() -> BTreeMap<String, ParsedFile> {
    let rows = [
        ("p/types.go", r#"package p
import (q "example/q"; _ "example/sideeffect")
type Alias = q.T
type Inner struct {Alias; Callback func(int)}
type Outer struct {*Inner}
type I interface {M(int)}
func invoke(cb func(int)) { cb(1) }
func target(x int) {}
func recovering(x int) {recover()}
func run(i I, o Outer) {i.M(1); o.M(2); invoke(target); invoke(recovering); o.Callback(3)}
func register(o *Inner) {o.Callback = target; o.Callback = recovering}
func recovery() {defer func(){recover()}()}
func live() { _ = Outer{}; _ = new(Inner); _ = &Inner{} }
"#),
        ("q/types.go", "package q\ntype T struct{}\nfunc (T) M(x int) {}\n"),
        ("q/z.go", "package q\ntype T struct{Last int}\nfunc (T) M(x int) {}\ntype Alias = T\n"),
        ("p/z.go", "package p\ntype Inner struct{Callback func(string)}\nfunc (Inner) M(x int) {}\n"),
        ("dot/types.go", "package dot\nimport . \"example/q\"\ntype S struct {T}\nfunc f(x T) {}\n"),
        ("versions/types.go", "package versions\nimport (\"example/widget/v2\"; \"gopkg.in/yaml.v3\")\ntype S struct{widget.T; yaml.T}\n"),
        ("tag/types_linux.go", "//go:build linux\n\npackage tag\ntype S struct{X int}\n"),
        ("tag/types_windows.go", "//go:build windows\n\npackage tag\ntype S struct{X string}\n"),
        ("p/types_test.go", "package p\nimport . \"example/unknown\"\ntype TestOnly struct{Missing}\n"),
        ("empty/empty.go", "package empty\n"),
        ("broken/broken.go", "package broken\nimport (bad \"example/bad\"\ntype S struct{bad.T}\n"),
    ];
    let mut files: BTreeMap<_, _> = rows
        .iter()
        .map(|(path, source)| {
            (
                (*path).into(),
                ParsedFile::parse(path, source, Language::Go).unwrap(),
            )
        })
        .collect();
    files.insert(
        "other.py".into(),
        ParsedFile::parse(
            "other.py",
            "import example\ndef f(): pass\n",
            Language::Python,
        )
        .unwrap(),
    );
    files
}

#[test]
fn ordered_preparation_matches_frozen_maps_and_primes_only_go() {
    let files = fixture();
    for local in ["", "example/local", "other/local"] {
        let paths = files.keys().map(|p| (p.clone(), local.into())).collect();
        let rows = prepare(&files, &paths);
        assert_eq!(
            rows.iter().map(|(p, _)| *p).collect::<Vec<_>>(),
            files
                .iter()
                .filter(|(_, f)| f.language == Language::Go)
                .map(|(p, _)| p.as_str())
                .collect::<Vec<_>>()
        );
        let (memo, scans) = measured(|| SignatureImportMemo::parallel(&files, &paths));
        assert_eq!(scans, files.len() - 1);
        for (path, parsed) in &files {
            assert_eq!(
                memo.entries[path.as_str()].get().is_some(),
                parsed.language == Language::Go
            );
            let expected = signature_imports_reference(parsed, Some(local));
            assert_eq!(memo.get(path), &expected, "{path}");
        }
        assert!(memo.get("missing.go").is_empty());
        let (_, scans) = measured(|| {
            for path in files.keys() {
                let _ = memo.get(path);
            }
        });
        assert_eq!(scans, 0, "prepared and empty maps must be reused");
    }
    assert!(prepare(&BTreeMap::new(), &BTreeMap::new()).is_empty());
    let non_go = BTreeMap::from([(
        "a.py".into(),
        ParsedFile::parse("a.py", "import a", Language::Python).unwrap(),
    )]);
    assert!(prepare(&non_go, &BTreeMap::new()).is_empty());
}

#[test]
fn final_edges_and_call_stats_match_serial_reference() {
    let files = fixture();
    let mut inputs = ScopeGraphBuildInputs::from_files_convention(&files);
    inputs
        .manifest_snapshot
        .insert_regular("go.mod".into(), b"module example\n\ngo 1.24\n".to_vec());
    crate::build_pool::install(|| {
        let expected = with_serial_reference(|| {
            CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
        });
        let expected_stats = crate::navigation::queries::call_stats(&expected);
        let expected_edges = crate::navigation::queries::call_site_dump(&expected);
        assert!(!expected_edges.is_empty());
        assert!(!expected.go_callback_parameters.is_empty());
        assert!(!expected.go_promoted_selector_snapshot().owners.is_empty());
        assert!(!expected.interface_method_names.is_empty());
        println!(
            "GO_DISPATCH_EQ edges={:x} stats={:x}",
            Sha256::digest(serde_json::to_vec(&expected_edges).unwrap()),
            Sha256::digest(serde_json::to_vec(&expected_stats).unwrap())
        );
        for _ in 0..3 {
            let actual = CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs));
            assert_eq!(actual.calls, expected.calls);
            assert_eq!(actual.callers, expected.callers);
            assert_eq!(
                serde_json::to_vec(&crate::navigation::queries::call_site_dump(&actual)).unwrap(),
                serde_json::to_vec(&expected_edges).unwrap()
            );
            assert_eq!(
                serde_json::to_vec(&crate::navigation::queries::call_stats(&actual)).unwrap(),
                serde_json::to_vec(&expected_stats).unwrap()
            );
        }
    });
}
