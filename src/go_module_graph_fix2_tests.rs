use super::*;

fn snapshot(entries: &[(&str, &str)]) -> ManifestSnapshot {
    let mut snapshot = ManifestSnapshot::default();
    for (path, contents) in entries {
        snapshot.insert_regular((*path).to_string(), contents.as_bytes().to_vec());
    }
    snapshot
}

#[test]
fn module_versions_follow_x_mod_semver_syntax() {
    for version in [
        "v1.2.3",
        "v0.0.0-20200101000000-abcdef123456",
        "v2.0.0+incompatible",
        "v1",
        "v1.2",
    ] {
        assert!(valid_module_version(version), "valid version: {version}");
    }
    for version in ["vbogus", "v@x", "v01.2.3", "v1.2.3-", "1.2.3", "v1.2.3+"] {
        assert!(!valid_module_version(version), "invalid version: {version}");
    }
}

#[test]
fn malformed_versions_in_every_active_directive_invalidate_the_workspace() {
    for directive in [
        "require example.com/a vbogus",
        "exclude example.com/a vbogus",
        "replace example.com/a vbogus => ./fork",
        "replace example.com/a => replacement.example/a vbogus",
    ] {
        let go_mod = format!("module example.com/root\n{directive}\n");
        let mut graph = GoModuleGraph::new(
            Path::new("/repo"),
            &snapshot(&[("go.mod", go_mod.as_str())]),
        );

        assert!(
            graph.telemetry().workspace_invalid,
            "directive: {directive}"
        );
        assert_eq!(graph.telemetry().active, 0, "directive: {directive}");
        assert_eq!(
            graph.import_path_for_dir("pkg"),
            Err(GoImportPathReason::WorkspaceInvalid),
            "directive: {directive}"
        );
    }
}

#[test]
fn malformed_version_in_inactive_module_is_subtree_local() {
    let mut graph = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[
            ("go.mod", "module example.com/root\n"),
            (
                "inactive/go.mod",
                "module example.com/inactive\nrequire example.com/a vbogus\n",
            ),
        ]),
    );

    assert!(!graph.telemetry().workspace_invalid);
    assert_eq!(graph.telemetry().active, 1);
    assert_eq!(
        graph.import_path_for_dir("pkg"),
        Ok("example.com/root/pkg".to_string())
    );
    assert_eq!(
        graph.import_path_for_dir("inactive/pkg"),
        Err(GoImportPathReason::Malformed)
    );
}

#[test]
fn valid_retract_versions_preserve_active_identities() {
    for directive in [
        "retract v1.0.0",
        "retract [v1.0.0, v1.2.0] // security rationale",
        "retract [v1.2.0, v1.0.0]",
    ] {
        let go_mod = format!("module example.com/root\n{directive}\n");
        let mut graph = GoModuleGraph::new(
            Path::new("/repo"),
            &snapshot(&[("go.mod", go_mod.as_str())]),
        );

        assert!(
            !graph.telemetry().workspace_invalid,
            "directive: {directive}"
        );
        assert_eq!(graph.telemetry().active, 1, "directive: {directive}");
        assert_eq!(
            graph.import_path_for_dir("pkg"),
            Ok("example.com/root/pkg".to_string()),
            "directive: {directive}"
        );
    }
}

#[test]
fn malformed_retract_bounds_follow_active_and_inactive_layering() {
    for directive in [
        "retract vbogus",
        "retract [v1.0.0, vbogus]",
        "retract [v1. 0.0, v1.2.0]",
    ] {
        let go_mod = format!("module example.com/root\n{directive}\n");
        let mut graph = GoModuleGraph::new(
            Path::new("/repo"),
            &snapshot(&[("go.mod", go_mod.as_str())]),
        );

        assert!(
            graph.telemetry().workspace_invalid,
            "directive: {directive}"
        );
        assert_eq!(graph.telemetry().active, 0, "directive: {directive}");
        assert_eq!(
            graph.import_path_for_dir("pkg"),
            Err(GoImportPathReason::WorkspaceInvalid),
            "directive: {directive}"
        );
    }

    let mut graph = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[
            ("go.mod", "module example.com/root\n"),
            (
                "inactive/go.mod",
                "module example.com/inactive\nretract vbogus\n",
            ),
        ]),
    );

    assert!(!graph.telemetry().workspace_invalid);
    assert_eq!(graph.telemetry().active, 1);
    assert_eq!(
        graph.import_path_for_dir("pkg"),
        Ok("example.com/root/pkg".to_string())
    );
    assert_eq!(
        graph.import_path_for_dir("inactive/pkg"),
        Err(GoImportPathReason::Malformed)
    );
}

#[test]
fn workspace_replace_path_overrides_versioned_module_replace() {
    let root = "module example.com/root\nrequire original.example/a v1.0.0\nreplace original.example/a v1.0.0 => ./bad\n";
    let work = "go 1.22\nuse .\nreplace original.example/a => ./good\n";
    let mut graph = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[
            ("go.mod", root),
            ("go.work", work),
            ("good/go.mod", "module good.example/a\n"),
            ("bad/go.mod", "module bad.example/a\n"),
        ]),
    );

    assert!(!graph.telemetry().workspace_invalid);
    assert_eq!(graph.telemetry().replaces_parsed, 2);
    assert_eq!(graph.telemetry().replaces_applied, 1);
    assert_eq!(graph.provider_path("good"), Some("original.example/a"));
    assert_eq!(graph.provider_path("bad"), None);
    assert!(!graph.replacement_is_unproven("original.example/a"));
    assert!(!graph.replacement_dir_is_unproven("bad"));
    assert_eq!(
        graph.import_path_for_dir("good/pkg"),
        Ok("original.example/a/pkg".to_string())
    );
}

#[test]
fn versioned_module_replace_without_workspace_override_remains_unproven() {
    let root = "module example.com/root\nrequire original.example/a v1.0.0\nreplace original.example/a v1.0.0 => ./bad\n";
    let graph = GoModuleGraph::new(
        Path::new("/repo"),
        &snapshot(&[("go.mod", root), ("bad/go.mod", "module bad.example/a\n")]),
    );

    assert!(!graph.telemetry().workspace_invalid);
    assert_eq!(graph.telemetry().replaces_parsed, 1);
    assert_eq!(graph.telemetry().replaces_applied, 0);
    assert_eq!(graph.provider_path("bad"), None);
    assert!(graph.replacement_is_unproven("original.example/a"));
    assert!(graph.replacement_dir_is_unproven("bad"));
}
