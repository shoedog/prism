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
fn prometheus_shaped_workspace_accepts_dotless_active_main_module() {
    let root = module("github.com/prometheus/prometheus");
    let compliance = module("compliance");
    let documentation =
        module("github.com/prometheus/prometheus/documentation/examples/remote_storage");
    let sigv4 = module("github.com/prometheus/prometheus/sigv4");
    let web_ui = module("github.com/prometheus/prometheus/web/ui");
    let mut graph = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[
            (
                "go.work",
                Some(
                    "go 1.26\nuse (\n.\n./compliance\n./documentation/examples/remote_storage\n./sigv4\n./web/ui\n)\n",
                ),
            ),
            ("go.mod", Some(&root)),
            ("compliance/go.mod", Some(&compliance)),
            (
                "documentation/examples/remote_storage/go.mod",
                Some(&documentation),
            ),
            ("sigv4/go.mod", Some(&sigv4)),
            ("web/ui/go.mod", Some(&web_ui)),
        ]),
    );

    assert_eq!(graph.telemetry().modules, 5);
    assert_eq!(graph.telemetry().active, 5);
    assert!(!graph.telemetry().workspace_invalid);
    assert_eq!(
        graph.module_path_kind("compliance"),
        Some(PathKind::MainModule)
    );
    assert_eq!(
        graph.import_path_for_dir("storage"),
        Ok("github.com/prometheus/prometheus/storage".to_string())
    );
    assert_eq!(
        graph.import_path_for_dir("compliance/rules"),
        Ok("compliance/rules".to_string())
    );
    assert_eq!(
        graph.import_path_for_dir("documentation/examples/remote_storage/pkg"),
        Ok(
            "github.com/prometheus/prometheus/documentation/examples/remote_storage/pkg"
                .to_string()
        )
    );
    assert_eq!(
        graph.import_path_for_dir("sigv4/pkg"),
        Ok("github.com/prometheus/prometheus/sigv4/pkg".to_string())
    );
    assert_eq!(
        graph.import_path_for_dir("web/ui/module"),
        Ok("github.com/prometheus/prometheus/web/ui/module".to_string())
    );
}

#[test]
fn active_main_and_dependency_module_paths_keep_distinct_validation() {
    let cases = [
        ("module bad!path\n", true),
        ("module example.com/root\nrequire bad!path v1.0.0\n", true),
        ("module example.com/root\nrequire compliance v1.0.0\n", true),
    ];

    for (go_mod, workspace_invalid) in cases {
        let graph = GoModuleGraph::new(Path::new("/repo"), &snapshot(&[("go.mod", Some(go_mod))]));
        assert_eq!(
            graph.telemetry().workspace_invalid,
            workspace_invalid,
            "go.mod: {go_mod:?}"
        );
        assert_eq!(graph.telemetry().active, 0, "go.mod: {go_mod:?}");
    }
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

#[test]
fn workspace_replace_overrides_module_replace_before_usability_is_tested() {
    let root = "module example.com/root\nrequire original.example/a v0.0.0\nreplace original.example/a => ./local-a\n";
    let local = module("local.example/a");
    let work = "go 1.22\nuse .\nreplace original.example/a => remote.example/a v1.2.3\n";
    let graph = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[
            ("go.mod", Some(root)),
            ("go.work", Some(work)),
            ("local-a/go.mod", Some(&local)),
        ]),
    );

    assert_eq!(graph.telemetry().replaces_parsed, 2);
    assert_eq!(graph.telemetry().replaces_applied, 0);
    assert_eq!(graph.provider_path("local-a"), None);
    assert!(graph.replacement_is_unproven("original.example/a"));
}

#[test]
fn active_module_replace_union_applies_distinct_required_local_targets() {
    let root = "module example.com/root\nrequire original.example/a v0.0.0\nreplace original.example/a => ./fork-a\n";
    let second = "module example.com/second\nrequire original.example/b v0.0.0\nreplace original.example/b => ../fork-b\n";
    let fork_a = module("fork.example/a");
    let fork_b = module("fork.example/b");
    let graph = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[
            ("go.work", Some("go 1.22\nuse (\n.\n./second\n)\n")),
            ("go.mod", Some(root)),
            ("second/go.mod", Some(second)),
            ("fork-a/go.mod", Some(&fork_a)),
            ("fork-b/go.mod", Some(&fork_b)),
        ]),
    );

    assert!(!graph.telemetry().workspace_invalid);
    assert_eq!(graph.telemetry().replaces_parsed, 2);
    assert_eq!(graph.telemetry().replaces_applied, 2);
    assert_eq!(graph.provider_path("fork-a"), Some("original.example/a"));
    assert_eq!(graph.provider_path("fork-b"), Some("original.example/b"));
}

