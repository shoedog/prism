use super::{load_ast_sources, SkipReason, SkippedFile, SEVERE_PARSE_ERROR_RATE};
use std::fs;
use std::path::Path;

fn write(root: &Path, path: &str, source: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, source).unwrap();
}

fn skip_paths(skipped: &[SkippedFile]) -> Vec<(&str, SkipReason)> {
    skipped
        .iter()
        .map(|skip| (skip.path.as_str(), skip.reason.clone()))
        .collect()
}

#[test]
fn source_selection_prunes_excluded_directories_and_filters_walk_skips() {
    let repo = tempfile::tempdir().unwrap();
    let root = repo.path();
    for path in ["pkg/x.py", "pkg/generated/y.py", ".hidden/z.py"] {
        write(root, path, "pass\n");
    }
    let includes_python = |path: &str| path.ends_with(".py");
    let (files, skipped) =
        load_ast_sources(root, &includes_python, &["generated"], |_| 0.0).unwrap();
    assert_eq!(files.keys().collect::<Vec<_>>(), ["pkg/x.py"]);
    assert!(skipped.is_empty());

    let (files, skipped) = load_ast_sources(root, &|_| true, &[], |_| 0.0).unwrap();
    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        ["pkg/generated/y.py", "pkg/x.py"]
    );
    assert_eq!(skip_paths(&skipped), [(".hidden/", SkipReason::Hidden)]);
}

#[test]
fn source_load_propagates_a_root_io_error() {
    let repo = tempfile::tempdir().unwrap();
    write(repo.path(), "x.py", "pass\n");
    let result = load_ast_sources(&repo.path().join("x.py"), &|_| true, &[], |_| 0.0);
    let Err(error) = result else {
        panic!("a file cannot be the repository root");
    };
    assert!(error.to_string().contains("failed to read repository root"));
    let (files, skipped) = load_ast_sources(repo.path(), &|_| true, &[], |_| 0.0).unwrap();
    assert_eq!(files.keys().collect::<Vec<_>>(), ["x.py"]);
    assert!(skipped.is_empty());
}

#[test]
fn selected_sources_use_the_supplied_parse_cutoff() {
    let repo = tempfile::tempdir().unwrap();
    let root = repo.path();
    write(root, "boundary.py", "boundary = True\n");
    write(root, "above.py", "boundary = False\n");
    let (files, skipped) = load_ast_sources(root, &|_| true, &[], |parsed| {
        if parsed.source.contains("False") {
            SEVERE_PARSE_ERROR_RATE + 0.01
        } else {
            SEVERE_PARSE_ERROR_RATE
        }
    })
    .unwrap();
    assert_eq!(&*files["boundary.py"].source, "boundary = True\n");
    assert_eq!(files.len(), 1);
    assert_eq!(
        skip_paths(&skipped),
        [("above.py", SkipReason::ParseFailed)]
    );
}

#[cfg(unix)]
#[test]
fn selected_sources_do_not_alias_literal_backslashes_to_separators() {
    let repo = tempfile::tempdir().unwrap();
    let root = repo.path();
    write(root, "pkg/x.py", "pass\n");
    write(root, r"pkg\x.py", "pass\n");
    let (files, skipped) = load_ast_sources(root, &|p| p.ends_with(".py"), &[], |_| 0.0).unwrap();
    assert_eq!(files.keys().collect::<Vec<_>>(), ["pkg/x.py"]);
    assert_eq!(
        skip_paths(&skipped),
        [(r"pkg\x.py", SkipReason::UnrepresentablePath)]
    );
}

#[cfg(unix)]
#[test]
fn source_key_uses_lossy_text_only_for_non_utf8_refusals() {
    use super::source_files::source_key;
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let root = Path::new("repo");
    let invalid = OsStr::from_bytes(b"invalid-\xff.py");
    let replacement = "invalid-\u{fffd}.py";
    assert_eq!(
        source_key(root, &root.join(invalid)),
        Err(replacement.into())
    );
    assert_eq!(
        source_key(root, &root.join(replacement)),
        Ok(replacement.into())
    );
}

