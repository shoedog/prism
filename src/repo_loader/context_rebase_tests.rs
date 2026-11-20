//! #602 S4b1 rebase controls. Every repository is REAL: written to a temporary directory,
//! loaded by `load_repo`, captured by `scope_graph_build_inputs`; no cwd change. The only
//! SYNTHETIC steps are labeled where they occur (an injected `c-census:` manifest entry, a
//! hand-set skipped-testdata count, probes added through the snapshot's own methods).

use super::context_rebase::rebase_scope_inputs;
use super::*;
use crate::resolution::js_ts::paths::Resolver;

const ALIAS_FILES: [(&str, &str); 5] = [
    (
        "tsconfig.json",
        r#"{"compilerOptions":{"moduleResolution":"node","paths":{"@lib":["lib/barrel.ts"]}}}"#,
    ),
    ("app.ts", "export function run(){return 1;}"),
    (
        "ctx.ts",
        "import {real} from '@lib'; export function caller(){real();}",
    ),
    (
        "lib/barrel.ts",
        "export {real} from './leaf'; export * from './missing';",
    ),
    ("lib/leaf.ts", "export function real(){return 1;}"),
];
const CONTEXT: [&str; 3] = ["ctx.ts", "lib/barrel.ts", "lib/leaf.ts"];

