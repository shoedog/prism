use super::proto_writes::{GuardState, KeyRole, ProtoWrite, ProtoWriteClass};
use crate::{ast::ParsedFile, languages::Language};
use serde::Deserialize;
use std::collections::BTreeMap;
use tree_sitter::Node;

const MATRIX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/proto_writes_matrix");

#[derive(Deserialize)]
struct Row {
    id: String,
    file: String,
    write_line: usize,
    truth: String,
    want_class: String,
    want_guard: String,
    attacker_names: Vec<String>,
}

// Derive the target from assignment positions, independently of the fact extractor.
fn targets(node: Node<'_>, line: usize, spans: &mut Vec<(usize, usize)>) {
    let mut parent = node;
    if matches!(node.kind(), "member_expression" | "subscript_expression")
        && node.start_position().row + 1 == line
    {
        while let Some(outer) = parent.parent() {
            let field = match outer.kind() {
                "assignment_expression"
                | "augmented_assignment_expression"
                | "assignment_pattern"
                | "for_in_statement" => Some("left"),
                "update_expression" => Some("argument"),
                "pair_pattern" => Some("value"),
                "array_pattern"
                | "object_pattern"
                | "rest_pattern"
                | "parenthesized_expression"
                | "non_null_expression" => {
                    parent = outer;
                    continue;
                }
                _ => None,
            };
            if field.and_then(|field| outer.child_by_field_name(field)) == Some(parent) {
                if matches!(outer.kind(), "assignment_pattern" | "pair_pattern") {
                    parent = outer;
                    continue;
                }
                spans.push((node.start_byte(), node.end_byte()));
            }
            break;
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        targets(child, line, spans);
    }
}

fn matches_target_facts(
    facts: &[&ProtoWrite],
    mut expected: Vec<(usize, usize)>,
    class: ProtoWriteClass,
    allowed: &[String],
    required: &[&str],
) -> bool {
    let mut actual: Vec<_> = facts
        .iter()
        .map(|fact| (fact.start_byte, fact.end_byte))
        .collect();
    actual.sort_unstable();
    expected.sort_unstable();
    actual == expected
        && facts.iter().all(|fact| {
            let selectors: Vec<_> = fact
                .key_operands
                .iter()
                .filter(|operand| operand.role == KeyRole::Selector)
                .map(|operand| operand.base.as_str())
                .collect();
            fact.class == class
                && fact.guard == GuardState::None
                && selectors
                    .iter()
                    .any(|base| allowed.iter().any(|name| name == base))
                && required.iter().all(|base| selectors.contains(base))
        })
}

#[test]
fn matrix_assertion_refuses_extra_targets_and_missing_required_witnesses() {
    let file = js("function f(o,req,entry) { o[req].x=1; o[entry].x=2; }");
    let facts = file.js_ts_proto_writes();
    let [good, wrong] = facts.as_slice() else {
        panic!("{facts:?}")
    };
    let allowed = ["entry".to_owned(), "req".to_owned()];
    let check = |facts: &[&ProtoWrite], expected| {
        matches_target_facts(
            facts,
            expected,
            ProtoWriteClass::NestedKeys,
            &allowed,
            &["req"],
        )
    };
    let span = (good.start_byte, good.end_byte);
    assert!(check(&[good], vec![span]));
    assert!(!check(&[good, good], vec![span]));
    assert!(!check(&[good, wrong], vec![span]));
    assert!(!check(&[wrong], vec![(wrong.start_byte, wrong.end_byte)]));
}

#[test]
fn matrix_nonrecursive_polluting_rows() {
    let rows = std::fs::read_to_string(format!("{MATRIX}/rows.jsonl")).unwrap();
    let mut counts = BTreeMap::<String, (usize, usize)>::new();
    let mut failures = Vec::new();
    assert_eq!(rows.lines().count(), 326);
    for row in rows
        .lines()
        .map(|line| serde_json::from_str::<Row>(line).unwrap())
    {
        if row.truth != "pollutes" || row.want_class == "recursive_merge" {
            continue;
        }
        let source = std::fs::read_to_string(format!("{MATRIX}/{}", row.file)).unwrap();
        let language = if row.file.ends_with(".ts") {
            Language::TypeScript
        } else {
            Language::JavaScript
        };
        let parsed = ParsedFile::parse(&row.file, &source, language).unwrap();
        let mut spans = Vec::new();
        targets(parsed.tree.root_node(), row.write_line, &mut spans);
        if row.id == "pw04" {
            assert_eq!(spans, [(31, 114), (67, 71)]);
        } else if row.want_class != "none" {
            assert_eq!(spans.len(), 1, "{}: designated targets {spans:?}", row.id);
        }
        assert!(matches!(row.want_guard.as_str(), "none" | "partial"));
        let facts = parsed.js_ts_proto_writes();
        let on_line: Vec<_> = facts
            .iter()
            .filter(|fact| fact.line == row.write_line)
            .collect();
        let class = match row.want_class.as_str() {
            "nested_keys" => Some(ProtoWriteClass::NestedKeys),
            "path_walk" => Some(ProtoWriteClass::PathWalk),
            "none" => None,
            other => panic!("unknown class {other}"),
        };
        let passed = if let Some(class) = class {
            // The contract names both positive witnesses for this loop and the
            // request path, rather than the local receiver, for t3-051.
            let required: &[&str] = match row.id.as_str() {
                "t3-051" => &["req"],
                "t3-054" => &["pending", "key"],
                _ => &[],
            };
            matches_target_facts(
                &on_line,
                spans.clone(),
                class,
                &row.attacker_names,
                required,
            )
        } else {
            on_line.is_empty()
        };
        let count = counts.entry(row.want_class).or_default();
        count.0 += usize::from(passed);
        count.1 += 1;
        if !passed {
            failures.push(format!("{} at {:?}: {on_line:?}", row.id, spans));
        }
    }
    eprintln!("matrix pass/total {counts:?}");
    assert_eq!(counts.values().map(|count| count.1).sum::<usize>(), 134);
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn js(source: &str) -> ParsedFile {
    ParsedFile::parse("test.js", source, Language::JavaScript).unwrap()
}

#[test]
fn same_line_inventory_distinguishes_stable_reads_rebinding_and_owners() {
    let source = concat!(
        "function update(root, v, key, next, field) {\n",
        "  if (key) root[key][field] = other[key]; root[key][field] = key;\n",
        "  root[key][field] = key; key = next; sink(key);\n",
        "  [key].map(key => read(key)); read(key); root[key][field] = missing;\n",
        "  read(key); read(unknown);\n",
        "}\n"
    );
    let file = js(source);
    let reads = |line| {
        file.js_ts_name_reads(line)
            .into_iter()
            .filter(|read| read.base == "key")
            .collect::<Vec<_>>()
    };
    let stable = reads(2);
    assert_eq!(stable.len(), 5);
    assert!(stable.iter().all(|read| {
        read.binding == stable[0].binding
            && read.owner == stable[0].owner
            && read.reaching_versions == stable[0].reaching_versions
            && read.resolution_complete
    }));
    let facts = file.js_ts_proto_writes();
    assert_eq!(facts.iter().filter(|write| write.line == 2).count(), 2);
    assert!(facts.iter().filter(|write| write.line == 2).all(|write| {
        let selector = write
            .key_operands
            .iter()
            .find(|operand| operand.role == KeyRole::Selector)
            .unwrap();
        selector.binding == stable[0].binding
            && selector.reaching_versions == stable[0].reaching_versions
            && selector.start_byte > stable[0].span.start_byte
    }));
    let rebound = reads(3);
    assert_eq!(rebound.len(), 3);
    assert_eq!(rebound[0].reaching_versions, rebound[1].reaching_versions);
    assert_eq!(rebound[0].binding, rebound[2].binding);
    assert_ne!(rebound[0].reaching_versions, rebound[2].reaching_versions);
    assert_eq!(
        rebound[2].reaching_versions[0].start_byte,
        source.find("key = next").unwrap()
    );

    let sibling = reads(4);
    assert_eq!(sibling.len(), 4);
    assert_ne!(sibling[1].owner, sibling[2].owner);
    assert_ne!(sibling[1].binding, sibling[2].binding);
    assert_eq!(sibling[2].binding, sibling[3].binding);
    assert!(file
        .js_ts_name_reads(4)
        .iter()
        .any(|read| read.path == "missing" && read.binding.is_none() && !read.resolution_complete));
    assert_eq!(reads(5).len(), 1);
    assert!(file
        .js_ts_name_reads(5)
        .iter()
        .any(|read| read.path == "unknown" && !read.resolution_complete));
    assert!(file.js_ts_name_reads(6).is_empty());
    assert!(file.js_ts_name_reads(0).is_empty());
}

#[test]
fn shadowed_or_mutated_reads_remain_in_the_inventory() {
    let file = js(concat!(
        "function run(opts, part, one, two, cond) {\n",
        "  read(opts.key); opts[part] = two; read(opts.key);\n",
        "  if (cond) part = one; read(part);\n",
        "  { read(part); let part = two; read(part); }\n",
        "}\n"
    ));
    let reads = file.js_ts_name_reads(2);
    let member: Vec<_> = reads
        .iter()
        .filter(|read| read.path == "opts.key")
        .collect();
    assert_eq!(member.len(), 2);
    assert!(member[0].resolution_complete);
    assert!(!member[1].resolution_complete);
    let join = file
        .js_ts_name_reads(3)
        .into_iter()
        .find(|read| read.base == "part")
        .unwrap();
    assert_eq!(join.reaching_versions.len(), 2);
    let local: Vec<_> = file
        .js_ts_name_reads(4)
        .into_iter()
        .filter(|read| read.base == "part")
        .collect();
    assert_eq!(local.len(), 2);
    assert_ne!(local[0].binding, join.binding);
    assert_eq!(local[0].binding, local[1].binding);
    assert!(!local[0].resolution_complete);
    assert!(local[1].resolution_complete);
}

#[test]
fn earlier_consumed_element_is_a_selector_and_leaf_is_a_property() {
    let source = concat!(
        "function run(root, keyPath, value) {\n",
        "  let cursor = root;\n",
        "  for (let i = 0; i < keyPath.length - 1; i++) cursor = cursor[keyPath[i]];\n",
        "  cursor[keyPath[keyPath.length - 1]] = value;\n",
        "}\n"
    );
    let file = js(source);
    let facts = file.js_ts_proto_writes();
    let [fact] = facts.as_slice() else {
        panic!("{facts:?}")
    };
    let [earlier, final_key] = fact.key_operands.as_slice() else {
        panic!("{fact:?}")
    };
    assert_eq!(earlier.name, final_key.name);
    assert_eq!(earlier.binding, final_key.binding);
    assert_eq!((earlier.line, earlier.role), (3, KeyRole::Selector));
    assert_eq!((final_key.line, final_key.role), (4, KeyRole::Property));
    assert_eq!(&source[earlier.start_byte..earlier.end_byte], "keyPath[i]");
    assert_eq!(
        &source[final_key.start_byte..final_key.end_byte],
        "keyPath[keyPath.length - 1]"
    );
    assert!(file
        .js_ts_name_reads(4)
        .iter()
        .any(|read| read.path == "keyPath.length"));
}

#[test]
fn captured_inline_steps_live_until_the_cursor_is_replaced() {
    let source = concat!(
        "function run(root, segments, field, value) {\n",
        "  let cursor = root;\n",
        "  segments.forEach(key => { cursor = cursor[key]; });\n",
        "  cursor[field] = value;\n",
        "  cursor = { ...cursor };\n",
        "  cursor[field] = value;\n",
        "  cursor = root[field];\n",
        "  function deferred() { cursor[field] = value; }\n",
        "  cursor = root;\n",
        "  cursor[field] = value;\n",
        "}\n"
    );
    let facts = js(source).js_ts_proto_writes();
    let [fact] = facts.as_slice() else {
        panic!("{facts:?}")
    };
    assert_eq!(
        (fact.line, fact.class, fact.function.as_str()),
        (4, ProtoWriteClass::PathWalk, "run")
    );
    let key = fact
        .key_operands
        .iter()
        .find(|key| key.name == "key")
        .unwrap();
    assert_eq!((key.line, key.role), (3, KeyRole::Selector));
    assert!(
        fact.owner.start_byte < key.owner.start_byte && key.owner.end_byte < fact.owner.end_byte
    );
}

#[test]
fn wrapped_logical_and_pattern_targets_receive_individual_facts() {
    let source = concat!(
        "function update(o: any, k: string, p: string, src: any) {\n",
        "  (o as any)![k as string][p] ??= src;\n",
        "  [o[k], o[k][p]] = src;\n",
        "  ({ a: o[k].prototype.x, b: o[k].private.x } = src);\n",
        "}\n"
    );
    let file = ParsedFile::parse("test.ts", source, Language::TypeScript).unwrap();
    let facts = file.js_ts_proto_writes();
    assert_eq!(
        facts
            .iter()
            .map(|fact| &source[fact.start_byte..fact.end_byte])
            .collect::<Vec<_>>(),
        ["(o as any)![k as string][p]", "o[k][p]", "o[k].prototype.x"]
    );
    assert!(facts
        .iter()
        .all(|fact| fact.class == ProtoWriteClass::NestedKeys
            && fact.key_operands.iter().any(|operand| {
                operand.role == KeyRole::Selector
                    && operand.name == "k"
                    && &source[operand.start_byte..operand.end_byte] == "k"
            })));
}

#[test]
fn designated_sibling_and_chained_writes_have_separate_spans() {
    for (name, line, expected) in [
        ("t2-110", 7, "catalog[section][field]"),
        ("t2-127", 7, "sections[section][field]"),
    ] {
        let source = std::fs::read_to_string(format!("{MATRIX}/cases/{name}.js")).unwrap();
        let facts: Vec<_> = js(&source)
            .js_ts_proto_writes()
            .into_iter()
            .filter(|fact| fact.line == line)
            .collect();
        assert_eq!(
            facts
                .iter()
                .map(|fact| &source[fact.start_byte..fact.end_byte])
                .collect::<Vec<_>>(),
            [expected]
        );
    }
}

#[test]
fn oversized_owner_does_not_suppress_adjacent_owners() {
    let source = format!(
        "function big(o,k,p) {{ {} }} function small(o,k,p) {{ o[k][p]=1; }}",
        "o[k][p]=1;".repeat(10_001)
    );
    let facts = js(&source).js_ts_proto_writes();
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].function, "small");
}

