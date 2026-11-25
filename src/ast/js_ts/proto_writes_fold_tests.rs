use super::{js, KeyRole, MATRIX};
use crate::ast::js_ts::proto_writes::{NameRead, ProtoWrite};
use crate::{ast::ParsedFile, languages::Language};

#[path = "proto_writes_coverage_tests.rs"]
mod coverage;
#[path = "proto_writes_round2_tests.rs"]
mod round2;
#[path = "proto_writes_round3_tests.rs"]
mod round3;

fn reads(file: &ParsedFile, line: usize, path: &str) -> Vec<NameRead> {
    file.js_ts_name_reads(line)
        .into_iter()
        .filter(|read| read.path == path)
        .collect()
}

fn read(file: &ParsedFile, line: usize, path: &str) -> NameRead {
    let values = reads(file, line, path);
    assert_eq!(values.len(), 1, "{path}:{line}: {values:?}");
    values.into_iter().next().unwrap()
}

fn fact_lines(file: &ParsedFile) -> Vec<usize> {
    file.js_ts_proto_writes()
        .iter()
        .map(|fact| fact.line)
        .collect()
}

fn selector(fact: &ProtoWrite, line: usize, path: &str) -> bool {
    fact.key_operands
        .iter()
        .any(|key| key.role == KeyRole::Selector && key.line == line && key.name == path)
}

#[test]
fn var_initialization_and_parameter_environment() {
    let file = js(concat!(
        "const outer = 1;\n",
        "function f(p = outer) {\n",
        "  read(k);\n",
        "  var k = p; var k; var outer = 2;\n",
        "  read(k);\n",
        "}\n",
        "function g(p) { read(p); var p; read(p); var p = 1; read(p); }\n",
    ));
    let default = read(&file, 2, "outer");
    assert!(default.resolution_complete);
    assert_eq!(default.binding.unwrap().start_byte, 6);
    let early = read(&file, 3, "k");
    let late = read(&file, 5, "k");
    assert!(!early.resolution_complete && early.reaching_versions.is_empty());
    assert!(late.resolution_complete && early.binding == late.binding && late.binding.is_some());
    let formal = reads(&file, 7, "p");
    assert_eq!(formal.len(), 3);
    assert!(formal
        .iter()
        .all(|r| r.resolution_complete && r.binding == formal[0].binding));
    assert_eq!(formal[0].reaching_versions, formal[1].reaching_versions);
    assert_ne!(formal[1].reaching_versions, formal[2].reaching_versions);

    let loop_file = js("function f(root,path,n){ let cur=root; for(let i=0;i<n;i++){ var prev; if(i) prev[path[i]]=1; prev=cur; cur=cur[path[i]]; } }");
    assert!(!loop_file.js_ts_proto_writes().is_empty());

    let split = js("function h(p = 0) { var p; read(p); let local; read(local); }");
    assert!(read(&split, 1, "p").resolution_complete);
    assert!(read(&split, 1, "local").resolution_complete);
}

#[test]
fn deferred_writes_union_sites_in_declaring_and_sibling_owners() {
    for effect in [
        "const pick = () => { key = path; }; pick();",
        "xs.sort(() => { key = path; });",
        "Array.from(xs, () => { key = path; });",
        "text.replace(re, () => { key = path; });",
        "new Promise(() => { key = path; });",
        "_.each(xs, () => { key = path; });",
        "xs.forEach(async () => { key = path; });",
        "class C { field = (key = path); }",
    ] {
        let source = format!(
            "function f(o, path, xs, stable) {{\n let key = 'literal'; const early = () => read(key);\n {effect}\n o[key].x = 1; read(stable);\n}}"
        );
        let file = js(&source);
        let early = read(&file, 2, "key");
        let late = read(&file, 4, "key");
        assert!(
            !early.resolution_complete && !late.resolution_complete,
            "{effect}: {early:?}, {late:?}"
        );
        assert_eq!(early.reaching_versions, late.reaching_versions, "{effect}");
        assert_eq!(late.reaching_versions.len(), 2, "{effect}");
        assert!(read(&file, 4, "stable").resolution_complete);
        assert_eq!(fact_lines(&file), [4], "{effect}");
    }

    let file = js(concat!(
        "function f(o) { let key = o;\n",
        "  const a = () => read(key); key = 'final';\n",
        "  read(key);\n",
        "}\n",
    ));
    let captured = read(&file, 2, "key");
    assert!(!captured.resolution_complete);
    assert_eq!(captured.reaching_versions.len(), 2);
    assert!(read(&file, 3, "key").resolution_complete);
}

