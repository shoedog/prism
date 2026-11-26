use super::{fact_lines, js, read, reads, selector};
use crate::{ast::ParsedFile, languages::Language};
use std::fmt::Write;

fn wide_state() -> String {
    let mut source = String::new();
    for index in 0..512 {
        write!(source, "let v{index}=0;").unwrap();
    }
    source
}

#[test]
fn owner_budget_refuses_expensive_forks_but_preserves_their_reads() {
    // With this state, 106 joins leave just under 3,900 work units.
    // Leave about 700 for the first fork, or about 2,000 to reach the catch fork.
    let declarations = wide_state();
    let joins = "if (flag) {}".repeat(106);
    let mut outcomes = Vec::new();
    for (branch, path, padding) in [
        ("while (flag) { o.slot; }", "o.slot", 3_188),
        ("try { o.slot; } catch {}", "o.slot", 3_188),
        (
            "try {} catch { const local = {}; local.slot; }",
            "local.slot",
            1_838,
        ),
        ("switch (flag) { case 0: o.slot; }", "o.slot", 3_188),
        ("if (flag) { o.slot; }", "o.slot", 3_188),
        ("key ||= o.slot;", "o.slot", 3_188),
        ("class C { field = o; }", "o", 3_188),
        ("sink?.(o.slot);", "o.slot", 3_188),
        ("o?.[key];", "key", 3_188),
        ("yield (key = o.slot);", "o.slot", 3_188),
        ("let [item = o.slot] = [];", "o.slot", 3_188),
    ] {
        for (prefix, padding, refused) in [("", 0, false), (joins.as_str(), padding, true)] {
            let source = format!(
                "function* f(key, flag) {{ const o = {{}};{declarations}{prefix}{}\n{branch}\n}}",
                ";".repeat(padding)
            );
            let file = js(&source);
            assert!(!file.tree.root_node().has_error(), "{branch}");
            let mut capacity = 100;
            let listed = file.js_ts_listed_name_reads(2, &mut capacity);
            let value = listed.iter().find(|value| value.path == path);
            let value = value.unwrap_or_else(|| panic!("{branch}: missing {path} in {listed:?}"));
            assert_eq!(value.incomplete_reason, None, "{branch}");
            assert!(value.binding.is_some(), "{branch}");
            let actual = (value.candidate_history_refused, value.resolution_complete);
            if actual != (refused, !refused) {
                outcomes.push((branch, refused, actual));
            }
        }
    }
    assert!(
        outcomes.is_empty(),
        "unexpected budget outcomes: {outcomes:?}"
    );
}

#[test]
fn refused_inventory_keeps_loads_and_pattern_inputs() {
    for refused in [false, true] {
        let mut source = String::from(concat!(
            "function f(key, scope) { const o = {};\n",
            "  ({[key]: x = o.fallback} = o[key].nested.value);\n",
            "  const methods = { [o.method]() { return 0; } };\n",
            "  with (scope) {}\n",
            "  o[key].counter++;\n",
        ));
        if refused {
            source.push_str(&"o[0]=1;".repeat(10_001));
        }
        source.push_str("\n}");
        let file = js(&source);
        assert!(!file.tree.root_node().has_error());
        for (line, path) in [
            (2, "key"),
            (2, "o.fallback"),
            (2, "o.[].nested.value"),
            (3, "o.method"),
            (4, "scope"),
            (5, "o.[].counter"),
        ] {
            let mut capacity = 100;
            let listed = file.js_ts_listed_name_reads(line, &mut capacity);
            let matching: Vec<_> = listed.iter().filter(|value| value.path == path).collect();
            assert!(!matching.is_empty(), "{line}:{path}: {listed:?}");
            for value in matching {
                assert_eq!(value.candidate_history_refused, refused, "{line}:{path}");
                assert_eq!(value.resolution_complete, !refused, "{line}:{path}");
                assert_eq!(value.incomplete_reason, None, "{line}:{path}");
                assert!(value.binding.is_some(), "{line}:{path}");
            }
        }
        assert_eq!(reads(&file, 2, "key").len(), 2);
        assert!(reads(&file, 2, "x").is_empty());
    }
}

