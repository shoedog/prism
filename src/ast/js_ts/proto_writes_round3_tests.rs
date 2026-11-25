use super::{fact_lines, js, read, reads, selector};
use crate::{ast::ParsedFile, languages::Language};
use std::fmt::Write;

#[test]
fn escaped_spellings_share_a_binding_or_refuse_it() {
    let file = js(
        r#"function f(root,input,stable){ let k="fixed"; \u006b=input;
        root[k].x=1; use(stable); }"#,
    );
    let key = read(&file, 2, "k");
    assert!(!key.resolution_complete);
    assert_eq!(key.incomplete_reason, Some("identity_escape"));
    assert_eq!(key.reaching_versions.len(), 2);
    let member = read(&file, 2, "root.[]");
    assert_eq!(member.incomplete_reason, Some("identity_escape"));
    assert!(key
        .reaching_versions
        .iter()
        .all(|span| member.reaching_versions.contains(span)));
    assert_eq!(fact_lines(&file), [2]);
    assert!(read(&file, 2, "stable").resolution_complete);
}

#[test]
fn implicit_arguments_never_claims_the_outer_formal() {
    let file = js("function f(arguments) { return [
        function g(){return arguments},
        () => arguments
    ]; }");
    let inner = read(&file, 2, "arguments");
    assert!(!inner.resolution_complete);
    assert_eq!(inner.incomplete_reason, Some("implicit_arguments"));
    assert!(inner.binding.is_none());
    let arrow = read(&file, 3, "arguments");
    assert!(arrow.resolution_complete);
    assert!(arrow.binding.is_some());
}

#[test]
fn syntactic_direct_eval_through_a_formal_refuses_versions() {
    let file = js("function f(eval,k,n){ eval('k=n');
        return k; }");
    let key = read(&file, 2, "k");
    assert!(!key.resolution_complete);
    assert_eq!(key.incomplete_reason, Some("direct_eval"));
    let control = js("function f(k) { function eval(text) {} eval('k=1');
        return k; }");
    assert!(read(&control, 2, "k").resolution_complete);
}

#[test]
fn deferred_member_read_keeps_later_member_stores() {
    let file = js("const o={key:0}; class C {
        x=o.key;
    }
    o.key=1; new C();");
    let read = read(&file, 2, "o.key");
    assert!(!read.resolution_complete);
    assert_eq!(read.incomplete_reason, Some("member_deferred"));
    assert_eq!(read.reaching_versions.len(), 2);
}

#[test]
fn member_read_keeps_opaque_argument_effects() {
    let file = js("function f(o){ helper(o.child);
        return o.child.key; }");
    let key = read(&file, 2, "o.child.key");
    assert!(!key.resolution_complete);
    assert_eq!(key.incomplete_reason, Some("member_call"));
    assert_eq!(key.reaching_versions.len(), 2);
}

#[test]
fn native_reflection_is_opaque_where_it_can_execute_user_code() {
    for call in [
        "Object.values(o)",
        "Object.entries(o)",
        "Object.keys(o)",
        "Reflect.get(o, 'other')",
    ] {
        let file = js(&format!(
            "function f(o) {{ {call};\n return o.key; }}\n const input = {{ get other() {{ this.key = 2; }} }};"
        ));
        let key = read(&file, 2, "o.key");
        assert_eq!(key.incomplete_reason, Some("member_call"), "{call}");
        assert_eq!(key.reaching_versions.len(), 2);
    }
    let pure = js("function f(o) { Array.isArray(o);\n return o.key; }");
    assert!(read(&pure, 2, "o.key").resolution_complete);
}

#[test]
fn member_read_keeps_possible_alias_stores() {
    for prefix in ["function f(a,b){", "function f(){const a=one(), b=two();"] {
        let file = js(&format!("{prefix} a.x=1;\n return b.x; }}"));
        let read = read(&file, 2, "b.x");
        assert!(!read.resolution_complete);
        assert_eq!(read.incomplete_reason, Some("member_alias"));
        assert_eq!(read.reaching_versions.len(), 2);
    }
    let control = js("function f(){const a={x:0}, b={x:0}; a.x=1;
        return b.x; }");
    assert!(read(&control, 2, "b.x").resolution_complete);
}

#[test]
fn accessor_evidence_refuses_matching_member_versions() {
    let file = js("function get(o){
        return o.key; }
    const input={get key(){return Math.random()}};
    const own={key:1};
    own.key;");
    assert_eq!(
        read(&file, 2, "o.key").incomplete_reason,
        Some("member_accessor")
    );
    assert!(read(&file, 5, "own.key").resolution_complete);
}

#[test]
fn abrupt_finally_path_keeps_selected_loop_receiver() {
    let file = js(
        "function f(root,k,flags){let c=root; for(const again of flags){
        c.x=1;
        try { c=root[k]; if(again) continue; } finally {}
        c=root;
    }}",
    );
    let cursor = read(&file, 2, "c");
    assert!(!cursor.resolution_complete);
    assert_eq!(cursor.incomplete_reason, None);
    assert_eq!(cursor.reaching_versions.len(), 3);
    assert_eq!(fact_lines(&file), [2]);
}

#[test]
fn default_only_switch_has_no_unmatched_path() {
    let file = js(
        "function f(root,k,t){let c=root[k]; switch(t){default: c={}}
        c.x=1; }",
    );
    assert!(fact_lines(&file).is_empty());
    let bypass = js("function f(root,k,t){let c=root[k]; switch(t){case 0: c={}}
        c.x=1; }");
    assert_eq!(fact_lines(&bypass), [2]);
    let cursor = read(&bypass, 2, "c");
    assert!(!cursor.resolution_complete);
    assert_eq!(cursor.incomplete_reason, None);
    assert_eq!(cursor.reaching_versions.len(), 2);
}

