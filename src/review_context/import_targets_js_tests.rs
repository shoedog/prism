//! JS/TS attribution: local shadowing, relative-specifier precedence, and the
//! tsconfig alias lookup (attribution mode, not the strict resolver).
use super::import_targets_tests::{
    assert_external, assert_files, assert_loaded, assert_same_file, assert_unknown,
    assert_unresolvable, attribute, attribute_loaded, attribute_nth, write_repo,
};

const NODE10: &str = r#"{"compilerOptions":{"moduleResolution":"node10","baseUrl":".","paths":{"@lib/*":["src/lib/*"]}},"include":["**/*"]}"#;

// ---- local shadowing: a nearer declaration is not the import ----

#[test]
fn shadowed_namespace_is_unknown_and_unshadowed_is_attributed() {
    let dir = write_repo(&[
        (
            "app.js",
            "import * as ns from './lib';\nexport function h(ns, x) { ns.f(x); }\nexport function g(x) { ns.f(x); }\n",
        ),
        ("lib.js", "export function f(x) {}\n"),
    ]);
    assert_unknown(attribute_nth(&dir, "app.js", "f", 0));
    assert_files(attribute_nth(&dir, "app.js", "f", 1), &["lib.js"]);
}

#[test]
fn shadowed_named_import_is_unknown_and_unshadowed_is_attributed() {
    let dir = write_repo(&[
        (
            "app.js",
            "import f from './f';\nexport function h(f, x) { f(x); }\nexport function g(x) { f(x); }\n",
        ),
        ("f.js", "export default function f(x) {}\n"),
    ]);
    assert_unknown(attribute_nth(&dir, "app.js", "f", 0));
    assert_files(attribute_nth(&dir, "app.js", "f", 1), &["f.js"]);
}

#[test]
fn local_declarations_shadow_imports() {
    let dir = write_repo(&[
        (
            "app.ts",
            "import { f } from './f';\nimport * as ns from './lib';\n\
             export function a(x: string) { const ns = make(); ns.f(x); }\n\
             export function b(x: string) { { let f = pick(); f(x); } }\n\
             export function c({ ns }: { ns: any }, x: string) { ns.f(x); }\n\
             export function d(x: string) { f(x); ns.f(x); }\n",
        ),
        ("f.ts", "export function f(x: string) {}\n"),
        ("lib.ts", "export function f(x: string) {}\n"),
    ]);
    // Sites of `f` in source order: a, b, c, then d's two.
    assert_unknown(attribute_nth(&dir, "app.ts", "f", 0)); // a: ns.f, local ns
    assert_unknown(attribute_nth(&dir, "app.ts", "f", 1)); // b: block-local f
    assert_unknown(attribute_nth(&dir, "app.ts", "f", 2)); // c: destructured ns
    assert_files(attribute_nth(&dir, "app.ts", "f", 3), &["f.ts"]); // d: f()
    assert_files(attribute_nth(&dir, "app.ts", "f", 4), &["lib.ts"]); // d: ns.f()
}

#[test]
fn shadowed_cjs_alias_is_unknown_and_unshadowed_is_attributed() {
    let dir = write_repo(&[
        (
            "app.js",
            "const lib = require('./lib');\nexport function h(lib, x) { lib.f(x); }\nexport function g(x) { lib.f(x); }\n",
        ),
        ("lib.js", "exports.f = function (x) {};\n"),
    ]);
    assert_unknown(attribute_nth(&dir, "app.js", "f", 0));
    assert_files(attribute_nth(&dir, "app.js", "f", 1), &["lib.js"]);
}

#[test]
fn shadowed_same_file_function_is_unknown_and_nested_function_is_same_file() {
    let dir = write_repo(&[(
        "app.js",
        "function helper(x) {}\nexport function a(helper, x) { helper(x); }\n\
         export function b(x) { function inner(y) {} inner(x); }\nexport function c(x) { helper(x); }\n",
    )]);
    assert_unknown(attribute_nth(&dir, "app.js", "helper", 0));
    assert_same_file(attribute(&dir, "app.js", "inner"));
    assert_same_file(attribute_nth(&dir, "app.js", "helper", 1));
}

#[test]
fn this_receiver_is_unknown() {
    let dir = write_repo(&[(
        "app.js",
        "import f from './f';\nexport class C { m(x) { this.f(x); } }\n",
    )]);
    assert_unknown(attribute(&dir, "app.js", "f"));
}

// ---- relative specifiers: the precedence winner, not every candidate ----