#[test]
fn yield_with_a_write_preserves_the_skipped_and_evaluated_versions() {
    for (statement, complete) in [("yield (key = next);", false), ("key = next;", true)] {
        let file = js(&format!(
            "function* f(key, next, stable) {{ {statement}\n return [key, stable]; }}"
        ));
        let key = read(&file, 2, "key");
        assert_eq!(key.resolution_complete, complete, "{statement}");
        assert_eq!(key.incomplete_reason, None);
        assert_eq!(key.reaching_versions.len(), if complete { 1 } else { 2 });
        assert!(read(&file, 2, "stable").resolution_complete);
    }
}

#[test]
fn reducer_uses_only_value_producing_accumulator_paths() {
    for (carrier, selected, unresolved) in [
        ("(copy = (0, flag && acc))", true, None),
        ("(copy = (0, acc && external))", false, Some("external")),
        ("(copy = (0, external || acc))", true, Some("external")),
        ("(copy = (0, external || other))", false, Some("external")),
        ("acc.prototype", true, None),
        ("external.prototype", false, Some("external.prototype.[]")),
    ] {
        let file = ParsedFile::parse(
            "reducer.ts",
            &format!(
                "function f(keys: string[], root: any, other: any, flag: boolean) {{\n\
                 keys.reduce((acc: any, key) => {{ let copy; return {carrier}[key]; }}, root).x = 1;\n}}"
            ),
            Language::TypeScript,
        )
        .unwrap();
        assert!(!file.tree.root_node().has_error(), "{carrier}");
        let facts = fact_lines(&file);
        assert_eq!(
            facts,
            if selected { &[2][..] } else { &[][..] },
            "{carrier}"
        );
        assert!(read(&file, 2, "key").resolution_complete);
        if let Some(path) = unresolved {
            let unknown = read(&file, 2, path);
            assert!(!unknown.resolution_complete);
            assert_eq!(unknown.incomplete_reason, None);
        }
    }
}

#[test]
fn retained_slot_requires_a_live_return_or_local_assignment() {
    let file = js(concat!(
        "function f(keys, root, sink, index, flag) {\n",
        "  keys.reduce((acc, key) => { return acc; return acc[key] = acc[key] || {}; }, root).x = 1;\n",
        "  keys.reduce((acc, key) => { return acc[key] = acc[key] || {}; }, root).x = 1;\n",
        "  sink.slot = (root[index] = root[index] || {});\n",
        "  let child; child = (root[index] = root[index] || {});\n",
        "  const fixed = (root.prototype[index] = root.prototype[index] || {});\n",
        "}",
    ));
    assert_eq!(fact_lines(&file), [3, 3, 5, 6]);
    let unreachable = read(&file, 2, "acc.[]");
    assert!(!unreachable.resolution_complete);
    assert_eq!(unreachable.incomplete_reason, None);
    for (line, path) in [(3, "key"), (5, "index")] {
        let controls = reads(&file, line, path);
        assert!(!controls.is_empty(), "{line}:{path}");
        assert!(controls.iter().all(|key| key.resolution_complete));
    }
}

#[test]
fn reflection_requires_the_two_argument_get_form() {
    for (args, selected) in [("root, key, external", false), ("root, key", true)] {
        let file = js(&format!(
            "function f(root, key) {{\n Reflect.get({args}).slot = 1; }}"
        ));
        assert_eq!(fact_lines(&file), if selected { &[2][..] } else { &[][..] });
        assert!(read(&file, 2, "key").resolution_complete);
        if !selected {
            let external = read(&file, 2, "external");
            assert!(!external.resolution_complete);
            assert_eq!(external.incomplete_reason, None);
        }
    }
}

#[test]
fn earlier_array_selection_contributes_only_its_selected_element() {
    for (offset, selected) in [("position", false), ("0", true)] {
        let file = js(&format!(
            "function f(root, chosen, discarded, position) {{\n\
             const key = [chosen, discarded].at({offset});\n\
             root[key].slot = 1;\n}}"
        ));
        let facts = file.js_ts_proto_writes();
        assert_eq!(facts.len(), 1);
        assert!(selector(&facts[0], 3, "key"));
        assert_eq!(selector(&facts[0], 2, "chosen"), selected);
        assert!(!selector(&facts[0], 2, "discarded"));
        assert!(!selector(&facts[0], 2, "position"));
        assert!(read(&file, 3, "key").resolution_complete);
    }
}