#[test]
fn same_line_unresolved_competitor_retains_its_program_owner() {
    let source = "function f(o, key, p) { o[key][p] = key; } read(key);";
    let file = js(source);
    let reads: Vec<_> = file
        .js_ts_name_reads(1)
        .into_iter()
        .filter(|read| read.base == "key")
        .collect();
    assert_eq!(reads.len(), 3);
    assert_eq!(reads[0].binding, reads[1].binding);
    assert_eq!(reads[2].owner.end_byte, source.len());
    assert!(
        reads[2].binding.is_none()
            && reads[2].reaching_versions.is_empty()
            && !reads[2].resolution_complete
    );
}

#[test]
fn finally_restores_the_receiver_on_continue_and_break() {
    let file = js(concat!(
        "function f(root, keys, choose) {\n",
        "  let cursor = root;\n",
        "  for (const key of keys) {\n",
        "    try { cursor = root[key]; if (choose) continue; break; }\n",
        "    finally { cursor = root; }\n",
        "  }\n",
        "  cursor.field = 1;\n",
        "  cursor = root[choose];\n",
        "  cursor.field = 2;\n",
        "}\n",
    ));
    let facts = file.js_ts_proto_writes();
    assert_eq!(facts.iter().map(|fact| fact.line).collect::<Vec<_>>(), [9]);
}