#[test]
fn mapped_arguments_respects_strictness_and_simple_formals() {
    for (parameter, directive, complete) in [
        ("key", "", false),
        ("key", "'use strict';", true),
        ("key = 0", "", true),
    ] {
        let file = js(&format!(
            "function f({parameter}, next) {{ {directive}\n read(key); arguments[0] = next; read(key);\n}}"
        ));
        let keys = reads(&file, 2, "key");
        assert_eq!(keys.len(), 2);
        assert!(
            keys.iter()
                .all(|r| r.binding.is_some() && r.resolution_complete == complete),
            "{parameter} {directive}: {keys:?}"
        );
    }
    let file = js("let arguments; function f(key) { var arguments; arguments[0] = 1; read(key); }");
    let mapped = read(&file, 1, "key");
    assert!(!mapped.resolution_complete && mapped.reaching_versions.len() == 2);
    let file = js("function f(arguments, key) { arguments[0] = 1; read(key); }");
    assert!(read(&file, 1, "key").resolution_complete);
    let file = js("export function f(key) { arguments[0] = 1; read(key); }");
    assert!(read(&file, 1, "key").resolution_complete);
}

#[test]
fn class_with_export_and_tsx_value_positions() {
    let file = js(concat!(
        "let k = 1; const env = {};\n",
        "with(env) { const local = 1; read(k); read(local); }\n",
        "const C = class Name { static { var k = 2; read(k); } };\n",
        "read(k);\n",
        "export { C as Renamed };\n",
    ));
    let dynamic = read(&file, 2, "k");
    assert!(dynamic.binding.is_none() && !dynamic.resolution_complete);
    assert!(read(&file, 2, "local").resolution_complete);
    assert!(reads(&file, 3, "Name").is_empty());
    let inner = read(&file, 3, "k");
    let outer = read(&file, 4, "k");
    assert!(inner.resolution_complete && outer.resolution_complete);
    assert_ne!(inner.binding, outer.binding);
    assert!(read(&file, 5, "C").resolution_complete);
    assert!(reads(&file, 5, "Renamed").is_empty());

    let tsx = ParsedFile::parse(
        "test.tsx",
        concat!(
            "const decorate = (x: any) => x;\n",
            "class View { @decorate method() {} }\n",
            "const render = <section><View></View></section>;\n",
            "read(View);\n",
        ),
        Language::Tsx,
    )
    .unwrap();
    assert!(read(&tsx, 2, "decorate").resolution_complete);
    assert!(read(&tsx, 3, "View").resolution_complete);
    assert!(reads(&tsx, 3, "section").is_empty());
    assert!(read(&tsx, 4, "View").resolution_complete);
}

#[test]
fn member_effects_follow_aliases_and_the_captured_receiver() {
    let file = js(concat!(
        "function f(opts, next) { const other={key:0, child:{key:0}};\n",
        "  Object.assign(opts, {key: next}); const a=opts.key, b=other.key;\n",
        "  const alias = other.child; alias.key = next; read(other.child.key);\n",
        "}\n",
        "function g(obj) { const alias = obj, next={key:0};\n",
        "  obj.key = (obj = next); const a=alias.key, b=obj.key;\n",
        "}\n",
        "function h(obj) { const free={key:0}; const later = () => { const alias = obj.child; alias.key = 1; };\n",
        "  read(obj.key); read(free.key);\n",
        "}\n",
    ));
    for (line, changed, stable) in [
        (2, "opts.key", "other.key"),
        (6, "alias.key", "obj.key"),
        (9, "obj.key", "free.key"),
    ] {
        assert!(
            !read(&file, line, changed).resolution_complete,
            "{changed}:{line}"
        );
        assert!(
            read(&file, line, stable).resolution_complete,
            "{stable}:{line}"
        );
    }
    let child = read(&file, 3, "other.child.key");
    assert!(!child.resolution_complete);
    assert_eq!(child.reaching_versions.len(), 2);
}

