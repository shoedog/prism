//! Source/capture proof for direct CJS callable terminals; not require snapshots.
use super::module_binding_audit_test::{check, Disposition};
use prism::{
    ast::ParsedFile, call_graph::CallGraph, languages::Language, resolution::ResolutionConfidence,
};
use std::collections::BTreeMap;

const APP: &str =
    "const { item: invoke } = require('./origin');\nfunction run(value) { return invoke(value); }";
macro_rules! case {
    ($id:ident, $expected:ident, $source:expr) => {
        #[test]
        fn $id() {
            check(stringify!($id), $source, "", APP, Disposition::$expected);
        }
    };
}
case!(
    terminal_function,
    Supported,
    "function origin(input) { return input; }\nexports.item = origin;"
);
case!(
    terminal_hoisted_function,
    Supported,
    "exports.item = origin; function origin(input) { return input; }"
);
case!(
    terminal_async_function,
    Supported,
    "async function origin(input) { return input; }\nexports.item = origin;"
);
case!(
    terminal_generator,
    Supported,
    "function* origin(input) { yield input; }\nexports.item = origin;"
);
case!(
    terminal_async_generator,
    Supported,
    "async function* origin(input) { yield input; }\nexports.item = origin;"
);
case!(
    terminal_const_arrow,
    Supported,
    "const origin = (input) => input;\nexports.item = origin;"
);
case!(
    terminal_let_arrow,
    Supported,
    "let origin = (input) => input;\nexports.item = origin;"
);
case!(
    terminal_var_arrow,
    Supported,
    "var origin = (input) => input;\nexports.item = origin;"
);
case!(
    terminal_function_expression,
    Supported,
    "const origin = function(input) { return input; };\nexports.item = origin;"
);
case!(
    terminal_named_expression_same_name,
    Supported,
    "const origin = function origin(input) { return input; };\nexports.item = origin;"
);
case!(terminal_comments, Supported, "const /*a*/ origin /*b*/ = /*c*/ (input) => input;\nmodule.exports = { /*d*/ item: /*e*/ origin };");
case!(terminal_post_capture_rebind, Supported, "function origin(input) { return input; }\nfunction other() {} exports.item = origin; origin = other;");
case!(
    terminal_arrow_post_capture_rebind,
    Supported,
    "let origin = (input) => input;\nfunction other() {} exports.item = origin; origin = other;"
);
case!(terminal_shadow_write, Supported, "function origin(input) { return input; }\nfunction unrelated(origin) { origin = 0; } exports.item = origin;");
case!(terminal_sibling_value, Supported, "function origin(input) { return input; }\nconst version = 1; module.exports = { item: origin, version };");
case!(
    terminal_nested_only,
    Refused,
    "function holder() { function origin(input) { return input; } }\nexports.item = origin;"
);
case!(terminal_nested_decoy, Refused, "function origin(input) { return input; }\nfunction holder() { function origin(input) { return input; } } exports.item = origin;");
case!(terminal_duplicate_function, Refused, "function origin(input) { return input; }\nfunction origin(input) { return 0; } exports.item = origin;");
case!(
    terminal_block_function,
    Refused,
    "if (false) { function origin(input) { return input; } }\nexports.item = origin;"
);
case!(
    terminal_duplicate_value,
    Refused,
    "function origin(input) { return input; }\nvar origin = 0; exports.item = origin;"
);
case!(
    terminal_early_const,
    Refused,
    "exports.item = origin; const origin = (input) => input;"
);
case!(
    terminal_early_let,
    Refused,
    "exports.item = origin; let origin = (input) => input;"
);
case!(
    terminal_early_var,
    Refused,
    "exports.item = origin; var origin = (input) => input;"
);
case!(
    terminal_early_expression,
    Refused,
    "exports.item = origin; const origin = function(input) { return input; };"
);
case!(terminal_uninitialized, Refused, "let origin; function holder() { function origin(input) { return input; } } exports.item = origin;");
case!(terminal_pre_capture_rebind, Refused, "function origin(input) { return input; }\nfunction other() {} origin = other; exports.item = origin;");
case!(
    terminal_arrow_pre_capture_rebind,
    Refused,
    "let origin = (input) => input;\nfunction other() {} origin = other; exports.item = origin;"
);
case!(terminal_escaping_write, Refused, "function origin(input) { return input; }\nfunction change() { origin = other; } change(); exports.item = origin;");
case!(terminal_late_hoisted_write, Refused, "function origin(input) { return input; }\nchange(); exports.item = origin; function change() { origin = other; }");
case!(
    terminal_conditional_write,
    Refused,
    "function origin(input) { return input; }\nif (true) origin = other; exports.item = origin;"
);
case!(
    terminal_destructuring_write,
    Refused,
    "function origin(input) { return input; }\n[origin] = other; exports.item = origin;"
);
case!(
    terminal_compound_write,
    Refused,
    "function origin(input) { return input; }\norigin += 1; exports.item = origin;"
);
case!(
    terminal_update_write,
    Refused,
    "function origin(input) { return input; }\norigin++; exports.item = origin;"
);
case!(
    terminal_loop_write,
    Refused,
    "function origin(input) { return input; }\nfor (origin of values) {} exports.item = origin;"
);
case!(terminal_late_nested_write, Refused, "function origin(input) { return input; }\nexports.item = origin; function change() { origin = other; }");
case!(
    terminal_late_compound_write,
    Refused,
    "function origin(input) { return input; }\nexports.item = origin; origin += 1;"
);
case!(
    terminal_late_arrow_reassignment,
    Refused,
    "function origin(input) { return input; }\nexports.item = origin;\norigin = (input) => input;"
);
case!(
    terminal_wrapped_initializer,
    Refused,
    "const origin = memo((input) => input);\nexports.item = origin;"
);
case!(terminal_named_expression_decoy, Refused, "const origin = function inner(input) { return input; };\nfunction holder() { function origin(input) { return 0; } } exports.item = origin;");
#[test]
fn terminal_type_collision() {
    for (lang, ext) in [(Language::TypeScript, "ts"), (Language::Tsx, "tsx")] {
        let files: BTreeMap<_, _> = [("origin", "function origin(input) { return input; }\ntype origin = string; exports.item = origin;"), ("app", APP)].into_iter().map(|(p,s)| {
            let p = format!("{p}.{ext}");
            let parsed = ParsedFile::parse(&p, s, lang).unwrap();
            assert!(!parsed.tree.root_node().has_error());
            (p, parsed)
        }).collect();
        let mut subset = CallGraph::build_direct_subset(&files, &files.keys().cloned().collect());
        subset.apply_js_export_resolution();
        for cg in [CallGraph::build(&files), subset] {
            let site = cg
                .calls
                .values()
                .flatten()
                .find(|s| s.callee_name == "invoke")
                .unwrap();
            assert!(cg
                .resolve_call_site_full(site)
                .resolved
                .iter()
                .all(|r| r.confidence != ResolutionConfidence::Exact));
        }
    }
}
case!(terminal_duplicate_unproven_first, Refused, "function origin(input) { return input; }\nconst value = 1; exports.item = value; exports.item = origin;");
case!(terminal_duplicate_unproven_last, Refused, "function origin(input) { return input; }\nconst value = 1; exports.item = origin; exports.item = value;");