#[cfg(unix)]
#[test]
fn walk_refuses_manifest_and_source_symlinks() {
    use crate::manifest_snapshot::ManifestSnapshotEntry;
    use std::os::unix::fs::symlink;

    let repo = tempfile::tempdir().unwrap();
    let root = repo.path();
    write(root, "real/x.py", "pass\n");
    write(root, "real/go.mod", "module example.test/real\n");
    symlink("real/x.py", root.join("link.py")).unwrap();
    symlink("real/go.mod", root.join("go.mod")).unwrap();
    symlink("real", root.join("linked")).unwrap();

    let loaded = super::load_repo(root).unwrap();
    assert_eq!(loaded.files.keys().collect::<Vec<_>>(), ["real/x.py"]);
    assert!(matches!(
        loaded.manifest_snapshot.get("real/go.mod"),
        Some(ManifestSnapshotEntry::Regular { bytes, .. }) if bytes == b"module example.test/real\n"
    ));
    assert_eq!(
        loaded.manifest_snapshot.get("go.mod"),
        Some(&ManifestSnapshotEntry::SymlinkRefused)
    );
    let mut paths = skip_paths(&loaded.skipped);
    paths.sort_by_key(|(path, _)| *path);
    assert_eq!(
        paths,
        [
            ("go.mod", SkipReason::Symlink),
            ("link.py", SkipReason::Symlink),
            ("linked", SkipReason::Symlink),
            ("real/go.mod", SkipReason::Unsupported),
        ]
    );
}

#[test]
fn walk_excludes_go_testdata_but_keeps_other_sources_in_that_directory() {
    let repo = tempfile::tempdir().unwrap();
    let root = repo.path();
    write(root, "testdata/x.go", "package fixture\n");
    write(root, "testdata/x.py", "pass\n");
    write(root, "fixtures/x.go", "package fixture\n");
    let (files, skipped) = load_ast_sources(root, &|_| true, &[], |_| 0.0).unwrap();
    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        ["fixtures/x.go", "testdata/x.py"]
    );
    assert_eq!(
        skip_paths(&skipped),
        [("testdata/x.go", SkipReason::GoTestdata)]
    );
}

#[cfg(unix)]
#[test]
fn walk_refuses_unreadable_child_directories_manifests_and_sources() {
    use std::io::ErrorKind;
    use std::os::unix::fs::PermissionsExt;

    let repo = tempfile::tempdir().unwrap();
    let root = repo.path();
    write(root, "locked/hidden.py", "pass\n");
    write(root, "go.mod", "module example.test/locked\n");
    write(root, "locked.py", "pass\n");
    write(root, "open.py", "pass\n");
    let locked = ["locked", "go.mod", "locked.py"].map(|path| root.join(path));
    for path in &locked {
        fs::set_permissions(path, fs::Permissions::from_mode(0)).unwrap();
    }
    let directory_error = fs::read_dir(&locked[0]).err().map(|error| error.kind());
    let file_errors = [1, 2].map(|i| fs::read(&locked[i]).err().map(|error| error.kind()));
    let result = load_ast_sources(root, &|_| true, &[], |_| 0.0);
    for (path, mode) in locked.iter().zip([0o700, 0o600, 0o600]) {
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }
    if directory_error.is_none() && file_errors == [None, None] {
        eprintln!("permission refusal requires a user subject to fixture mode bits");
        return;
    }
    assert_eq!(directory_error, Some(ErrorKind::PermissionDenied));
    assert_eq!(file_errors, [Some(ErrorKind::PermissionDenied); 2]);

    let (files, skipped) = result.unwrap();
    assert_eq!(files.keys().collect::<Vec<_>>(), ["open.py"]);
    let mut paths = skip_paths(&skipped);
    paths.sort_by_key(|(path, _)| *path);
    assert_eq!(
        paths,
        [
            ("go.mod", SkipReason::Unreadable),
            ("locked.py", SkipReason::Unreadable),
            ("locked/", SkipReason::Unreadable),
        ]
    );
}

#[cfg(unix)]
#[test]
fn walk_records_a_selected_file_that_disappears_before_metadata() {
    use std::sync::atomic::{AtomicBool, Ordering};

    let repo = tempfile::tempdir().unwrap();
    let root = repo.path();
    write(root, "removed.py", "pass\n");
    write(root, "present.py", "pass\n");
    let removed = AtomicBool::new(false);
    let include = |path: &str| {
        if path == "removed.py" && !removed.swap(true, Ordering::Relaxed) {
            fs::remove_file(root.join(path)).unwrap();
        }
        true
    };
    let (files, skipped) = load_ast_sources(root, &include, &[], |_| 0.0).unwrap();
    assert!(removed.into_inner());
    assert_eq!(files.keys().collect::<Vec<_>>(), ["present.py"]);
    assert_eq!(
        skip_paths(&skipped),
        [("removed.py", SkipReason::Unreadable)]
    );
}
