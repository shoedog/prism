//! JS/TS attribution tests on tempdir repos, plus the shared fixture helpers.
//! The helpers are the whole interface the language test files use, so a
//! control run against an older `Attribution` shape only swaps this module.
use super::import_targets::{Attribution, ImportTargets, InRepo};
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

/// Attribute the `nth` (by source position, from 0) call to `callee` in
/// `caller`. `loaded` files count as already analysed; the caller does too
/// unless `caller_loaded` is false (a self-import then reaches `finish` with a
/// target that is the caller but not in the loaded set).
pub(super) fn attribute_with(
    dir: &TempDir,
    caller: &str,
    callee: &str,
    nth: usize,
    loaded: &[&str],
    caller_loaded: bool,
) -> Attribution {
    let parsed = parse(dir.path(), caller);
    let mut files = BTreeMap::new();
    files.insert(caller.to_string(), parsed);
    let graph = CallGraph::build(&files);
    let mut sites: Vec<_> = graph
        .calls
        .values()
        .flatten()
        .filter(|site| site.callee_name == callee)
        .collect();
    sites.sort_by_key(|site| site.start_byte);
    let site = (*sites
        .get(nth)
        .unwrap_or_else(|| panic!("no call #{nth} to {callee} in {caller}")))
    .clone();
    let (inputs, listing) =
        crate::repo_loader::scope_graph_build_inputs_with_listing(dir.path(), &files);
    let mut loaded: BTreeSet<String> = loaded.iter().map(|s| s.to_string()).collect();
    if caller_loaded {
        loaded.insert(caller.to_string());
    }
    let mut targets = ImportTargets::new(&inputs, &listing, loaded).expect("repo listing");
    targets.targets_for_call(caller, &files[caller], &site)
}

pub(super) fn attribute_loaded(
    dir: &TempDir,
    caller: &str,
    callee: &str,
    extra_loaded: &[&str],
) -> Attribution {
    attribute_with(dir, caller, callee, 0, extra_loaded, true)
}

pub(super) fn attribute(dir: &TempDir, caller: &str, callee: &str) -> Attribution {
    attribute_loaded(dir, caller, callee, &[])
}

/// The nth call to `callee`, nothing else loaded.
pub(super) fn attribute_nth(dir: &TempDir, caller: &str, callee: &str, nth: usize) -> Attribution {
    attribute_with(dir, caller, callee, nth, &[], true)
}

/// The `InRepo` payload, or a panic naming what came back.
pub(super) fn in_repo(found: &Attribution) -> &InRepo {
    match found {
        Attribution::InRepo(hit) => hit,
        other => panic!("expected InRepo, got {other:?}"),
    }
}

pub(super) fn assert_files(found: Attribution, expected: &[&str]) {
    assert_eq!(in_repo(&found).files, expected, "{found:?}");
}

pub(super) fn assert_unknown(found: Attribution) {
    assert_eq!(found, Attribution::Unknown);
}

pub(super) fn assert_external(found: Attribution) {
    assert_eq!(found, Attribution::External);
}

pub(super) fn assert_same_file(found: Attribution) {
    assert_eq!(found, Attribution::SameFile);
}

pub(super) fn assert_loaded(found: Attribution) {
    assert!(matches!(found, Attribution::Loaded(_)), "{found:?}");
}

pub(super) fn assert_unresolvable(found: Attribution, expected_spec: &str) {
    match found {
        Attribution::Unresolvable { spec, .. } => assert_eq!(spec, expected_spec),
        other => panic!("expected Unresolvable({expected_spec}), got {other:?}"),
    }
}

fn app(import: &str, call: &str) -> String {
    format!("{import}\nexport function h(x) {{ {call} }}\n")
}

#[test]
fn js_esm_named_relative_import() {
    let dir = write_repo(&[
        (
            "app.js",
            &app("import { runCmd } from './lib/cmd.js';", "runCmd(x);"),
        ),
        ("lib/cmd.js", "export function runCmd(c) {}\n"),
    ]);
    assert_files(attribute(&dir, "app.js", "runCmd"), &["lib/cmd.js"]);
}

#[test]
fn js_spelling_maps_to_ts_source() {
    let dir = write_repo(&[
        ("app.ts", &app("import { f } from './util.js';", "f(x);")),
        ("util.ts", "export function f(x: string) {}\n"),
    ]);
    assert_files(attribute(&dir, "app.ts", "f"), &["util.ts"]);
}

#[test]
fn js_directory_specifier_finds_index_file() {
    let dir = write_repo(&[
        ("app.js", &app("import { f } from './lib';", "f(x);")),
        ("lib/index.js", "export function f(x) {}\n"),
    ]);
    assert_files(attribute(&dir, "app.js", "f"), &["lib/index.js"]);
}

#[test]
fn js_parent_directory_specifier() {
    let dir = write_repo(&[
        (
            "src/app/h.js",
            &app("import { f } from '../shared/x';", "f(x);"),
        ),
        ("src/shared/x.js", "export function f(v) {}\n"),
    ]);
    assert_files(attribute(&dir, "src/app/h.js", "f"), &["src/shared/x.js"]);
}

