use super::{bounded_parse, fact_lines, js, read, reads, selector, MATRIX};
use crate::{ast::ParsedFile, languages::Language};
use std::fmt::Write;

#[test]
fn saved_loop_cursor_stays_silent_on_frozen_safe_row() {
    let source = std::fs::read_to_string(format!("{MATRIX}/cases/t2-165.js")).unwrap();
    assert!(js(&source).js_ts_proto_writes().is_empty());
    let retained = source.replace("cursor = start;", "if (matches) cursor = start;");
    assert_eq!(fact_lines(&js(&retained)), [17]);
}

#[test]
fn optional_chain_joins_bypassed_and_executed_effects() {
    for expression in ["obj?.[k = next]", "obj?.(k = next)", "obj?.m.n(k = next)"] {
        let file = js(&format!(
            "function f(obj, k, next, stable) {{ {expression};\n read(k); read(stable); }}"
        ));
        let key = read(&file, 2, "k");
        assert!(!key.resolution_complete, "{expression}: {key:?}");
        assert_eq!(key.reaching_versions.len(), 2, "{expression}: {key:?}");
        assert!(read(&file, 2, "stable").resolution_complete);
    }
    let file = js("function f(obj,k,next) { (obj?.m)(k=next);\n read(k); }");
    assert!(read(&file, 2, "k").resolution_complete);
}

#[test]
fn finally_inventory_includes_implicit_throw_from_body_and_handler() {
    for handler in [
        "",
        " catch { cursor = root[other]; work(); cursor = root; }",
    ] {
        let file = js(&format!(
            concat!(
                "function f(root, key, other) {{ let cursor = root;\n",
                "try {{ cursor = root[key]; work(); cursor = root; }}{handler}\n",
                "finally {{ cursor.x = 1; }}\n",
                "cursor = root;\n cursor.x = 2; }}"
            ),
            handler = handler
        ));
        let cursor = read(&file, 3, "cursor");
        assert!(!cursor.resolution_complete);
        assert_eq!(
            cursor.reaching_versions.len(),
            if handler.is_empty() { 3 } else { 5 }
        );
        assert_eq!(fact_lines(&file), [3]);
        assert!(read(&file, 5, "cursor").resolution_complete);
    }
}

#[test]
fn direct_eval_refuses_visible_resolution() {
    let file = js(concat!(
        "function f(k, next) { const g = () => (eval)('k = next'); g();\n",
        "read(k); }\n",
    ));
    assert!(!read(&file, 2, "k").resolution_complete);
    let control = js("function f(k, eval) { eval('label');\n read(k); }");
    assert!(read(&control, 2, "k").resolution_complete);
}

#[test]
fn deferred_var_read_stays_incomplete_before_initializer() {
    let file = js(concat!(
        "function g() { const take = () => read(k); take(); var k = 1;\n",
        "read(k); const stable = 2; const other = () => read(stable); other(); }",
    ));
    let deferred = read(&file, 1, "k");
    assert!(!deferred.resolution_complete);
    assert_eq!(deferred.reaching_versions, [deferred.binding.unwrap()]);
    assert!(read(&file, 2, "k").resolution_complete);
    assert!(read(&file, 2, "stable").resolution_complete);
}

#[test]
fn shorthand_arguments_have_mapped_candidate_writes() {
    for (directive, complete) in [("", false), ("'use strict';", true)] {
        let file = js(&format!(
            concat!(
                "function f(key, next) {{ {directive} const bag = {{arguments}};\n",
                "bag.arguments[0] = next; read(key); }}"
            ),
            directive = directive
        ));
        let key = read(&file, 2, "key");
        assert_eq!(key.resolution_complete, complete);
        assert_eq!(key.reaching_versions.len(), if complete { 1 } else { 2 });
    }
}

#[test]
fn alternative_member_targets_invalidate_both_possible_receivers() {
    for store in [
        "(flag ? a : b).x = v",
        "(flag ? a : b).x++",
        "delete (flag ? a : b).x",
    ] {
        let file = js(&format!(
            concat!(
                "function f(flag, a, b, c, v) {{ {store};\n",
                "read(a.x); read(b.x); read(c.x); }}"
            ),
            store = store
        ));
        for path in ["a.x", "b.x"] {
            let affected = read(&file, 2, path);
            assert!(!affected.resolution_complete, "{store}: {path}");
            assert_eq!(affected.reaching_versions.len(), 2);
        }
        assert!(read(&file, 2, "c.x").resolution_complete);
    }
}

#[test]
fn refused_inline_callback_preserves_outer_candidate_writes() {
    let mut source =
        String::from("function f(list, o, k, next, stable) {\n list.forEach(() => { k = next;");
    source.push_str(&"o[0]=1;".repeat(10_001));
    source.push_str("});\n read(k); read(stable);\n}");
    let file = js(&source);
    let key = read(&file, 3, "k");
    assert!(!key.resolution_complete);
    assert_eq!(key.reaching_versions.len(), 2);
    assert!(read(&file, 3, "stable").resolution_complete);
}