#[test]
fn conflicting_active_module_replaces_invalidate_unless_workspace_overrides() {
    let root = "module example.com/root\nrequire original.example/a v0.0.0\nreplace original.example/a => ./fork-a\n";
    let second = "module example.com/second\nreplace original.example/a => ../fork-b\n";
    let fork_a = module("fork.example/a");
    let fork_b = module("fork.example/b");
    let common = [
        ("go.mod", Some(root)),
        ("second/go.mod", Some(second)),
        ("fork-a/go.mod", Some(fork_a.as_str())),
        ("fork-b/go.mod", Some(fork_b.as_str())),
    ];
    let conflicting_work = "go 1.22\nuse (\n.\n./second\n)\n";
    let overridden_work =
        "go 1.22\nuse (\n.\n./second\n)\nreplace original.example/a => ./fork-a\n";

    let mut conflicting = vec![("go.work", Some(conflicting_work))];
    conflicting.extend(common);
    let graph = GoModuleGraph::new(Path::new("/repo"), &snapshot(&conflicting));
    assert!(graph.telemetry().workspace_invalid);

    let mut overridden = vec![("go.work", Some(overridden_work))];
    overridden.extend(common);
    let graph = GoModuleGraph::new(Path::new("/repo"), &snapshot(&overridden));
    assert!(!graph.telemetry().workspace_invalid);
    assert_eq!(graph.provider_path("fork-a"), Some("original.example/a"));
    assert_eq!(graph.provider_path("fork-b"), None);
}

#[test]
fn active_workspace_module_path_wins_over_a_replace_of_itself() {
    let root = "module example.com/root\nrequire go.etcd.io/etcd/api/v3 v3.0.0\nreplace go.etcd.io/etcd/api/v3 => ./api\n";
    let api = module("go.etcd.io/etcd/api/v3");
    let graph = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[
            ("go.work", Some("go 1.22\nuse (\n.\n./api\n)\n")),
            ("go.mod", Some(root)),
            ("api/go.mod", Some(&api)),
        ]),
    );

    assert!(!graph.telemetry().workspace_invalid);
    assert_eq!(graph.telemetry().replaces_parsed, 1);
    assert_eq!(graph.telemetry().replaces_applied, 0);
    assert_eq!(graph.provider_path("api"), Some("go.etcd.io/etcd/api/v3"));
    assert!(!graph.replacement_is_unproven("go.etcd.io/etcd/api/v3"));
}

#[test]
fn wildcard_replace_requires_an_active_main_module_requirement() {
    let root = "module example.com/root\nreplace original.example/a => ./fork\n";
    let fork = module("fork.example/a");
    let graph = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[("go.mod", Some(root)), ("fork/go.mod", Some(&fork))]),
    );

    assert_eq!(graph.telemetry().replaces_parsed, 1);
    assert_eq!(graph.telemetry().replaces_applied, 0);
    assert_eq!(graph.provider_path("fork"), None);
    assert!(!graph.replacement_is_unproven("original.example/a"));
}

#[test]
fn version_specific_replace_is_unproven_even_with_a_matching_requirement() {
    let root = "module example.com/root\nrequire original.example/a v1.0.0\nreplace original.example/a v1.0.0 => ./fork\n";
    let fork = module("fork.example/a");
    let graph = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[("go.mod", Some(root)), ("fork/go.mod", Some(&fork))]),
    );

    assert_eq!(graph.telemetry().replaces_applied, 0);
    assert!(graph.replacement_is_unproven("original.example/a"));
    assert!(graph.replacement_dir_is_unproven("fork"));
}

#[test]
fn local_replace_target_must_be_inside_regular_and_valid() {
    let root_template = |target: &str| {
        format!(
            "module example.com/root\nrequire original.example/a v0.0.0\nreplace original.example/a => {target}\n"
        )
    };
    let cases = [
        ("./missing", Vec::new()),
        (
            "./malformed",
            vec![("malformed/go.mod", Some("module bad path\n"))],
        ),
        ("./linked", vec![("linked/go.mod", None)]),
        ("../outside", Vec::new()),
    ];

    for (target, extras) in cases {
        let root = root_template(target);
        let mut entries = vec![("go.mod", Some(root.as_str()))];
        entries.extend(extras);
        let graph = GoModuleGraph::new(Path::new("/repo"), &snapshot(&entries));
        assert_eq!(graph.telemetry().replaces_applied, 0, "target: {target}");
        assert!(
            graph.replacement_is_unproven("original.example/a"),
            "target: {target}"
        );
    }
}

#[test]
fn relative_parent_replace_that_stays_in_repo_is_applied() {
    let main = "module example.com/main\nrequire original.example/a v0.0.0\nreplace original.example/a => ../fork\n";
    let fork = module("fork.example/a");
    let graph = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[
            ("go.work", Some("go 1.22\nuse ./main\n")),
            ("main/go.mod", Some(main)),
            ("fork/go.mod", Some(&fork)),
        ]),
    );
    assert_eq!(graph.telemetry().replaces_applied, 1);
    assert_eq!(graph.provider_path("fork"), Some("original.example/a"));
}

