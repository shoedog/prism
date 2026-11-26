use super::{js, KeyRole, ProtoWrite};

pub(super) fn nk04_operands(facts: &[&ProtoWrite]) -> bool {
    let [fact] = facts else {
        return false;
    };
    fact.key_operands
        .iter()
        .map(|operand| (operand.base.as_str(), operand.role))
        .eq([("kind", KeyRole::Selector), ("id", KeyRole::Property)])
}

fn lists_target(source: &str, target: &str) -> bool {
    let start = source.rfind(target).expect("target must occur in fixture");
    js(source)
        .js_ts_proto_writes()
        .iter()
        .any(|fact| (fact.start_byte, fact.end_byte) == (start, start + target.len()))
}

macro_rules! matrix_case {
    ($name:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/proto_writes_matrix/cases/",
            $name,
            ".js"
        ))
    };
}

#[test]
fn repeated_callback_table_store_stays_listed() {
    assert!(!lists_target(matrix_case!("t3-008"), "table[group][field]"));
    assert!(lists_target(
        "function f(items, src, p, v) { const table = Object.create(null); items.forEach(k => { if (table[k]) table[k][p] = v; table[k] = src[k]; }); }",
        "table[k][p]",
    ));
}

#[test]
fn returned_reduce_table_can_become_the_next_accumulator() {
    assert!(!lists_target(matrix_case!("t3-008"), "table[group][field]"));
    assert!(lists_target(
        "function f(items, src, p, v) { const table = Object.create(null); items.reduce((acc, k) => { acc[k] = src[k]; if (table[k]) table[k][p] = v; return table; }, {}); }",
        "table[k][p]",
    ));
}

#[test]
fn returned_null_walk_root_can_become_the_next_accumulator() {
    assert!(!lists_target(matrix_case!("t2-097"), "cursor[field]"));
    assert!(lists_target(
        "function f(items, src, path, field, value) {
            const root = Object.create(null);
            items.reduce((acc, k) => {
                acc[k] = src[k];
                let cursor = root;
                for (const key of path.split('.')) {
                    if (cursor[key] == null) cursor[key] = Object.create(null);
                    cursor = cursor[key];
                }
                cursor[field] = value;
                return root;
            }, {});
        }",
        "cursor[field]",
    ));
}

#[test]
fn numeric_lookup_checks_all_reaching_slot_values() {
    assert!(!lists_target(
        matrix_case!("t3-003"),
        "grid[node.level][node.lane]"
    ));
    assert!(lists_target(
        "function f(c, key) { let proto = {}; if (c) proto = Object.prototype; const rows = [proto]; rows[Number(key)].field = 1; }",
        "rows[Number(key)].field",
    ));
}

#[test]
fn numeric_lookup_retains_slots_reachable_through_assignment_aliases() {
    assert!(!lists_target(
        matrix_case!("t3-003"),
        "grid[node.level][node.lane]"
    ));
    assert!(lists_target(
        "function f(key) { const rows = [{}]; let a; a = rows; a[0] = Object.prototype; rows[Number(key)].field = 1; }",
        "rows[Number(key)].field",
    ));
}

#[test]
fn numeric_lookup_retains_slots_written_by_patterns() {
    assert!(!lists_target(
        matrix_case!("t3-003"),
        "grid[node.level][node.lane]"
    ));
    assert!(lists_target(
        "function f(key) { const rows = [{}]; [rows[0]] = [Object.prototype]; rows[Number(key)].field = 1; }",
        "rows[Number(key)].field",
    ));
}

#[test]
fn numeric_lookup_retains_slots_written_by_iteration_heads() {
    assert!(!lists_target(
        matrix_case!("t3-003"),
        "grid[node.level][node.lane]"
    ));
    assert!(lists_target(
        "function f(key) { const rows = [{}]; for (rows[0] of [Object.prototype]); rows[Number(key)].field = 1; }",
        "rows[Number(key)].field",
    ));
}

#[test]
fn numeric_lookup_retains_slots_exposed_to_unknown_calls() {
    assert!(!lists_target(
        matrix_case!("t3-003"),
        "grid[node.level][node.lane]"
    ));
    assert!(lists_target(
        "function f(key, mutate) { const rows = [{}]; mutate(rows); rows[Number(key)].field = 1; }",
        "rows[Number(key)].field",
    ));
}

#[test]
fn installation_must_survive_until_receiver_capture() {
    assert!(!lists_target(matrix_case!("t2-078"), "root[key][field]"));
    assert!(lists_target(
        matrix_case!("t2-110"),
        "catalog[section][field]"
    ));
    assert!(lists_target(
        "function f(raw, child) { const t = {}; const k = String(raw); Object.defineProperty(t,k,{value:{},writable:true}); (t[k]=child,t[k].flag=1); }",
        "t[k].flag",
    ));
}

#[test]
fn literal_computed_global_store_disables_the_intrinsic() {
    assert!(!lists_target(
        matrix_case!("nk09a"),
        "out[parseInt(a, 10)][k]"
    ));
    assert!(lists_target(
        "globalThis[\"parseInt\"] = x => x; function f(rows,k) { rows[parseInt(k)].flag=1; }",
        "rows[parseInt(k)].flag",
    ));
    for global in ["window", "self", "global"] {
        for member in [".parseInt", "['parseInt']"] {
            let source = format!(
                "{global}{member} = x => x; function f(rows,k) {{ rows[parseInt(k)].flag=1; }}"
            );
            assert!(lists_target(&source, "rows[parseInt(k)].flag"), "{source}");
        }
    }
}

#[test]
fn unknown_computed_global_store_disables_intrinsic_proofs() {
    assert!(!lists_target(
        matrix_case!("nk09a"),
        "out[parseInt(a, 10)][k]"
    ));
    assert!(lists_target(
        "globalThis[name] = x => x; function f(rows,k) { rows[parseInt(k)].flag=1; }",
        "rows[parseInt(k)].flag",
    ));
}

#[test]
fn record_method_call_invalidates_numeric_field_proof() {
    assert!(!lists_target(
        matrix_case!("t3-003"),
        "grid[node.level][node.lane]"
    ));
    assert!(lists_target(
        "function f(grid,k) { for (const r of [{n:0, set:function(v){this.n=v;}}]) { r.set(k); grid[r.n].flag=1; } }",
        "grid[r.n].flag",
    ));
}

#[test]
fn numeric_lookup_retains_slots_with_unproved_call_results() {
    assert!(!lists_target(
        matrix_case!("t3-003"),
        "grid[node.level][node.lane]"
    ));
    assert!(lists_target(
        "function f(i) { const rows=[Reflect.get(Object,\"prototype\")]; rows[Number(i)].flag=1; }",
        "rows[Number(i)].flag",
    ));
    assert!(!lists_target(
        matrix_case!("t2-171"),
        "swatches[index][field]"
    ));
    assert!(lists_target(
        "function f(index, child) { const rows = Array(1).fill(child); rows[Number(index)].flag = 1; }",
        "rows[Number(index)].flag",
    ));
}

#[test]
fn sparse_array_fallback_requires_an_unmodified_local_record() {
    assert!(!lists_target(matrix_case!("t2-140"), "rows[index][field]"));
    assert!(lists_target(
        "function f(key) { const rows = [,]; const fallback = { 0: {} }; fallback[0] = Array.prototype; Object.setPrototypeOf(rows, fallback); rows[Number(key)].field = 1; }",
        "rows[Number(key)].field",
    ));
}