#[test]
fn captures_target_once_and_preserves_intermediate_exception_versions() {
    let file = js(concat!(
        "function f(root, k, n) {\n",
        "  get()[k] = (k = n); read(k);\n",
        "  let cursor = root;\n",
        "  try { cursor = root[k]; work(); cursor = root; } catch { read(cursor); }\n",
        "  cursor.x = 1;\n",
        "  try { cursor = root[k]; cursor = root; } catch {}\n",
        "  cursor = root; cursor.x = 2;\n",
        "}\n",
    ));
    let key = reads(&file, 2, "k");
    assert_eq!(key.len(), 2);
    assert!(key
        .iter()
        .all(|r| r.resolution_complete && r.reaching_versions.len() == 1));
    assert_ne!(key[0].reaching_versions, key[1].reaching_versions);
    let caught = read(&file, 4, "cursor");
    assert!(!caught.resolution_complete && caught.reaching_versions.len() >= 2);
    assert_eq!(fact_lines(&file), [5]);
}

#[test]
fn tagged_continue_reaches_increment_but_break_does_not() {
    let file = js(concat!(
        "function f(root, k, cond) { let cursor = root;\n",
        "  again: for (; cond; cursor = root[k]) { continue again; }\n",
        "  cursor.x = 1; cursor = root;\n",
        "  stop: for (; cond; cursor = root[k]) { break stop; }\n",
        "  cursor.x = 1;\n",
        "}\n",
    ));
    assert_eq!(fact_lines(&file), [3]);
}

#[test]
fn operand_alternatives_inline_consumption_and_primitive_reuse() {
    let source = std::fs::read_to_string(format!("{MATRIX}/cases/t3-054.js")).unwrap();
    let facts = js(&source).js_ts_proto_writes();
    assert!(selector(facts.last().unwrap(), 10, "key"));
    let file = js(concat!(
        "function f(o,k,p,other) {\n",
        "  const parts = [k]; o[parts.pop()][p] = 1;\n",
        "  o[k ||= 'default'][p] = 1;\n",
        "  const s = String(other); const child = o[s];\n",
        "  child[s] = 1; child[p] = 2;\n",
        "  let t = String(other); const next = o[t];\n",
        "  t = String(p); next[t] = 1;\n",
        "}\n",
    ));
    let facts = file.js_ts_proto_writes();
    assert_eq!(
        facts.iter().map(|fact| fact.line).collect::<Vec<_>>(),
        [2, 3, 5, 5, 7]
    );
    assert!(selector(&facts[0], 2, "k"));
    assert!(selector(&facts[1], 3, "k"));
    assert!(selector(&facts[2], 5, "s"));
    assert!(!selector(&facts[3], 5, "p"));
    assert!(!selector(&facts[4], 7, "t"));
    assert_eq!(
        reads(&file, 4, "s")[0].reaching_versions,
        reads(&file, 5, "s")[0].reaching_versions
    );
}

#[test]
fn element_versions_include_the_index_and_change_after_consumption() {
    let file = js(concat!(
        "function f(o, i, j) { const parts=[0,1];\n",
        "  const a=parts[i], b=parts[i]; i = j; const c=parts[i];\n",
        "  parts.shift(); read(parts[i]);\n",
        "}\n",
    ));
    let elements = reads(&file, 2, "parts.[]");
    assert_eq!(elements.len(), 3);
    assert!(elements.iter().all(|r| r.resolution_complete));
    assert_eq!(elements[0].reaching_versions.len(), 2);
    assert_eq!(elements[0].reaching_versions, elements[1].reaching_versions);
    assert_ne!(elements[1].reaching_versions, elements[2].reaching_versions);
    assert!(!read(&file, 3, "parts.[]").resolution_complete);
}

#[test]
fn selector_value_fallback_covers_receivers_calls_and_tagged_templates() {
    let file = js(concat!(
        "function f(o, untouched, k, p, path, i, tag, discard) {\n",
        "  o[k.toLowerCase()][p] = 1;\n",
        "  o[String(k).trim()][p] = 1;\n",
        "  o[k?.trim()][p] = 1;\n",
        "  o = o[path.split('.')[i]]; o.x = 1;\n",
        "  while (path.length) o = o[path.shift()]; o[p] = 1;\n",
        "  o[tag`text${k}`][p] = 1;\n",
        "  o[(discard, k)][p] = 1;\n",
        "  untouched[void discard][p] = 1; untouched[typeof discard][p] = 1;\n",
        "  o[path.at(i)][p] = 1;\n",
        "  untouched[lookup()[k]][p] = 1;\n",
        "}\n",
    ));
    let facts = file.js_ts_proto_writes();
    assert_eq!(
        facts.iter().map(|f| f.line).collect::<Vec<_>>(),
        [2, 3, 4, 5, 6, 7, 8, 10, 11]
    );
    for (fact, base) in facts
        .iter()
        .zip(["k", "k", "k", "path", "path", "k", "k", "path", "k"])
    {
        assert!(
            fact.key_operands
                .iter()
                .any(|op| op.base == base && op.role == KeyRole::Selector),
            "{fact:?}"
        );
        assert!(
            fact.key_operands.iter().all(|op| op.base != "discard"),
            "{fact:?}"
        );
    }
}