#[test]
fn duplicate_effective_paths_among_active_modules_invalidate_the_workspace() {
    let first = module("example.com/duplicate");
    let second = module("example.com/duplicate");
    let graph = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[
            ("go.work", Some("go 1.22\nuse (\n.\n./second\n)\n")),
            ("go.mod", Some(&first)),
            ("second/go.mod", Some(&second)),
        ]),
    );

    assert!(graph.telemetry().workspace_invalid);
    assert_eq!(graph.telemetry().active, 0);
    assert_eq!(graph.provider_path(""), None);
    assert_eq!(graph.provider_path("second"), None);
}

#[test]
fn effective_identity_uses_the_nearest_active_provider_and_blocks_inactive_nested_modules() {
    let root = module("example.com/root");
    let nested = module("example.com/nested");
    let deeper = module("example.com/deeper");
    let mut graph = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[
            ("go.work", Some("go 1.22\nuse (\n.\n./nested/deeper\n)\n")),
            ("go.mod", Some(&root)),
            ("nested/go.mod", Some(&nested)),
            ("nested/deeper/go.mod", Some(&deeper)),
        ]),
    );

    assert_eq!(
        graph.import_path_for_dir("pkg"),
        Ok("example.com/root/pkg".to_string())
    );
    assert_eq!(
        graph.import_path_for_dir("nested/pkg"),
        Err(GoImportPathReason::InactiveModule)
    );
    assert_eq!(
        graph.import_path_for_dir("nested/deeper/pkg/v2"),
        Ok("example.com/deeper/pkg/v2".to_string())
    );
}

#[test]
fn replacement_identity_uses_the_lhs_instead_of_the_targets_declared_path() {
    let root = "module example.com/root\nrequire original.example/a v0.0.0\nreplace original.example/a => ./fork\n";
    let fork = module("fork.example/a");
    let mut graph = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[("go.mod", Some(root)), ("fork/go.mod", Some(&fork))]),
    );

    assert_eq!(
        graph.import_path_for_dir("fork/p"),
        Ok("original.example/a/p".to_string())
    );
}

#[test]
fn identity_reports_each_fail_closed_reason_at_the_nearest_boundary() {
    let root = module("example.com/root");
    let versioned = "module example.com/versioned\n";
    let active = "module example.com/root\nrequire original.example/a v1.0.0\nreplace original.example/a v1.0.0 => ./versioned\n";
    let mut graph = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[
            ("go.mod", Some(active)),
            ("bad/go.mod", Some("module bad path\n")),
            ("linked/go.mod", None),
            ("versioned/go.mod", Some(versioned)),
        ]),
    );
    assert_eq!(
        graph.import_path_for_dir("bad/pkg"),
        Err(GoImportPathReason::Malformed)
    );
    assert_eq!(
        graph.import_path_for_dir("linked/pkg"),
        Err(GoImportPathReason::Symlink)
    );
    assert_eq!(
        graph.import_path_for_dir("versioned/pkg"),
        Err(GoImportPathReason::ReplaceUnproven)
    );

    let mut none = GoModuleGraph::new(Path::new("/repo"), &ManifestSnapshot::default());
    assert_eq!(
        none.import_path_for_dir("pkg"),
        Err(GoImportPathReason::NoGoMod)
    );

    let mut invalid = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[("go.work", Some("go nope\n")), ("go.mod", Some(&root))]),
    );
    assert_eq!(
        invalid.import_path_for_dir("pkg"),
        Err(GoImportPathReason::WorkspaceInvalid)
    );
}

#[test]
fn directory_identities_are_memoized_and_each_snapshot_manifest_is_parsed_once() {
    let root = module("example.com/root");
    let nested = module("example.com/nested");
    let snapshot = snapshot(&[
        ("go.work", Some("go 1.22\nuse .\n")),
        ("go.mod", Some(&root)),
        ("nested/go.mod", Some(&nested)),
        ("bad/go.mod", Some("module bad path\n")),
        ("linked/go.mod", None),
        ("pkg/testdata/go.mod", Some(&nested)),
    ]);
    let mut graph = GoModuleGraph::new(Path::new("/repo"), &snapshot);

    assert_eq!(
        graph.import_path_for_dir("pkg"),
        graph.import_path_for_dir("pkg")
    );
    assert_eq!(graph.memo_len(), 1);
    assert_eq!(
        graph.manifest_parse_counts(),
        &BTreeMap::from([
            ("bad/go.mod".to_string(), 1),
            ("go.mod".to_string(), 1),
            ("go.work".to_string(), 1),
            ("nested/go.mod".to_string(), 1),
        ])
    );
}
