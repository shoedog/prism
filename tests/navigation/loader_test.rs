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
