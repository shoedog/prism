use super::*;
use crate::repo_loader::{load_repo, scope_graph_build_inputs, SkipReason};
use std::cell::RefCell;
use std::os::unix::fs::{symlink, PermissionsExt};

// Synchronous, one-shot barriers are isolated per test thread.
type Hook = Box<dyn FnOnce()>;
thread_local! { static HOOK: RefCell<Option<(&'static str, Hook)>> = RefCell::new(None); }
pub(super) fn barrier(stage: &str) {
    let hook = HOOK.with(|h| {
        if h.borrow().as_ref().is_some_and(|(s, _)| *s == stage) {
            h.borrow_mut().take().map(|(_, f)| f)
        } else {
            None
        }
    });
    if let Some(hook) = hook {
        hook();
    }
}
pub(crate) fn on(stage: &'static str, f: impl FnOnce() + 'static) {
    HOOK.with(|h| *h.borrow_mut() = Some((stage, Box::new(f))));
}
fn source(root: &Path, rel: &str, text: &str) {
    fs::create_dir_all(root.join(rel).parent().unwrap()).unwrap();
    fs::write(root.join(rel), text).unwrap();
}
fn mode(root: &Path, rel: &str, mode: u32) {
    fs::set_permissions(root.join(rel), fs::Permissions::from_mode(mode)).unwrap();
}
fn fixture() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    source(d.path(), "a.py", "def old():\n    return 1\n");
    d
}
fn assert_probe(repo: &LoadedRepo, stale: bool, label: &str) {
    let report = FreshnessProbe::from_loaded_repo(repo).check();
    assert_eq!(report.stale, stale, "{label}: {report:?}");
}
const CONFIG: &str = r#"{"compilerOptions":{"baseUrl":".","paths":{"@/*":["*"]}}}"#;
const IMPORT: &str = "import {x} from '@/missing'; export const y = x;";

#[test]
fn freshness_before_capture_and_after_capture() {
    let d = fixture();
    let b = "def fresh():\n    return 2\n";
    source(d.path(), "a.py", b);
    let repo = load_repo(d.path()).unwrap();
    assert_probe(&repo, false, "edit before read is clean");
    source(d.path(), "a.py", b);
    assert_probe(&repo, false, "touch-only must be clean");
    source(d.path(), "a.py", &b.replace('2', "3"));
    assert_probe(&repo, true, "same-size edit binds loaded hash");
    source(d.path(), "a.py", "def edited():\n    pass\n");
    assert_probe(&repo, true, "post-read mismatch");
    source(d.path(), "a.py", b);
    let p = d.path().join("a.py");
    on("source-read", move || fs::write(p, b).unwrap());
    assert_probe(&repo, false, "one re-check certifies a touch");
    let p = d.path().join("a.py");
    on("source-read", move || {
        fs::write(p, "def during_validation():\n    pass\n").unwrap()
    });
    assert_probe(&repo, true, "validation race");
    let probe = FreshnessProbe::from_loaded_repo(&load_repo(d.path()).unwrap());
    source(d.path(), "a.py", "def after_validation():\n    pass\n");
    assert!(probe.check().stale, "service recheck");
    let repo = load_repo(d.path()).unwrap();
    let root = d.path().to_path_buf();
    on("directories-validated", move || {
        source(&root, "new.py", "x=1")
    });
    assert_probe(&repo, true, "directory membership race");
}

#[test]
fn freshness_manifest_resolver_inputs() {
    for (path, old, new) in [
        ("tsconfig.json", CONFIG, "{}"),
        ("Cargo.toml", "[package]\nname='a'", "[package]\nname='b'"),
        ("node_modules/pkg/package.json", "{}", "{\"name\":\"b\"}"),
        (
            "node_modules/types.d.ts",
            "declare var a;",
            "declare var b;",
        ),
    ] {
        let d = fixture();
        source(d.path(), "tsconfig.json", CONFIG);
        source(d.path(), "main.ts", IMPORT);
        source(d.path(), path, old);
        let repo = load_repo(d.path()).unwrap();
        source(d.path(), path, new);
        assert_probe(&repo, true, path);
    }
    for replacement in ["create", "delete", "directory", "symlink", "external"] {
        let outer = tempfile::tempdir().unwrap();
        let root = outer.path().join("repo");
        fs::create_dir(&root).unwrap();
        source(&root, "tsconfig.json", CONFIG);
        source(&root, "main.ts", IMPORT);
        if replacement != "create" {
            source(&root, "missing.ts", "export const x=1;");
        }
        let repo = load_repo(&root).unwrap();
        match replacement {
            "create" => source(&root, "missing.ts", "export const x=2;"),
            "external" => source(
                outer.path(),
                "node_modules/@types/new/index.d.ts",
                "declare var a;",
            ),
            other => {
                fs::remove_file(root.join("missing.ts")).unwrap();
                if other == "directory" {
                    fs::create_dir(root.join("missing.ts")).unwrap();
                }
                if other == "symlink" {
                    symlink("main.ts", root.join("missing.ts")).unwrap();
                }
            }
        }
        assert_probe(&repo, true, replacement);
    }
    let d = fixture();
    source(d.path(), "Cargo.toml", "[package]\nname='a'\n");
    let repo = load_repo(d.path()).unwrap();
    fs::remove_file(d.path().join("Cargo.toml")).unwrap();
    symlink("a.py", d.path().join("Cargo.toml")).unwrap();
    assert_probe(&repo, true, "manifest symlink refusal");
    assert_probe(&load_repo(d.path()).unwrap(), false, "stable refusal");
}

#[test]
fn freshness_loader_skip_reasons() {
    let d = fixture();
    for (p, text) in [
        ("notes.txt", "hello"),
        ("target/x.py", "x=1"),
        ("testdata/x.go", "package x"),
        (".hidden/x.py", "x=1"),
        ("bad.py", "}\n"),
        ("locked.py", "x=1"),
    ] {
        source(d.path(), p, text);
    }
    fs::write(d.path().join("invalid.py"), [255, 254]).unwrap();
    fs::write(d.path().join("large.py"), vec![b' '; 2 * 1024 * 1024 + 1]).unwrap();
    symlink("a.py", d.path().join("linked.py")).unwrap();
    mode(d.path(), "locked.py", 0);
    let repo = load_repo(d.path()).unwrap();
    for reason in [
        SkipReason::Unsupported,
        SkipReason::Ignored,
        SkipReason::GoTestdata,
        SkipReason::Symlink,
        SkipReason::Hidden,
        SkipReason::TooLarge {
            bytes: 2 * 1024 * 1024 + 1,
        },
        SkipReason::Unreadable,
        SkipReason::NotUtf8,
        SkipReason::ParseFailed,
    ] {
        assert!(
            repo.skipped.iter().any(|s| s.reason == reason),
            "fixture must exercise {reason:?}"
        );
    }
    assert_probe(&repo, false, "stable skips");
    mode(d.path(), "locked.py", 0o600);
}

fn unreadable_subdirectory(edit: bool) {
    let d = fixture();
    fs::create_dir(d.path().join("locked")).unwrap();
    mode(d.path(), "locked", 0);
    assert!(fs::read_dir(d.path().join("locked")).is_err());
    let repo = load_repo(d.path()).unwrap();
    if edit {
        source(d.path(), "new.py", "x=1\n");
    }
    assert_probe(&repo, edit, "unreadable sibling membership");
    mode(d.path(), "locked", 0o700);
}
#[test]
fn freshness_unreadable_subdirectory_edit() {
    unreadable_subdirectory(true);
}
#[test]
fn freshness_unreadable_subdirectory_unchanged() {
    unreadable_subdirectory(false);
}

#[test]
fn freshness_consumed_input_unavailable() {
    let d = fixture();
    source(d.path(), "tsconfig.json", CONFIG);
    let repo = load_repo(d.path()).unwrap();
    for path in ["a.py", "tsconfig.json"] {
        mode(d.path(), path, 0);
        let probe = FreshnessProbe::from_loaded_repo(&repo);
        assert!(probe.check().stale, "unavailable input: {path}");
        mode(d.path(), path, 0o600);
        assert!(probe.check().stale, "sticky evidence: {path}");
    }
}

#[test]
fn freshness_stable_incomplete_and_unrelated_churn() {
    let d = fixture();
    source(d.path(), "tsconfig.json", CONFIG);
    source(d.path(), "target/unrelated.txt", "ignored");
    let repo = load_repo(d.path()).unwrap();
    source(d.path(), "target/unrelated.txt", "churn");
    let current = scope_graph_build_inputs(d.path(), &repo.files);
    assert_eq!(
        repo.manifest_hashes, current.manifest_hashes,
        "equal topology"
    );
    let probe = FreshnessProbe::from_loaded_repo(&repo);
    assert!(!probe.check().stale);
    let count = probe.tracked_len();
    source(d.path(), "target/unrelated.txt", "more content");
    assert!(!probe.check().stale);
    assert_eq!(probe.tracked_len(), count);
    source(d.path(), "build/new.txt", "new skipped directory");
    assert_ne!(
        repo.manifest_hashes,
        scope_graph_build_inputs(d.path(), &repo.files).manifest_hashes
    );
    assert_probe(&repo, true, "skipped directory topology");
}

#[test]
fn freshness_budget_and_completeness() {
    let d = fixture();
    source(d.path(), "tsconfig.json", CONFIG);
    source(d.path(), "node_modules/huge.d.ts", "");
    // Sparse metadata-only budget stop: the scanner rejects before allocating/reading.
    fs::File::options()
        .write(true)
        .open(d.path().join("node_modules/huge.d.ts"))
        .unwrap()
        .set_len(1024 * 1024 * 1024 + 1)
        .unwrap();
    let repo = load_repo(d.path()).unwrap();
    let inputs = repo.scope_graph_inputs.as_ref().unwrap();
    assert!(!inputs.js_paths_snapshot.complete);
    assert_probe(&repo, false, "unchanged budget stop is clean");
    let p = d.path().join("node_modules/huge.d.ts");
    on("inputs-between", move || fs::remove_file(p).unwrap());
    assert_probe(&repo, true, "completeness changed");
}
