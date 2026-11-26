use super::lists_target;

fn check(source: &str, target: &str, listed: bool) {
    let actual = lists_target(source, target);
    if actual != listed {
        let preview: String = source.chars().take(512).collect();
        panic!("{target} listed: expected {listed}, got {actual}. Source prefix: {preview}");
    }
}

#[test]
fn script_var_key_is_visible_through_global_object() {
    for initial in ["0", "1 - 1"] {
        let target = "rows[k].f";
        let script = format!(
            "var k = {initial}; function reset() {{ window.k = 'x'; }}
             const rows = [{{}}]; reset(); {target} = 1;"
        );
        let controls = [
            format!("function run() {{ {script} }}"),
            script.replacen("var k", "let k", 1),
            script.replacen("var k", "const k", 1),
            format!("export {{}}; {script}"),
            format!("import './setup.js'; {script}"),
        ];
        for source in controls {
            check(&source, target, false);
        }
        check(&script, target, true);
    }
}

#[test]
fn rebound_script_function_key_is_visible_through_global_object() {
    let source = "function k() {} k = 1 - 1; window.k = 'x';
                  const rows = [{}]; rows[k].f = 1;";
    check(
        &format!("function run() {{ {source} }}"),
        "rows[k].f",
        false,
    );
    check(&format!("export {{}}; {source}"), "rows[k].f", false);
    check(source, "rows[k].f", true);
}

#[test]
fn script_var_counter_is_visible_through_global_object() {
    let source = "const rows = [{}];
                  for (var i = 0; (globalThis.i = 'x', i < 1); i++) rows[i].x = 1;";
    check(
        &format!("function run() {{ {source} }}"),
        "rows[i].x",
        false,
    );
    check(&source.replace("var i", "let i"), "rows[i].x", false);
    check(&format!("export {{}}; {source}"), "rows[i].x", false);
    check(source, "rows[i].x", true);
}

#[test]
fn script_var_receiver_is_visible_through_global_object() {
    let source = "var rows = [{}]; window.rows = other; rows[+i].f = 1;";
    for control in [
        format!("function run(i, other) {{ {source} }}"),
        source.replace("var rows", "let rows"),
        source.replace("var rows", "const rows"),
        format!("export {{}}; {source}"),
    ] {
        check(&control, "rows[+i].f", false);
    }
    check(source, "rows[+i].f", true);
}

#[test]
fn script_var_install_base_is_visible_through_global_object() {
    let source = "var rows = {}; const key = `${label}`;
                  window.rows = other;
                  Object.defineProperty(rows, key, {value: {}});
                  rows[key].f = 1;";
    check(
        &format!("function run() {{ {source} }}"),
        "rows[key].f",
        false,
    );
    check(
        &source.replace("var rows", "const rows"),
        "rows[key].f",
        false,
    );
    check(&format!("export {{}}; {source}"), "rows[key].f", false);
    check(source, "rows[key].f", true);
}

const INSTALL: &str = "function f(raw, shared, value) {
    const rows = {}; const key = `${raw}`;
    Object.defineProperty(rows, key, {value: {}, writable: true});
    rows[key].f = value;
}";
const INSTALLED_WRITE: &str = "rows[key].f";

#[test]
fn installation_requires_a_literal_or_template_key() {
    check(INSTALL, INSTALLED_WRITE, false);
    check(
        &INSTALL.replace("`${raw}`", "'note'"),
        INSTALLED_WRITE,
        false,
    );
    check(
        &INSTALL.replace("`${raw}`", "String(raw)"),
        INSTALLED_WRITE,
        true,
    );
}

#[test]
fn installation_refuses_shadowed_or_explicitly_reassigned_define_property() {
    check(INSTALL, INSTALLED_WRITE, false);
    check(
        &INSTALL.replace("f(raw", "f(Object, raw"),
        INSTALLED_WRITE,
        true,
    );
    check(
        &format!("Object.defineProperty = () => {{}}; {INSTALL}"),
        INSTALLED_WRITE,
        true,
    );
}

#[test]
fn installation_requires_the_same_key_and_an_adjacent_private_data_slot() {
    let source = INSTALL.replace(
        "const key = `${raw}`;",
        "const key = `${raw}`; const otherKey = `alt:${raw}`;",
    );
    check(&source, INSTALLED_WRITE, false);
    for (old, replacement) in [
        ("rows, key,", "rows, otherKey,"),
        ("value: {}", "get: () => shared"),
        ("value: {}", "value: shared"),
        ("rows[key].f", "0; rows[key].f"),
        ("const rows = {}", "const rows = shared"),
    ] {
        check(&source.replace(old, replacement), INSTALLED_WRITE, true);
    }
}

#[test]
fn named_finite_table_refuses_a_second_reference_that_writes() {
    let target = "rows[Names.label].f";
    let source = "const Names = {label:'note'}; const rows = [{}]; rows[Names.label].f = 1;";
    check(source, target, false);
    check(
        &source.replace("const rows", "Names.label = 'x'; const rows"),
        target,
        true,
    );
}

#[test]
fn numeric_guard_requires_the_same_value_on_every_path() {
    let target = "rows[key].f";
    let source = "function f(rows, key, flag, other) {
        if (typeof key === 'number' && flag) { rows[key].f = 1; }
    }";
    check(source, target, false);
    check(&source.replace("&&", "||"), target, true);
    check(
        &source.replace("{ rows", "{ key = other; rows"),
        target,
        true,
    );
}

#[test]
fn numeric_record_requires_dot_access_to_the_original_binding() {
    let source = "function f(grid) {
        for (const item of [{level: 0}]) { item.level += 1; grid[item.level].f = 1; }
    }";
    check(source, "grid[item.level].f", false);
    check(
        &source.replace("item.level].f", "item['level']].f"),
        "grid[item['level']].f",
        true,
    );
    check(
        &source.replace("grid[item", "const alias = item; grid[alias"),
        "grid[alias.level].f",
        true,
    );
}

#[test]
fn numeric_record_refuses_an_exported_collection() {
    let source = "const rows = [{level: 0}];
                  function f(grid) { for (const item of rows) grid[item.level].f = 1; }";
    check(source, "grid[item.level].f", false);
    check(&format!("export {source}"), "grid[item.level].f", true);
    check(
        &format!("{source} export {{ rows }};"),
        "grid[item.level].f",
        true,
    );
}

#[test]
fn call_result_slot_models_standard_array_from_identity() {
    let source = "function f(i, k) {
        const rows = Array.from({length: 1}, () => ({})); rows[+i][k] = 1;
    }";
    check(source, "rows[+i][k]", false);
    check(
        &source.replace("Array.from", "makeRows"),
        "rows[+i][k]",
        true,
    );
    check(
        &source.replace("() => ({})", "() => shared"),
        "rows[+i][k]",
        true,
    );
    check(
        &format!("Array.from = () => [shared]; {source}"),
        "rows[+i][k]",
        false,
    );
}

#[path = "proto_writes_review_coverage_tests.rs"]
mod coverage;