#[test]
fn js_default_import_call() {
    let dir = write_repo(&[
        ("app.js", &app("import run from './run';", "run(x);")),
        ("run.js", "export default function run(x) {}\n"),
    ]);
    assert_files(attribute(&dir, "app.js", "run"), &["run.js"]);
}

#[test]
fn js_namespace_member_call() {
    let dir = write_repo(&[
        ("app.js", &app("import * as ns from './lib';", "ns.f(x);")),
        ("lib.js", "export function f(x) {}\n"),
    ]);
    assert_files(attribute(&dir, "app.js", "f"), &["lib.js"]);
}

#[test]
fn js_cjs_destructured_require() {
    let dir = write_repo(&[
        (
            "app.js",
            &app("const { runCmd: go } = require('./lib/cmd');", "go(x);"),
        ),
        ("lib/cmd.js", "exports.runCmd = function (c) {};\n"),
    ]);
    assert_files(attribute(&dir, "app.js", "go"), &["lib/cmd.js"]);
}

#[test]
fn js_cjs_alias_member_call() {
    let dir = write_repo(&[
        ("app.js", &app("const lib = require('./lib');", "lib.f(x);")),
        ("lib.js", "exports.f = function (x) {};\n"),
    ]);
    assert_files(attribute(&dir, "app.js", "f"), &["lib.js"]);
}

#[test]
fn js_bare_package_is_external() {
    let dir = write_repo(&[(
        "app.js",
        "import { exec } from 'child_process';\nconst sh = require('shelljs');\nexport function h(x) { exec(x); sh.run(x); }\n",
    )]);
    assert_external(attribute(&dir, "app.js", "exec"));
    assert_external(attribute(&dir, "app.js", "run"));
}

#[test]
fn js_missing_relative_file_is_unresolvable() {
    let dir = write_repo(&[("app.js", &app("import { f } from './nope';", "f(x);"))]);
    assert_unresolvable(attribute(&dir, "app.js", "f"), "./nope");
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
    for (callee, spec) in [
        ("a", "./node_modules/pkg/index"),
        ("b", "./dist/out"),
        ("c", "./vendor/v"),
    ] {
        assert_unresolvable(attribute(&dir, "app.js", callee), spec);
    }
}

#[test]
fn js_barrel_attributes_to_the_barrel_file() {
    let dir = write_repo(&[
        ("app.js", &app("import { f } from './lib';", "f(x);")),
        ("lib/index.js", "export { f } from './impl';\n"),
        ("lib/impl.js", "export function f(x) {}\n"),
    ]);
    assert_files(attribute(&dir, "app.js", "f"), &["lib/index.js"]);
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
    let (inputs, listing) =
        crate::repo_loader::scope_graph_build_inputs_with_listing(dir.path(), &files);
    let mut targets = ImportTargets::new(&inputs, &listing, BTreeSet::new()).unwrap();
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
fn js_same_file_function_is_same_file() {
    let dir = write_repo(&[(
        "app.js",
        "function helper(x) {}\nexport function h(x) { helper(x); }\n",
    )]);
    assert_same_file(attribute(&dir, "app.js", "helper"));
}

#[test]
fn js_unbound_calls_are_unknown() {
    let dir = write_repo(&[(
        "app.js",
        "function run(v) {}\nexport function h(obj, x) { obj.run(x); mystery(x); }\n",
    )]);
    assert_unknown(attribute(&dir, "app.js", "run"));
    assert_unknown(attribute(&dir, "app.js", "mystery"));
}

#[test]
fn js_method_on_a_module_level_object_is_not_the_same_named_function() {
    let dir = write_repo(&[(
        "app.js",
        "const obj = make();\nfunction run(v) {}\nexport function h(x) { obj.run(x); }\n",
    )]);
    assert_unknown(attribute(&dir, "app.js", "run"));
}

#[test]
fn js_target_in_loaded_set_is_loaded() {
    let dir = write_repo(&[
        (
            "app.js",
            &app("import { runCmd } from './lib/cmd.js';", "runCmd(x);"),
        ),
        ("lib/cmd.js", "export function runCmd(c) {}\n"),
    ]);
    assert_loaded(attribute_loaded(&dir, "app.js", "runCmd", &["lib/cmd.js"]));
}

#[test]
fn root_identifier_takes_only_plain_dotted_paths() {
    use super::import_targets::root_identifier;
    assert_eq!(root_identifier("ns"), Some("ns"));
    assert_eq!(root_identifier("ns.sub"), Some("ns"));
    assert_eq!(root_identifier("ns?.sub"), Some("ns"));
    assert_eq!(root_identifier("a[0]"), None);
    assert_eq!(root_identifier("f().x"), None);
    assert_eq!(root_identifier(".x"), None);
}

#[test]
fn js_subscripted_receiver_is_unknown_and_plain_one_is_attributed() {
    let dir = write_repo(&[
        (
            "app.js",
            "import * as ns from './lib';\nexport function a(x) { ns[0].f(x); }\nexport function b(x) { ns.f(x); }\n",
        ),
        ("lib.js", "export function f(x) {}\n"),
    ]);
    assert_unknown(attribute_nth(&dir, "app.js", "f", 0));
    assert_files(attribute_nth(&dir, "app.js", "f", 1), &["lib.js"]);
}
