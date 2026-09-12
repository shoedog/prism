//! Producer mutation barriers, not require-time snapshot/forwarding authority.
use super::module_binding_audit_test::{check, Disposition};
use prism::{ast::ParsedFile, languages::Language};

const APP: &str =
    "const { item: invoke } = require('./origin');\nfunction run(value) { return invoke(value); }";
const BASE: &str = "function origin(input) { return input; }\nexports.item = origin;\n";

macro_rules! mutation {
    ($id:ident, $suffix:expr) => {
        #[test]
        fn $id() {
            check(
                stringify!($id),
                &format!("{BASE}{}", $suffix),
                "",
                APP,
                Disposition::Refused,
            );
        }
    };
}
mutation!(
    alias_exports,
    "const alias = exports; alias.item = () => 0;"
);
mutation!(
    alias_module,
    "const alias = module; alias.exports.item = () => 0;"
);
mutation!(
    alias_module_exports,
    "const alias = module.exports; alias.item = () => 0;"
);
mutation!(
    shorthand_escape,
    "const box = { exports }; box.exports.item = () => 0;"
);
mutation!(this_alias, "this.item = () => 0;");
mutation!(computed_write, "exports['item'] = () => 0;");
mutation!(computed_module_write, "module.exports['item'] = () => 0;");
mutation!(computed_whole_replacement, "module['exports'] = {};");
mutation!(delete_member, "delete exports.item;");
mutation!(compound_write, "exports.item += 1;");
mutation!(logical_write, "exports.item &&= () => 0;");
mutation!(update_member, "exports.item++;");
mutation!(
    reflective_write,
    "Object.defineProperty(exports, 'item', { value: () => 0 });"
);
mutation!(
    reflective_assign,
    "Object.assign(module.exports, { item: () => 0 });"
);
mutation!(
    argument_escape,
    "function change(o) { o.item = () => 0; } change(exports);"
);
mutation!(
    nested_write,
    "function change() { exports.item = () => 0; } change();"
);
mutation!(conditional_write, "if (true) exports.item = () => 0;");
mutation!(eval_write, "eval('exports.item = () => 0');");
mutation!(expression_overwrite, "exports.item = () => 0;");
mutation!(
    module_expression_overwrite,
    "module.exports.item = () => 0;"
);
mutation!(
    duplicate_identifier,
    "function replacement() {} exports.item = replacement;"
);

macro_rules! source {
    ($id:ident, $expected:ident, $source:expr) => {
        #[test]
        fn $id() {
            check(stringify!($id), $source, "", APP, Disposition::$expected);
        }
    };
}
source!(whole_replacement, Refused, "function origin(input) { return input; }\nmodule.exports = { item: origin }; module.exports = {};");
source!(whole_replacement_expression, Refused, "function origin(input) { return input; }\nmodule.exports = { item: origin }; module.exports = make();");
source!(
    replacement_member_mix,
    Refused,
    "function origin(input) { return input; }\nmodule.exports.item = origin; module.exports = {};"
);
source!(computed_object_collision, Refused, "function origin(input) { return input; }\nmodule.exports = { item: origin, ['item']: () => 0 };");
source!(
    method_object_collision,
    Refused,
    "function origin(input) { return input; }\nmodule.exports = { item: origin, item() {} };"
);
source!(
    expression_object_collision,
    Refused,
    "function origin(input) { return input; }\nmodule.exports = { item: origin, item: () => 0 };"
);
source!(getter_object_collision, Refused, "function origin(input) { return input; }\nmodule.exports = { item: origin, get item() { return () => 0; } };");
source!(safe_exports_member, Supported, BASE);
source!(
    safe_module_member,
    Supported,
    "function origin(input) { return input; }\nmodule.exports.item = origin;"
);
source!(
    safe_module_object,
    Supported,
    "function origin(input) { return input; }\nmodule.exports = { item: origin };"
);
source!(
    safe_comments,
    Supported,
    "function origin(input) { return input; }\nmodule /*a*/ . exports /*b*/ . item = /*c*/ origin;"
);
source!(safe_distinct_members, Supported, "function origin(input) { return input; }\nfunction other() {} exports.item = origin; exports.other = other;");
source!(safe_strings_properties, Supported, "function origin(input) { return input; }\nconst text = 'exports module eval this'; const o = { exports: 1, module: 2 }; exports.item = origin;");
source!(
    safe_quoted_key,
    Supported,
    "function origin(input) { return input; }\nmodule.exports = { 'item': origin };"
);
source!(
    safe_arrow,
    Supported,
    "const origin = (input) => input;\nexports.item = origin;"
);
source!(
    safe_function_expression,
    Supported,
    "const origin = function(input) { return input; };\nexports.item = origin;"
);
source!(
    escaped_key_collision,
    Refused,
    r"function origin(input) { return input; }
module.exports = { item: origin, '\x69tem': origin };"
);
source!(duplicate_object_identifiers, Refused, "function origin(input) { return input; }\nfunction other() {} module.exports = { item: origin, 'item': other };");
source!(replacement_then_member, Refused, "function origin(input) { return input; }\nmodule.exports = { item: origin }; module.exports.other = origin;");
source!(prototype_setter, Refused, "function origin(input) { return input; }\nmodule.exports = { item: origin, __proto__: origin };");
mutation!(with_write, "with (exports) { item = () => 0; }");
mutation!(
    destructured_escape,
    "const { exports: alias } = module; alias.item = () => 0;"
);
mutation!(shadowed_exports_pattern, "const { exports } = other;");