#[test]
fn relative_specifier_takes_the_extension_precedence_winner() {
    let dir = write_repo(&[
        (
            "app.js",
            "import { f } from './x';\nexport function h(v) { f(v); }\n",
        ),
        ("x.ts", "export function f(v) {}\n"),
        ("x.js", "export function f(v) {}\n"),
    ]);
    assert_files(attribute(&dir, "app.js", "f"), &["x.js"]);
}

#[test]
fn file_beats_directory_index_and_a_loaded_winner_is_loaded() {
    let dir = write_repo(&[
        (
            "src/a.ts",
            "import { f } from './util';\nexport function h(x: string) { f(x); }\n",
        ),
        ("src/util.ts", "export default function (x: string) {}\n"),
        ("src/util/index.ts", "export function g() {}\n"),
    ]);
    assert_files(attribute(&dir, "src/a.ts", "f"), &["src/util.ts"]);
    // The loser `src/util/index.ts` is not outside the set the call reaches.
    assert_loaded(attribute_loaded(&dir, "src/a.ts", "f", &["src/util.ts"]));
}

#[test]
fn five_same_stem_files_still_name_the_winner() {
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
    assert_files(attribute(&dir, "app.js", "f"), &["x.js"]);
}

// ---- tsconfig alias attribution ----

fn alias_repo(config: &str, extra: &[(&str, &str)]) -> tempfile::TempDir {
    let mut files = vec![
        ("tsconfig.json", config),
        (
            "src/app.ts",
            "import { f } from '@lib/cmd';\nexport function h(x: string) { f(x); }\n",
        ),
    ];
    files.extend_from_slice(extra);
    write_repo(&files)
}

#[test]
fn paths_alias_resolves_to_its_target() {
    let dir = alias_repo(
        NODE10,
        &[("src/lib/cmd.ts", "export function f(x: string) {}\n")],
    );
    assert_files(attribute(&dir, "src/app.ts", "f"), &["src/lib/cmd.ts"]);
}

#[test]
fn alias_with_missing_target_is_unresolvable_not_external() {
    let dir = write_repo(&[
        ("tsconfig.json", NODE10),
        (
            "src/app.ts",
            "import { f } from '@lib/gone';\nimport { g } from 'express';\nexport function h(x: string) { f(x); g(x); }\n",
        ),
    ]);
    assert_unresolvable(attribute(&dir, "src/app.ts", "f"), "@lib/gone");
    assert_external(attribute(&dir, "src/app.ts", "g"));
}

#[test]
fn fallback_target_arrays_are_candidates_in_target_order() {
    let config = r#"{"compilerOptions":{"moduleResolution":"node10","baseUrl":".","paths":{"@lib/*":["lib/*","fallback/*"]}}}"#;
    let only_second = alias_repo(config, &[("fallback/cmd.ts", "export function f() {}\n")]);
    assert_files(
        attribute(&only_second, "src/app.ts", "f"),
        &["fallback/cmd.ts"],
    );
    let both = alias_repo(
        config,
        &[
            ("lib/cmd.ts", "export function f() {}\n"),
            ("fallback/cmd.ts", "export function f() {}\n"),
        ],
    );
    assert_files(
        attribute(&both, "src/app.ts", "f"),
        &["lib/cmd.ts", "fallback/cmd.ts"],
    );
    let neither = alias_repo(config, &[]);
    assert_unresolvable(attribute(&neither, "src/app.ts", "f"), "@lib/cmd");
}

#[test]
fn base_url_without_paths_resolves_bare_specifiers() {
    let config =
        r#"{"compilerOptions":{"moduleResolution":"node10","baseUrl":"src"},"include":["**/*"]}"#;
    let dir = write_repo(&[
        ("tsconfig.json", config),
        (
            "src/app.ts",
            "import { f } from 'lib/cmd';\nimport { g } from 'react';\nexport function h(x: string) { f(x); g(x); }\n",
        ),
        ("src/lib/cmd.ts", "export function f(x: string) {}\n"),
    ]);
    assert_files(attribute(&dir, "src/app.ts", "f"), &["src/lib/cmd.ts"]);
    // No file under baseUrl: a package, not an alias.
    assert_external(attribute(&dir, "src/app.ts", "g"));
}

