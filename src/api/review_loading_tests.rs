//! #602 S4a facade controls over real repositories (REAL) and a few injected capture inputs
//! (SYNTHETIC, named where used). The parent test process never changes its working directory;
//! the cwd control runs in a child process, as in `review_context_tests.rs`.

use super::*;
use crate::api::{load_review_inputs, ReviewOptions};
use crate::languages::Language;
use std::path::Path;

const MAIN_JS: &str = "export const main = 1;\n";
const MAIN_DIFF: &str =
    r#"{"files":[{"file_path":"main.js","modify_type":"Modified","diff_lines":[]}]}"#;
/// The per-file source limit of the reader (`repo_loader::MAX_FILE_BYTES`).
const LIMIT: usize = 2 * 1024 * 1024;

fn write_all(root: &Path, files: &[(&str, &str)]) {
    for (name, text) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

/// REAL: files under `<tmp>/repo`, the named subset loaded as the review's diff files.
fn fixture(files: &[(&str, &str)], loaded: &[&str]) -> (tempfile::TempDir, ReviewInputs) {
    let dir = tempfile::tempdir().unwrap();
    write_all(&dir.path().join("repo"), files);
    let diff = serde_json::json!({"files": loaded.iter().map(|file| serde_json::json!({
        "file_path": file, "modify_type": "Modified", "diff_lines": []
    })).collect::<Vec<_>>()});
    let opts = ReviewOptions::new(dir.path().join("repo"));
    (dir, load_review_inputs(&opts, &diff.to_string()).unwrap())
}

fn loader(inputs: &ReviewInputs) -> ReviewContextLoader<'_> {
    ReviewContextLoader::new(inputs).unwrap()
}

fn outcome(attempt: &ContextReadAttempt) -> String {
    match &attempt.outcome {
        ContextReadOutcome::Loaded(file) => format!("Loaded({})", file.path),
        ContextReadOutcome::Skipped(skip) => format!("Skipped({skip:?})"),
        ContextReadOutcome::SourceCapacityBound => "Bound".to_string(),
    }
}

fn check(attempt: &ContextReadAttempt, bytes: usize, want: &str) {
    assert_eq!(
        (attempt.attempted_read_bytes, outcome(attempt)),
        (bytes as u64, want.to_string())
    );
}

fn loaded(attempt: ContextReadAttempt) -> ContextFile {
    match attempt.outcome {
        ContextReadOutcome::Loaded(file) => file,
        _ => panic!("expected a loaded file, got {}", outcome(&attempt)),
    }
}

fn snapshot(inputs: &ReviewInputs) -> String {
    let files: Vec<_> = inputs
        .files
        .iter()
        .map(|(path, file)| {
            let counts = (file.parse_error_count, file.parse_node_count);
            (path, &file.source, file.language, counts)
        })
        .collect();
    format!(
        "{files:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{}",
        inputs.sources,
        inputs.parse_quality,
        inputs.parse_warnings,
        inputs.load_warnings,
        serde_json::to_string(&inputs.scope_graph_inputs).unwrap()
    )
}