#[test]
fn reducer_checks_its_accumulator_and_reachable_returns_and_pattern_punctuation() {
    let file = js(concat!(
        "function f(root, keys, other, flag, k) {\n",
        "  keys.reduce((acc,key) => other[key] = other[key] || {}, root).x = 1;\n",
        "  keys.reduce((acc,key) => acc[key] = acc[key] || {}).x = 1;\n",
        "  keys.reduce((acc,key) => { const b = flag ? acc : acc; return b[key]; }, root).x = 1;\n",
        "  keys.reduce((acc,key) => { return acc; return acc[key]; }, root).x = 1;\n",
        "  const [/* , */ c] = [root[k], {}]; c.x = 1;\n",
        "  const [, e] = [root[k], {}]; e.x = 1;\n",
        "  keys.reduce((acc,key) => acc[key] = (key=k, acc[key] || {}), root).x = 1;\n",
        "}\n",
    ));
    assert_eq!(fact_lines(&file), [4, 6, 8]);
    let outer = file.js_ts_proto_writes().pop().unwrap();
    assert!(file.source[outer.start_byte..outer.end_byte].ends_with("root).x"));
}

#[test]
fn exact_write_cap_keeps_reads_and_larger_owners_only_inventory() {
    // Put synthetic stores on separate lines. A single query for a minified
    // line could legitimately materialize every candidate for every receiver.
    let source = format!(
        "function edge(o,k,p) {{ {} }}\nfunction big(o,k,p) {{ {} }}\nfunction small(o,k,p) {{ o[k][p]=1; }}",
        "o[k][p]=1;\n".repeat(10_000),
        "o[k][p]=1;\n".repeat(10_001)
    );
    let file = js(&source);
    let facts = file.js_ts_proto_writes();
    assert_eq!(facts.len(), 10_001);
    assert_eq!(facts.last().unwrap().function, "small");
    assert!(read(&file, 1, "k").resolution_complete);
    let skipped = file.js_ts_name_reads(10_002);
    assert!(!skipped.is_empty());
    assert!(skipped.iter().all(|r| !r.resolution_complete));
    assert!(read(&file, 20_004, "k").resolution_complete);
}

#[test]
fn same_slot_respects_key_rebinding_inside_a_store() {
    let file = js(concat!(
        "function f(root, k, other) {\n",
        "  let cursor = root;\n",
        "  cursor = cursor[k] = (k = other, cursor[k] || {});\n",
        "  cursor = cursor[k] = cursor[k] || {};\n",
        "}\n",
    ));
    assert_eq!(fact_lines(&file), [4]);
}

#[test]
fn logical_assignment_keeps_the_bypass_version() {
    let file = js(concat!(
        "function f(input, other) { let key = input;\n",
        "  key ||= other; read(key);\n",
        "  key = other; read(key);\n",
        "}\n",
    ));
    let bypass = reads(&file, 2, "key");
    assert_eq!(bypass.len(), 2);
    assert!(bypass.iter().all(|r| r.resolution_complete));
    assert_eq!(bypass[0].reaching_versions.len(), 1);
    assert_eq!(bypass[1].reaching_versions.len(), 2);
    assert_eq!(read(&file, 3, "key").reaching_versions.len(), 1);
}

#[test]
fn implicit_catch_refuses_a_complete_read_with_no_explicit_throw() {
    let file = js(concat!(
        "function f(key) {\n",
        "  try { work(); } catch { read(key); }\n",
        "  read(key);\n",
        "}\n",
    ));
    let caught = read(&file, 2, "key");
    assert!(!caught.resolution_complete);
    assert_eq!(caught.reaching_versions.len(), 1);
    // A separate owner is unaffected by refusal at a catch entry.
    let control = js("function f(key) { work(); read(key); }");
    assert!(read(&control, 1, "key").resolution_complete);
}

