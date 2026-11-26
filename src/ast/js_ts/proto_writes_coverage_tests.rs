use super::{fact_lines, js, read, selector};
use std::fmt::Write;

#[test]
fn accessor_shadow_proof_rejects_unknown_and_large_literals() {
    for literal in [
        "{...other}",
        "{[dynamic]: 1}",
        "{other: 1}",
        "{a:0,b:0,c:0,d:0,e:0,f:0,g:0,h:0,i:0,j:0,k:0,l:0,m:0,n:0,o:0,p:0,q:0,r:0,s:0,t:0,u:0,v:0,w:0,x:0,y:0,z:0,a1:0,b1:0,c1:0,d1:0,e1:0,f1:0,g1:0}",
    ] {
        let file = js(&format!("const evidence={{get key() {{return 1}}}};\nfunction f(other,dynamic) {{const obj={literal};\n return obj.key; }}"));
        let value = read(&file, 3, "obj.key");
        assert!(!value.resolution_complete, "{literal}: {value:?}");
        assert_eq!(value.incomplete_reason, Some("member_accessor"), "{literal}");
    }
    let control = js("const evidence={get key(){return 1}};\nconst obj={'key':1};\nobj.key;");
    assert!(read(&control, 3, "obj.key").resolution_complete);
}

#[test]
fn listing_refuses_history_when_capacity_is_exhausted() {
    let file = js("function f(o){ helper(o.child);\n return o.child.key; }");
    let ordinary = read(&file, 2, "o.child.key");
    assert_eq!(ordinary.incomplete_reason, Some("member_call"));
    assert!(!ordinary.reaching_versions.is_empty());
    let mut capacity = 0;
    let listed = file.js_ts_listed_name_reads(2, &mut capacity);
    let value = listed
        .iter()
        .find(|value| value.path == "o.child.key")
        .unwrap();
    assert!(value.candidate_history_refused);
    assert!(value.reaching_versions.is_empty());
    let mut capacity = 100;
    let listed = file.js_ts_listed_name_reads(2, &mut capacity);
    let value = listed
        .iter()
        .find(|value| value.path == "o.child.key")
        .unwrap();
    assert!(!value.candidate_history_refused);
    assert_eq!(value.reaching_versions, ordinary.reaching_versions);
}

#[test]
fn escaped_rebinding_is_counted_and_listed_on_its_read_line() {
    let file = js(concat!(
        "function f(stable) { let key = 0; ",
        r"\u006bey = 1;",
        "\n return [key, stable]; }",
    ));
    let key = read(&file, 2, "key");
    assert!(!key.resolution_complete);
    assert_eq!(key.incomplete_reason, Some("identity_escape"));
    assert!(read(&file, 2, "stable").resolution_complete);
    assert_eq!(
        file.js_ts_name_read_counts().1.get("identity_escape"),
        Some(&1)
    );
    let reasons: Vec<_> = file
        .js_ts_name_read_reasons(2)
        .map(|(read, reason)| (&read.span, reason))
        .collect();
    assert_eq!(reasons, [(&key.span, "identity_escape")]);
    assert_eq!(file.js_ts_name_read_reasons(3).count(), 0);
}

#[test]
fn assigning_implicit_arguments_does_not_write_the_outer_formal() {
    let file = js(concat!(
        "function f(arguments, next) {\n",
        "  function g() { arguments = next;\n",
        "    return arguments; }\n",
        "  return arguments;\n",
        "}",
    ));
    let implicit = read(&file, 3, "arguments");
    assert!(!implicit.resolution_complete);
    assert_eq!(implicit.incomplete_reason, Some("implicit_arguments"));
    assert!(implicit.binding.is_none());
    let formal = read(&file, 4, "arguments");
    assert!(formal.resolution_complete);
    assert_eq!(formal.reaching_versions.len(), 1);
}

#[test]
fn redeclared_eval_is_refused_after_its_single_definition_proof_is_lost() {
    for (assignment, call, expected) in [
        ("eval = replacement;", "eval('key=1')", Some("direct_eval")),
        ("", "eval('key=1')", None),
        ("eval = replacement;", "eval?.('key=1')", None),
    ] {
        let file = js(&format!(
            "function f(key, replacement) {{ function eval(text) {{}} {assignment} {call};\n return key; }}"
        ));
        let key = read(&file, 2, "key");
        assert_eq!(
            key.resolution_complete,
            expected.is_none(),
            "{call}: {assignment}"
        );
        assert_eq!(key.incomplete_reason, expected);
    }
}