// A01
#[test]
fn capture_binds_the_relative_root_before_the_working_directory_changes() {
    const CHILD: &str = "PRISM_REVIEW_LOADING_ROOT_CHILD";
    const ORIGINAL: &str = "export const original = 1;\n";
    if let Some(alternate) = std::env::var_os(CHILD) {
        let inputs = load_review_inputs(&ReviewOptions::new("original"), MAIN_DIFF).unwrap();
        // A single selected test in its own process; the main runner never changes directory.
        std::env::set_current_dir(alternate).unwrap();
        let attempt = loader(&inputs).read("lib.js", 1000);
        check(&attempt, ORIGINAL.len(), "Loaded(lib.js)");
        assert_eq!(loaded(attempt).source, ORIGINAL);
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    write_all(
        dir.path(),
        &[
            ("original/main.js", MAIN_JS),
            ("original/lib.js", ORIGINAL),
            ("alternate/original/lib.js", "export const decoy = 1;\n"),
        ],
    );
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "api::review_loading::tests::capture_binds_the_relative_root_before_the_working_directory_changes",
            "--nocapture",
        ])
        .current_dir(dir.path())
        .env(CHILD, dir.path().join("alternate"))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("running 1 test"), "{stdout}");
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// A02
#[test]
fn public_scope_edits_and_disk_config_changes_do_not_move_the_captured_authority() {
    let lib = "export const lib = 1;\n";
    let files = [
        ("main.js", MAIN_JS),
        ("lib.js", lib),
        ("go.mod", "module example.com/app\n"),
        (
            "tsconfig.json",
            r#"{"compilerOptions":{"paths":{"@/*":["./*"]}}}"#,
        ),
        ("a.c", "int a(void) { return 0; }\n"),
        ("b.c", "int b(void) { return 0; }\n"),
    ];
    let (dir, mut inputs) = fixture(&files, &["main.js", "a.c"]);
    assert!(
        inputs
            .scope_graph_inputs
            .manifest_hashes
            .contains_key("c-census:review-incomplete"),
        "inadmissible precondition: the census gap entry must be present"
    );
    let published = serde_json::to_string(&inputs.scope_graph_inputs).unwrap();
    let captured = serde_json::to_string(loader(&inputs).captured_scope()).unwrap();
    assert_eq!(captured, published, "capture equals the published inputs");

    // Public edits and changed disk configuration after the load.
    let decoy = dir.path().join("decoy");
    write_all(&decoy, &[("lib.js", "export const decoy = 1;\n")]);
    let scope = &mut inputs.scope_graph_inputs;
    scope.repo_root = decoy;
    scope.all_file_paths.clear();
    scope.manifest_hashes.clear();
    scope.js_paths_snapshot.configs.clear();
    inputs.source_listing = Ok(std::sync::Arc::new(std::collections::BTreeSet::new()));
    write_all(
        &dir.path().join("repo"),
        &[
            ("go.mod", "module example.com/changed\n"),
            ("tsconfig.json", "{}"),
        ],
    );
    assert_ne!(
        serde_json::to_string(&inputs.scope_graph_inputs).unwrap(),
        published,
        "inadmissible precondition: the public inputs must now differ"
    );

    let loader = loader(&inputs);
    let after = serde_json::to_string(loader.captured_scope()).unwrap();
    assert_eq!(after, published, "the private capture is unchanged");
    assert!(loader.listing().contains("lib.js"));
    let attempt = loader.read("lib.js", 1000);
    check(&attempt, lib.len(), "Loaded(lib.js)");
    assert_eq!(loaded(attempt).source, lib);
    for original in ["main.js", "a.c"] {
        check(&loader.read(original, 1000), 0, "Skipped(OriginalFile)");
    }
}

// A03
#[test]
fn the_captured_listing_decides_membership_not_the_disk_after_capture() {
    let over = format!("var a = 1;{}", " ".repeat(LIMIT));
    let util = "export const util = 1;\n";
    let files = [
        ("main.js", MAIN_JS),
        ("lib.js", "export const lib = 1;\n"),
        ("util.js", util),
        ("big.js", over.as_str()),
    ];
    let (dir, inputs) = fixture(&files, &["main.js"]);
    let loader = loader(&inputs);
    assert!(
        loader.listing().contains("lib.js") && !loader.listing().contains("big.js"),
        "inadmissible precondition: lib.js listed, the oversized big.js not"
    );
    let root = dir.path().join("repo");
    write_all(
        &root,
        &[("big.js", "var a = 1;\n"), ("new.js", "var n = 1;\n")],
    );
    std::fs::remove_file(root.join("lib.js")).unwrap();
    check(&loader.read("new.js", 1000), 0, "Skipped(NotListed)");
    check(&loader.read("big.js", 1000), 0, "Skipped(NotListed)");
    let gone = loader.read("lib.js", 1000);
    assert_eq!(gone.attempted_read_bytes, 0);
    assert!(
        outcome(&gone).starts_with("Skipped(Unreadable { kind: NotFound"),
        "{}",
        outcome(&gone)
    );
    check(&loader.read("util.js", 1000), util.len(), "Loaded(util.js)");
}

