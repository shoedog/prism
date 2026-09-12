//! Callable-only ESM forwarding proof. Every positive names the terminal file,
//! declaration and line; refusal cases include same-spelled decoys.
use super::module_binding_audit_test::{check, Disposition};
use prism::{ast::ParsedFile, call_graph::CallGraph, languages::Language};
use std::collections::BTreeMap;

const ORIGIN: &str =
    "function origin(input) { return input; }\nexport { origin as item, origin as default };";
const BRIDGE: &str = "import { item as local } from './origin'; export { local as publicName };";
const APP: &str = "import { publicName as invoke } from './bridge';\nfunction run(value) { return invoke(value); }";

macro_rules! fixture {
    ($name:ident, $expected:ident, $origin:expr, $bridge:expr, $app:expr) => {
        #[test]
        fn $name() {
            check(
                stringify!($name),
                $origin,
                $bridge,
                $app,
                Disposition::$expected,
            );
        }
    };
}

fixture!(
    forward_default_specifier,
    Supported,
    ORIGIN,
    "import { default as local } from './origin'; export { local as publicName };",
    APP
);
fixture!(
    forward_default_export_list,
    Supported,
    ORIGIN,
    "import { item as local } from './origin'; export { local as default };",
    "import invoke from './bridge';\nfunction run(value) { return invoke(value); }"
);
fixture!(
    forward_default_to_default,
    Supported,
    ORIGIN,
    "import local from './origin'; export default local;",
    "import invoke from './bridge';\nfunction run(value) { return invoke(value); }"
);
fixture!(forward_comments, Supported, ORIGIN,
    "import /*a*/ { item /*b*/ as local } from /*c*/ './origin'; export /*d*/ { local /*e*/ as publicName };", APP);
fixture!(
    forward_export_before_import,
    Supported,
    ORIGIN,
    "export { local as publicName }; import { item as local } from './origin';",
    APP
);
fixture!(
    forward_direct_function_export,
    Supported,
    "export function origin(input) { return input; }",
    "import { origin as local } from './origin'; export { local as publicName };",
    APP
);
fixture!(
    forward_default_function_export,
    Supported,
    "export default function origin(input) { return input; }",
    "import local from './origin'; export { local as publicName };",
    APP
);

fixture!(forward_duplicate_import, Refused, ORIGIN,
    "import { item as local } from './origin'; import { item as local } from './origin'; export { local as publicName };", APP);
fixture!(
    forward_duplicate_same_clause,
    Refused,
    ORIGIN,
    "import { item as local, default as local } from './origin'; export { local as publicName };",
    APP
);
fixture!(forward_competing_value, Refused, ORIGIN,
    "import { item as local } from './origin'; const local = other; export { local as publicName };", APP);
fixture!(forward_destructured_competitor, Refused, ORIGIN,
    "import { item as local } from './origin'; const { x: local } = other; export { local as publicName };", APP);
fixture!(
    forward_written_import,
    Refused,
    ORIGIN,
    "import { item as local } from './origin'; local = other; export { local as publicName };",
    APP
);
fixture!(forward_escaping_write, Refused, ORIGIN,
    "import { item as local } from './origin'; function change() { local = other; } export { local as publicName };", APP);
fixture!(forward_duplicate_export, Refused, ORIGIN,
    "import { item as local } from './origin'; export { local as publicName }; export { local as publicName };", APP);
fixture!(
    forward_missing_origin,
    Refused,
    ORIGIN,
    "import { item as local } from './missing'; export { local as publicName };",
    APP
);
fixture!(
    forward_missing_member,
    Refused,
    ORIGIN,
    "import { missing as local } from './origin'; export { local as publicName };",
    APP
);
fixture!(
    forward_external_origin,
    Refused,
    ORIGIN,
    "import { item as local } from 'origin'; export { local as publicName };",
    APP
);
fixture!(
    forward_type_import,
    Refused,
    ORIGIN,
    "import type { item as local } from './origin'; export { local as publicName };",
    APP
);
fixture!(
    forward_mixed_type_import,
    Refused,
    ORIGIN,
    "import { type item as local } from './origin'; export { local as publicName };",
    APP
);
fixture!(
    forward_type_export,
    Refused,
    ORIGIN,
    "import { item as local } from './origin'; export type { local as publicName };",
    APP
);
fixture!(forward_type_collision, Refused, ORIGIN,
    "import { item as local } from './origin'; import type { item as local } from './origin'; export { local as publicName };", APP);
fixture!(
    forward_unstable_origin,
    Refused,
    "function origin(input) { return input; }\nexport { origin as item }; origin = other;",
    BRIDGE,
    APP
);
fixture!(forward_nested_origin_write, Refused,
    "function origin(input) { return input; }\nexport { origin as item }; function change() { origin = other; }", BRIDGE, APP);
fixture!(forward_duplicate_origin, Refused,
    "function origin(input) { return input; }\nfunction origin(input) { return 2; }\nexport { origin as item };", BRIDGE, APP);
fixture!(forward_nested_origin_decoy, Refused,
    "function origin(input) { return input; }\nfunction outer() { function origin(input) { return 2; } }\nexport { origin as item };", BRIDGE, APP);
fixture!(forward_origin_alias_not_declaration, Refused,
    "const origin = other;\nfunction outer() { function origin(input) { return 2; } }\nexport { origin as item };", BRIDGE, APP);