#[test]
fn reduce_write_fact_keeps_the_legacy_access_path_separate() {
    let source = std::fs::read_to_string(format!("{MATRIX}/cases/pw04.js")).unwrap();
    let file = js(&source);
    let target = file
        .tree
        .root_node()
        .descendant_for_byte_range(31, 114)
        .unwrap();
    assert_eq!((target.start_byte(), target.end_byte()), (31, 114));
    let paths = file.recorded_write_paths(target);
    assert_eq!(
        paths.iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["keys.slice(0, -1).reduce((o, k) => (o[]"]
    );
}

#[test]
fn reducer_requires_a_child_of_the_live_accumulator() {
    let file = js(concat!(
        "function f(root, other, keys) {\n",
        "  keys.reduce((o, k) => { const alias = o; return alias[k]; }, root).field = 1;\n",
        "  keys.reduce((o, k) => { o = other; return o[k]; }, root).field = 2;\n",
        "}\n",
    ));
    let facts = file.js_ts_proto_writes();
    assert_eq!(
        facts
            .iter()
            .map(|fact| (fact.line, fact.class))
            .collect::<Vec<_>>(),
        [(2, ProtoWriteClass::PathWalk)]
    );
}

#[test]
fn nullish_cursor_installation_reports_the_later_write() {
    let file = js(concat!(
        "function f(root, key) {\n",
        "  const selected = root[key] ??= {};\n",
        "  selected.enabled = true;\n",
        "}\n",
    ));
    let facts = file.js_ts_proto_writes();
    let [fact] = facts.as_slice() else {
        panic!("{facts:?}")
    };
    assert_eq!((fact.line, fact.class), (3, ProtoWriteClass::PathWalk));
    assert!(fact
        .key_operands
        .iter()
        .any(|key| key.name == "key" && key.line == 2));
}