fn write_repo(files: &[(&str, &str)]) -> tempfile::TempDir {
    let d = tempfile::TempDir::new().unwrap();
    for (name, text) in files {
        let p = d.path().join(name);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    d
}

fn without(all: &BTreeMap<String, ParsedFile>, drop: &[&str]) -> BTreeMap<String, ParsedFile> {
    all.iter()
        .filter(|(path, _)| !drop.contains(&path.as_str()))
        .map(|(path, file)| (path.clone(), file.clone()))
        .collect()
}

fn keys(files: &BTreeMap<String, ParsedFile>) -> BTreeSet<String> {
    files.keys().cloned().collect()
}

/// The alias repository: `app.ts` is the only original file; the union adds `ctx.ts`
/// (imports through the `@lib` alias), `lib/barrel.ts` and `lib/leaf.ts`.
struct Fixture {
    _dir: tempfile::TempDir,
    union: BTreeMap<String, ParsedFile>,
    captured: ScopeGraphBuildInputs,
}

fn fixture() -> Fixture {
    let dir = write_repo(&ALIAS_FILES);
    let union = load_repo(dir.path()).unwrap().files;
    let original = without(&union, &CONTEXT);
    assert_eq!(
        (keys(&union).len(), keys(&original).len()),
        (4, 1),
        "inadmissible precondition: the loader did not load the four alias sources"
    );
    let captured = scope_graph_build_inputs(dir.path(), &original);
    let snapshot = &captured.js_paths_snapshot;
    assert!(
        snapshot.complete && !snapshot.configs.is_empty(),
        "inadmissible precondition: the capture is not a complete configured snapshot"
    );
    for path in ["lib/leaf.ts", "lib/missing.ts"] {
        assert!(
            !snapshot.was_probed(path),
            "inadmissible precondition: the captured snapshot already probed {path}"
        );
    }
    Fixture {
        _dir: dir,
        union,
        captured,
    }
}

fn js_entries(hashes: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let js = |key: &String| key.starts_with("js_paths_");
    hashes
        .iter()
        .filter(|(k, _)| js(k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn non_js_entries(hashes: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let js = |key: &String| key.starts_with("js_paths_");
    hashes
        .iter()
        .filter(|(k, _)| !js(k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn assert_captured_values_kept(rebased: &ScopeGraphBuildInputs, captured: &ScopeGraphBuildInputs) {
    assert_eq!(rebased.repo_root, captured.repo_root, "repo_root");
    assert_eq!(
        rebased.manifest_snapshot, captured.manifest_snapshot,
        "manifest_snapshot"
    );
    assert_eq!(
        rebased.skipped_go_testdata_files, captured.skipped_go_testdata_files,
        "skipped_go_testdata_files"
    );
    assert_eq!(rebased.cfg, captured.cfg, "cfg");
    assert_eq!(rebased.complete, captured.complete, "complete");
    assert_eq!(
        rebased.js_paths_snapshot.configs, captured.js_paths_snapshot.configs,
        "JS config bytes"
    );
}

#[test]
fn rebase_primes_union_alias_and_export_facts() {
    let f = fixture();
    let rebased = rebase_scope_inputs(&f.captured, &f.union);
    for path in ["lib/leaf.ts", "lib/missing.ts"] {
        assert!(
            rebased.js_paths_snapshot.was_probed(path),
            "union priming did not probe {path}"
        );
    }
    let mut resolver = Resolver::new(&rebased.js_paths_snapshot);
    assert_eq!(
        resolver
            .resolve("ctx.ts", "@lib", &keys(&f.union))
            .as_deref(),
        Some("lib/barrel.ts")
    );
}

/// Read through `memo_values()` (a `cfg(test)` accessor in the detach module) plus `topology()`.
#[test]
fn rebase_leaves_original_bookkeeping_unchanged() {
    let f = fixture();
    let snapshot = &f.captured.js_paths_snapshot;
    let (topology, cells) = (snapshot.topology(), snapshot.memo_values());
    let rebased = rebase_scope_inputs(&f.captured, &f.union);
    assert_ne!(
        rebased.js_paths_snapshot.memo_values(),
        cells,
        "inadmissible precondition: the rebase recorded nothing new"
    );
    // Further resolver work on the rebased snapshot, through the resolver and the snapshot.
    let mut resolver = Resolver::new(&rebased.js_paths_snapshot);
    resolver.resolve("ctx.ts", "@lib", &keys(&f.union));
    rebased
        .js_paths_snapshot
        .first_pass_absent("lib/after-rebase.ts");
    assert_eq!(
        snapshot.memo_values(),
        cells,
        "a memo cell of the original moved"
    );
    assert_eq!(
        snapshot.topology(),
        topology,
        "the original's topology moved"
    );
    assert!(!snapshot.was_probed("lib/after-rebase.ts"));
    assert_eq!(
        f.captured.all_file_paths,
        BTreeSet::from(["app.ts".to_string()]),
        "the original's file paths moved"
    );
}

#[test]
fn rebase_uses_captured_manifests_and_config() {
    let d = write_repo(&[
        (
            "tsconfig.json",
            r#"{"compilerOptions":{"paths":{"@a":["a.ts"]}}}"#,
        ),
        ("go.mod", "module example.com/a\n\ngo 1.21\n"),
        ("app.ts", "export const app = 1;"),
    ]);
    let original = load_repo(d.path()).unwrap().files;
    let captured = scope_graph_build_inputs(d.path(), &original);
    // After the capture: rewrite both manifests, add a source file.
    std::fs::write(
        d.path().join("tsconfig.json"),
        r#"{"compilerOptions":{"paths":{"@b":["b.ts"]}}}"#,
    )
    .unwrap();
    std::fs::write(d.path().join("go.mod"), "module example.com/b\n\ngo 1.22\n").unwrap();
    std::fs::write(d.path().join("new_file.ts"), "export const added = 2;").unwrap();
    let fresh = scope_graph_build_inputs(d.path(), &original);
    assert!(
        fresh.manifest_snapshot != captured.manifest_snapshot
            && fresh.js_paths_snapshot.configs != captured.js_paths_snapshot.configs
            && non_js_entries(&fresh.manifest_hashes) != non_js_entries(&captured.manifest_hashes),
        "inadmissible precondition: a fresh capture does not see the disk rewrite"
    );
    let rebased = rebase_scope_inputs(&captured, &original);
    assert_captured_values_kept(&rebased, &captured);
    assert_eq!(
        non_js_entries(&rebased.manifest_hashes),
        non_js_entries(&captured.manifest_hashes)
    );
    assert!(!rebased.all_file_paths.contains("new_file.ts"));
    // The new file is in `all_file_paths` exactly when the union holds it.
    let reloaded = load_repo(d.path()).unwrap().files;
    assert!(
        reloaded.contains_key("new_file.ts"),
        "inadmissible precondition: the reload did not load the new file"
    );
    let with_new = rebase_scope_inputs(&captured, &reloaded);
    assert!(with_new.all_file_paths.contains("new_file.ts"));
    assert_captured_values_kept(&with_new, &captured);
}

#[test]
fn rebase_preserves_rust_and_root_inputs() {
    let d = write_repo(&[
        (
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        ),
        ("src/lib.rs", "pub fn f() {}"),
        ("src/extra.rs", "pub fn g() {}"),
    ]);
    // The review never loaded `src/extra.rs`: the capture's Rust coverage is incomplete.
    let original = without(&load_repo(d.path()).unwrap().files, &["src/extra.rs"]);
    let mut captured = scope_graph_build_inputs(d.path(), &original);
    assert!(
        captured.cfg.edition == 2021 && !captured.complete,
        "inadmissible precondition: expected the manifest edition and incomplete Rust coverage, got {} / {}",
        captured.cfg.edition,
        captured.complete
    );
    // SYNTHETIC: the loader helper always reports 0; set a count so a defaulted copy shows.
    captured.skipped_go_testdata_files = 3;
    let rebased = rebase_scope_inputs(&captured, &original);
    assert_captured_values_kept(&rebased, &captured);
    assert_eq!(rebased.repo_root, d.path());
    assert!(rebased.manifest_snapshot == captured.manifest_snapshot);
    assert_eq!(rebased.manifest_hashes, captured.manifest_hashes);
}

#[test]
fn rebase_sets_union_paths_and_recomputes_js_topology() {
    let mut f = fixture();
    // SYNTHETIC: an entry the rebase must not touch (a real census entry needs C sources).
    f.captured
        .manifest_hashes
        .insert("c-census:review-incomplete".into(), "kept".into());
    let rebased = rebase_scope_inputs(&f.captured, &f.union);
    assert_ne!(
        f.captured.all_file_paths,
        keys(&f.union),
        "inadmissible precondition"
    );
    assert_eq!(rebased.all_file_paths, keys(&f.union));
    assert_eq!(
        non_js_entries(&rebased.manifest_hashes),
        non_js_entries(&f.captured.manifest_hashes)
    );
    assert_eq!(
        rebased.manifest_hashes["c-census:review-incomplete"], "kept",
        "the injected non-JS entry changed"
    );
    assert_ne!(
        js_entries(&f.captured.manifest_hashes),
        rebased.js_paths_snapshot.topology(),
        "inadmissible precondition: union priming left the JS topology unchanged"
    );
    assert_eq!(
        js_entries(&rebased.manifest_hashes),
        rebased.js_paths_snapshot.topology()
    );
}

#[test]
fn rebase_without_js_config_only_rebases_paths() {
    let d = write_repo(&[
        ("app.ts", "export const app = 1;"),
        ("ctx.ts", "export const ctx = 2;"),
    ]);
    let union = load_repo(d.path()).unwrap().files;
    let original = without(&union, &["ctx.ts"]);
    let captured = scope_graph_build_inputs(d.path(), &original);
    assert!(
        captured.js_paths_snapshot.configs.is_empty()
            && js_entries(&captured.manifest_hashes).is_empty(),
        "inadmissible precondition: the capture holds a JS config"
    );
    let rebased = rebase_scope_inputs(&captured, &union);
    assert!(rebased.js_paths_snapshot.topology().is_empty());
    assert_eq!(rebased.manifest_hashes, captured.manifest_hashes);
    assert_captured_values_kept(&rebased, &captured);
    assert_eq!(rebased.all_file_paths, keys(&union));
    assert_ne!(rebased.all_file_paths, captured.all_file_paths);
}

#[test]
fn rebase_is_deterministic_and_repeatable() {
    let f = fixture();
    let first = rebase_scope_inputs(&f.captured, &f.union);
    // A probe on the first rebase must not reach any later one.
    first
        .js_paths_snapshot
        .first_pass_absent("lib/first-only.ts");
    let second = rebase_scope_inputs(&f.captured, &f.union);
    assert!(
        !second.js_paths_snapshot.was_probed("lib/first-only.ts"),
        "the second rebase saw the first one's probe"
    );
    let third = rebase_scope_inputs(&f.captured, &f.union);
    assert_captured_values_kept(&second, &third);
    assert_eq!(second.all_file_paths, third.all_file_paths);
    assert_eq!(second.manifest_hashes, third.manifest_hashes);
    assert_eq!(
        second.js_paths_snapshot.memo_values(),
        third.js_paths_snapshot.memo_values()
    );
    assert!(!f.captured.js_paths_snapshot.was_probed("lib/first-only.ts"));
}