#[test]
fn module_resolution_unset_or_bundler_still_attributes() {
    for mode in ["", r#""moduleResolution":"bundler","#] {
        let config = format!(
            r#"{{"compilerOptions":{{{mode}"baseUrl":".","paths":{{"@lib/*":["src/lib/*"]}}}}}}"#
        );
        let dir = alias_repo(&config, &[("src/lib/cmd.ts", "export function f() {}\n")]);
        assert_files(attribute(&dir, "src/app.ts", "f"), &["src/lib/cmd.ts"]);
        let missing = write_repo(&[
            ("tsconfig.json", &config),
            (
                "src/app.ts",
                "import { f } from '@lib/gone';\nexport function h(x: string) { f(x); }\n",
            ),
        ]);
        assert_unresolvable(attribute(&missing, "src/app.ts", "f"), "@lib/gone");
    }
}

#[test]
fn key_targeting_only_skipped_directories_is_not_an_alias() {
    let config = r#"{"compilerOptions":{"moduleResolution":"node10","baseUrl":".","paths":{"react":["node_modules/@types/react"],"@lib/*":["src/lib/*"]}}}"#;
    let dir = write_repo(&[
        ("tsconfig.json", config),
        (
            "src/app.ts",
            "import { createElement } from 'react';\nexport function h(x: string) { createElement(x); }\n",
        ),
        ("node_modules/@types/react/index.d.ts", ""),
    ]);
    assert_external(attribute(&dir, "src/app.ts", "createElement"));
}

#[test]
fn suffix_pattern_and_longest_prefix_and_exact_keys() {
    let config = r#"{"compilerOptions":{"baseUrl":".","paths":{
        "~/*-impl":["src/impl/*"],
        "@/*":["src/*"],
        "@/ui/*":["src/components/*"],
        "settings":["src/config/index"]}}}"#;
    let dir = write_repo(&[
        ("tsconfig.json", config),
        (
            "src/app.ts",
            "import { a } from '~/cmd-impl';\nimport { b } from '@/ui/button';\nimport { c } from '@/util';\nimport { d } from 'settings';\nimport { e } from '~/cmd-impl-missing';\n\
             export function h(x: string) { a(x); b(x); c(x); d(x); e(x); }\n",
        ),
        ("src/impl/cmd.ts", ""),
        ("src/components/button.ts", ""),
        ("src/util.ts", ""),
        ("src/ui/button.ts", ""),
        ("src/config/index.ts", ""),
    ]);
    assert_files(attribute(&dir, "src/app.ts", "a"), &["src/impl/cmd.ts"]);
    // `@/ui/*` has the longer prefix than `@/*`, so `src/ui/button.ts` is not it.
    assert_files(
        attribute(&dir, "src/app.ts", "b"),
        &["src/components/button.ts"],
    );
    assert_files(attribute(&dir, "src/app.ts", "c"), &["src/util.ts"]);
    assert_files(attribute(&dir, "src/app.ts", "d"), &["src/config/index.ts"]);
    // `~/cmd-impl-missing` does not end with `-impl`: no key matches.
    assert_external(attribute(&dir, "src/app.ts", "e"));
}

#[test]
fn extended_config_supplies_paths_and_the_nearest_config_owns() {
    let dir = write_repo(&[
        (
            "tsconfig.base.json",
            r#"{"compilerOptions":{"baseUrl":".","paths":{"@lib/*":["src/lib/*"]}}}"#,
        ),
        ("tsconfig.json", r#"{"extends":"./tsconfig.base.json"}"#),
        (
            "src/app.ts",
            "import { f } from '@lib/cmd';\nexport function h(x: string) { f(x); }\n",
        ),
        ("src/lib/cmd.ts", ""),
        ("tools/tsconfig.json", r#"{"compilerOptions":{}}"#),
        (
            "tools/run.ts",
            "import { g } from '@lib/cmd';\nexport function h(x: string) { g(x); }\n",
        ),
    ]);
    assert_files(attribute(&dir, "src/app.ts", "f"), &["src/lib/cmd.ts"]);
    // `tools/tsconfig.json` owns tools/run.ts and declares no alias.
    assert_external(attribute(&dir, "tools/run.ts", "g"));
}

#[test]
fn catch_all_paths_key_does_not_turn_packages_into_aliases() {
    let config = r#"{"compilerOptions":{"moduleResolution":"node10","baseUrl":".","paths":{"*":["types/*"]}},"include":["**/*"]}"#;
    let dir = write_repo(&[
        ("tsconfig.json", config),
        (
            "src/app.ts",
            "import { g } from 'express';\nexport function h(x: string) { g(x); }\n",
        ),
    ]);
    assert_external(attribute(&dir, "src/app.ts", "g"));
}
