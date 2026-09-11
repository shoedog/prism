//! Named baseline contracts, not a claim that every syntax form is supported.
//! Gap cases assert no Exact target; promoting one requires a captured RED and
//! changing its disposition to Supported. Negatives forbid every Exact target,
//! including same-spelled decoys. No source/compiler oracle is inferred here.
use prism::{
    ast::ParsedFile, call_graph::CallGraph, languages::Language, resolution::ResolutionConfidence,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
enum Disposition {
    Supported,
    SupportedJsParserGapTs,
    Gap,
    Refused,
}

const ESM: &str = "function origin() {}\nexport { origin as item, origin as default };";
const CJS: &str = "function origin() {}\nmodule.exports = { item: origin };";
const APP: &str = "import { publicName as invoke } from './bridge';\nfunction run() { invoke(); }";
const REQUIRE_APP: &str =
    "const { publicName: invoke } = require('./bridge');\nfunction run() { invoke(); }";

fn check(id: &str, origin: &str, bridge: &str, app: &str, expected: Disposition) {
    let mut failures = Vec::new();
    for (lang, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        if matches!(expected, Disposition::SupportedJsParserGapTs) && lang != Language::JavaScript {
            // TypeScript 5.9.3 createSourceFile reports no parse diagnostics for
            // this exact source; the vendored grammar currently rejects it.
            // This is a parser-gap assertion, never a resolution success.
            let parsed = ParsedFile::parse(&format!("bridge.{ext}"), bridge, lang).unwrap();
            assert!(
                parsed.tree.root_node().has_error(),
                "parser gap changed: {id}/{ext}; reassess the resolution contract"
            );
            println!("MODULE_PARSE_GAP {id}/{ext}");
            continue;
        }
        if lang == Language::JavaScript
            && [origin, bridge, app].iter().any(|s| {
                s.contains("import type ")
                    || s.contains("export type ")
                    || s.contains("{ type item")
            })
        {
            continue;
        }
        let sources = [("origin", origin), ("bridge", bridge), ("app", app),
            ("decoy", "function invoke() {}\nfunction local() {}\nfunction item() {}\nfunction publicName() {}")];
        let files: BTreeMap<_, _> = sources
            .into_iter()
            .map(|(stem, source)| {
                let path = format!("{stem}.{ext}");
                let parsed = ParsedFile::parse(&path, source, lang).unwrap();
                assert!(
                    !parsed.tree.root_node().has_error(),
                    "invalid fixture {id}/{path}"
                );
                (path, parsed)
            })
            .collect();
        let mut subset = CallGraph::build_direct_subset(&files, &files.keys().cloned().collect());
        // Raw subsets deliberately omit whole-program export resolution. Match
        // the production post-merge phase before comparing these observations.
        subset.apply_js_export_resolution();
        for (mode, cg) in [
            ("full", CallGraph::build(&files)),
            ("subset+exports", subset),
        ] {
            let callee = if app.contains("ns.item()") {
                "item"
            } else {
                "invoke"
            };
            let sites: Vec<_> = cg
                .calls
                .values()
                .flatten()
                .filter(|s| s.caller.file == format!("app.{ext}") && s.callee_name == callee)
                .collect();
            assert_eq!(sites.len(), 1, "{id}/{ext}/{mode}: call-site population");
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
                        format!("{:?}", r.kind),
                    )
                })
                .collect();
            println!("MODULE_AUDIT {id}/{ext}/{mode} {expected:?} {exact:?}");
            let valid = match expected {
                Disposition::Supported | Disposition::SupportedJsParserGapTs => {
                    exact.len() == 1
                        && exact[0].0 == format!("origin.{ext}")
                        && exact[0].1 == "origin"
                        && exact[0].2 == 1
                }
                Disposition::Gap | Disposition::Refused => exact.is_empty(),
            };
            if !valid {
                failures.push(format!("{ext}/{mode}: {exact:?}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{id}: expected {expected:?}; {}",
        failures.join("; ")
    );
}

macro_rules! case {
    ($id:ident, $disposition:ident, $origin:expr, $bridge:expr, $app:expr) => {
        #[test]
        fn $id() {
            check(
                stringify!($id),
                $origin,
                $bridge,
                $app,
                Disposition::$disposition,
            );
        }
    };
}

// Existing direct export/import routes and comment-invariance controls.
case!(
    esm_named_reexport,
    Supported,
    ESM,
    "export { item as publicName } from './origin';",
    APP
);
case!(
    esm_named_reexport_comments,
    Supported,
    ESM,
    "export /*a*/ { item /*b*/ as publicName } from /*c*/ './origin';",
    APP
);
case!(
    esm_default_reexport,
    Supported,
    ESM,
    "export { default as publicName } from './origin';",
    APP
);
case!(
    esm_star_reexport,
    Supported,
    "function origin() {}\nexport { origin as publicName };",
    "export * from './origin';",
    APP
);
case!(
    esm_direct_named_import,
    Supported,
    ESM,
    "",
    "import { item as invoke } from './origin';\nfunction run() { invoke(); }"
);
case!(
    esm_direct_named_import_comments,
    Supported,
    ESM,
    "",
    "import /*a*/ { item /*b*/ as invoke } from /*c*/ './origin';\nfunction run() { invoke(); }"
);
case!(
    esm_direct_default_import,
    Supported,
    ESM,
    "",
    "import invoke from './origin';\nfunction run() { invoke(); }"
);
case!(
    esm_namespace_import,
    Gap,
    ESM,
    "",
    "import * as ns from './origin';\nfunction run() { ns.item(); }"
);
case!(
    cjs_destructured_import,
    Supported,
    CJS,
    "",
    "const { item: invoke } = require('./origin');\nfunction run() { invoke(); }"
);
case!(cjs_destructured_import_comments, Supported, CJS, "", "const { item: invoke } = require(/*before*/ './origin' /*after*/);\nfunction run() { invoke(); }");
case!(
    cjs_member_export,
    Supported,
    "function origin() {}\nexports.publicName = origin;",
    "",
    "const { publicName: invoke } = require('./origin');\nfunction run() { invoke(); }"
);
case!(
    cjs_module_member_export,
    Supported,
    "function origin() {}\nmodule.exports.publicName = origin;",
    "",
    "const { publicName: invoke } = require('./origin');\nfunction run() { invoke(); }"
);

// Explicitly measured capability gaps, not aliases silently treated as locals.
case!(
    esm_import_then_export_same,
    Gap,
    ESM,
    "import { item } from './origin'; export { item as publicName };",
    APP
);
case!(
    esm_import_then_export_renamed,
    Gap,
    ESM,
    "import { item as local } from './origin'; export { local as publicName };",
    APP
);
case!(
    esm_default_import_then_export,
    Gap,
    ESM,
    "import local from './origin'; export { local as publicName };",
    APP
);
case!(
    esm_import_then_default_export,
    Gap,
    ESM,
    "import { item as local } from './origin'; export default local;",
    "import invoke from './bridge';\nfunction run() { invoke(); }"
);
case!(
    cjs_require_then_export_same,
    Gap,
    CJS,
    "const { item } = require('./origin'); exports.publicName = item;",
    REQUIRE_APP
);
case!(
    cjs_require_then_export_renamed,
    Gap,
    CJS,
    "const { item: local } = require('./origin'); exports.publicName = local;",
    REQUIRE_APP
);
case!(
    cjs_require_then_export_object,
    Gap,
    CJS,
    "const { item: local } = require('./origin'); module.exports = { publicName: local };",
    REQUIRE_APP
);
case!(
    cjs_require_member_then_export,
    Gap,
    CJS,
    "const local = require('./origin').item; exports.publicName = local;",
    REQUIRE_APP
);
case!(
    cjs_whole_module_forwarding,
    Gap,
    CJS,
    "const local = require('./origin'); module.exports = local;",
    "const { item: invoke } = require('./bridge');\nfunction run() { invoke(); }"
);
case!(
    cjs_direct_whole_module_forwarding,
    Gap,
    CJS,
    "module.exports = require('./origin');",
    "const { item: invoke } = require('./bridge');\nfunction run() { invoke(); }"
);
case!(cjs_extra_local_alias, Gap, CJS, "const { item: local } = require('./origin'); const forwarded = local; exports.publicName = forwarded;", REQUIRE_APP);
case!(esm_extra_local_alias, Gap, ESM, "import { item as local } from './origin'; const forwarded = local; export { forwarded as publicName };", APP);
case!(
    cjs_require_to_esm_export,
    Gap,
    CJS,
    "const { item: local } = require('./origin'); export { local as publicName };",
    APP
);

// Identity/write negatives. These assert absence of ALL Exact targets.
case!(
    missing_module,
    Refused,
    ESM,
    "export { item as publicName } from './missing';",
    APP
);
case!(
    missing_member,
    Refused,
    ESM,
    "export { missing as publicName } from './origin';",
    APP
);
case!(
    named_is_not_default,
    Refused,
    "function origin() {}\nexport default origin;",
    "export { item as publicName } from './origin';",
    APP
);
case!(
    star_is_not_default,
    Refused,
    ESM,
    "export * from './origin';",
    "import invoke from './bridge';\nfunction run() { invoke(); }"
);
case!(duplicate_export, Refused, ESM, "export { item as publicName } from './origin'; export { default as publicName } from './origin';", APP);
case!(
    cycle,
    Refused,
    ESM,
    "export { publicName } from './bridge';",
    APP
);
case!(shadowed_require, Refused, CJS, "", "function require(x) { return {}; }\nconst { item: invoke } = require('./origin');\nfunction run() { invoke(); }");
case!(
    shadowed_exports,
    Refused,
    "function origin() {}\nconst exports = {}; exports.publicName = origin;",
    "",
    "const { publicName: invoke } = require('./origin');\nfunction run() { invoke(); }"
);
case!(
    shadowed_module,
    Refused,
    "function origin() {}\nconst module = {exports: {}}; module.exports.publicName = origin;",
    "",
    "const { publicName: invoke } = require('./origin');\nfunction run() { invoke(); }"
);
case!(
    detached_exports,
    Refused,
    "function origin() {}\nmodule.exports = {}; exports.publicName = origin;",
    "",
    "const { publicName: invoke } = require('./origin');\nfunction run() { invoke(); }"
);
case!(
    overwritten_export,
    Refused,
    "function origin() {}\nexports.publicName = origin; exports.publicName = other;",
    "",
    "const { publicName: invoke } = require('./origin');\nfunction run() { invoke(); }"
);
case!(
    mutable_require,
    Refused,
    CJS,
    "",
    "let { item: invoke } = require('./origin'); invoke = other;\nfunction run() { invoke(); }"
);
case!(
    shadowed_consumer,
    Refused,
    ESM,
    "export { item as publicName } from './origin';",
    "import { publicName as invoke } from './bridge';\nfunction run(invoke) { invoke(); }"
);
case!(forwarded_local_decoy, Refused, ESM, "import { item as local } from './origin';\nfunction wrapper() { function local() {} }\nexport { local as publicName };", APP);
case!(
    type_only_import,
    Refused,
    ESM,
    "",
    "import type { item as invoke } from './origin';\nfunction run() { invoke(); }"
);
case!(
    type_only_reexport,
    Refused,
    ESM,
    "export type { item as publicName } from './origin';",
    APP
);
case!(
    type_only_import_forwarding,
    Refused,
    ESM,
    "import type { item as local } from './origin'; export { local as publicName };",
    APP
);
case!(
    cjs_same_name_forwarding,
    Gap,
    "function origin() {}\nexports.publicName = origin;",
    "const { publicName } = require('./origin'); exports.publicName = publicName;",
    REQUIRE_APP
);
case!(
    esm_same_name_forwarding,
    Gap,
    "function origin() {}\nexport { origin as publicName };",
    "import { publicName } from './origin'; export { publicName };",
    APP
);
case!(
    cjs_forwarding_comment,
    Gap,
    CJS,
    "const { item: local } = require(/*before*/ './origin'); exports.publicName = local;",
    REQUIRE_APP
);
case!(
    require_member_direct,
    Gap,
    CJS,
    "",
    "const invoke = require('./origin').item;\nfunction run() { invoke(); }"
);
case!(
    cjs_whole_value_direct,
    Gap,
    "function origin() {}\nmodule.exports = origin;",
    "",
    "const invoke = require('./origin');\nfunction run() { invoke(); }"
);

#[test]
fn module_labels_ignore_comment_trivia() {
    let mut failures = Vec::new();
    for lang in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for source in [
            "const local = require(COMMENT'./origin');",
            "const { item: local } = require(COMMENT'./origin');",
            "const local = require(COMMENT'./origin').item;",
            "const local = require(COMMENT'./origin')();",
        ] {
            for trivia in ["", "/*before*/ ", "//before\n"] {
                let source = source.replace("COMMENT", trivia);
                let parsed = ParsedFile::parse("app.ts", &source, lang).unwrap();
                assert!(!parsed.tree.root_node().has_error());
                let imports = parsed.extract_imports();
                println!("MODULE_LABEL {lang:?} {source:?} {imports:?}");
                if imports.get("local").map(String::as_str) != Some("./origin") {
                    failures.push(format!("{lang:?}: {source} => {imports:?}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

case!(
    mixed_type_only_reexport,
    Refused,
    ESM,
    "export { type item as publicName, default as ordinary } from './origin';",
    APP
);
case!(
    mixed_value_reexport_control,
    Supported,
    ESM,
    "export { type item as ignored, default as publicName } from './origin';",
    APP
);
case!(
    value_named_type_control,
    SupportedJsParserGapTs,
    "function origin() {}\nexport { origin as type };",
    "export { type as publicName } from './origin';",
    APP
);
case!(assigned_require, Refused, CJS, "", "require = replacement; const { item: invoke } = require('./origin');\nfunction run() { invoke(); }");
case!(
    assigned_exports,
    Refused,
    "function origin() {}\nexports = {}; exports.publicName = origin;",
    "",
    "const { publicName: invoke } = require('./origin');\nfunction run() { invoke(); }"
);
case!(
    assigned_module,
    Refused,
    "function origin() {}\nmodule = {exports:{}}; module.exports.publicName = origin;",
    "",
    "const { publicName: invoke } = require('./origin');\nfunction run() { invoke(); }"
);
case!(
    nested_ambient_shadow_control,
    Supported,
    "function origin() {}\nfunction unrelated(module, exports) {}\nexports.publicName = origin;",
    "",
    "const { publicName: invoke } = require('./origin');\nfunction run() { invoke(); }"
);
case!(
    destructured_local_decoy,
    Refused,
    ESM,
    "",
    "const { item: invoke } = unknown;\nfunction run() { invoke(); }"
);

#[test]
fn proven_local_callables_survive_fallback_barrier() {
    for source in [
        "function invoke() {}\nfunction run() { invoke(); }",
        "const invoke = () => {};\nfunction run() { invoke(); }",
        "function run() { function invoke() {} invoke(); }",
    ] {
        let parsed = ParsedFile::parse("local.js", source, Language::JavaScript).unwrap();
        let cg = CallGraph::build(&BTreeMap::from([("local.js".to_owned(), parsed)]));
        let site = cg
            .calls
            .values()
            .flatten()
            .find(|s| s.callee_name == "invoke")
            .unwrap();
        let resolved = cg.resolve_call_site_full(site).resolved;
        assert_eq!(resolved.len(), 1, "{source}: {resolved:?}");
        assert_eq!(resolved[0].confidence, ResolutionConfidence::Exact);
        assert_eq!(resolved[0].target.file, "local.js");
        assert_eq!(resolved[0].target.name, "invoke");
    }
}

#[test]
fn module_value_refusal_facts_survive_serialization() {
    let parsed = ParsedFile::parse(
        "app.ts",
        "const { item: invoke } = unknown;",
        Language::TypeScript,
    )
    .unwrap();
    let facts = parsed.extract_js_ts_export_facts();
    let json = serde_json::to_value(&facts).unwrap();
    assert_eq!(json["module_value_bindings"], serde_json::json!(["invoke"]));
    let restored: prism::js_exports::JsExportFacts = serde_json::from_value(json).unwrap();
    assert_eq!(restored, facts);
    let bytes = bincode::serialize(&facts).unwrap();
    let restored: prism::js_exports::JsExportFacts = bincode::deserialize(&bytes).unwrap();
    assert_eq!(restored, facts);
}