#[test]
fn parse_recovery_revokes_cjs_only() {
    for lang in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let parsed = ParsedFile::parse("origin.ts", &format!("{BASE}const = ;"), lang).unwrap();
        assert!(parsed.tree.root_node().has_error());
        assert!(!parsed
            .extract_js_ts_export_facts()
            .named
            .contains_key("item"));
    }
}

#[test]
fn unsafe_raw_facts_cannot_authorize_export() {
    let source = format!("{BASE}const alias = exports; alias.item = () => 0;");
    let parsed = ParsedFile::parse("origin.js", &source, Language::JavaScript).unwrap();
    let facts = parsed.extract_js_ts_export_facts();
    assert!(
        facts.conflicted.contains("item") || !facts.named.contains_key("item"),
        "unsafe raw target: {facts:?}"
    );
}

#[test]
fn esm_export_is_not_revoked_by_unrelated_cjs_escape() {
    check("esm_independent", "function origin(input) { return input; }\nexport { origin as item }; const alias = exports; alias.other = 0;", "", "import { item as invoke } from './origin';\nfunction run(value) { return invoke(value); }", Disposition::Supported);
}

#[test]
fn esm_same_name_survives_discarded_cjs_facts() {
    check("esm_same_name", "function origin(input) { return input; }\nexport { origin as item }; exports.item = origin; const alias = exports; alias.item = 0;", "", "import { item as invoke } from './origin';\nfunction run(value) { return invoke(value); }", Disposition::Supported);
}

#[test]
fn safe_cross_form_duplicate_is_still_conflicted() {
    check("cross_form_duplicate", "function origin(input) { return input; }\nexport { origin as item }; exports.item = origin;", "", "import { item as invoke } from './origin';\nfunction run(value) { return invoke(value); }", Disposition::Refused);
}

#[test]
fn duplicate_cjs_writes_do_not_poison_esm() {
    check("duplicate_cjs_esm", "function origin(input) { return input; }\nexport { origin as item }; exports.item = origin; exports.item = other;", "", "import { item as invoke } from './origin';\nfunction run(value) { return invoke(value); }", Disposition::Supported);
}

#[test]
fn duplicate_cjs_revokes_disjoint_cjs_fact() {
    check("duplicate_cjs_disjoint", "function origin(input) { return input; }\nexports.item = origin; exports.other = origin; exports.other = origin;", "", APP, Disposition::Refused);
}

source!(nested_parameter_shadow, Supported, "function origin(input) { return input; }\nfunction unrelated(module, exports) { exports.item = 0; module.name = 0; } exports.item = origin;");
source!(nested_block_shadow, Supported, "function origin(input) { return input; }\nfunction unrelated() { const exports = {}; exports.item = 0; } exports.item = origin;");
mutation!(escaped_exports_write, r"\u0065xports.item = () => 0;");
mutation!(escaped_module_write, r"\u006dodule.exports.item = () => 0;");
mutation!(
    escaped_property_write,
    r"function other() {} exports.it\u0065m = other;"
);
mutation!(escaped_eval_write, r"\u0065val('exports.item = () => 0');");
mutation!(wrapper_arguments_alias, "arguments[0].item = () => 0;");
mutation!(
    wrapper_module_argument,
    "arguments[2].exports.item = () => 0;"
);
