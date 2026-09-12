//! Retain enumerable rejected CJS names; no unknown-surface wildcard policy.
use super::module_binding_audit_test::{check, Disposition};
use prism::{
    ast::ParsedFile, call_graph::CallGraph, languages::Language, resolution::ResolutionConfidence,
};
use std::collections::BTreeMap;

pub(super) const LANGUAGES: [(Language, &str); 3] = [
    (Language::JavaScript, "js"),
    (Language::TypeScript, "ts"),
    (Language::Tsx, "tsx"),
];
pub(super) const CJS: &str =
    "function origin(input) { return input; }\nmodule.exports.item = origin;";

fn barrel(id: &str, bad: &str, bridge: &str, expected: bool) {
    let mut failures = Vec::new();
    for (lang, ext) in LANGUAGES {
        let files: BTreeMap<_,_> = [
            ("bad",bad), ("good","export function item(input) { return input; }"),
            ("bridge",bridge), ("barrel","export * from './bridge'; export * from './good';"),
            ("app","import { item as invoke } from './barrel'; function run(value) { return invoke(value); }"),
            ("decoy","function invoke(input) { return input; }"),
        ].into_iter().map(|(p,s)| {let p=format!("{p}.{ext}");let parsed=ParsedFile::parse(&p,s,lang).unwrap();assert!(!parsed.tree.root_node().has_error(),"{id}/{p}");(p,parsed)}).collect();
        let mut subset = CallGraph::build_direct_subset(&files, &files.keys().cloned().collect());
        subset.apply_js_export_resolution();
        for (mode, cg) in [
            ("full", CallGraph::build(&files)),
            ("subset+exports", subset),
        ] {
            let sites: Vec<_> = cg
                .calls
                .values()
                .flatten()
                .filter(|s| s.caller.file == format!("app.{ext}") && s.callee_name == "invoke")
                .collect();
            assert_eq!(sites.len(), 1);
            let exact: Vec<_> = cg
                .resolve_call_site_full(sites[0])
                .resolved
                .into_iter()
                .filter(|r| r.confidence == ResolutionConfidence::Exact)
                .map(|r| {
                    (
                        r.target.file.clone(),
                        r.target.name.clone(),
                        r.target.start_line,
                    )
                })
                .collect();
            println!("REFUSAL_AUDIT {id}/{ext}/{mode} {expected} {exact:?}");
            let want = if expected {
                vec![(format!("good.{ext}"), "item".into(), 1)]
            } else {
                vec![]
            };
            if exact != want {
                failures.push(format!("{ext}/{mode}: {exact:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{id}: {}", failures.join("; "));
}
macro_rules! rejected {
    ($id:ident, $suffix:expr) => {
        #[test]
        fn $id() {
            barrel(
                stringify!($id),
                &format!("{CJS}\n{}", $suffix),
                "export * from './bad';",
                false,
            );
        }
    };
}
rejected!(
    cjs_refusal_alias,
    "const alias = module.exports; alias.item = other;"
);
rejected!(cjs_refusal_alias_read_only, "const alias = module.exports;");
rejected!(cjs_refusal_computed, "module.exports[key] = other;");
rejected!(cjs_refusal_expression, "module.exports.item = () => 0;");
rejected!(
    cjs_refusal_reflective,
    "Object.assign(module.exports, other);"
);
rejected!(cjs_refusal_delete, "delete module.exports.item;");
rejected!(
    cjs_refusal_nested,
    "function change() { module.exports.item = other; }"
);
rejected!(cjs_refusal_replacement, "module.exports = other;");
rejected!(cjs_refusal_this, "this.item = other;");
rejected!(cjs_refusal_arguments, "arguments[0].item = other;");
rejected!(cjs_refusal_eval, "eval('change()');");
rejected!(cjs_refusal_escaped, "module.exports.it\\u0065m = other;");
rejected!(
    cjs_refusal_unsupported_sibling,
    "module.exports.other = () => 0;"
);

#[test]
fn cjs_refusal_named_then_star() {
    barrel(
        "named_then_star",
        &format!("{CJS} const alias = module.exports;"),
        "export { item } from './bad';",
        false,
    );
}
#[test]
fn cjs_refusal_import_forward_then_star() {
    barrel(
        "import_forward_then_star",
        &format!("{CJS} const alias = module.exports;"),
        "import { item as local } from './bad'; export { local as item };",
        false,
    );
}
#[test]
fn cjs_refusal_whole_object_sibling() {
    barrel("whole_object_sibling","function origin(input) { return input; } module.exports = { item: origin }; const alias = module.exports;","export * from './bad';",false);
}
#[test]
fn cjs_refusal_default_named_path() {
    barrel("default_named_path","function origin(input) { return input; } module.exports = origin; const alias = module.exports;","export { default as item } from './bad';",false);
}
#[test]
fn cjs_refusal_independent_esm() {
    for cjs in [
        "exports.item = origin; const alias = exports;",
        "exports.other = origin; const alias = exports;",
    ] {
        check("refusal_independent_esm",&format!("function origin(input) {{ return input; }}\nexport {{ origin as item }}; {cjs}"),"","import { item as invoke } from './origin'; function run(value) { return invoke(value); }",Disposition::Supported);
    }
}
#[test]
fn cjs_refusal_absent_name_controls() {
    for bad in [
        "",
        "export const value = 1;",
        "export class Client { method() { return this; } }",
        "export function unrelated() { return arguments; }",
        "eval('unrelated()');",
        "function unrelated(module, exports) { module.value = 0; exports.item = 0; }",
        "const exports = {}; exports.item = 0;",
    ] {
        barrel("absent_controls", bad, "export * from './bad';", true);
    }
}
#[test]
fn cjs_refusal_safe_other_name() {
    barrel(
        "safe_other_name",
        "function origin() {} exports.other = origin;",
        "export * from './bad';",
        true,
    );
}

// The scope write proof consumes source binding identity, not display names.
const FORWARD: &str = "import { item as local } from './origin'; export { local as publicName };";
const ESM_APP: &str = "import { publicName as invoke } from './bridge'; function run(value) { return invoke(value); }";
macro_rules! scope {
    ($id:ident, $expected:ident, $source:expr) => {
        #[test]
        fn $id() {
            check(
                stringify!($id),
                $source,
                FORWARD,
                ESM_APP,
                Disposition::$expected,
            );
        }
    };
}
scope!(cjs_refusal_scope_declaration_write,Refused,"function origin(input) { origin = other; return input; }\nfunction other() {} origin(); export { origin as item };");
scope!(cjs_refusal_scope_late_declaration_write,Refused,"function origin(input) { origin = other; return input; }\nfunction other() {} export { origin as item };");
scope!(cjs_refusal_scope_nested_callback_write,Refused,"function origin(input) { later(() => { origin = other; }); return input; }\nexport { origin as item };");
scope!(
    cjs_refusal_scope_parameter_shadow,
    Supported,
    "function origin(origin) { origin = other; return origin; }\nexport { origin as item };"
);
scope!(cjs_refusal_scope_catch_shadow,Supported,"function origin(input) { try {} catch(origin) { origin = other; } return input; }\nexport { origin as item };");
scope!(cjs_refusal_scope_unrelated_shadow,Supported,"function origin(input) { return input; }\nfunction unrelated(origin) { origin = other; } export { origin as item };");

macro_rules! cjs_scope {
    ($id:ident,$expected:ident,$source:expr) => {#[test] fn $id(){check(stringify!($id),$source,"","const { item: invoke } = require('./origin'); function run(value) { return invoke(value); }",Disposition::$expected);}};
}
cjs_scope!(cjs_refusal_scope_anonymous_write,Refused,"let origin = function(input) { origin = other; return input; };\nfunction other() {} origin(); exports.item = origin;");
cjs_scope!(cjs_refusal_scope_anonymous_async_write,Refused,"let origin = async function(input) { origin = other; return input; };\nfunction other() {} origin(); exports.item = origin;");
cjs_scope!(
    cjs_refusal_scope_anonymous_late_write,
    Refused,
    "let origin = function(input) { origin = other; return input; };\nexports.item = origin;"
);
cjs_scope!(
    cjs_refusal_scope_anonymous_parameter,
    Supported,
    "const origin = function(origin) { origin = other; return origin; };\nexports.item = origin;"
);
cjs_scope!(cjs_refusal_scope_named_expression,Supported,"const origin = function origin(input) { origin = other; return input; };\nexports.item = origin;");
cjs_scope!(
    cjs_refusal_scope_plain_anonymous,
    Supported,
    "const origin = function(input) { return input; };\nexports.item = origin;"
);

#[test]
fn cjs_refusal_class_scope_controls() {
    for (lang, ext) in LANGUAGES {
        for (extra, clean) in [
            ("function change() { Client = Other; }", false),
            (
                "const callback = function Client() { Client = Other; };",
                true,
            ),
            (
                "function outer() { function Client() { Client = Other; } }",
                true,
            ),
            ("function change(Client) { Client = Other; }", true),
        ] {
            let path = format!("client.{ext}");
            let source = format!("export class Client {{ m() {{}} }} {extra}");
            let parsed = ParsedFile::parse(&path, &source, lang).unwrap();
            assert!(!parsed.tree.root_node().has_error());
            let cg = CallGraph::build(&BTreeMap::from([(path.clone(), parsed)]));
            assert_eq!(
                cg.clean_class_spans.contains_key(&(path, "Client".into())),
                clean,
                "{ext}: {extra}"
            );
        }
    }
}
