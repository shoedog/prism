use super::{check, INSTALL, INSTALLED_WRITE};

#[test]
fn installed_slot_checks_installation_shape() {
    let source = INSTALL
        .replace("const rows = {};", "const rows = Object.create(null);")
        .replace("value: {}", "value: {label: [{text: 'note'}]}");
    check(&source, INSTALLED_WRITE, false);
    for (old, replacement) in [
        ("Object.create(null)", "Object.create(shared)"),
        (
            "Object.defineProperty(rows, key, {value: {label: [{text: 'note'}]}, writable: true});",
            "prepare(rows);",
        ),
        (
            "Object.defineProperty(rows, key, {value: {label: [{text: 'note'}]}, writable: true});",
            "const unused = 1;",
        ),
        ("rows, key, {value", "rows, ...keys, {value"),
        (
            "{value: {label: [{text: 'note'}]}, writable: true}",
            "descriptor",
        ),
        ("label: [{text: 'note'}]", "[raw]: {}"),
        ("label: [{text: 'note'}]", "__proto__: shared"),
        ("label: [{text: 'note'}]", "constructor: {}"),
        ("writable: true", "writable: shared"),
    ] {
        let fixture = source.replace(old, replacement);
        assert_ne!(fixture, source);
        check(&fixture, INSTALLED_WRITE, true);
    }
    check(
        &source.replace("rows[key].f", "rows[key][raw].label"),
        "rows[key][raw].label",
        true,
    );
    check(
        &source.replace("label: [{text: 'note'}]", "__proto__: null"),
        INSTALLED_WRITE,
        false,
    );
}

#[test]
fn direct_definition_write_is_specific_to_unshadowed_object() {
    check(
        &format!("function change(Object) {{ Object.defineProperty = noop; }} {INSTALL}"),
        INSTALLED_WRITE,
        false,
    );
    check(
        &format!("Object['defineProperty'] = noop; {INSTALL}"),
        INSTALLED_WRITE,
        true,
    );
}

#[test]
fn bounded_receivers_allow_direct_slots_and_constrained_input_uses() {
    check("function f(i) { ([{}])[+i].f = 1; }", "([{}])[+i].f", false);
    check(
        "function f(i) { ([shared])[+i].f = 1; }",
        "([shared])[+i].f",
        true,
    );
    let target = "rows[+i].f";
    let source = |uses| format!("function f(rows, i) {{ {uses} {target} = 1; return rows; }}");
    for uses in [
        "rows.length < 2;",
        "rows.length - 1;",
        "typeof rows.length;",
    ] {
        check(&source(uses), target, false);
    }
    for uses in [
        "consume(rows.length);",
        "consume(rows);",
        "function g() { return rows; }",
        "rows = shared;",
        "if (flag) rows = shared;",
        "function reset() { rows = shared; } reset();",
    ] {
        check(&source(uses), target, true);
    }
    check("const f = rows => { rows[+i].f = 1; };", target, false);
    check(
        "const f = rows => { const copy = rows; copy[+i].f = 1; };",
        "copy[+i].f",
        true,
    );
    check(
        "function f(i) { const rows = Array(1).fill(seed); rows[+i].f = 1; }",
        target,
        true,
    );
    check(
        "function f(sets, i) { for (const rows of sets) rows[+i].f = 1; }",
        target,
        true,
    );
}

#[test]
fn numeric_collections_check_insertions_and_direct_uses() {
    let source = |initial, before, body| {
        format!(
            "function f(grid, shared, key) {{
                const rows = {initial}; {before}
                for (const item of rows) {{ {body} grid[item.level].f = 1; }}
            }}"
        )
    };
    let target = "grid[item.level].f";
    for (initial, before, body, listed) in [
        (
            "[{level:0}]",
            "const seed = {level:1}; rows.push(seed);",
            "item.level++;",
            false,
        ),
        (
            "[{level:0}]",
            "const seed = {level:1}; rows.push(seed); consume(seed);",
            "item.level++;",
            true,
        ),
        (
            "[{level:0}]",
            "rows.push({level:1}); rows.length;",
            "item.level = 2;",
            false,
        ),
        (
            "[{level:0}]",
            "rows.push({level:1}); rows.length = key;",
            "",
            true,
        ),
        ("[{level:0}]", "rows.push = shared;", "", true),
        ("[{level:0}]", "rows.sort();", "", true),
        ("[{level:0}]", "", "delete item.level;", true),
        ("[{level:0}]", "", "item.level += key;", true),
        ("[{level:0}]", "", "const alias = item;", true),
        ("[{level:0}]", "", "item.constructor;", true),
        ("[{level:0},,]", "", "", true),
        ("[{level:0}, ...shared]", "", "", true),
        ("[{level:0, ...shared}]", "", "", true),
        ("[{level:0, [key]: 1}]", "", "", true),
        ("[{level:0, __proto__: null}]", "", "", true),
        ("[shared]", "", "", true),
    ] {
        check(&source(initial, before, body), target, listed);
    }
}

#[test]
fn bounded_domains_cover_value_joins_arithmetic_and_finite_tables() {
    let source = |value| {
        format!("function f(rows, key, flag) {{ const chosen = {value}; rows[chosen].f = 1; }}")
    };
    let target = "rows[chosen].f";
    for (value, listed) in [
        ("1 + 2", false),
        ("1 + key", false),
        ("10 + key", true),
        ("flag ? 1 : 'note'", false),
        ("flag ? 1 : key", true),
        ("(consume(key), key + 'x')", false),
        ("(consume(key), key + '')", true),
        ("flag ? `note` : 1", false),
        ("flag ? `prototype` : 1", true),
        (r"`note\u002d${key}`", true),
        ("({ 'label': 'note' }).label", false),
        ("({ 'label': 'note' }).other", true),
        ("({ label: 'note', __proto__: null }).label", true),
        ("({ [key]: 'note' }).label", true),
        ("({ 0: 'note' })[+key]", false),
        ("({ 0: 'note' })[key]", true),
        ("({ label: 'note' })[+key]", true),
    ] {
        check(&source(value), target, listed);
    }
}