#[test]
fn member_alias_proof_includes_a_destructured_formal() {
    for (formals, local, expected) in [
        ("o, {peer}", "", Some("member_alias")),
        ("o", "const peer = { key: 0 };", None),
    ] {
        let file = js(&format!(
            "function f({formals}) {{ {local} peer.key = 1;\n return o.key; }}"
        ));
        let key = read(&file, 2, "o.key");
        assert_eq!(key.resolution_complete, expected.is_none());
        assert_eq!(key.incomplete_reason, expected);
        assert_eq!(
            key.reaching_versions.len(),
            if expected.is_some() { 2 } else { 1 }
        );
    }
}

#[test]
fn opaque_calls_follow_nested_argument_values_but_respect_evaluation_order() {
    let file = js(concat!(
        "function f(o) {\n",
        "  o.child.key;\n",
        "  helper({ slot: [o.child] });\n",
        "  return o.child.key;\n",
        "}",
    ));
    let before = read(&file, 2, "o.child.key");
    let after = read(&file, 4, "o.child.key");
    assert!(before.resolution_complete);
    assert!(!after.resolution_complete);
    assert_eq!(after.incomplete_reason, Some("member_call"));
    assert_eq!(
        after.reaching_versions.len(),
        before.reaching_versions.len() + 1
    );
}

#[test]
fn deferred_field_refuses_member_read_but_computed_field_name_is_eager() {
    let file = js(concat!(
        "const o = { key: 0 }; class C {\n",
        "  [o.key] = 0;\n",
        "  x = o.key;\n",
        "}\n",
        "o.key = 1;",
    ));
    let eager = read(&file, 2, "o.key");
    let deferred = read(&file, 3, "o.key");
    assert!(eager.resolution_complete);
    assert!(!deferred.resolution_complete);
    assert_eq!(deferred.incomplete_reason, Some("member_deferred"));
    assert_eq!(
        deferred.reaching_versions.len(),
        eager.reaching_versions.len() + 1
    );
}

#[test]
fn computed_getter_refuses_a_literal_whose_own_key_is_unproven() {
    let file = js(concat!(
        "const name = 'key'; const evidence = { get [name]() { return 1; } };\n",
        r"const uncertain = {'\x6bey': 1}; const stable = {key: 1};",
        "\n[uncertain.key, stable.key];",
    ));
    let key = read(&file, 3, "uncertain.key");
    assert!(!key.resolution_complete);
    assert_eq!(key.incomplete_reason, Some("member_accessor"));
    assert!(read(&file, 3, "stable.key").resolution_complete);
}

#[test]
fn loop_finally_does_not_let_a_later_assignment_kill_a_continue_edge() {
    for (restore, expected_facts) in [("", &[2][..]), ("cursor = root;", &[][..])] {
        let file = js(&format!(
            "function f(root, key, flags, stable) {{ let cursor = root; for (const flag of flags) {{\n\
             cursor.x = stable;\n\
             try {{ cursor = root[key]; if (flag) continue; }} finally {{ {restore} }}\n\
             cursor = root;\n\
             }} }}"
        ));
        let cursor = read(&file, 2, "cursor");
        assert!(!cursor.resolution_complete);
        // Loop widening already makes the read incomplete, so it carries no new reason.
        assert_eq!(cursor.incomplete_reason, None);
        assert_eq!(fact_lines(&file), expected_facts);
        assert!(read(&file, 2, "stable").resolution_complete);
    }
}

#[test]
fn spread_and_noncanonical_array_positions_emit_only_the_unproven_selector() {
    for (expression, is_complete) in [
        ("[...xs, key].shift()", false),
        ("[key, ...xs].pop()", false),
        ("[...xs, key].at(0)", false),
        ("[0, key]['01']", false),
        ("[key, ...xs].shift()", true),
        ("[...xs, key].pop()", true),
        ("[0, key]['1']", true),
    ] {
        let file = js(&format!(
            "function f(o, xs, key) {{\n o[{expression}].x = 1; }}"
        ));
        let facts = file.js_ts_proto_writes();
        assert_eq!(facts.len(), 1, "{expression}");
        let selected = if is_complete { "key" } else { expression };
        assert!(selector(&facts[0], 2, selected), "{expression}");
        assert_eq!(facts[0].key_operands.len(), 1, "{expression}");
        let value = read(&file, 2, selected);
        assert_eq!(value.resolution_complete, is_complete, "{expression}");
        assert_eq!(value.incomplete_reason, None, "{expression}");
    }
}

