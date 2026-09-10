//! Exercise the actual build script in disposable tracked, untracked and gitless roots.
#[allow(dead_code)]
mod build_script {
    include!("../build.rs");

    #[test]
    fn emit_identity_child() {
        if std::env::var_os("PRISM_BUILD_IDENTITY_CHILD").is_some() {
            main();
        }
    }
}

use std::path::Path;
use std::process::Command;

const VENDOR: &str = "vendor/tree-sitter-typescript";

fn write(root: &Path, rel: &str, bytes: &[u8]) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "src/lib.rs", b"pub fn source() {}\n");
    write(
        dir.path(),
        "Cargo.toml",
        b"[package]\nname=\"fixture\"\nversion=\"1.0.0\"\n",
    );
    write(
        dir.path(),
        "Cargo.lock",
        b"[[package]]\nname = \"tree-sitter-typescript\"\nversion = \"0.23.2\"\n",
    );
    write(dir.path(), "build.rs", b"fn main() {}\n");
    dir
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=Build Identity Test",
            "-c",
            "user.email=build-identity@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn commit(root: &Path) {
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "fixture"]);
}

fn output(root: &Path) -> std::process::Output {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "build_script::emit_identity_child",
            "--nocapture",
        ])
        .env("PRISM_BUILD_IDENTITY_CHILD", "1")
        .env("CARGO_MANIFEST_DIR", root)
        .env("CARGO_PKG_VERSION", "1.0.0")
        .output()
        .unwrap()
}

fn identity(root: &Path) -> String {
    let out = output(root);
    assert!(
        out.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

fn value<'a>(output: &'a str, name: &str) -> &'a str {
    output
        .lines()
        .find_map(|line| line.strip_prefix(&format!("cargo:rustc-env={name}=")))
        .unwrap()
}

fn assert_changed(before: &str, after: &str) {
    for key in ["GRAMMAR_FINGERPRINT", "PRISM_CACHE_BUILD_IDENTITY"] {
        assert_ne!(
            value(before, key),
            value(after, key),
            "{key} ignored changed vendor bytes"
        );
    }
}

fn assert_restored(before: &str, after: &str) {
    for key in ["GRAMMAR_FINGERPRINT", "PRISM_CACHE_BUILD_IDENTITY"] {
        assert_eq!(
            value(before, key),
            value(after, key),
            "{key} did not restore"
        );
    }
}

#[test]
fn tracked_vendor_mutation_restore_add_delete_changes_both_identities() {
    let dir = fixture();
    let root = dir.path();
    write(
        root,
        &format!("{VENDOR}/typescript/src/parser.c"),
        b"original parser",
    );
    git(root, &["init", "-q"]);
    commit(root);
    let baseline = identity(root);
    assert_eq!(value(&baseline, "PRISM_BINARY_INPUT_DIRTY"), "0");
    for rel in [
        "typescript/src/parser.c",
        "tsx/src/parser.c",
        "typescript/src/tree_sitter/parser.h",
        "bindings/rust/build.rs",
        "bindings/rust/lib.rs",
        "common/define-grammar.js",
        "typescript/src/grammar.json",
        "typescript/src/node-types.json",
        "Cargo.toml",
        "LICENSE",
        "baseline.json",
    ] {
        let path = format!("{VENDOR}/{rel}");
        let prior = std::fs::read(root.join(&path)).ok();
        write(root, &path, b"same version changed bytes");
        let changed = identity(root);
        assert_changed(&baseline, &changed);
        assert_eq!(value(&changed, "PRISM_BINARY_INPUT_DIRTY"), "1", "{rel}");
        assert!(value(&changed, "GIT_SHA").ends_with("-dirty"), "{rel}");
        match prior {
            Some(bytes) => write(root, &path, &bytes),
            None => std::fs::remove_file(root.join(&path)).unwrap(),
        }
        assert_restored(&baseline, &identity(root));
    }
    std::fs::remove_file(root.join(format!("{VENDOR}/typescript/src/parser.c"))).unwrap();
    let deleted = identity(root);
    assert_changed(&baseline, &deleted);
    assert_eq!(value(&deleted, "PRISM_BINARY_INPUT_DIRTY"), "1");
}