fixture!(
    forward_origin_class_not_callable,
    Refused,
    "export class origin {}\nexport { origin as item };",
    BRIDGE,
    APP
);
fixture!(
    forward_origin_arrow_deferred,
    Refused,
    "const origin = (input) => input;\nexport { origin as item };",
    BRIDGE,
    APP
);
fixture!(
    forward_cjs_origin_deferred,
    Refused,
    "function origin(input) { return input; }\nexports.item = origin;",
    BRIDGE,
    APP
);
fixture!(
    forward_esm_to_cjs_deferred,
    Refused,
    ORIGIN,
    "import { item as local } from './origin'; exports.publicName = local;",
    APP
);
fixture!(
    forward_eval_deferred,
    Refused,
    ORIGIN,
    "import { item as local } from './origin'; eval(''); export { local as publicName };",
    APP
);

#[test]
fn forward_type_declarations_and_recovery_refuse() {
    for (lang, ext) in [(Language::TypeScript, "ts"), (Language::Tsx, "tsx")] {
        for addition in [
            "interface local {}",
            "namespace local {}",
            "enum local { A }",
            "const broken = ;",
        ] {
            let source = format!("{BRIDGE}\n{addition}");
            let parsed = ParsedFile::parse(&format!("bridge.{ext}"), &source, lang).unwrap();
            let facts = parsed.extract_js_ts_export_facts();
            let files = BTreeMap::from([(format!("bridge.{ext}"), facts)]);
            let out = prism::js_exports::resolve_js_exports(&files, &|_, _| {
                Some(format!("origin.{ext}"))
            });
            assert!(out.resolved.is_empty());
            // A missing origin alone also produces no resolution. Assert the
            // producer refused the imported-local fact itself.
            assert!(files[&format!("bridge.{ext}")]
                .conflicted
                .contains("publicName"));
        }
    }
}

#[test]
fn forward_origin_proof_serializes() {
    let parsed = ParsedFile::parse("origin.ts", ORIGIN, Language::TypeScript).unwrap();
    let facts = parsed.extract_js_ts_export_facts();
    let json = serde_json::to_value(&facts).unwrap();
    assert_eq!(
        json["forwardable_function_locals"],
        serde_json::json!(["origin"])
    );
    let bytes = bincode::serialize(&facts).unwrap();
    let restored: prism::js_exports::JsExportFacts = bincode::deserialize(&bytes).unwrap();
    assert_eq!(restored, facts);
}

#[test]
fn forward_parameter_flow_reaches_origin() {
    super::module_binding_flow_test::flows(ORIGIN, BRIDGE, APP, &[("origin.ts", "origin")]);
}

#[test]
fn forward_chain_depth_cycle_and_serialized_graph() {
    for bridges in [1, 2, 3] {
        let mut files = BTreeMap::from([
            (
                "origin.ts".into(),
                ParsedFile::parse("origin.ts", ORIGIN, Language::TypeScript).unwrap(),
            ),
            (
                "app.ts".into(),
                ParsedFile::parse("app.ts", APP, Language::TypeScript).unwrap(),
            ),
        ]);
        for i in 0..bridges {
            let name = if i == 0 {
                "bridge.ts".to_owned()
            } else {
                format!("bridge{i}.ts")
            };
            let (path, member) = if i + 1 == bridges {
                ("./origin".to_owned(), "item")
            } else {
                (format!("./bridge{}", i + 1), "publicName")
            };
            let source = format!(
                "import {{ {member} as local }} from '{path}'; export {{ local as publicName }};"
            );
            files.insert(
                name.clone(),
                ParsedFile::parse(&name, &source, Language::TypeScript).unwrap(),
            );
        }
        let cg = CallGraph::build(&files);
        let mut restored: CallGraph =
            bincode::deserialize(&bincode::serialize(&cg).unwrap()).unwrap();
        restored.apply_js_export_resolution();
        for cg in [cg, restored] {
            let target = cg
                .js_ts_resolved_exports
                .get("bridge.ts")
                .and_then(|e| e.get("publicName"));
            assert_eq!(target.is_some(), bridges <= 2, "{bridges} hops");
            if let Some(target) = target {
                assert_eq!(
                    (&*target.file, &*target.local_name),
                    ("origin.ts", "origin")
                );
            }
        }
    }
    check(
        "forward_cycle",
        ORIGIN,
        "import { publicName as local } from './bridge'; export { local as publicName };",
        APP,
        Disposition::Refused,
    );
}

fixture!(forward_nearer_parameter_write, Supported, ORIGIN,
    "import { item as local } from './origin'; function unrelated(local) { local = other; } export { local as publicName };", APP);
fixture!(forward_nearer_catch_write, Supported, ORIGIN,
    "import { item as local } from './origin'; try {} catch (local) { local = other; } export { local as publicName };", APP);
fixture!(
    forward_origin_parameter_shadow,
    Supported,
    "function origin(origin) { return origin; }\nexport { origin as item };",
    BRIDGE,
    APP
);
fixture!(
    forward_origin_require_use,
    Supported,
    "function origin(input) { return require(input); }\nexport { origin as item };",
    BRIDGE,
    APP
);
fixture!(
    forward_origin_eval_refusal,
    Refused,
    "function origin(input) { return input; }\nexport { origin as item }; eval('');",
    BRIDGE,
    APP
);
fixture!(
    forward_origin_generator_deferred,
    Refused,
    "function* origin(input) { yield input; }\nexport { origin as item };",
    BRIDGE,
    APP
);
fixture!(
    forward_namespace_import_deferred,
    Refused,
    ORIGIN,
    "import * as local from './origin'; export { local as publicName };",
    APP
);
fixture!(
    forward_with_deferred,
    Refused,
    ORIGIN,
    "import { item as local } from './origin'; with (other) {} export { local as publicName };",
    APP
);