#[test]
fn forty_nested_loops_and_inline_callbacks_are_bounded() {
    const DEPTH: usize = 40;
    let mut source = String::from("function f(root, key, flag, list, stable) { let cursor=root;\n");
    for i in 0..DEPTH {
        source.push_str(&format!("for (var i{i}=0; i{i}<1; i{i}++) {{ "));
    }
    source.push_str("cursor=root[key];");
    source.push_str(&"}".repeat(DEPTH));
    for _ in 0..DEPTH {
        source.push_str("do {");
    }
    source.push_str("cursor=root[key];");
    source.push_str(&"} while(flag);".repeat(DEPTH));
    for _ in 0..DEPTH {
        source.push_str("list.forEach(() => {");
    }
    source.push_str("cursor=root[key];");
    source.push_str(&"});".repeat(DEPTH));
    source.push_str("\ncursor.x=1; read(stable);\n}");
    let file = bounded_parse(source);
    assert_eq!(fact_lines(&file), [3]);
    assert!(read(&file, 3, "stable").resolution_complete);
}

#[test]
fn minified_two_mib_file_refuses_expensive_owner_but_keeps_adjacent_owner() {
    let mut source = String::from("function giant(o,k,p) { read(p); o[k][p]=1;\n");
    for index in 0..2_048 {
        source.push_str(&format!("let b{index}=k;"));
    }
    while source.len() < 2 * 1024 * 1024 {
        source.push_str("if(p)b1=k;");
    }
    source.push_str("read(k);}\nfunction small(o,k,p) { o[k][p]=1; }");
    let file = bounded_parse(source);
    assert_eq!(fact_lines(&file), [3]);
    assert!(file
        .js_ts_name_reads(1)
        .iter()
        .all(|r| !r.resolution_complete));
    let skipped = file.js_ts_name_reads(2);
    assert!(skipped.iter().any(|r| r.path == "k"));
    assert!(skipped.iter().all(|r| !r.resolution_complete));
    assert!(read(&file, 3, "k").resolution_complete);
}

fn bounded_parse(source: String) -> ParsedFile {
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let file = js(&source);
        file.js_ts_proto_writes();
        send.send(file).unwrap();
    });
    // The probe asserts the work budget holds, not speed. On the coverage-instrumented CI
    // runner it took 31.2 s at c21952a8 and 30.1 to 31.6 s on fold 3 before the byte scan,
    // and 23.4 s on 136ddee0; 30 s leaves a margin of 6.6 s over that last measurement.
    receive
        .recv_timeout(std::time::Duration::from_secs(30))
        .expect("prototype-write analysis did not finish within 30 seconds")
}

#[test]
fn incomplete_single_version_cannot_discharge_a_selector() {
    let file = js(concat!(
        "function f(o) { const key='literal';\n",
        "  o[key].x = 1;\n",
        "  try { work(); } catch { o[key].x = 2; }\n",
        "}\n",
    ));
    let complete = read(&file, 2, "key");
    let incomplete = read(&file, 3, "key");
    assert!(complete.resolution_complete && !incomplete.resolution_complete);
    assert_eq!(complete.reaching_versions, incomplete.reaching_versions);
    assert_eq!(fact_lines(&file), [3]);
}

#[test]
fn long_receiver_alias_chain_has_a_live_selector_at_its_tail() {
    let mut source = String::from("function f(root,key){let c0=root[key];");
    for index in 1..=4_096 {
        source.push_str(&format!("let c{index}=c{};", index - 1));
    }
    source.push_str("\nc4096.x=1; c4096={}; c4096.x=2; }");
    let file = bounded_parse(source);
    assert_eq!(fact_lines(&file), [2]);
    assert!(selector(&file.js_ts_proto_writes()[0], 1, "key"));
}

#[test]
fn loop_entry_includes_a_member_write_through_an_alias_created_in_the_body() {
    let file = js(concat!(
        "function f(obj, flag) { const other={child:{key:0}};\n",
        "  while (flag) { const alias = obj.child;\n",
        "    const a=obj.child.key, b=other.child.key; alias.key = 1;\n",
        "  }\n",
        "}\n",
    ));
    let affected = read(&file, 3, "obj.child.key");
    assert!(!affected.resolution_complete);
    assert_eq!(affected.reaching_versions.len(), 2);
    assert!(read(&file, 3, "other.child.key").resolution_complete);
}

#[test]
fn reducer_uses_the_return_that_completes_after_finally() {
    let file = js(concat!(
        "function f(root, keys) {\n",
        "  keys.reduce((acc,key) => { try { return acc[key]; } finally { return acc; } }, root).x = 1;\n",
        "  keys.reduce((acc,key) => { try { return acc[key]; } finally { sideEffect(); } }, root).x = 2;\n",
        "}\n",
    ));
    assert_eq!(fact_lines(&file), [3]);
}
