use super::*;

fn snapshot(entries: &[(&str, Option<&str>)]) -> ManifestSnapshot {
    let mut snapshot = ManifestSnapshot::default();
    for (path, contents) in entries {
        if let Some(contents) = contents {
            snapshot.insert_regular((*path).to_string(), contents.as_bytes().to_vec());
        } else {
            snapshot.insert_symlink_refused((*path).to_string());
        }
    }
    snapshot
}

fn module(path: &str) -> String {
    format!("module {path}\n\ngo 1.22\n")
}

#[test]
fn construction_discovers_declared_modules_and_root_default_active_set() {
    let root = module("example.com/root");
    let nested = module("example.com/nested");
    let graph = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[("go.mod", Some(&root)), ("nested/go.mod", Some(&nested))]),
    );

    assert_eq!(graph.telemetry().modules, 2);
    assert_eq!(graph.telemetry().active, 1);
    assert_eq!(graph.active_dirs(), &BTreeSet::from([String::new()]));
    assert!(!graph.telemetry().workspace_invalid);
}

#[test]
fn construction_without_root_module_or_workspace_has_no_active_modules() {
    let nested = module("example.com/nested");
    let graph = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[("nested/go.mod", Some(&nested))]),
    );

    assert_eq!(graph.telemetry().modules, 1);
    assert_eq!(graph.telemetry().active, 0);
    assert!(graph.active_dirs().is_empty());
}

#[test]
fn workspace_use_accepts_relative_absolute_and_normalized_parent_paths() {
    let root = module("example.com/root");
    let nested = module("example.com/nested");
    let sibling = module("example.com/sibling");
    let work = "go 1.22\nuse (\n .\n ./nested/deeper/..\n /repo/sibling\n)\n";
    let graph = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[
            ("go.work", Some(work)),
            ("go.mod", Some(&root)),
            ("nested/go.mod", Some(&nested)),
            ("sibling/go.mod", Some(&sibling)),
        ]),
    );

    assert_eq!(graph.telemetry().active, 3);
    assert_eq!(
        graph.active_dirs(),
        &BTreeSet::from([String::new(), "nested".to_string(), "sibling".to_string()])
    );
    assert!(!graph.telemetry().workspace_invalid);
}

#[test]
fn workspace_can_activate_a_nested_module_without_a_root_go_mod() {
    let nested = module("example.com/nested");
    let graph = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[
            ("go.work", Some("go 1.22\nuse ./nested\n")),
            ("nested/go.mod", Some(&nested)),
        ]),
    );

    assert_eq!(graph.telemetry().active, 1);
    assert_eq!(graph.active_dirs(), &BTreeSet::from(["nested".to_string()]));
    assert!(!graph.telemetry().workspace_invalid);
}

#[test]
fn malformed_workspace_or_unproven_use_invalidates_the_whole_workspace() {
    let valid = module("example.com/valid");
    let malformed = "module bad path\n";
    let cases = [
        ("go nope\nuse ./valid extra\n", Vec::new()),
        ("go 1.22\nuse ../outside\n", Vec::new()),
        ("go 1.22\nuse ./missing\n", Vec::new()),
        (
            "go 1.22\nuse ./bad\n",
            vec![("bad/go.mod", Some(malformed))],
        ),
        ("go 1.22\nuse ./linked\n", vec![("linked/go.mod", None)]),
    ];

    for (work, extras) in cases {
        let mut entries = vec![
            ("go.work", Some(work)),
            ("valid/go.mod", Some(valid.as_str())),
        ];
        entries.extend(extras);
        let graph = GoModuleGraph::new(Path::new("/repo"), &snapshot(&entries));
        assert!(graph.telemetry().workspace_invalid, "go.work: {work:?}");
        assert_eq!(graph.telemetry().active, 0, "go.work: {work:?}");
    }
}

#[test]
fn malformed_or_symlinked_root_active_module_invalidates_without_go_work() {
    for entry in [Some("module bad path\n"), None] {
        let graph = GoModuleGraph::new(Path::new("/repo"), &snapshot(&[("go.mod", entry)]));
        assert!(graph.telemetry().workspace_invalid);
        assert_eq!(graph.telemetry().active, 0);
    }
}

#[test]
fn testdata_manifests_are_not_declared_modules() {
    let root = module("example.com/root");
    let ignored = module("example.com/ignored");
    let graph = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[
            ("go.mod", Some(&root)),
            ("pkg/testdata/go.mod", Some(&ignored)),
        ]),
    );
    assert_eq!(graph.telemetry().modules, 1);
}

#[test]
fn graph_starts_with_an_empty_directory_identity_memo() {
    let graph = GoModuleGraph::new(Path::new("/repo"), &ManifestSnapshot::default());
    assert_eq!(graph.memo_len(), 0);
}
