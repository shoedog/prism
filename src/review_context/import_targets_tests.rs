//! JS/TS attribution tests on tempdir repos, plus the shared fixture helpers.
use super::import_targets::{Attribution, ImportTargets};
use crate::ast::ParsedFile;
use crate::call_graph::CallGraph;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use tempfile::TempDir;

pub(super) fn write_repo(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (name, text) in files {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    dir
}

fn parse(root: &Path, rel: &str) -> ParsedFile {
    let source = std::fs::read_to_string(root.join(rel)).unwrap();
    let language = Language::from_path(rel).unwrap();
    ParsedFile::parse(rel, &source, language).unwrap()
}

/// Attribute the first call to `callee` found in `caller`, with `extra_loaded`
/// files counted as already analysed next to the caller itself.
pub(super) fn attribute_loaded(
    dir: &TempDir,
    caller: &str,
    callee: &str,
    extra_loaded: &[&str],
) -> Attribution {
    let parsed = parse(dir.path(), caller);
    let mut files = BTreeMap::new();
    files.insert(caller.to_string(), parsed);
    let graph = CallGraph::build(&files);
    let site = graph
        .calls
        .values()
        .flatten()
        .find(|site| site.callee_name == callee)
        .unwrap_or_else(|| panic!("no call to {callee} in {caller}"))
        .clone();
    let inputs = crate::repo_loader::scope_graph_build_inputs(dir.path(), &files);
    let mut loaded: BTreeSet<String> = extra_loaded.iter().map(|s| s.to_string()).collect();
    loaded.insert(caller.to_string());
    let mut targets = ImportTargets::new(&inputs, loaded).expect("repo listing");
    targets.targets_for_call(caller, &files[caller], &site)
}

pub(super) fn attribute(dir: &TempDir, caller: &str, callee: &str) -> Attribution {
    attribute_loaded(dir, caller, callee, &[])
}

pub(super) fn in_repo(paths: &[&str]) -> Attribution {
    Attribution::InRepo(paths.iter().map(|p| p.to_string()).collect())
}

fn unresolvable(spec: &str) -> Attribution {
    Attribution::Unresolvable { spec: spec.into() }
}

#[test]
fn listing_is_memoized_on_scope_graph_inputs() {
    let dir = write_repo(&[("a.js", "export function f(){}")]);
    let inputs = crate::repo_loader::scope_graph_build_inputs(dir.path(), &BTreeMap::new());
    assert!(inputs.source_listing.get().is_some(), "walked by the build");
    let first = inputs.source_listing().unwrap();
    let second = inputs.source_listing().unwrap();
    assert!(std::sync::Arc::ptr_eq(&first, &second));
    assert!(first.contains("a.js"));
}

#[test]
fn js_esm_named_relative_import() {
    let dir = write_repo(&[
        (
            "app.js",
            "import { runCmd } from './lib/cmd.js';\nexport function h(x) { runCmd(x); }\n",
        ),
        ("lib/cmd.js", "export function runCmd(c) {}\n"),
    ]);
    assert_eq!(
        attribute(&dir, "app.js", "runCmd"),
        in_repo(&["lib/cmd.js"])
    );
}

#[test]
fn js_spelling_maps_to_ts_source() {
    let dir = write_repo(&[
        (
            "app.ts",
            "import { f } from './util.js';\nexport function h(x: string) { f(x); }\n",
        ),
        ("util.ts", "export function f(x: string) {}\n"),
    ]);
    assert_eq!(attribute(&dir, "app.ts", "f"), in_repo(&["util.ts"]));
}

#[test]
fn js_directory_specifier_finds_index_file() {
    let dir = write_repo(&[
        (
            "app.js",
            "import { f } from './lib';\nexport function h(x) { f(x); }\n",
        ),
        ("lib/index.js", "export function f(x) {}\n"),
    ]);
    assert_eq!(attribute(&dir, "app.js", "f"), in_repo(&["lib/index.js"]));
}

#[test]
fn js_parent_directory_specifier() {
    let dir = write_repo(&[
        (
            "src/app/h.js",
            "import { f } from '../shared/x';\nexport function h(v) { f(v); }\n",
        ),
        ("src/shared/x.js", "export function f(v) {}\n"),
    ]);
    assert_eq!(
        attribute(&dir, "src/app/h.js", "f"),
        in_repo(&["src/shared/x.js"])
    );
}

#[test]
fn js_default_import_call() {
    let dir = write_repo(&[
        (
            "app.js",
            "import run from './run';\nexport function h(x) { run(x); }\n",
        ),
        ("run.js", "export default function run(x) {}\n"),
    ]);
    assert_eq!(attribute(&dir, "app.js", "run"), in_repo(&["run.js"]));
}

#[test]
fn js_namespace_member_call() {
    let dir = write_repo(&[
        (
            "app.js",
            "import * as ns from './lib';\nexport function h(x) { ns.f(x); }\n",
        ),
        ("lib.js", "export function f(x) {}\n"),
    ]);
    assert_eq!(attribute(&dir, "app.js", "f"), in_repo(&["lib.js"]));
}

#[test]
fn js_cjs_destructured_require() {
    let dir = write_repo(&[
        (
            "app.js",
            "const { runCmd: go } = require('./lib/cmd');\nfunction h(x) { go(x); }\n",
        ),
        ("lib/cmd.js", "exports.runCmd = function (c) {};\n"),
    ]);
    assert_eq!(attribute(&dir, "app.js", "go"), in_repo(&["lib/cmd.js"]));
}

#[test]
fn js_cjs_alias_member_call() {
    let dir = write_repo(&[
        (
            "app.js",
            "const lib = require('./lib');\nfunction h(x) { lib.f(x); }\n",
        ),
        ("lib.js", "exports.f = function (x) {};\n"),
    ]);
    assert_eq!(attribute(&dir, "app.js", "f"), in_repo(&["lib.js"]));
}

#[test]
fn js_tsconfig_paths_alias() {
    let dir = write_repo(&[
        (
            "tsconfig.json",
            r#"{"compilerOptions":{"moduleResolution":"node10","baseUrl":".","paths":{"@lib/*":["src/lib/*"]}},"include":["**/*"]}"#,
        ),
        (
            "src/app.ts",
            "import { f } from '@lib/cmd';\nexport function h(x: string) { f(x); }\n",
        ),
        ("src/lib/cmd.ts", "export function f(x: string) {}\n"),
    ]);
    assert_eq!(
        attribute(&dir, "src/app.ts", "f"),
        in_repo(&["src/lib/cmd.ts"])
    );
}

#[test]
fn js_alias_with_missing_target_is_unresolvable_not_external() {
    let dir = write_repo(&[
        (
            "tsconfig.json",
            r#"{"compilerOptions":{"moduleResolution":"node10","baseUrl":".","paths":{"@lib/*":["src/lib/*"]}},"include":["**/*"]}"#,
        ),
        (
            "src/app.ts",
            "import { f } from '@lib/gone';\nimport { g } from 'express';\nexport function h(x: string) { f(x); g(x); }\n",
        ),
    ]);
    assert_eq!(
        attribute(&dir, "src/app.ts", "f"),
        unresolvable("@lib/gone")
    );
    assert_eq!(attribute(&dir, "src/app.ts", "g"), Attribution::External);
}

#[test]
fn js_bare_package_is_external() {
    let dir = write_repo(&[(
        "app.js",
        "import { exec } from 'child_process';\nconst sh = require('shelljs');\nexport function h(x) { exec(x); sh.run(x); }\n",
    )]);
    assert_eq!(attribute(&dir, "app.js", "exec"), Attribution::External);
    assert_eq!(attribute(&dir, "app.js", "run"), Attribution::External);
}

#[test]
fn js_missing_relative_file_is_unresolvable() {
    let dir = write_repo(&[(
        "app.js",
        "import { f } from './nope';\nexport function h(x) { f(x); }\n",
    )]);
    assert_eq!(attribute(&dir, "app.js", "f"), unresolvable("./nope"));
}

#[test]
fn js_skipped_directories_are_never_targets() {
    let dir = write_repo(&[
        (
            "app.js",
            "import { a } from './node_modules/pkg/index';\nimport { b } from './dist/out';\nimport { c } from './vendor/v';\nexport function h(x) { a(x); b(x); c(x); }\n",
        ),
        ("node_modules/pkg/index.js", "export function a(x) {}\n"),
        ("dist/out.js", "export function b(x) {}\n"),
        ("vendor/v.js", "export function c(x) {}\n"),
    ]);
    for callee in ["a", "b", "c"] {
        assert!(
            matches!(
                attribute(&dir, "app.js", callee),
                Attribution::Unresolvable { .. }
            ),
            "{callee} must not attribute into a skipped directory"
        );
    }
}

#[test]
fn js_ambiguous_candidates_are_all_returned_sorted() {
    let dir = write_repo(&[
        (
            "app.js",
            "import { f } from './x';\nexport function h(v) { f(v); }\n",
        ),
        ("x.ts", "export function f(v) {}\n"),
        ("x.js", "export function f(v) {}\n"),
    ]);
    assert_eq!(attribute(&dir, "app.js", "f"), in_repo(&["x.js", "x.ts"]));
}

#[test]
fn js_barrel_attributes_to_the_barrel_file() {
    let dir = write_repo(&[
        (
            "app.js",
            "import { f } from './lib';\nexport function h(x) { f(x); }\n",
        ),
        ("lib/index.js", "export { f } from './impl';\n"),
        ("lib/impl.js", "export function f(x) {}\n"),
    ]);
    assert_eq!(attribute(&dir, "app.js", "f"), in_repo(&["lib/index.js"]));
}

#[test]
fn js_barrel_one_hop_follows_the_reexport() {
    let dir = write_repo(&[
        ("lib/index.js", "export { f as g } from './impl';\n"),
        ("lib/impl.js", "export function f(x) {}\n"),
    ]);
    let barrel = parse(dir.path(), "lib/index.js");
    let mut files = BTreeMap::new();
    files.insert("lib/index.js".to_string(), barrel);
    let inputs = crate::repo_loader::scope_graph_build_inputs(dir.path(), &files);
    let mut targets = ImportTargets::new(&inputs, BTreeSet::new()).unwrap();
    let hop = targets
        .follow_barrel("lib/index.js", &files["lib/index.js"], "g")
        .expect("re-export hop");
    assert_eq!(
        (hop.file.as_str(), hop.member.as_str()),
        ("lib/impl.js", "f")
    );
    assert!(targets
        .follow_barrel("lib/index.js", &files["lib/index.js"], "absent")
        .is_none());
}

#[test]
fn js_same_file_function_is_local() {
    let dir = write_repo(&[(
        "app.js",
        "function helper(x) {}\nexport function h(x) { helper(x); }\n",
    )]);
    assert_eq!(attribute(&dir, "app.js", "helper"), Attribution::Local);
}

#[test]
fn js_unbound_calls_are_unknown() {
    let dir = write_repo(&[(
        "app.js",
        "function run(v) {}\nexport function h(obj, x) { obj.run(x); mystery(x); }\n",
    )]);
    assert_eq!(attribute(&dir, "app.js", "run"), Attribution::Unknown);
    assert_eq!(attribute(&dir, "app.js", "mystery"), Attribution::Unknown);
}

#[test]
fn js_target_in_loaded_set_is_local() {
    let dir = write_repo(&[
        (
            "app.js",
            "import { runCmd } from './lib/cmd.js';\nexport function h(x) { runCmd(x); }\n",
        ),
        ("lib/cmd.js", "export function runCmd(c) {}\n"),
    ]);
    assert_eq!(
        attribute_loaded(&dir, "app.js", "runCmd", &["lib/cmd.js"]),
        Attribution::Local
    );
}

#[test]
fn js_catch_all_paths_key_does_not_turn_packages_into_aliases() {
    let dir = write_repo(&[
        (
            "tsconfig.json",
            r#"{"compilerOptions":{"moduleResolution":"node10","baseUrl":".","paths":{"*":["types/*"]}},"include":["**/*"]}"#,
        ),
        (
            "src/app.ts",
            "import { g } from 'express';\nexport function h(x: string) { g(x); }\n",
        ),
    ]);
    assert_eq!(attribute(&dir, "src/app.ts", "g"), Attribution::External);
}

#[test]
fn js_too_many_candidates_are_unresolvable() {
    let dir = write_repo(&[
        (
            "app.js",
            "import { f } from './x';\nexport function h(v) { f(v); }\n",
        ),
        ("x.js", ""),
        ("x.jsx", ""),
        ("x.mjs", ""),
        ("x.cjs", ""),
        ("x.ts", ""),
    ]);
    assert_eq!(attribute(&dir, "app.js", "f"), unresolvable("./x"));
}
