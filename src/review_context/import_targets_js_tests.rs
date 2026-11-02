//! JS/TS attribution: local shadowing, relative-specifier precedence, and the
//! tsconfig alias lookup (attribution mode, not the strict resolver).
use super::import_targets_tests::{
    assert_external, assert_files, assert_loaded, assert_same_file, assert_unknown,
    assert_unresolvable, attribute, attribute_loaded, attribute_nth, in_repo, write_repo,
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

// ---- post-cap fold: scope-correct imports, extends merging, unusable bases ----

#[test]
fn nested_require_does_not_rebind_the_program_import() {
    let dir = write_repo(&[
        (
            "app.js",
            "import { f } from './good';\n\
             function other() { const f = require('./bad'); return f; }\n\
             export function h(x) { f(x); }\n",
        ),
        ("good.js", "export function f(x) {}\n"),
        ("bad.js", "module.exports = function f(x) {};\n"),
    ]);
    let found = attribute(&dir, "app.js", "f");
    assert_files(found.clone(), &["good.js"]);
    assert_eq!(in_repo(&found).spec, "./good");
    // Control: the site inside `other` does see its own `require`.
    let inner = write_repo(&[
        (
            "app.js",
            "import { f } from './good';\n\
             function other() { const f = require('./bad'); return f(1); }\n",
        ),
        ("good.js", ""),
        ("bad.js", ""),
    ]);
    assert_unknown(attribute(&inner, "app.js", "f"));
}

#[test]
fn nested_require_with_a_different_name_does_not_leak_either() {
    let dir = write_repo(&[
        (
            "app.js",
            "const lib = require('./cjs');\n\
             function other() { const lib = require('./decoy'); return lib; }\n\
             export function h(x) { lib.k(x); }\n",
        ),
        ("cjs.js", ""),
        ("decoy.js", ""),
    ]);
    assert_files(attribute(&dir, "app.js", "k"), &["cjs.js"]);
}

#[test]
fn an_exported_require_is_a_program_import() {
    let dir = write_repo(&[
        (
            "app.js",
            "export const lib = require('./cjs');\nexport function h(x) { lib.k(x); }\n",
        ),
        ("cjs.js", ""),
    ]);
    assert_files(attribute(&dir, "app.js", "k"), &["cjs.js"]);
}

const PATHS_PARENT: &str = r#"{"compilerOptions":{"baseUrl":"src","paths":{"@lib/*":["lib/*"]}}}"#;

#[test]
fn extends_array_merges_parents_field_by_field() {
    for order in [
        r#"["./tsconfig.paths.json","./tsconfig.strict.json"]"#,
        r#"["./tsconfig.strict.json","./tsconfig.paths.json"]"#,
    ] {
        let dir = write_repo(&[
            ("tsconfig.json", &format!(r#"{{"extends":{order}}}"#)),
            ("tsconfig.paths.json", PATHS_PARENT),
            (
                "tsconfig.strict.json",
                r#"{"compilerOptions":{"strict":true}}"#,
            ),
            (
                "app.ts",
                "import { f } from '@lib/cmd';\nexport function h(x: string) { f(x); }\n",
            ),
            ("src/lib/cmd.ts", ""),
        ]);
        assert_files(attribute(&dir, "app.ts", "f"), &["src/lib/cmd.ts"]);
    }
    // A later parent overrides only the fields it sets: its `paths` win, the
    // earlier parent's `baseUrl` stays.
    let dir = write_repo(&[
        (
            "tsconfig.json",
            r#"{"extends":["./tsconfig.paths.json","./tsconfig.over.json"]}"#,
        ),
        ("tsconfig.paths.json", PATHS_PARENT),
        (
            "tsconfig.over.json",
            r#"{"compilerOptions":{"paths":{"@lib/*":["util/*"]}}}"#,
        ),
        (
            "app.ts",
            "import { f } from '@lib/cmd';\nexport function h(x: string) { f(x); }\n",
        ),
        ("src/util/cmd.ts", ""),
        ("util/cmd.ts", ""),
        ("src/lib/cmd.ts", ""),
    ]);
    assert_files(attribute(&dir, "app.ts", "f"), &["src/util/cmd.ts"]);
}

#[test]
fn unavailable_config_is_unknown_never_external() {
    let app = "import { f } from '@lib/cmd';\nimport { g } from 'express';\nexport function h(x: string) { f(x); g(x); }\n";
    let cases: [(&str, &[(&str, &str)]); 5] = [
        // A parent the snapshot did not capture (its name is not a tsconfig name).
        (
            r#"{"extends":"./configs/base.json"}"#,
            &[("configs/base.json", PATHS_PARENT)],
        ),
        // A parent outside the repository.
        (r#"{"extends":"../shared/tsconfig.json"}"#, &[]),
        // An unparseable owning config.
        (r#"{"compilerOptions": {"#, &[]),
        // A cycle.
        (
            r#"{"extends":"./tsconfig.a.json"}"#,
            &[
                ("tsconfig.a.json", r#"{"extends":"./tsconfig.b.json"}"#),
                ("tsconfig.b.json", r#"{"extends":"./tsconfig.a.json"}"#),
            ],
        ),
        // A listed parent that is itself unparseable.
        (
            r#"{"extends":"./tsconfig.a.json"}"#,
            &[("tsconfig.a.json", "not json")],
        ),
    ];
    for (config, extra) in cases {
        let mut files = vec![("tsconfig.json", config), ("app.ts", app)];
        files.extend_from_slice(extra);
        files.push(("src/lib/cmd.ts", ""));
        let dir = write_repo(&files);
        assert_unknown(attribute(&dir, "app.ts", "f"));
        assert_unknown(attribute(&dir, "app.ts", "g"));
    }
}

#[test]
fn extends_of_a_package_and_a_self_contained_config_stay_usable() {
    // A package parent (`@tsconfig/...`) is not in the repository and sets no
    // `paths`: ignored, not "unavailable".
    let dir = write_repo(&[
        (
            "tsconfig.json",
            r#"{"extends":"@tsconfig/node18/tsconfig.json","compilerOptions":{"baseUrl":"src","paths":{"@lib/*":["lib/*"]}}}"#,
        ),
        (
            "app.ts",
            "import { f } from '@lib/cmd';\nimport { g } from 'express';\nexport function h(x: string) { f(x); g(x); }\n",
        ),
        ("src/lib/cmd.ts", ""),
    ]);
    assert_files(attribute(&dir, "app.ts", "f"), &["src/lib/cmd.ts"]);
    assert_external(attribute(&dir, "app.ts", "g"));
}

#[test]
fn explicit_base_url_outside_the_repo_does_not_fall_back_to_the_config_directory() {
    let app = "import { f } from '@lib/cmd';\nimport { g } from 'express';\nimport { k } from 'lib/cmd';\n\
               export function h(x: string) { f(x); g(x); k(x); }\n";
    let decoy = write_repo(&[
        (
            "tsconfig.json",
            r#"{"compilerOptions":{"baseUrl":"../shared","paths":{"@lib/*":["lib/*"]}}}"#,
        ),
        ("app.ts", app),
        ("lib/cmd.ts", "export function f() {}\n"),
    ]);
    // `paths` targets hang off the unusable base: no repo file, no guess.
    assert_unknown(attribute(&decoy, "app.ts", "f"));
    // A specifier no key matches is probed under the base, outside the repo:
    // the decoy `lib/cmd.ts` is not it, and `express` stays a package.
    assert_external(attribute(&decoy, "app.ts", "g"));
    assert_external(attribute(&decoy, "app.ts", "k"));
    // Control: a base that normalises inside the repo is used.
    let inside = write_repo(&[
        (
            "tsconfig.json",
            r#"{"compilerOptions":{"baseUrl":"./src/../root","paths":{"@lib/*":["lib/*"]}}}"#,
        ),
        ("app.ts", app),
        ("root/lib/cmd.ts", "export function f() {}\n"),
        ("lib/cmd.ts", "export function f() {}\n"),
    ]);
    let found = attribute(&inside, "app.ts", "f");
    assert_eq!(in_repo(&found).files, ["root/lib/cmd.ts"]);
    assert_files(attribute(&inside, "app.ts", "k"), &["root/lib/cmd.ts"]);
}

#[test]
fn paths_longest_prefix_tie_takes_the_first_key() {
    // `@/*` and `@/*cmd` both match with a two-byte literal prefix.
    let config =
        r#"{"compilerOptions":{"baseUrl":".","paths":{"@/*":["src/*"],"@/*cmd":["alt/*cmd"]}}}"#;
    let dir = write_repo(&[
        ("tsconfig.json", config),
        (
            "app.ts",
            "import { f } from '@/lib/cmd';\nexport function h(x: string) { f(x); }\n",
        ),
        ("src/lib/cmd.ts", ""),
        ("alt/lib/cmd.ts", ""),
    ]);
    assert_files(attribute(&dir, "app.ts", "f"), &["src/lib/cmd.ts"]);
}