#[test]
fn terminal_each_capture_is_independent() {
    let source = "function origin(input) { return input; }\nfunction other() {} exports.item = origin; origin = other; exports.later = origin;";
    check(
        "terminal_first_capture",
        source,
        "",
        APP,
        Disposition::Supported,
    );
    check(
        "terminal_later_capture",
        source,
        "",
        &APP.replace("item:", "later:"),
        Disposition::Refused,
    );
}

#[test]
fn terminal_default_and_shorthand_nested_decoy() {
    let prefix = "function holder() { function origin(input) { return input; } }\n";
    check(
        "terminal_default_decoy",
        &format!("{prefix}module.exports = origin;"),
        "",
        "import invoke from './origin'; function run(value) { return invoke(value); }",
        Disposition::Refused,
    );
    check(
        "terminal_shorthand_decoy",
        &format!("{prefix}module.exports = {{ origin }};"),
        "",
        &APP.replace("item:", "origin:"),
        Disposition::Refused,
    );
}

#[test]
fn terminal_import_collision() {
    check("terminal_import_collision", "import { item as origin } from './bridge';\nfunction holder() { function origin(input) { return input; } } exports.item = origin;", "export function item() {}", APP, Disposition::Refused);
}

// An unproven exported name must not disappear into a star-branch absence.
fn barrel(bad: &str, via_named: bool, expected: bool) {
    barrel_with_bridge(
        bad,
        if via_named {
            "export { item } from './bad';"
        } else {
            "export * from './bad';"
        },
        expected,
    );
}