#[test]
fn numeric_counters_require_a_dominating_start_and_numeric_writes() {
    let target = "rows[i].f";
    let source = |init, update| {
        format!(
        "function f(rows, key) {{ let i = key; for ({init}; i < 3; {update}) {{ {target}=1; }} }}"
    )
    };
    check(&source("i = 0", "i++"), target, false);
    check(&source("i = 0", "i += 1"), target, false);
    check(&source("i = key", "i++"), target, true);
    check(&source("i = 0", "i += key"), target, true);
    check(&source("i += 0", "i++"), target, true);
    check(&source("", "i++"), target, true);
    check(
        "function f(rows) { for (let i = 0, j = 1; i < 3; i++) rows[i].f = 1; }",
        target,
        true,
    );
    check(
        "function f(rows, key) { let i = key; i -= 1; rows[i].f = 1; }",
        target,
        false,
    );
    check(
        "function f(rows, key) { let i = key; i += 1; rows[i].f = 1; }",
        target,
        true,
    );
}

#[test]
fn numeric_type_exit_requires_leaving_the_path() {
    let target = "rows[key].f";
    let source = |guard| format!("function f(rows, key, flag) {{ {guard} {target} = 1; }}");
    check(
        &source("if (typeof key !== 'number' || flag) { return; }"),
        target,
        false,
    );
    check(
        &source("if (typeof key !== 'number' && flag) { return; }"),
        target,
        true,
    );
    check(
        &source("if (!(typeof key === 'number')) throw key;"),
        target,
        false,
    );
    check(
        &source("if (!(typeof key === 'number')) notify(key);"),
        target,
        true,
    );
}

#[test]
fn numeric_record_requires_a_direct_for_of_binding() {
    let target = "grid[item.level].f";
    let source = |head, before| {
        format!("function f(grid, flag) {{ for ({head}) {{ {before} {target} = 1; }} }}")
    };
    check(&source("let item of [{level: 0}]", ""), target, false);
    check(&source("let item in [{level: 0}]", ""), target, true);
    check(&source("let [item] of [[{level: 0}]]", ""), target, true);
    check(
        &source("let item of [{level: 0}]", "if (flag) item = replacement;"),
        target,
        true,
    );
    let script = "const grid = [{}];
                  for (var item of [{level: 0}]) {
                      window.item = replacement; grid[item.level].f = 1;
                  }";
    check(&format!("export {{}}; {script}"), target, false);
    check(script, target, true);
}

#[test]
fn optional_key_proofs_leave_facts_at_their_work_limits() {
    let target = "rows[key].f";
    for parts in [2, 1_660] {
        let value = format!("`note:{}`", "${x}".repeat(parts));
        let source = format!("function f(rows, x) {{ const key = {value}; {target} = 1; }}");
        check(&source, target, parts > 2);
    }
    for parts in [2, 8_300] {
        let value = "0,".repeat(parts);
        let source = format!("function f(rows) {{ const key = ({value} 1); {target} = 1; }}");
        check(&source, target, parts > 2);
    }
    for size in [2, 8_300] {
        let value = "x".repeat(size);
        let source =
            format!("function f(rows, flag) {{ const key = flag ? '{value}' : 1; {target} = 1; }}");
        check(&source, target, size > 2);
    }
}

#[test]
fn optional_receiver_proofs_leave_facts_at_their_work_and_depth_limits() {
    let target = "rows[+i].f";
    for count in [2, 8_300] {
        let statements = "noop();".repeat(count);
        let source = format!("function f(rows, i) {{ {statements} {target} = 1; }}");
        check(&source, target, count > 2);
    }
    for count in [2, 1_500] {
        let statements = "noop(other(0));".repeat(count);
        let source = format!(
            "function f(grid) {{
                for (const item of [{{level: 0}}]) {{ {statements} grid[item.level].f = 1; }}
            }}"
        );
        check(&source, "grid[item.level].f", count > 2);
    }
    for depth in [2, 26] {
        let private = format!("{}{{}}{}", "[".repeat(depth), "]".repeat(depth));
        let source = format!("function f(i) {{ const rows = {private}; {target} = 1; }}");
        check(&source, target, depth > 2);
    }
}

#[test]
fn numeric_counter_proofs_bound_write_and_ancestor_walks() {
    let target = "rows[i].f";
    check(
        "const rows = [{}]; let i = earlier; for (i = 0; i < 3; i++) rows[i].f = 1;",
        target,
        false,
    );
    check(
        "function f(rows, labels) {
            let i = 0; while (test()) { for (i of labels) {}; rows[i].f = 1; i++; }
        }",
        target,
        true,
    );
    for depth in [2, 132] {
        let source = format!(
            "function f(rows) {{ let i = 0; while (test()) {{
                {} {target} = 1; {} i++;
            }} }}",
            "{".repeat(depth),
            "}".repeat(depth)
        );
        check(&source, target, depth > 2);
    }
    for count in [2, 8_300] {
        let writes = "i = later;".repeat(count);
        let source = format!(
            "function f(rows, later) {{ let i; for (i = 0; i < 3; i++) {target} = 1; {writes} }}"
        );
        check(&source, target, count > 2);
    }
}
