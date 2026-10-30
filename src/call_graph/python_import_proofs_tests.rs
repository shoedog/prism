use super::*;
use crate::ast::ParsedFile;
use crate::call_graph::CallGraph;
use crate::languages::Language;

thread_local! {
    pub(super) static OWNER_KEY_VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn graph(sources: &[(&str, &str)]) -> CallGraph {
    let files = sources
        .iter()
        .map(|(path, source)| {
            let language = Language::from_path(path).unwrap();
            (
                path.to_string(),
                ParsedFile::parse(path, source, language).unwrap(),
            )
        })
        .collect();
    CallGraph::build(&files)
}

fn differential(graph: &CallGraph) -> BTreeSet<(String, String, String, String)> {
    let actual = python_imported_class_proof_keys(
        &graph.import_bindings,
        &graph.indexed_files,
        &graph.clean_class_spans,
        &graph.python_inert_initializers,
    );
    let expected = reference::python_imported_class_proof_keys(
        &graph.import_bindings,
        &graph.indexed_files,
        &graph.clean_class_spans,
        &graph.python_inert_initializers,
    );
    assert_eq!(actual, expected);
    actual
}

fn key(caller: &str, receiver: &str, file: &str, owner: &str) -> (String, String, String, String) {
    (caller.into(), receiver.into(), file.into(), owner.into())
}

fn binding(
    module: &str,
    local: &str,
    member: Option<&str>,
    kind: ImportBindingKind,
) -> ImportBinding {
    ImportBinding {
        module_path: module.into(),
        local: local.into(),
        member: member.map(str::to_string),
        kind,
        eligible: true,
    }
}

#[test]
fn synthetic_repos_match_frozen_reference() {
    let graph = graph(&[
        ("pkg/__init__.py", ""),
        ("pkg/models.py", "class Client:\n    pass\nclass Zebra:\n    pass\n"),
        ("pkg/y.py", "class Y:\n    pass\n"),
        ("pkg/sub/__init__.py", ""),
        ("pkg/sub/x.py", "class X:\n    pass\n"),
        ("pkg/app.py", "import pkg.models\nimport pkg.models as models\nfrom pkg.models import Client as Imported\nfrom .models import Zebra\nfrom . import models as relative\nfrom pkg.sub import x\n"),
        ("pkg/sub/app.py", "from .. import y\nfrom ..models import Client\nfrom . import x\n"),
        ("ns/models.py", "class Namespace:\n    pass\n"),
        ("box/__init__.py", "class Package:\n    pass\n"),
        ("left/twin.py", "class Twin:\n    pass\n"),
        ("right/twin.py", "class Other:\n    pass\n"),
        ("pkg/models.ts", "export class Foreign {}"),
        ("only.js", "export class Only {}"),
        ("app.py", "import ns.models as names\nfrom ns import models as submodule\nfrom ns.models import Namespace\nimport box\nfrom box import Package\nfrom twin import Twin\nimport twin\nfrom only import Only\nfrom stub import Stub\nfrom missing import Gone\n"),
        ("poison.py", "from pkg.models import Client\nClient = 1\nfrom pkg.models import *\n"),
    ]);
    let keys = differential(&graph);
    for expected in [
        key("pkg/app.py", "pkg.models.Client", "pkg/models.py", "Client"),
        key("pkg/app.py", "models.Zebra", "pkg/models.py", "Zebra"),
        key("pkg/app.py", "Imported", "pkg/models.py", "Client"),
        key("pkg/app.py", "Zebra", "pkg/models.py", "Zebra"),
        key("pkg/app.py", "relative.Client", "pkg/models.py", "Client"),
        key("pkg/app.py", "x.X", "pkg/sub/x.py", "X"),
        key("pkg/sub/app.py", "y.Y", "pkg/y.py", "Y"),
        key("pkg/sub/app.py", "Client", "pkg/models.py", "Client"),
        key("pkg/sub/app.py", "x.X", "pkg/sub/x.py", "X"),
        key("app.py", "names.Namespace", "ns/models.py", "Namespace"),
        key("app.py", "submodule.Namespace", "ns/models.py", "Namespace"),
        key("app.py", "Namespace", "ns/models.py", "Namespace"),
        key("app.py", "box.Package", "box/__init__.py", "Package"),
        key("app.py", "Package", "box/__init__.py", "Package"),
    ] {
        assert!(keys.contains(&expected), "missing {expected:?}");
    }
    assert!(!keys.iter().any(|(caller, receiver, file, _)| {
        caller == "poison.py"
            || ["Twin", "twin.Twin", "Only", "Stub", "Gone"].contains(&receiver.as_str())
            || file.ends_with(".ts")
            || file.ends_with(".js")
            || file.ends_with(".pyi")
    }));
}

#[test]
fn relative_proofs_separate_callers_and_cached_misses() {
    let graph = graph(&[
        ("a/app.py", "from .models import Client\n"),
        ("b/app.py", "from .models import Client\n"),
        ("c/app.py", "from .models import Client\n"),
        ("b/models.py", "class Client:\n    pass\n"),
        ("c/models.py", "class Client:\n    pass\n"),
    ]);
    assert_eq!(
        differential(&graph),
        BTreeSet::from([
            key("b/app.py", "Client", "b/models.py", "Client"),
            key("c/app.py", "Client", "c/models.py", "Client"),
        ])
    );
}

#[test]
fn memo_matches_lookup_for_every_path_shape() {
    let files: BTreeSet<_> = [
        "a/models.py",
        "b/models.py",
        "a/pkg/__init__.py",
        "a/sub/x.py",
        "a/__init__.py",
        "b/__init__.py",
        "a/x.py",
        "b/x.py",
        "a/models.ts",
        "only.js",
        "stub.pyi",
        "ns/leaf.py",
        "ns/leaf/__init__.py",
        "unique.py",
        "unique.ts",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    let mut memo = ModuleFileMemo::new(&files);
    for module in [
        "a.models",
        "models",
        ".models",
        " .models ",
        "..models",
        ".",
        "..",
        ".pkg",
        "a.pkg",
        "ns.leaf",
        "unique",
        "only",
        "stub",
        "missing",
        "",
        "./models",
        "../models",
        " ../models ",
        "a/models",
        ".odd/path",
        "...x",
    ] {
        for caller in [
            "a/app.py",
            "b/app.py",
            "a/sub/app.py",
            "app.py",
            "a/other.py",
        ] {
            assert_eq!(
                memo.resolve(module, caller),
                reference::unique_python_module_file(module, caller, &files),
                "module {module:?}, caller {caller:?}"
            );
        }
    }
    assert_eq!(memo.resolve("models", "a/app.py"), None, "ambiguous stem");
    assert_eq!(
        memo.resolve("ns.leaf", "a/app.py"),
        None,
        "module/package collision"
    );
    assert_eq!(memo.resolve("only", "app.py"), None, "JS is excluded");
    assert_eq!(memo.resolve("stub", "app.py"), None, "stubs are excluded");
}

#[test]
fn memo_reuses_successes_and_failures_at_the_exact_scope() {
    let files = BTreeSet::from([
        "a/models.py".into(),
        "b/models.py".into(),
        "unique.py".into(),
    ]);
    let mut memo = ModuleFileMemo::new(&files);
    for caller in ["a/app.py", "b/app.py", "top.py"] {
        assert_eq!(
            memo.resolve("unique", caller).map(String::as_str),
            Some("unique.py")
        );
        assert_eq!(memo.resolve("models", caller), None);
        assert_eq!(memo.resolve("missing", caller), None);
    }
    assert_eq!(
        memo.scans, 3,
        "absolute paths share successes and negative results"
    );
    assert_eq!(
        memo.resolve(".models", "a/app.py").map(String::as_str),
        Some("a/models.py")
    );
    assert_eq!(
        memo.resolve(".models", "a/other.py").map(String::as_str),
        Some("a/models.py")
    );
    assert_eq!(memo.scans, 4, "same directory shares the relative result");
    assert_eq!(
        memo.resolve(".models", "b/app.py").map(String::as_str),
        Some("b/models.py")
    );
    assert_eq!(memo.resolve(".models", "top.py"), None);
    assert_eq!(memo.scans, 6);
    let empty = BTreeSet::new();
    assert_eq!(
        ModuleFileMemo::new(&empty).resolve("unique", "a/app.py"),
        None
    );
}

#[test]
fn whole_computation_shares_route_and_enumeration_lookups() {
    let files = BTreeSet::from([
        "pkg/models.py".into(),
        "left/twin.py".into(),
        "right/twin.py".into(),
    ]);
    let spans = BTreeMap::from([(("pkg/models.py".into(), "Client".into()), (0, 10))]);
    let bindings = BTreeMap::from_iter(["app.py", "other/app.py"].map(|caller| {
        (
            caller.to_string(),
            vec![
                binding(
                    "pkg.models",
                    "Client",
                    Some("Client"),
                    ImportBindingKind::MemberImport,
                ),
                binding(
                    "pkg.models",
                    "Second",
                    Some("Second"),
                    ImportBindingKind::MemberImport,
                ),
                binding(
                    "pkg.models",
                    "models",
                    None,
                    ImportBindingKind::AliasedModuleImport,
                ),
                binding("missing", "gone", None, ImportBindingKind::ModuleImport),
                binding(
                    "twin",
                    "twins",
                    None,
                    ImportBindingKind::AliasedModuleImport,
                ),
            ],
        )
    }));
    let mut memo = ModuleFileMemo::new(&files);
    let inert = BTreeSet::new();
    let actual = proof_keys_with_memo(&bindings, &spans, &inert, &mut memo);
    assert_eq!(
        actual,
        reference::python_imported_class_proof_keys(&bindings, &files, &spans, &inert)
    );
    assert_eq!(actual.len(), 4);
    assert_eq!(
        memo.scans, 3,
        "one success, one missing and one ambiguous module across all bindings/callers"
    );
}

#[test]
fn owner_range_preserves_order_and_file_boundaries() {
    let spans = BTreeMap::from_iter(
        [
            ("a.py", "Before"),
            ("file.py", ""),
            ("file.py", "Z"),
            ("file.py", "α"),
            ("file.py\0", "Neighbor"),
            ("file.py/next", "Child"),
            ("z.py", "After"),
        ]
        .map(|(file, owner)| ((file.into(), owner.into()), (0, 1))),
    );
    for file in [
        "",
        "a.py",
        "file.py",
        "file.py\0",
        "missing.py",
        "z.py",
        "zz.py",
    ] {
        let expected: Vec<_> = spans
            .keys()
            .filter(|(f, _)| f == file)
            .map(|(_, owner)| owner)
            .collect();
        assert_eq!(class_owners(file, &spans).collect::<Vec<_>>(), expected);
    }
    assert_eq!(
        class_owners("file.py", &spans)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["", "Z", "α"]
    );
    assert_eq!(class_owners("a.py", &BTreeMap::new()).count(), 0);
}

#[test]
fn owner_range_does_not_visit_unrelated_classes() {
    let mut spans = BTreeMap::new();
    for i in 0..1000 {
        for prefix in ["a", "z"] {
            spans.insert((format!("{prefix}{i:04}.py"), "Other".into()), (0, 1));
        }
    }
    for owner in ["", "Client", "Zebra"] {
        spans.insert(("m.py".into(), owner.into()), (0, 1));
    }
    for (file, expected, visit_bound) in [
        ("m.py", vec!["", "Client", "Zebra"], 4),
        ("n.py", vec![], 1),
        ("zzzz.py", vec![], 0),
    ] {
        OWNER_KEY_VISITS.with(|count| count.set(0));
        let owners: Vec<_> = class_owners(file, &spans).map(String::as_str).collect();
        assert_eq!(owners, expected);
        let visits = OWNER_KEY_VISITS.with(|count| count.get());
        assert_eq!(
            visits, visit_bound,
            "{file}: visit counter must execute and stop at the file boundary"
        );
    }
}

fn route_value(route: PythonImportedClassRoute) -> (u8, Option<(String, String)>) {
    match route {
        PythonImportedClassRoute::NotImported => (0, None),
        PythonImportedClassRoute::Blocked => (1, None),
        PythonImportedClassRoute::Proven {
            defining_file,
            owner,
        } => (2, Some((defining_file, owner))),
    }
}

#[test]
fn uncached_route_matches_reference_including_refusals() {
    let graph = graph(&[
        ("pkg/models.py", "class Client:\n    pass\n"),
        (
            "pkg/app.py",
            "from .models import Client\nimport pkg.models as models\n",
        ),
        (
            "app.py",
            "import pkg.models\nfrom pkg import models as sub\n",
        ),
        ("blocked.py", "from pkg.models import Client\nClient = 1\n"),
    ]);
    for caller in ["pkg/app.py", "app.py", "blocked.py", "absent.py"] {
        for receiver in [
            "Client",
            "models.Client",
            "pkg.models.Client",
            "sub.Client",
            "models.Missing",
            "absent",
            "sub.bad.Client",
        ] {
            let bindings = graph.import_bindings.get(caller).map(Vec::as_slice);
            assert_eq!(
                route_value(python_imported_class_route(
                    caller,
                    receiver,
                    bindings,
                    &graph.indexed_files,
                    &graph.clean_class_spans,
                    &graph.python_inert_initializers
                )),
                route_value(reference::python_imported_class_route(
                    caller,
                    receiver,
                    bindings,
                    &graph.indexed_files,
                    &graph.clean_class_spans,
                    &graph.python_inert_initializers
                )),
                "{caller}: {receiver}"
            );
        }
    }
}

#[test]
fn ineligible_and_malformed_bindings_keep_old_refusals() {
    let files = BTreeSet::from(["pkg/models.py".into()]);
    let spans = BTreeMap::from([(("pkg/models.py".into(), "Client".into()), (0, 10))]);
    let mut ineligible = binding(
        "pkg.models",
        "bad",
        None,
        ImportBindingKind::AliasedModuleImport,
    );
    ineligible.eligible = false;
    let bindings = BTreeMap::from([(
        "app.py".into(),
        vec![
            ineligible,
            binding("pkg.models", "*", None, ImportBindingKind::WildcardImport),
            binding(
                "pkg.models",
                "broken",
                None,
                ImportBindingKind::MemberImport,
            ),
            binding(
                "pkg.models",
                "wrong",
                Some("bad.name"),
                ImportBindingKind::MemberImport,
            ),
            binding(
                "pkg.models",
                "Client",
                Some("Client"),
                ImportBindingKind::MemberImport,
            ),
            binding(
                "pkg.models",
                "Client",
                Some("Client"),
                ImportBindingKind::MemberImport,
            ),
            binding("", "empty", None, ImportBindingKind::ModuleImport),
        ],
    )]);
    let inert = BTreeSet::new();
    let actual = python_imported_class_proof_keys(&bindings, &files, &spans, &inert);
    assert_eq!(
        actual,
        reference::python_imported_class_proof_keys(&bindings, &files, &spans, &inert)
    );
    assert!(actual.is_empty());
}