fn barrel_with_bridge(bad: &str, bridge: &str, expected: bool) {
    for (lang, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        let files: BTreeMap<_,_> = [
            ("bad", bad), ("good", "export function item(input) { return input; }"),
            ("bridge", bridge),
            ("barrel", "export * from './bridge'; export * from './good';"),
            ("app", "import { item as invoke } from './barrel'; function run(value) { return invoke(value); }"),
        ].into_iter().map(|(p,s)| {let p=format!("{p}.{ext}");let f=ParsedFile::parse(&p,s,lang).unwrap();assert!(!f.tree.root_node().has_error());(p,f)}).collect();
        let mut subset = CallGraph::build_direct_subset(&files, &files.keys().cloned().collect());
        subset.apply_js_export_resolution();
        for cg in [CallGraph::build(&files), subset] {
            let site = cg
                .calls
                .values()
                .flatten()
                .find(|s| s.caller.file == format!("app.{ext}") && s.callee_name == "invoke")
                .unwrap();
            let targets: Vec<_> = cg
                .resolve_call_site_full(site)
                .resolved
                .into_iter()
                .filter(|r| r.confidence == ResolutionConfidence::Exact)
                .collect();
            assert_eq!(
                targets.len(),
                usize::from(expected),
                "barrel {ext}, bridge={bridge}, bad={bad}"
            );
            if expected {
                assert_eq!(targets[0].target.file, format!("good.{ext}"));
            }
        }
    }
}
#[test]
fn terminal_unproven_star_claim() {
    barrel("const value = 1; exports.item = value;", false, false);
}
#[test]
fn terminal_unproven_named_then_star_claim() {
    barrel("const value = 1; exports.item = value;", true, false);
}
#[test]
fn terminal_conflicted_star_claim() {
    barrel("export function item() {} export { item };", false, false);
}
#[test]
fn terminal_cjs_conflicted_star_claim() {
    barrel(
        "const value = 1; exports.item = value; exports.item = value;",
        false,
        false,
    );
}
#[test]
fn terminal_cjs_conflicted_named_then_star_claim() {
    barrel(
        "const value = 1; exports.item = value; exports.item = value;",
        true,
        false,
    );
}
#[test]
fn terminal_cjs_conflicted_disjoint_star_claim() {
    barrel("function item(input) { return input; } exports.item = item; exports.other = item; exports.other = item;", false, false);
}
#[test]
fn terminal_absent_star_name_is_not_blocked() {
    barrel("export const value = 1;", false, true);
}

#[test]
fn terminal_rejected_forwarding_arrow_claim() {
    barrel_with_bridge(
        "export const item = (input) => input;",
        "import { item as local } from './bad'; export { local as item };",
        false,
    );
}
#[test]
fn terminal_rejected_forwarding_class_claim() {
    barrel_with_bridge(
        "export class item {}",
        "import { item as local } from './bad'; export { local as item };",
        false,
    );
}

case!(terminal_declaration_self_write, Refused, "function origin(input) { origin = other; return input; }\nfunction other() {} origin(); exports.item = origin;");
case!(terminal_generator_self_write, Refused, "function* origin(input) { origin = other; yield input; }\nfunction other() {} origin().next(); exports.item = origin;");
case!(terminal_expression_self_write, Supported, "const origin = function origin(input) { origin = other; return input; };\nfunction other() {} origin(); exports.item = origin;");
case!(terminal_parameter_self_shadow_write, Supported, "function origin(origin) { origin = other; return origin; }\nfunction other() {} exports.item = origin;");