#[test]
fn eager_class_and_target_receivers_remain_in_value_inventory() {
    let source = concat!(
        "const C = 1, V = 2; export { type C, V };\n",
        "function f(o, opts, newer, p, k, next) {\n",
        "  o[opts.key].x = 1; opts = newer; opts.key[p] = 0;\n",
        "  class K { static x = (k = 1); [k = next]() {}; method(@decorate(k) arg: any) {} }\n",
        "  read(k);\n",
        "}"
    );
    let file = ParsedFile::parse("round2.ts", source, Language::TypeScript).unwrap();
    assert!(!file.tree.root_node().has_error());
    assert!(reads(&file, 1, "C").is_empty());
    assert!(read(&file, 1, "V").resolution_complete);
    let options = reads(&file, 3, "opts.key");
    assert_eq!(options.len(), 2);
    assert_ne!(options[0].reaching_versions, options[1].reaching_versions);
    assert!(read(&file, 4, "decorate").binding.is_none());
    let final_key = read(&file, 5, "k");
    let initializer = source.find("k = 1").unwrap();
    assert_eq!(final_key.reaching_versions[0].start_byte, initializer);
}

#[test]
fn selectors_include_consumed_values_and_exclude_discarded_inputs() {
    let file = js(concat!(
        "import * as util from 'util';\n",
        "function* f(o,k,discard,p,api,flag) {\n",
        "o[[discard,k].pop()][p]=1;\n",
        "o[[k,discard].shift()][p]=1;\n",
        "o[k += ''][p]=1;\n",
        "o[api.at(k)][p]=1;\n",
        "o[[k].at(discard)][p]=1;\n",
        "o[util.norm(k)][p]=1;\n",
        "o[[discard,k].at(1)][p]=1;\n",
        "o[[discard,k].at(-1)][p]=1;\n",
        "o[k === discard][p]=1; o[!k][p]=1; o[k in api][p]=1; o[k instanceof api][p]=1;\n",
        "o[yield k][p]=1;\n",
        "const s = `${k}!`; const c = o[s];\n c[s] = 1;\n",
        "}"
    ));
    assert_eq!(fact_lines(&file), [3, 4, 5, 6, 7, 8, 9, 10, 14]);
    for fact in &file.js_ts_proto_writes()[..8] {
        assert!(selector(fact, fact.line, "k"), "{fact:?}");
        assert!(!selector(fact, fact.line, "discard"), "{fact:?}");
        assert!(!selector(fact, fact.line, "util"), "{fact:?}");
    }
    assert!(selector(&file.js_ts_proto_writes()[8], 14, "s"));
}

#[test]
fn wide_program_and_many_closures_finish_with_sparse_callable_state() {
    let mut source = String::new();
    for n in 0..50_000 {
        write!(source, "let a{n}=1;").unwrap();
    }
    source.push_str("\nfunction fact(o,k,p){ o[k][p]=1; read(a49999); }");
    let file = bounded_parse(source);
    assert_eq!(fact_lines(&file), [2]);
    assert!(read(&file, 2, "a49999").resolution_complete);

    let mut source = String::new();
    for n in 0..100_000 {
        write!(source, "function f{n}(){{}}").unwrap();
    }
    source.push_str("\nfunction small(o,k,p){ o[k][p]=1; }");
    assert_eq!(fact_lines(&bounded_parse(source)), [2]);
}

#[test]
fn repeated_ternary_aliases_finish_with_a_separate_fact_control() {
    let mut source = String::from("function f(o,k,flag){ let a0=o[k];");
    for n in 1..=160 {
        let prior = n - 1;
        write!(source, "let a{n}=flag?a{prior}:a{prior};").unwrap();
    }
    source.push_str(" a160.x=1; }\nfunction small(o,k){ let a=o[k]; a.x=1; }");
    assert!(fact_lines(&bounded_parse(source)).contains(&2));
}

#[test]
fn member_reads_after_opaque_calls_keep_possible_write_sites() {
    let file = js(concat!(
        "function f(a,b,c) {\n",
        "  helper(a); b.reset();\n",
        "  read(a.x); read(b.x); read(c.x);\n",
        "}"
    ));
    for path in ["a.x", "b.x"] {
        let affected = read(&file, 3, path);
        assert!(!affected.resolution_complete);
        assert_eq!(affected.reaching_versions.len(), 2);
    }
    assert!(read(&file, 3, "c.x").resolution_complete);
}

#[test]
fn switch_keeps_skipped_effects_and_limits_state_copies() {
    let file = js(concat!(
        "function f(t, k, next, stable) {\n",
        "  switch (t) { default: k = next; break; }\n",
        "  read(k); read(stable);\n",
        "}"
    ));
    let changed = read(&file, 3, "k");
    assert!(!changed.resolution_complete);
    assert_eq!(changed.reaching_versions.len(), 2);
    assert!(read(&file, 3, "stable").resolution_complete);

    let mut source = String::from("function wide(o,k,p){");
    for n in 0..4_000 {
        write!(source, "let a{n}=1;").unwrap();
    }
    source.push_str("switch(k){");
    for n in 0..255 {
        write!(source, "case {n}: a0={n}; break;").unwrap();
    }
    source.push_str("}\n o[k][p]=1; }\nfunction small(o,k,p){o[k][p]=1;}");
    let file = bounded_parse(source);
    assert_eq!(fact_lines(&file), [3]);
    assert!(!read(&file, 2, "k").resolution_complete);
    assert!(read(&file, 3, "k").resolution_complete);
}

#[test]
fn positional_destructuring_reads_each_slot_once() {
    let mut pattern = String::new();
    let mut values = String::new();
    for n in 0..4_000 {
        write!(pattern, "a{n},").unwrap();
        values.push_str("root,");
    }
    let file = bounded_parse(format!(
        "function f(root,k) {{ const [{pattern}cursor,,after] = [{values}root[k],,root];\n cursor.x=1;\n after.x=1; }}"
    ));
    assert_eq!(fact_lines(&file), [2]);
    assert!(selector(&file.js_ts_proto_writes()[0], 1, "k"));
    assert!(read(&file, 3, "after").resolution_complete);
}