// A05
#[test]
fn unavailable_authority_is_typed_and_never_changes_the_baseline_inputs() {
    let (dir, mut inputs) = fixture(&[("main.js", MAIN_JS)], &["main.js"]);
    assert!(ReviewContextLoader::new(&inputs).is_ok());
    let root = dir.path().join("repo");
    let canonical = observe_root(&root).unwrap();
    let (scope, listing) = (&inputs.scope_graph_inputs, &inputs.source_listing);
    assert!(capture(&root, Ok(canonical.clone()), scope, listing).is_ok());

    // SYNTHETIC injections of the three causes into the capture step.
    let broken: crate::repo_loader::SourceListing = Err(crate::repo_loader::ListingError {
        dir: String::new(),
        message: "scripted listing failure".to_string(),
    });
    let listing_error = capture(&root, Ok(canonical.clone()), scope, &broken).err();
    assert!(
        matches!(&listing_error, Some(ContextUnavailable::ListingUnavailable { message })
            if message.contains("scripted listing failure")),
        "{listing_error:?}"
    );
    let scripted = ContextUnavailable::RootUnavailable {
        message: "scripted root failure".to_string(),
    };
    let observed = capture(&root, Err(scripted.clone()), scope, listing).err();
    assert_eq!(observed, Some(scripted.clone()));
    let other = dir.path().join("other");
    std::fs::create_dir(&other).unwrap();
    let moved = capture(&root, Ok(observe_root(&other).unwrap()), scope, listing).err();
    assert_eq!(moved, Some(ContextUnavailable::RootChangedDuringCapture));

    // The loader reports whatever the capture recorded.
    for cause in [
        scripted,
        ContextUnavailable::RootChangedDuringCapture,
        ContextUnavailable::ListingUnavailable {
            message: "scripted".to_string(),
        },
    ] {
        inputs.context_capture = Err(cause.clone());
        assert_eq!(ReviewContextLoader::new(&inputs).err(), Some(cause));
    }

    // REAL: a missing root still loads the baseline, with no loading authority.
    let missing = dir.path().join("missing");
    let baseline = load_review_inputs(&ReviewOptions::new(missing), r#"{"files":[]}"#)
        .expect("inadmissible precondition: the baseline load of a missing root must succeed");
    assert!(baseline.files.is_empty() && baseline.load_warnings.is_empty());
    assert!(matches!(
        ReviewContextLoader::new(&baseline).err(),
        Some(ContextUnavailable::RootUnavailable { .. })
    ));
}

// A06
#[test]
fn language_support_and_exclusions_are_decided_before_any_read() {
    let files = [
        ("main.js", MAIN_JS),
        ("a.js", "export const a = 1;\n"),
        ("b.jsx", "export const b = <div />;\n"),
        ("c.ts", "export const c: number = 1;\n"),
        ("d.tsx", "export const d = <div />;\n"),
        ("e.py", "x = 1\n"),
        ("f.go", "package p\n\nfunc F() {}\n"),
        ("g.rs", "pub fn g() {}\n"),
        ("h.java", "class H {}\n"),
        ("h_test.go", "package p\n"),
        ("testdata/i.go", "package p\n"),
        ("pkg/testdata/j.go", "package p\n"),
    ];
    let (_dir, inputs) = fixture(&files, &["main.js"]);
    let loader = loader(&inputs);
    for (path, language) in [
        ("a.js", Language::JavaScript),
        ("b.jsx", Language::JavaScript),
        ("c.ts", Language::TypeScript),
        ("d.tsx", Language::Tsx),
        ("e.py", Language::Python),
        ("f.go", Language::Go),
    ] {
        assert!(
            loader.listing().contains(path),
            "inadmissible precondition: {path}"
        );
        let file = loaded(loader.read(path, 1000));
        assert_eq!((file.path.as_str(), file.parsed.language), (path, language));
        assert!(file.parse_quality.is_none(), "{path}");
    }
    for path in [
        "g.rs",
        "h.java",
        "h_test.go",
        "testdata/i.go",
        "pkg/testdata/j.go",
    ] {
        assert!(
            loader.listing().contains(path),
            "inadmissible precondition: {path}"
        );
        check(&loader.read(path, 1000), 0, "Skipped(Unsupported)");
    }
    check(&loader.read("zz.js", 1000), 0, "Skipped(NotListed)");
}

// A07
#[test]
fn a_changed_regular_source_is_read_fresh_while_the_baseline_stays_original() {
    let (dir, inputs) = fixture(
        &[("main.js", MAIN_JS), ("lib.js", "export const old = 1;\n")],
        &["main.js"],
    );
    let fresh = "export const fresh = 22;\n";
    write_all(
        &dir.path().join("repo"),
        &[("lib.js", fresh), ("main.js", "export const main = 2;\n")],
    );
    let loader = loader(&inputs);
    let attempt = loader.read("lib.js", 1000);
    check(&attempt, fresh.len(), "Loaded(lib.js)");
    let file = loaded(attempt);
    assert_eq!(
        (file.source.as_str(), file.parsed.source.as_str()),
        (fresh, fresh)
    );
    assert_eq!(inputs.sources["main.js"], MAIN_JS);
    check(&loader.read("main.js", 1000), 0, "Skipped(OriginalFile)");
}

// A09
#[cfg(unix)]
#[test]
fn replaced_links_are_refused_with_zero_source_bytes_and_ordinary_chains_load() {
    use std::os::unix::fs::symlink;
    let files = [
        ("main.js", MAIN_JS),
        ("src/a.js", "var a = 1;\n"),
        ("src/b.js", "var b = 1;\n"),
        ("lib/c.js", "var c = 1;\n"),
        ("d.js", "var d = 1;\n"),
        ("nested/deep/f.js", "var f = 1;\n"),
    ];
    let (dir, inputs) = fixture(&files, &["main.js"]);
    let (root, out) = (dir.path().join("repo"), dir.path().join("outside"));
    write_all(
        &out,
        &[("evil.js", "var e = 1;\n"), ("lib/c.js", "var e = 2;\n")],
    );
    let loader = loader(&inputs);
    check(
        &loader.read("nested/deep/f.js", 1000),
        11,
        "Loaded(nested/deep/f.js)",
    );

    std::fs::remove_file(root.join("src/a.js")).unwrap();
    symlink(root.join("src/b.js"), root.join("src/a.js")).unwrap();
    check(&loader.read("src/a.js", 1000), 0, "Skipped(UnsafePath)");
    std::fs::remove_file(root.join("d.js")).unwrap();
    symlink(out.join("evil.js"), root.join("d.js")).unwrap();
    check(&loader.read("d.js", 1000), 0, "Skipped(UnsafePath)");
    std::fs::remove_dir_all(root.join("lib")).unwrap();
    symlink(out.join("lib"), root.join("lib")).unwrap();
    check(&loader.read("lib/c.js", 1000), 0, "Skipped(UnsafePath)");
    check(&loader.read("src/b.js", 1000), 11, "Loaded(src/b.js)");
}

// A09, root mapping
#[cfg(unix)]
#[test]
fn a_root_symlink_is_accepted_at_capture_and_refused_once_retargeted() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let (real1, real2, link) = (
        dir.path().join("real1"),
        dir.path().join("real2"),
        dir.path().join("link"),
    );
    write_all(
        &real1,
        &[("main.js", MAIN_JS), ("lib.js", "var one = 1;\n")],
    );
    write_all(&real2, &[("lib.js", "var two = 2;\n")]);
    symlink(&real1, &link).unwrap();
    let inputs = load_review_inputs(&ReviewOptions::new(link.clone()), MAIN_DIFF).unwrap();
    let loader = loader(&inputs);
    let attempt = loader.read("lib.js", 1000);
    check(&attempt, 13, "Loaded(lib.js)");
    assert_eq!(loaded(attempt).source, "var one = 1;\n");

    std::fs::remove_file(&link).unwrap();
    symlink(&real2, &link).unwrap();
    check(&loader.read("lib.js", 1000), 0, "Skipped(UnsafePath)");
}

