use super::pattern_formals_tests::{evidence, refs};
use crate::ast::ParsedFile;
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

// These fixtures put the only matching body identifier in the return operand.
// A simple assignment target in this syntax walk contributes no value read.
fn syntax_read_count(source: &str, name: &str) -> u64 {
    let parsed = ParsedFile::parse("test", source, Language::JavaScript).unwrap();
    assert!(!parsed.tree.root_node().has_error(), "{source}");
    let owner = parsed.all_functions()[0];
    let mut stack = vec![owner.child_by_field_name("body").unwrap()];
    let mut count = 0;
    while let Some(node) = stack.pop() {
        if node.kind() == "assignment_expression" {
            stack.extend(node.child_by_field_name("right"));
            continue;
        }
        if node.kind() == "identifier" && parsed.node_text(&node) == name {
            count += 1;
        }
        stack.extend(node.named_children(&mut node.walk()));
    }
    count
}

#[test]
fn member_read_modify_write_counts_and_justifies_an_entry_def() {
    for (operand, count) in [
        ("o.k++", 1),
        ("++o.k", 1),
        ("o.k += 1", 1),
        ("o.k ||= 1", 1),
        ("o.k &&= 1", 1),
        ("o.k ??= 1", 1),
        ("o[k]++", 1),
        ("o[k] += 1", 1),
        ("o[k] ||= 1", 1),
        ("o.k = 1", 0),
        ("o[k] = 1", 0),
    ] {
        let source = format!("function C({{o}}) {{ return {operand} }}");
        let expected = syntax_read_count(&source, "o");
        assert_eq!(expected, count, "{operand}: syntax fixture");
        let receipt = evidence(&source, Language::JavaScript);
        assert_eq!(receipt.counters()["candidates"], expected, "{operand}");
        assert!(receipt.files["test"]
            .attempts
            .iter()
            .all(|a| a.justifies_def));
        assert_eq!(
            refs(&source, Language::JavaScript)
                .defs
                .values()
                .flatten()
                .any(|def| def.start_byte == 12),
            count != 0,
            "{operand}",
        );
    }
}

#[test]
fn synthetic_eval_guard_refuses_a_counted_read() {
    for (source, disposition, guarded, signature_writes) in [
        (
            r#"({a}) => { eval("0"); return a }"#,
            "refused_param_list_write_effect",
            true,
            1,
        ),
        (
            r#"({a}, p = eval("a = 5")) => { return a; }"#,
            "refused_param_list_write_effect",
            true,
            2,
        ),
        (r#"({a}) => { eval?.("0"); return a }"#, "emitted", false, 1),
    ] {
        let expected = syntax_read_count(source, "a");
        assert_eq!(expected, 1);
        let receipt = evidence(source, Language::JavaScript);
        assert_eq!(receipt.counters()["candidates"], expected, "{source}");
        assert_eq!(
            receipt.counters()["unresolved_param_list_write_sites"],
            signature_writes,
            "{source}"
        );
        // All controls contain the destructuring Get. Only the
        // signature default adds a second site.
        if signature_writes == 2 {
            let start = source.find("eval(").unwrap();
            let end = source[start..].find(')').unwrap() + start + 1;
            assert!(receipt.files["test"]
                .signature_write_sites
                .iter()
                .any(|(_, site)| *site == [start, end]));
        }
        let attempt = &receipt.files["test"].attempts[0];
        assert_eq!(attempt.disposition.as_str(), disposition, "{source}");
        assert_eq!(attempt.justifies_def, !guarded, "{source}");
        let graph = refs(source, Language::JavaScript);
        assert_eq!(
            graph.defs.values().flatten().any(|def| def.start_byte == 2),
            !guarded,
            "{source}",
        );
        assert_eq!(
            graph.edges.iter().any(|edge| edge.from.start_byte == 2),
            !guarded,
            "{source}",
        );
    }
}

#[test]
fn computed_parameter_key_replaces_the_entry_value_before_the_body() {
    for (key, read, disposition) in [
        ("(o = 5)", "o", "refused_s1_replaced"),
        ("(flag ? (o = 5) : 0)", "o", "emitted"),
        ("0", "o", "emitted"),
        ("(o = 5)", "o.k", "refused_param_list_write_effect"),
    ] {
        // A separately placed signature line preserves the legacy replacement
        // edge at cfg_incomplete. The first-line replacement is pinned at Exact
        // in the reaching-definition regression.
        let source = format!("function C({{o}},\n  {{[{key}]: y}}) {{\n  return {read};\n}}");
        let receipt = evidence(&source, Language::JavaScript);
        let start = source.find("return ").unwrap() + "return ".len();
        let attempt = receipt.files["test"]
            .attempts
            .iter()
            .find(|attempt| attempt.key.operand == [start, start + read.len()])
            .unwrap();
        assert_eq!(attempt.disposition.as_str(), disposition, "{key}/{read}");
        let graph = refs(&source, Language::JavaScript);
        assert_eq!(
            graph.edges.iter().any(|edge| edge.from.start_byte == 12),
            disposition == "emitted",
            "{key}/{read}",
        );
        if disposition == "refused_s1_replaced" {
            let replacement = graph
                .edges
                .iter()
                .find(|edge| {
                    edge.from.start_byte == 21
                        && edge.from.end_byte == 22
                        && edge.to.start_byte == start
                        && edge.to.end_byte == start + 1
                })
                .expect("computed-key replacement edge");
            assert_eq!(
                graph.labels[&(replacement.from.clone(), replacement.to.clone())],
                FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
            );
        }
    }
}

/// The ledger's candidate count must equal an independent syntax count of the same
/// reads, whichever path (admitted, guard-refused, read-modify-write) each one takes.
#[test]
fn ledger_candidates_equal_the_syntax_read_count() {
    for source in [
        "function C({a}) { return a + a }",
        "function C({a}) { return a.k + a.j }",
        "function C({a}) { a.k = 1; return a.j }",
        "function C({a}) { a.k++; return a.j + a.k }",
        "function C({a}) { a[0] += 1; return a }",
        "function C({a}) { a.k ??= 1; return 0 }",
        "function C({a}) { eval(\"0\"); return a }",
        "({a}) => { eval(\"0\"); return a + a }",
        "({a}) => { eval?.(\"0\"); return a }",
    ] {
        let expected = syntax_read_count(source, "a");
        let receipt = evidence(source, Language::JavaScript);
        assert_eq!(receipt.counters()["candidates"], expected, "{source}");
    }
}