#[test]
fn retained_store_requires_the_same_slot_when_wrapped_in_a_value() {
    for (store, complete, facts) in [
        ("root[key] = (0, flag ? {} : root[key])", true, &[2][..]),
        ("root[key] = (0, flag ? {} : root[other])", true, &[][..]),
        (
            "root[missing] = (0, flag ? {} : root[missing])",
            false,
            &[][..],
        ),
    ] {
        let file = js(&format!(
            "function f(key, other, flag) {{ const root = {{}};\n const child = ({store}); }}"
        ));
        let value = read(&file, 2, "root.[]");
        assert_eq!(value.resolution_complete, complete, "{store}");
        assert_eq!(value.incomplete_reason, None);
        assert_eq!(fact_lines(&file), facts, "{store}");
    }
    for operator in ["||=", "??="] {
        let file = js(&format!(
            "function f(key) {{ const root = {{}}; const child = (root[key] {operator} root[key]); }}"
        ));
        assert!(fact_lines(&file).is_empty(), "{operator}");
    }
}

#[test]
fn charged_destructuring_census_refuses_the_file_at_its_work_limit() {
    // A balanced tree gives 2,048 syntactic roots without a deep recursion chain.
    let mut roots = String::from("o");
    for _ in 0..11 {
        roots = format!("({roots} || {roots})");
    }
    let mut pattern = String::new();
    for index in 0..2_048 {
        write!(pattern, "v{index},").unwrap();
    }
    for (pattern, refused) in [("v0,", false), (&pattern, true)] {
        let file = js(&format!(
            "function wide(o) {{ const [{pattern}] = {roots}; }}\n\
             function small(o) {{ return o.key; }}"
        ));
        let (work, rhs_nodes) = file.js_ts_proto_write_census_work();
        assert_eq!(rhs_nodes, 4_095);
        if refused {
            assert!((4_000_001..=4_002_048).contains(&work), "{work}");
        } else {
            assert!(work < 40_000, "{work}");
        }
        let mut capacity = 100;
        let reads = file.js_ts_listed_name_reads(2, &mut capacity);
        let value = reads.iter().find(|read| read.path == "o.key").unwrap();
        assert_eq!(value.candidate_history_refused, refused);
        assert_eq!(value.resolution_complete, !refused);
    }
}

fn owner_with_branch_joins(name: &str, joins: usize) -> String {
    let mut body = format!("function {name}(o) {{ const flag = true;");
    for index in 0..512 {
        write!(body, "let v{index}=0;").unwrap();
    }
    body.push_str(&"if (flag) {}".repeat(joins));
    body.push_str("return o.key;}\n");
    body
}

#[test]
fn transfer_owner_budget_refuses_only_the_expensive_owner() {
    let file = js(&format!(
        "{}{}",
        owner_with_branch_joins("wide", 128),
        owner_with_branch_joins("small", 96)
    ));
    assert!(read(&file, 2, "o.key").resolution_complete);
    let mut capacity = 100;
    let reads = file.js_ts_listed_name_reads(1, &mut capacity);
    let value = reads.iter().find(|read| read.path == "o.key").unwrap();
    assert!(value.candidate_history_refused);
    assert!(!value.resolution_complete);
    assert_eq!(value.incomplete_reason, None);
}

#[test]
fn transfer_file_budget_refuses_later_owners_that_fit_individually() {
    let body = owner_with_branch_joins("f", 96);
    let standalone = js(&body);
    assert!(read(&standalone, 1, "o.key").resolution_complete);
    let combined = js(&body.repeat(10));
    assert!(read(&combined, 1, "o.key").resolution_complete);
    let mut capacity = 100;
    let reads = combined.js_ts_listed_name_reads(10, &mut capacity);
    let value = reads.iter().find(|read| read.path == "o.key").unwrap();
    assert!(value.candidate_history_refused);
    assert!(!value.resolution_complete);
    assert_eq!(value.incomplete_reason, None);
}
