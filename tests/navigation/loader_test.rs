use prism::cpg_cache::compute_topology_key;
use prism::repo_loader::{load_repo, SkipReason};

#[test]
fn loads_supported_files_and_records_skips() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("a.py"), "def f():\n    return 1\n").unwrap();
    std::fs::create_dir(root.join("target")).unwrap();
    std::fs::write(root.join("target/b.py"), "def g(): pass\n").unwrap(); // built dir -> skipped
    std::fs::write(root.join("notes.md"), "# hi\n").unwrap(); // unsupported -> skipped

    let repo = load_repo(root).unwrap();
    assert!(repo.files.contains_key("a.py"));
    assert!(!repo.files.contains_key("target/b.py"));
    assert!(repo.file_hashes.contains_key("a.py"));
    assert!(repo
        .skipped
        .iter()
        .any(|s| s.path == "notes.md" && s.reason == SkipReason::Unsupported));
    assert!(repo
        .skipped
        .iter()
        .any(|s| s.path.starts_with("target/") && s.reason == SkipReason::Ignored));
}

#[test]
fn missing_root_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing");

    assert!(load_repo(&missing).is_err());
}

#[test]
fn severe_parse_errors_are_recorded_as_parse_failed() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("bad.py"), "}\n").unwrap();

    let repo = load_repo(root).unwrap();
    assert!(!repo.files.contains_key("bad.py"));
    assert!(!repo.file_hashes.contains_key("bad.py"));
    assert!(repo
        .skipped
        .iter()
        .any(|s| s.path == "bad.py" && s.reason == SkipReason::ParseFailed));
}

#[cfg(unix)]
#[test]
fn built_in_names_take_precedence_over_symlinks() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let linked = root.join("linked");
    std::fs::create_dir(&linked).unwrap();
    std::os::unix::fs::symlink(&linked, root.join("target")).unwrap();

    let repo = load_repo(root).unwrap();
    assert!(repo
        .skipped
        .iter()
        .any(|s| s.path.starts_with("target/") && s.reason == SkipReason::Ignored));
    assert!(!repo
        .skipped
        .iter()
        .any(|s| s.path == "target" && s.reason == SkipReason::Symlink));
}

#[test]
fn go_testdata_files_are_skipped_but_other_languages_are_loaded() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("pkg/testdata/helpers")).unwrap();
    std::fs::write(
        root.join("pkg/testdata/helpers/impl.go"),
        "package helpers\ntype Impl struct{}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("pkg/testdata/helpers/helper.py"),
        "def helper():\n    return 1\n",
    )
    .unwrap();

    let repo = load_repo(root).unwrap();

    assert!(!repo.files.contains_key("pkg/testdata/helpers/impl.go"));
    assert!(repo.files.contains_key("pkg/testdata/helpers/helper.py"));
    let skip = repo
        .skipped
        .iter()
        .find(|skip| skip.path == "pkg/testdata/helpers/impl.go")
        .expect("Go testdata skip is recorded");
    assert_eq!(skip.reason, SkipReason::GoTestdata);
}

#[test]
fn go_work_add_and_edit_change_manifest_topology() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("main.go"), "package main\n").unwrap();

    let absent = load_repo(root).unwrap();
    std::fs::write(root.join("go.work"), "go 1.22\nuse .\n").unwrap();
    let added = load_repo(root).unwrap();
    std::fs::write(root.join("go.work"), "go 1.23\nuse .\n").unwrap();
    let edited = load_repo(root).unwrap();

    assert!(!absent.manifest_hashes.contains_key("go.work"));
    assert!(added.manifest_hashes.contains_key("go.work"));
    let absent_key = compute_topology_key(&absent.file_hashes, &absent.manifest_hashes);
    let added_key = compute_topology_key(&added.file_hashes, &added.manifest_hashes);
    let edited_key = compute_topology_key(&edited.file_hashes, &edited.manifest_hashes);
    assert_ne!(absent_key, added_key);
    assert_ne!(added_key, edited_key);
}

#[cfg(unix)]
#[test]
fn symlinked_go_manifests_record_kind_without_hashing_target_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("manifest-target"), "module example.com/root\n").unwrap();
    std::os::unix::fs::symlink("manifest-target", root.join("go.mod")).unwrap();
    std::os::unix::fs::symlink("manifest-target", root.join("go.work")).unwrap();

    let before = load_repo(root).unwrap();
    std::fs::write(root.join("manifest-target"), "module example.com/changed\n").unwrap();
    let after = load_repo(root).unwrap();

    assert_eq!(before.manifest_hashes["go.mod"], "symlink_refused");
    assert_eq!(before.manifest_hashes["go.work"], "symlink_refused");
    assert_eq!(
        compute_topology_key(&before.file_hashes, &before.manifest_hashes),
        compute_topology_key(&after.file_hashes, &after.manifest_hashes)
    );
}