#[test]
fn native_array_selectors_exclude_discarded_inputs() {
    let file = js("function f(o,k,d,api){
        o[['fixed'].pop(d)].x=1; o[['fixed'].shift(d)].x=1;
        o[[k,d].at()].x=1; o[[d,k].at(1,d)].x=1; o[[d,k][1]].x=1;
        o[api.at(k,d)].x=1; o['extension'.pop(d)].x=1; o['extension'.shift(d)].x=1;
    }");
    let facts = file.js_ts_proto_writes();
    assert!(facts.iter().all(|fact| fact.line != 2));
    assert_eq!(facts.iter().filter(|fact| fact.line == 3).count(), 3);
    for fact in facts.iter().filter(|fact| fact.line == 3) {
        assert!(selector(fact, 3, "k"));
        assert!(!selector(fact, 3, "d"));
    }
    let custom: Vec<_> = facts.iter().filter(|fact| fact.line == 4).collect();
    assert_eq!(custom.len(), 3);
    assert!(custom.into_iter().all(|fact| selector(fact, 4, "d")));
}

#[test]
fn class_key_excludes_type_and_deferred_field_values() {
    let file = ParsedFile::parse(
        "round3.ts",
        "function f(o,NS,decoy,k){
            o[class implements NS.I { x=decoy; static x=k }].x=1;
        }",
        Language::TypeScript,
    )
    .unwrap();
    assert!(!file.tree.root_node().has_error());
    let facts = file.js_ts_proto_writes();
    assert_eq!(facts.len(), 1);
    assert!(selector(&facts[0], 2, "k"));
    assert!(!selector(&facts[0], 2, "NS"));
    assert!(!selector(&facts[0], 2, "decoy"));
    assert!(reads(&file, 2, "NS").is_empty());
}

#[test]
fn eager_method_write_uses_the_enclosing_owner() {
    let file = js("function f(o,k){
        class C { [(o[k].x=1)](){o[k].y=1} }
        const obj={[(o[k].z=1)](){o[k].w=1}};
    }");
    let facts = file.js_ts_proto_writes();
    assert_eq!(facts.len(), 4);
    for pair in facts.chunks_exact(2) {
        assert_eq!(pair[0].owner.start_byte, 0);
        assert_ne!(pair[1].owner.start_byte, 0);
        for fact in pair {
            assert_eq!(fact.owner, fact.key_operands[0].owner);
        }
    }
}

#[test]
fn refused_inventory_preserves_an_evaluated_receiver_prefix() {
    let mut source = String::from("function large(o,k){\n o[k].x.y=1;\n");
    source.push_str(&"o[0]=1;".repeat(10_001));
    source.push_str("}\nfunction small(o,k){o[k].x.y=1;}");
    let file = js(&source);
    for line in [2, 4] {
        assert_eq!(reads(&file, line, "o.[]").len(), 1);
        assert_eq!(reads(&file, line, "o.[].x").len(), 1);
    }
}

#[test]
fn uncertain_array_element_keeps_only_the_direct_selector() {
    let file = js("function f(o,k,d,i){
        o[[d,k].at(i)].x=1;
        o[[d,k][i]].y=1;
    }");
    let facts = file.js_ts_proto_writes();
    assert_eq!(facts.len(), 2);
    for (fact, path) in facts.iter().zip(["[d,k].at(i)", "[d,k][i]"]) {
        assert_eq!(fact.key_operands.len(), 1);
        assert!(selector(fact, fact.line, path));
        let key = read(&file, fact.line, path);
        assert!(!key.resolution_complete);
        assert_eq!(key.incomplete_reason, None);
    }
}

#[test]
fn destructuring_census_charges_one_rhs_scan() {
    let n = 4_000;
    let mut source = String::from("function wide(o,k){const [");
    for index in 0..n {
        write!(source, "a{index},").unwrap();
    }
    source.push_str("] = (");
    source.push_str(&"o,".repeat(n));
    source.push_str("o); o[k].x=1;}");
    let file = js(&source);
    let (work, rhs_nodes) = file.js_ts_proto_write_census_work();
    assert!((n..n + 16).contains(&rhs_nodes), "{rhs_nodes}");
    assert!((rhs_nodes..16 * n).contains(&work), "{work}");
    assert_eq!(fact_lines(&file), [1]);
}

#[test]
fn refused_line_listing_bounds_materialized_candidate_spans() {
    let n = 2_500;
    let mut source = String::from("function small(o,k){o[k].x=1;} function big(o,k,next){");
    source.push_str(&"o[k]=1;k=next;".repeat(n));
    source.push('\n');
    source.push_str(&"o[0]=1;".repeat(10_001 - n));
    source.push('}');
    let file = js(&source);
    let mut capacity = 500_000;
    let listed = file.js_ts_listed_name_reads(1, &mut capacity);
    let refused: Vec<_> = listed
        .iter()
        .filter(|read| read.candidate_history_refused)
        .collect();
    assert!(refused.iter().filter(|read| read.base == "k").count() >= n);
    assert!(refused
        .iter()
        .all(|read| !read.resolution_complete && read.reaching_versions.is_empty()));
    assert!(
        listed
            .iter()
            .map(|read| read.reaching_versions.len())
            .sum::<usize>()
            <= 4
    );
    assert!(500_000 - capacity <= 4);
    assert_eq!(fact_lines(&file), [1]);
}