#[test]
fn member_mutation_through_a_direct_alias_does_not_follow_a_rebind() {
    let file = js(concat!(
        "function f(opts, key, replacement) {\n",
        "  const alias = opts;\n",
        "  alias[key] = 1; read(opts.key);\n",
        "  opts = replacement; read(opts.key); read(alias.key);\n",
        "}\n",
    ));
    let after_mutation = file.js_ts_name_reads(3);
    assert!(after_mutation
        .iter()
        .any(|read| read.path == "opts.key" && !read.resolution_complete));
    let after_rebind = file.js_ts_name_reads(4);
    assert!(after_rebind
        .iter()
        .any(|read| read.path == "opts.key" && read.resolution_complete));
    assert!(after_rebind
        .iter()
        .any(|read| read.path == "alias.key" && !read.resolution_complete));
}

#[test]
fn iteration_requires_a_synchronous_callback_in_the_callback_position() {
    let file = js(concat!(
        "function f(root, keys, identity, field) {\n",
        "  let cursor = root;\n",
        "  keys.forEach(identity, key => { cursor = cursor[key]; });\n",
        "  cursor[field] = 1;\n",
        "  keys.forEach(async key => { await pause(); cursor = cursor[key]; });\n",
        "  cursor[field] = 2;\n",
        "  keys.forEach(key => { cursor = cursor[key]; });\n",
        "  cursor[field] = 3;\n",
        "}\n",
    ));
    let facts = file.js_ts_proto_writes();
    assert_eq!(facts.iter().map(|fact| fact.line).collect::<Vec<_>>(), [8]);
}

#[path = "proto_writes_fold_tests.rs"]
mod fold;