#[test]
fn untracked_and_ignored_vendor_bytes_are_inputs_and_dirty() {
    let dir = fixture();
    let root = dir.path();
    write(
        root,
        ".gitignore",
        b"vendor/tree-sitter-typescript/generated/\n",
    );
    git(root, &["init", "-q"]);
    commit(root);
    let baseline = identity(root);
    for rel in ["typescript/src/parser.c", "generated/parser.c"] {
        let path = format!("{VENDOR}/{rel}");
        write(root, &path, b"untracked bytes");
        let added = identity(root);
        assert_changed(&baseline, &added);
        assert_eq!(value(&added, "PRISM_BINARY_INPUT_DIRTY"), "1");
        assert!(value(&added, "GIT_SHA").ends_with("-dirty"));
        write(root, &path, b"changed untracked bytes");
        assert_changed(&added, &identity(root));
        std::fs::remove_file(root.join(&path)).unwrap();
        assert_restored(&baseline, &identity(root));
    }
}

#[test]
fn gitless_vendor_mutation_restore_add_delete_is_content_bound() {
    let dir = fixture();
    let root = dir.path();
    let baseline = identity(root);
    assert_eq!(value(&baseline, "GIT_SHA"), "unknown");
    let rel = format!("{VENDOR}/tsx/src/parser.c");
    write(root, &rel, b"original");
    let added = identity(root);
    assert_changed(&baseline, &added);
    write(root, &rel, b"modified");
    assert_changed(&added, &identity(root));
    write(root, &rel, b"original");
    assert_restored(&added, &identity(root));
    std::fs::remove_file(root.join(&rel)).unwrap();
    assert_restored(&baseline, &identity(root));
}

#[test]
fn vendor_root_is_watched_even_before_creation_and_docs_remain_outside_inputs() {
    let dir = fixture();
    let root = dir.path();
    let baseline = identity(root);
    assert!(baseline
        .lines()
        .any(|line| line == format!("cargo:rerun-if-changed={}/{VENDOR}", root.display())));
    write(root, "docs/note.md", b"not a binary input");
    assert_restored(&baseline, &identity(root));
    let mut expected = 0xcbf29ce484222325u64;
    for byte in b"tree-sitter-typescript@0.23.2" {
        expected ^= u64::from(*byte);
        expected = expected.wrapping_mul(0x100000001b3);
    }
    assert_eq!(
        value(&baseline, "GRAMMAR_FINGERPRINT"),
        format!("{expected:016x}")
    );
}

#[cfg(unix)]
#[test]
fn vendor_symlinks_refuse_incomplete_or_external_identity() {
    let dir = fixture();
    let root = dir.path();
    std::fs::create_dir_all(root.join(VENDOR)).unwrap();
    std::os::unix::fs::symlink(root.join("src"), root.join(VENDOR).join("linked-inputs")).unwrap();
    let out = output(root);
    assert!(!out.status.success(), "vendor symlink silently accepted");
    assert!(String::from_utf8_lossy(&out.stderr)
        .contains("vendored grammar inputs must be regular files or directories"));
}

#[cfg(unix)]
#[test]
fn symlinked_vendor_parent_cannot_import_an_external_tree() {
    let dir = fixture();
    let outside = tempfile::tempdir().unwrap();
    write(
        outside.path(),
        "tree-sitter-typescript/parser.c",
        b"external parser",
    );
    std::os::unix::fs::symlink(outside.path(), dir.path().join("vendor")).unwrap();
    let out = output(dir.path());
    assert!(
        !out.status.success(),
        "symlinked vendor parent silently accepted"
    );
    assert!(String::from_utf8_lossy(&out.stderr)
        .contains("vendored grammar parent must be a real directory"));
}

#[cfg(unix)]
#[test]
fn literal_backslash_vendor_names_do_not_alias_directory_separators() {
    let dir = fixture();
    let root = dir.path();
    let slash = format!("{VENDOR}/common/a/b.h");
    let backslash = format!("{VENDOR}/common/a\\b.h");
    write(root, &slash, b"slash header");
    write(root, &backslash, b"backslash header");
    let baseline = identity(root);
    for (path, original) in [
        (&backslash, b"backslash header".as_slice()),
        (&slash, b"slash header".as_slice()),
    ] {
        write(root, path, b"changed header");
        assert_changed(&baseline, &identity(root));
        write(root, path, original);
        assert_restored(&baseline, &identity(root));
        std::fs::remove_file(root.join(path)).unwrap();
        assert_changed(&baseline, &identity(root));
        write(root, path, original);
        assert_restored(&baseline, &identity(root));
    }
}