#[test]
fn labeled_continue_keeps_the_receiver_until_finally_restores_it() {
    for (restore, expected) in [("", &[3][..]), ("cursor = root;", &[][..])] {
        let file = js(&format!(
            "function f(root, key, flag, stable) {{ let cursor = root;\n\
             outer: inner: while (flag) {{\n\
             cursor.slot = stable;\n\
             try {{ cursor = root[key]; if (flag) continue outer; }} finally {{ {restore} }}\n\
             cursor = root;\n}} }}"
        ));
        assert!(!file.tree.root_node().has_error());
        assert_eq!(fact_lines(&file), expected, "{restore}");
        if restore.is_empty() {
            let cursor = read(&file, 3, "cursor");
            assert!(!cursor.resolution_complete);
            assert_eq!(cursor.incomplete_reason, None);
        }
        assert!(read(&file, 3, "stable").resolution_complete);
    }
}

#[test]
fn reducer_keeps_a_return_pending_before_an_unrelated_try() {
    for (first_return, expected) in [("acc[key]", &[2][..]), ("acc", &[][..])] {
        let file = js(&format!(
            "function f(keys, root, flag) {{\n\
             keys.reduce((acc, key) => {{ if (flag) return {first_return}; try {{ return acc; }} finally {{}} }} , root).slot = 1;\n}}"
        ));
        assert_eq!(fact_lines(&file), expected, "{first_return}");
        assert!(read(&file, 2, "flag").resolution_complete);
    }
}

#[test]
fn long_member_chain_keeps_the_subscript_bypass_past_the_proof_limit() {
    for (links, complete) in [(126, true), (127, false)] {
        let file = js(&format!(
            "function f(key, next, stable) {{ const o = {{}};\n\
             o{}[(key = next)];\n\
             return [key, stable];\n}}",
            ".x".repeat(links),
        ));
        let key = read(&file, 3, "key");
        assert_eq!(key.resolution_complete, complete, "{links}");
        assert_eq!(key.incomplete_reason, None);
        assert_eq!(key.reaching_versions.len(), if complete { 1 } else { 2 });
        assert!(read(&file, 3, "stable").resolution_complete);
    }
}

#[test]
fn retained_literal_slot_never_promotes_the_fixed_selector() {
    for (key, path, complete) in [
        ("'fixed'", "root.fixed", true),
        ("`fixed`", "root.[]", false),
    ] {
        let file = js(&format!(
            "function f() {{ const root = {{}};\n\
             const value = (root[{key}] = root[{key}] || {{}});\n}}"
        ));
        assert!(fact_lines(&file).is_empty(), "{key}");
        let value = read(&file, 2, path);
        assert_eq!(value.resolution_complete, complete);
        assert_eq!(value.incomplete_reason, None);
    }
}

#[test]
fn refused_inline_callback_keeps_its_outer_cursor_store() {
    let declarations = wide_state();
    for (joins, refused) in [(0, false), (110, true)] {
        let source = format!(
            "function f(list, root, key) {{ let cursor = root;\n\
             list.forEach(() => {{ const flag = true; cursor = root[key]; {declarations}{} }});\n\
             return cursor;\n}}",
            "if (flag) {}".repeat(joins),
        );
        let file = js(&source);
        let cursor = read(&file, 3, "cursor");
        assert_eq!(cursor.resolution_complete, !refused);
        assert_eq!(cursor.incomplete_reason, None);
        assert_eq!(cursor.reaching_versions.len(), 2);
        assert_eq!(
            cursor.reaching_versions[1].start_byte,
            source.find("cursor = root[key]").unwrap()
        );
    }
}

#[test]
fn refused_reducer_does_not_publish_a_child_of_its_accumulator() {
    let declarations = wide_state();
    for (joins, refused) in [(0, false), (110, true)] {
        let file = js(&format!(
            "function f(keys, root) {{\n\
             keys.reduce((acc, key) => {{ const flag = true; {declarations}{}\n\
             return acc[key]; }}, root).slot = 1;\n}}",
            "if (flag) {}".repeat(joins),
        ));
        assert_eq!(fact_lines(&file), if refused { &[][..] } else { &[2][..] });
        let mut capacity = 100;
        let listed = file.js_ts_listed_name_reads(3, &mut capacity);
        let key = listed.iter().find(|value| value.path == "key").unwrap();
        assert_eq!(key.candidate_history_refused, refused);
        assert_eq!(key.resolution_complete, !refused);
        assert_eq!(key.incomplete_reason, None);
    }
}