// C01
#[test]
fn reads_leave_the_original_inputs_untouched_and_attempts_need_no_ledger() {
    let lib = "export const lib = 1;\n";
    let files = [
        ("main.js", MAIN_JS),
        ("lib.js", lib),
        ("bad.js", "var a = 1;\n"),
    ];
    let (dir, inputs) = fixture(&files, &["main.js"]);
    std::fs::write(dir.path().join("repo/bad.js"), [0xffu8, 0xfe, 0x00]).unwrap();
    let before = snapshot(&inputs);
    let loader = loader(&inputs);
    let mut attempted = 0;
    let mut staged = Vec::new();
    for (path, remaining, bytes, want) in [
        ("lib.js", 1000, lib.len(), "Loaded(lib.js)"),
        ("bad.js", 1000, 3, "Skipped(NotUtf8)"),
        ("lib.js", 5, 6, "Bound"),
        ("zz.js", 1000, 0, "Skipped(NotListed)"),
        ("main.js", 1000, 0, "Skipped(OriginalFile)"),
    ] {
        let attempt = loader.read(path, remaining);
        check(&attempt, bytes, want);
        attempted += attempt.attempted_read_bytes;
        staged.push(attempt);
    }
    // Dropping the staged payloads undoes nothing: there is no ledger in this API.
    drop(staged);
    assert_eq!(attempted, (lib.len() + 3 + 6) as u64);
    assert_eq!(snapshot(&inputs), before);
}
