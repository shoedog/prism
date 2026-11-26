use super::{js, KeyRole, ProtoWrite};

pub(super) fn required_matrix_operands(row: &str, facts: &[&ProtoWrite]) -> bool {
    let required: &[_] = match row {
        "nk04" => &[("kind", KeyRole::Selector), ("id", KeyRole::Property)],
        "t3-030" => &[
            ("path", KeyRole::Selector),
            ("font", KeyRole::Selector),
            ("unicode", KeyRole::Property),
        ],
        _ => return true,
    };
    let [fact] = facts else {
        return false;
    };
    fact.key_operands
        .iter()
        .map(|operand| (operand.base.as_str(), operand.role))
        .eq(required.iter().copied())
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
    assert!(lists_target(
        "function f(items, src, p, v) { const table = Object.create(null); items.forEach(k => { if (table[k]) table[k][p] = v; table[k] = src[k]; }); }",
        "table[k][p]",
    ));
}

#[test]
fn returned_reduce_table_can_become_the_next_accumulator() {
    assert!(lists_target(
        "function f(items, src, p, v) { const table = Object.create(null); items.reduce((acc, k) => { acc[k] = src[k]; if (table[k]) table[k][p] = v; return table; }, {}); }",
        "table[k][p]",
    ));
}

#[test]
fn returned_null_walk_root_can_become_the_next_accumulator() {
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
fn literal_computed_global_store_keeps_the_call_key_listed() {
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
fn unknown_computed_global_store_keeps_the_call_key_listed() {
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
    assert!(lists_target(
        "function f(index, child) { const rows = Array(1).fill(child); rows[Number(index)].flag = 1; }",
        "rows[Number(index)].flag",
    ));
}

#[test]
fn sparse_array_fallback_requires_an_unmodified_local_record() {
    assert!(lists_target(
        "function f(key) { const rows = [,]; const fallback = { 0: {} }; fallback[0] = Array.prototype; Object.setPrototypeOf(rows, fallback); rows[Number(key)].field = 1; }",
        "rows[Number(key)].field",
    ));
}

fn assert_listed_cases(cases: &[(&str, &str, &str)]) {
    let missing: Vec<_> = cases
        .iter()
        .filter_map(|(label, source, target)| (!lists_target(source, target)).then_some(label))
        .collect();
    assert!(missing.is_empty(), "missing facts: {missing:?}");
}

#[test]
fn captured_table_children_stay_listed() {
    assert_listed_cases(&[
        ("module", "const table = Object.create(null); function put(k,p,v,src){ if (table[k]) table[k][p]=v; table[k]=src[k]; }", "table[k][p]"),
        ("enclosing function", "function setup(){ const table = Object.create(null); function put(k,p,v,src){ if (table[k]) table[k][p]=v; table[k]=src[k]; } return put; }", "table[k][p]"),
        ("callback", "function setup(events,p,v,src){ const table = Object.create(null); events.on('entry', k => { if (table[k]) table[k][p]=v; table[k]=src[k]; }); }", "table[k][p]"),
        ("grouped table", "function setup(items,p,v,src){ const table = Object.groupBy(items, item => item.kind); function put(k){ if (table[k]) table[k][p]=v; table[k]=src[k]; } return put; }", "table[k][p]"),
    ]);
}

#[test]
fn repeated_null_walk_targets_stay_listed() {
    assert_listed_cases(&[
        (
            "loop",
            r#"function build(entries){ const root=Object.create(null); for (const [path,field,value] of entries){ let cursor=root; for (const key of path.split(".")){ if(cursor[key]==null) cursor[key]=Object.create(null); cursor=cursor[key]; } cursor[field]=value; } return root; }"#,
            "cursor[field]",
        ),
        (
            "captured root",
            r#"const root=Object.create(null); function put(path,field,value){ let cursor=root; for (const key of path.split(".")){ if(cursor[key]==null) cursor[key]=Object.create(null); cursor=cursor[key]; } cursor[field]=value; } put("one",field,value); put("one.two",other,next);"#,
            "cursor[field]",
        ),
    ]);
}

#[test]
fn global_and_pattern_stores_leave_call_keys_listed() {
    let cases = [
        (
            "assign",
            "Object.assign(globalThis,{parseInt:x=>x});",
            "parseInt",
        ),
        (
            "defineProperty",
            "Object.defineProperty(globalThis,'parseInt',{value:x=>x});",
            "parseInt",
        ),
        (
            "set",
            "Reflect.set(globalThis,'parseInt',x=>x);",
            "parseInt",
        ),
        ("alias", "const g=globalThis; g.parseInt=x=>x;", "parseInt"),
        (
            "object pattern",
            "({a: Math.floor} = {a: x=>x});",
            "Math.floor",
        ),
        ("array pattern", "[Math.floor = x=>x] = [];", "Math.floor"),
        ("script receiver", "this.parseInt=x=>x;", "parseInt"),
    ];
    let missing: Vec<_> = cases
        .into_iter()
        .filter_map(|(label, prefix, callee)| {
            let target = format!("rows[{callee}(k)].flag");
            let source = format!("{prefix} function f(rows,k){{ {target}=1; }}");
            (!lists_target(&source, &target)).then_some(label)
        })
        .collect();
    assert!(missing.is_empty(), "missing facts: {missing:?}");
}

#[test]
fn aliased_slot_writes_leave_call_keys_listed() {
    assert_listed_cases(&[
        ("direct", "function f(key){ const src=[{}]; src[0]=Object.prototype; const rows=src; rows[Number(key)].field=1; }", "rows[Number(key)].field"),
        ("conditional", "function f(key,c){ const src=[{}]; src[0]=Object.prototype; const rows=c ? src : [{}]; rows[Number(key)].field=1; }", "rows[Number(key)].field"),
    ]);
}

#[test]
fn formerly_discharged_targets_stay_listed() {
    assert_listed_cases(&[
        (
            "private child",
            matrix_case!("t3-008"),
            "table[group][field]",
        ),
        ("null walk", matrix_case!("t2-097"), "cursor[field]"),
        (
            "intrinsic conversion",
            matrix_case!("nk09a"),
            "out[parseInt(a, 10)][k]",
        ),
        (
            "sparse fallback",
            matrix_case!("t2-140"),
            "rows[index][field]",
        ),
    ]);
}

#[test]
fn bounded_keys_keep_only_their_proven_step() {
    for (positive, negative) in [
        ("+key", "key"),
        ("key - other", "key + other"),
        ("key === 0", "key || 0"),
        ("typeof key", "key"),
        ("`x:${key}`", "`${key}`"),
        ("`${key}:x`", "`${key}`"),
        ("'x:' + key", "'' + key"),
        ("({0: 'label'})[+key]", "({0: 'prototype'})[+key]"),
        ("key ? 'name' : 'label'", "key ? 'name' : 'prototype'"),
    ] {
        let target = "rows[chosen][field]";
        let source = |key: &str| {
            format!("function f(rows,key,other,field){{ const chosen = {key}; {target}=1; }}")
        };
        assert!(!lists_target(&source(positive), target), "{positive}");
        assert!(lists_target(&source(negative), target), "{negative}");
    }
}

#[test]
fn bounded_index_requires_an_admitted_call_result() {
    for (initial, listed) in [
        ("Array.from({ length: 1 }, () => ({}))", false),
        ("Array.from({ length: 1 }, () => item)", true),
        ("Array(1).fill({})", false),
        ("Array(1).fill(initial)", false),
        ("Array(1).fill(item)", true),
        ("makeRows()", true),
        ("[makeItem()]", true),
    ] {
        let source = format!(
            "function f(index,item,makeRows,makeItem){{ const initial={{}}; const rows={initial}; rows[+index].field=1; }}"
        );
        assert_eq!(
            lists_target(&source, "rows[+index].field"),
            listed,
            "{initial}"
        );
    }
}

#[test]
fn proven_installation_and_numeric_record_have_one_token_controls() {
    let install = matrix_case!("t2-078");
    let target = "root[key][field]";
    assert!(!lists_target(install, target));
    assert!(lists_target(
        &install.replace("= value;", "+= value;"),
        target,
    ));

    let record = matrix_case!("t3-003");
    let target = "grid[node.level][node.lane]";
    assert!(!lists_target(record, target));
    assert!(lists_target(
        &record.replace("node.level += 1;", "node.level += labels;"),
        target,
    ));
}

#[test]
fn numeric_type_test_must_establish_the_guarded_value() {
    let target = "rows[key].flag";
    let source = "function f(rows,key){ if (typeof key === 'number') rows[key].flag=1; }";
    assert!(!lists_target(source, target));
    assert!(lists_target(&source.replace("===", "!=="), target));
}

#[test]
fn bounded_indices_keep_opaque_or_modified_local_slots_listed() {
    for declaration in [
        "const rows = [Object.prototype];",
        "const rows = [{}]; rows[0] = Object.prototype;",
        "const src = [{}]; src[0] = Object.prototype; const rows = src;",
        "const src = [{}]; src[0] = Object.prototype; const rows = c ? src : [{}];",
        "const rows = Array.from({length: 1}, () => ({})); rows[0] = Object.prototype;",
    ] {
        let source = format!("function f(index,c){{ {declaration} rows[+index].field=1; }}");
        assert!(lists_target(&source, "rows[+index].field"), "{declaration}");
    }
}

#[test]
fn named_finite_table_must_be_file_local() {
    let target = "rows[Names.label].flag";
    let source = "const Names = { label: 'note' }; function f(rows) { rows[Names.label].flag=1; }";
    assert!(!lists_target(source, target));
    assert!(lists_target(&format!("export {source}"), target));
}

#[test]
fn bounded_input_keeps_direct_member_changes_listed() {
    let target = "rows[+index].flag";
    let source = "function f(rows,index) { typeof rows.length; rows[+index].flag=1; }";
    assert!(!lists_target(source, target));
    assert!(lists_target(&source.replace("typeof", "delete"), target));
}
