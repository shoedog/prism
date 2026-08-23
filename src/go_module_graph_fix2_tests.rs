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
