use super::{js, KeyRole};

#[test]
fn captured_record_collection_requires_every_insertion_and_update_to_be_numeric() {
    for (insert, update, expected) in [
        ("{ level: 0 }", "item.level += 1", 0),
        ("external", "item.level += 1", 1),
        ("{ level: 0 }", "item.level = external", 1),
    ] {
        let file = js(&format!(
            "function annotate(grid, external, field) {{
                const items = [];
                function build() {{ items.push({insert}); }}
                build();
                for (const item of items) {{
                    {update};
                    grid[item.level][field] = 1;
                }}
            }}"
        ));
        assert_eq!(
            file.js_ts_proto_writes().len(),
            expected,
            "{insert}, {update}"
        );
    }
}

#[test]
fn numeric_proofs_keep_rebound_intrinsics_and_visible_shared_slots() {
    for (source, expected) in [
        (
            "function f(rows, key) { rows[Math.floor(key)].field = 1; }",
            0,
        ),
        (
            "Math.floor = identity; function f(rows, key) { rows[Math.floor(key)].field = 1; }",
            1,
        ),
        (
            "function f(key) { const rows = [{}]; rows[Number(key)].field = 1; }",
            0,
        ),
        (
            "function f(key) { const rows = [Object.prototype]; rows[Number(key)].field = 1; }",
            1,
        ),
        (
            "function f(rows, key) { let index = 0; index += Number(key); rows[index].field = 1; }",
            0,
        ),
        (
            "function f(rows, key) { let index = key; index += 0; rows[index].field = 1; }",
            1,
        ),
    ] {
        assert_eq!(js(source).js_ts_proto_writes().len(), expected, "{source}");
    }
}

#[test]
fn private_children_stay_live_after_an_escape_or_an_opaque_store() {
    for (prefix, value, expected) in [
        ("", "value", 0),
        ("consume(table[key]);", "value", 1),
        ("table[key] = external;", "value", 1),
        ("", "(consume(table[key]), value)", 1),
    ] {
        let file = js(&format!(
            "function f(key, field, value, external) {{
                const table = Object.create(null);
                if (!(key in table)) table[key] = {{}};
                {prefix}
                table[key][field] = {value};
                return table;
            }}"
        ));
        assert_eq!(
            file.js_ts_proto_writes().len(),
            expected,
            "{prefix}, {value}"
        );
    }
}

#[test]
fn numeric_leaf_omits_its_operand_and_preserves_the_receiver_selector() {
    let file = js("function f(rows, key, offset) { rows[key][+offset] = 1; }");
    let facts = file.js_ts_proto_writes();
    let [fact] = facts.as_slice() else {
        panic!("{facts:?}")
    };
    let [selector] = fact.key_operands.as_slice() else {
        panic!("{fact:?}")
    };
    assert_eq!(
        (selector.base.as_str(), selector.role),
        ("key", KeyRole::Selector)
    );
}

#[test]
fn own_slot_enumeration_requires_the_same_closed_table() {
    for (initial, iteration, expected) in [
        ("{ draft: {} }", "const key of Object.keys(table)", 0),
        ("{ draft: {} }", "const [key] of Object.entries(table)", 0),
        ("{ draft: {} }", "const key of Object.keys(other)", 1),
        ("{ draft: other }", "const key of Object.keys(table)", 1),
    ] {
        let file = js(&format!(
            "function f(other) {{
                const table = {initial};
                for ({iteration}) table[key].label = 'draft';
                return table;
            }}"
        ));
        assert_eq!(
            file.js_ts_proto_writes().len(),
            expected,
            "{initial}, {iteration}"
        );
    }
    let value_escapes = js("function f() { const table = { draft: {} }; for (const [, child] of Object.entries(table)) consume(child); for (const key of Object.keys(table)) table[key].label = 'draft'; }");
    assert_eq!(value_escapes.js_ts_proto_writes().len(), 1);
}

#[test]
fn native_offsets_and_loop_counters_require_runtime_proofs() {
    for (source, expected) in [
        ("function f(rows, input) { const bytes = Uint8Array.from(input); rows[bytes[0]].field = 1; }", 0),
        ("function f(rows, bytes) { rows[bytes[0]].field = 1; }", 1),
        ("function f(rows, values, key) { rows[Array.prototype.indexOf.call(values, key)].field = 1; }", 0),
        ("function f(rows, values, key) { rows[values.indexOf(key)].field = 1; }", 1),
        ("function f(rows, run) { let i = 0; while (run()) { rows[i].field = 1; i += 1; } }", 0),
        ("function f(rows, run, key) { let i = 0; while (run()) { rows[i].field = 1; i = key; } }", 1),
        ("function f(rows, key, callbacks) { var i; for (i = 0; i < 1; i++) { callbacks.push(() => { rows[i].field = 1; }); } i = key; }", 1),
    ] {
        assert_eq!(js(source).js_ts_proto_writes().len(), expected, "{source}");
    }
}
