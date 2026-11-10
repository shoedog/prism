//! Java rows `J-01..J-06` of the alias-twin occurrence table: parity with the slice-0 oracle's pinned
//! cases (the `EXPECT base:disposition:rule` tokens of `tests/fixtures/alias_twin_java/*.java` are
//! the oracle's own answers, re-checked against `classify.py` through the survey diagnostic), the
//! scoping and negative rows, and the producer effect (no target def or edge at a binding write;
//! storage writes and unsupported forms keep theirs).
use super::{Disposition, TwinPolicy};
use crate::ast::ParsedFile;
use crate::data_flow::build::{FileRefs, Producer};
use crate::data_flow::VarAccessKind;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

fn parse(source: &str) -> ParsedFile {
    ParsedFile::parse("A.java", source, Language::Java).unwrap()
}

fn disposition_name(d: Disposition) -> &'static str {
    match d {
        Disposition::BindingWrite => "binding-write",
        Disposition::StorageWrite => "storage-write",
        Disposition::Unsupported => "unsupported",
    }
}

/// `line -> {"base:disposition:rule"}` for every lvalue span of every owner of the file.
fn classified(parsed: &ParsedFile) -> BTreeMap<usize, BTreeSet<String>> {
    let mut out: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    for func in parsed.all_functions() {
        let (start, end) = parsed.node_line_range(&func);
        let lines: BTreeSet<usize> = (start..=end).collect();
        for span in parsed.assignment_lvalue_spans_on_lines(&func, &lines) {
            let c = super::java::classify(parsed, &span);
            out.entry(span.line).or_default().insert(format!(
                "{}:{}:{}",
                span.path,
                disposition_name(c.disposition),
                c.rule
            ));
        }
    }
    out
}

fn expected_tokens(source: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (i, line) in source.lines().enumerate() {
        if let Some(pos) = line.find("EXPECT ") {
            for token in line[pos + "EXPECT ".len()..].split_whitespace() {
                out.push((i + 1, token.to_string()));
            }
        }
    }
    out
}

fn check_fixture(name: &str, source: &str) {
    let got = classified(&parse(source));
    let expected = expected_tokens(source);
    assert!(expected.len() >= 10, "{name}: fixture pins too little");
    let missing: Vec<_> = expected
        .iter()
        .filter(|(line, token)| !got.get(line).is_some_and(|set| set.contains(token)))
        .map(|(line, token)| format!("{name}:{line} {token} (got {:?})", got.get(line)))
        .collect();
    assert!(missing.is_empty(), "{missing:#?}");
}

#[test]
fn oracle_pinned_cases_j() {
    check_fixture(
        "J.java",
        include_str!("../../../../tests/fixtures/alias_twin_java/J.java"),
    );
}

#[test]
fn oracle_pinned_cases_j2() {
    check_fixture(
        "J2.java",
        include_str!("../../../../tests/fixtures/alias_twin_java/J2.java"),
    );
}

fn dispositions(source: &str, line: usize) -> BTreeSet<String> {
    classified(&parse(source)).remove(&line).unwrap_or_default()
}

// ---- the oracle's unit cases for the Java rows (`classifier/unit_cases.py`) ----

#[test]
fn unit_visible_local_is_a_binding_write() {
    let src = "class A { void t(S o) {\n S f = o;\n f = o;\n} }\n";
    assert!(dispositions(src, 3).contains("f:binding-write:J-01"));
}

#[test]
fn unit_same_named_local_in_another_block_is_unsupported() {
    let src = "class A { void t(S o) {\n { S f = o; }\n f = o;\n} }\n";
    assert!(dispositions(src, 3).contains("f:unsupported:J-01"));
}

#[test]
fn unit_declaration_after_the_use_is_unsupported() {
    let src = "class A { S f; void t(S o) {\n f = o;\n S f = o;\n} }\n";
    assert!(dispositions(src, 2).contains("f:unsupported:J-01"));
    assert!(dispositions(src, 3).contains("f:binding-write:J-02"));
}

// ---- scoping and negatives (stricter than, never looser than, the oracle) ----

#[test]
fn implicit_field_and_static_field_assignment_are_unsupported() {
    let src = "class A { S fld; static S sfld; void t(S o) {\n fld = o;\n sfld = o;\n} }\n";
    assert!(dispositions(src, 2).contains("fld:unsupported:J-01"));
    assert!(dispositions(src, 3).contains("sfld:unsupported:J-01"));
}

#[test]
fn a_local_after_the_loop_does_not_leak_out_of_a_for_each_or_a_for_init() {
    // The oracle's scope for these declarations is the enclosing block (a field of the same name
    // assigned after the loop would be read as a local binding). The scoped walk does not.
    let src = "class A { S e; S i; void t(S o, S[] arr) {\n for (S e : arr) { e = o; }\n e = o;\n for (S i = o; i != null; i = o) { }\n i = o;\n} }\n";
    assert!(dispositions(src, 2).contains("e:binding-write:J-01"));
    assert!(dispositions(src, 3).contains("e:unsupported:J-01"));
    assert!(dispositions(src, 4).contains("i:binding-write:J-01"));
    assert!(dispositions(src, 5).contains("i:unsupported:J-01"));
}

#[test]
fn a_catch_parameter_and_try_resource_do_not_leak_out_of_their_statement() {
    let src = "class A { S ex; S r; void t(S o) {\n try (S r = o) { r = o; } catch (E ex) { ex = o; }\n ex = o;\n r = o;\n} }\n";
    let line2 = dispositions(src, 2);
    assert!(line2.contains("r:binding-write:J-01"), "{line2:?}");
    assert!(line2.contains("ex:binding-write:J-01"), "{line2:?}");
    assert!(dispositions(src, 3).contains("ex:unsupported:J-01"));
    assert!(dispositions(src, 4).contains("r:unsupported:J-01"));
}

#[test]
fn an_anonymous_class_field_shadows_the_enclosing_local() {
    // The oracle reports the enclosing local's declaration as visible inside the anonymous class
    // body (binding-write). The assignment writes the anonymous class's own field.
    let src = "class A { void t(S o) {\n S s = o;\n Object x = new Object() { S s; void m() {\n s = o;\n } };\n} }\n";
    let line4 = dispositions(src, 4);
    assert!(line4.contains("s:unsupported:J-01"), "{line4:?}");
    assert!(
        !line4.iter().any(|t| t.starts_with("s:binding-write")),
        "{line4:?}"
    );
}

#[test]
fn inferred_lambda_parameters_carry_no_declaration_evidence() {
    let src = "class A { void t(S o) {\n Consumer<S> c = p -> { p = o; };\n Consumer<S> d = (S q) -> { q = o; };\n} }\n";
    assert!(dispositions(src, 2).contains("p:unsupported:J-01"));
    assert!(dispositions(src, 3).contains("q:binding-write:J-01"));
}

#[test]
fn a_declaration_without_initialiser_and_a_field_declarator_rows() {
    let src = "class A { S fld = null;\n void t(S o) {\n S late;\n late = o;\n} }\n";
    // The field declarator is outside any owner: only the local rows are reachable here.
    assert!(dispositions(src, 3).contains("late:binding-write:J-03"));
    assert!(dispositions(src, 4).contains("late:binding-write:J-01"));
}

#[test]
fn storage_writes_through_a_visible_local_are_storage_and_unknown_bases_are_not() {
    let src = "class A { S fld; void t(S o, int[] arr) {\n S u = o;\n u.f = 1;\n arr[0] = 2;\n fld.f = 3;\n fld2[1] = 4;\n} }\n";
    assert!(dispositions(src, 3).contains("u.f:storage-write:J-05"));
    assert!(dispositions(src, 4).contains("arr[]:storage-write:J-06"));
    assert!(dispositions(src, 5).contains("fld.f:unsupported:J-05"));
    assert!(dispositions(src, 6).contains("fld2[]:unsupported:J-06"));
}

#[test]
fn read_positions_are_never_binding_writes() {
    let src =
        "class A { void t(S o) {\n S k = o;\n int x = foo(k);\n int y = k + 1;\n S z = (k);\n} }\n";
    for line in [3, 4, 5] {
        assert!(
            !dispositions(src, line)
                .iter()
                .any(|t| t.contains(":binding-write:") && t.starts_with("k:")),
            "line {line}: {:?}",
            dispositions(src, line)
        );
    }
    assert!(dispositions(src, 3).contains("k:unsupported:X-READ"));
}

// ---- the policy and the producer effect ----

fn policy_bytes(parsed: &ParsedFile) -> BTreeSet<usize> {
    let mut out = BTreeSet::new();
    for func in parsed.all_functions() {
        let (start, end) = parsed.node_line_range(&func);
        let lines: BTreeSet<usize> = (start..=end).collect();
        let (alias_map, _) = Producer::build_alias_map(parsed, &func, &lines);
        let spans = parsed.assignment_lvalue_spans_on_lines(&func, &lines);
        let policy = TwinPolicy::for_owner(parsed, &func, &spans, &alias_map);
        out.extend(
            spans
                .iter()
                .map(|s| s.start_byte)
                .filter(|b| policy.suppresses_span(*b)),
        );
    }
    out
}

#[test]
fn the_policy_suppresses_exactly_the_binding_write_occurrences() {
    let src =
        "class A { S fld; void t(S o, S q) {\n S u = o;\n u = q;\n u.f = 1;\n fld = o;\n} }\n";
    let parsed = parse(src);
    let lines: Vec<usize> = policy_bytes(&parsed)
        .into_iter()
        .map(|b| parsed.line_for_byte(b))
        .collect();
    assert_eq!(lines, vec![2, 3], "declaration and reassignment of u only");
}

#[test]
fn other_languages_get_an_empty_policy() {
    let c = "void t(struct S *o, struct S *q) {\n struct S *u = o;\n u = q;\n u->f = 1;\n}\n";
    let py = "def t(o, q):\n    u = o\n    u = q\n    u.f = 1\n";
    for (file, language, source) in [("a.c", Language::C, c), ("a.py", Language::Python, py)] {
        let parsed = ParsedFile::parse(file, source, language).unwrap();
        assert!(policy_bytes(&parsed).is_empty(), "{file}");
    }
}

fn refs(source: &str) -> FileRefs {
    let parsed = parse(source);
    Producer::file_refs(&"A.java".to_string(), &parsed)
}

/// Alias twins: Defs of another path sitting at the very bytes of a Def of `alias`.
fn twins(r: &FileRefs, alias: &str) -> Vec<(String, usize)> {
    let all: Vec<_> = r
        .defs
        .values()
        .flatten()
        .filter(|d| d.kind == VarAccessKind::Def)
        .collect();
    let starts: BTreeSet<usize> = all
        .iter()
        .filter(|d| d.path.base == alias)
        .map(|d| d.start_byte)
        .collect();
    all.iter()
        .filter(|d| d.path.base != alias && starts.contains(&d.start_byte))
        .map(|d| (d.path.to_string(), d.line))
        .collect()
}

#[test]
fn a_binding_write_mints_no_target_def_or_edge_but_keeps_the_alias_defs() {
    let src = "class A { void t(S o, S q) {\n S u = o;\n u = q;\n use(o);\n use(u);\n} }\n";
    let r = refs(src);
    assert!(twins(&r, "u").is_empty(), "{:?}", twins(&r, "u"));
    // The alias binding's own defs and their edges to the read of u remain.
    assert!(r
        .edges
        .iter()
        .any(|e| e.from.path.base == "u" && e.from.line == 3 && e.to.line == 5));
    // Emission-site pin: no edge leaves a path other than the alias from the bytes of the
    // binding-write occurrence `u` of `u = q;` (the flow-insensitive alias map resolves `u` to `q`
    // here; the RHS read's own `Def(q)`, at other bytes, stays).
    let write_byte = src.find("u = q").unwrap();
    assert!(
        !r.edges
            .iter()
            .any(|e| e.from.path.base != "u" && e.from.start_byte == write_byte),
        "{:?}",
        r.edges
    );
}

#[test]
fn a_storage_write_through_the_alias_keeps_its_target_twin() {
    let src = "class A { void t(S o) {\n S u = o;\n u.f = 1;\n use(o.f);\n} }\n";
    let r = refs(src);
    assert_eq!(twins(&r, "u"), vec![("o.f".to_string(), 3)], "{:?}", r.defs);
    assert!(
        r.edges.iter().any(|e| e.from.path.base == "o"
            && e.from.path.has_fields()
            && e.from.line == 3
            && e.to.line == 4),
        "write-through edge o.f@3 -> o.f@4 must survive: {:?}",
        r.edges
    );
}

#[test]
fn an_unsupported_occurrence_keeps_its_twin() {
    // `fld` has no local declaration: an implicit field, kept exactly as before.
    let src = "class A { S fld; void t(S o) {\n fld = o;\n use(o);\n} }\n";
    let r = refs(src);
    assert_eq!(twins(&r, "fld"), vec![("o".to_string(), 2)], "{:?}", r.defs);
    assert!(
        r.edges
            .iter()
            .any(|e| e.from.path.base == "o" && e.from.line == 2 && e.to.line == 3),
        "{:?}",
        r.edges
    );
}

// ---- refusal R1: member-name tokens spelled like the alias target ----

fn suppressed_lines(src: &str) -> Vec<usize> {
    let parsed = parse(src);
    policy_bytes(&parsed)
        .into_iter()
        .map(|b| parsed.line_for_byte(b))
        .collect()
}

#[test]
fn a_method_name_token_of_the_target_spelling_refuses_the_removal() {
    // `size` is the alias target; `.size()` is a token the occurrence extraction cannot tell from
    // the variable, so the twin stays (today's behaviour) for this owner.
    let with_token = "class A { Integer size; int f(R r) {\n Integer result = size;\n result = 3;\n return g(r).size();\n} }\n";
    assert!(
        suppressed_lines(with_token).is_empty(),
        "{:?}",
        suppressed_lines(with_token)
    );
    let without = "class A { Integer size; int f(R r) {\n Integer result = size;\n result = 3;\n return g(r).count();\n} }\n";
    assert_eq!(suppressed_lines(without), vec![2, 3]);
}

#[test]
fn another_objects_member_name_refuses_but_this_member_does_not() {
    let other =
        "class A { S key; void f(N node) {\n S k = key;\n k = mk();\n use(node.key);\n} }\n";
    assert!(suppressed_lines(other).is_empty());
    let this_member =
        "class A { S key; void f(N node) {\n S k = key;\n k = mk();\n use(this.key);\n} }\n";
    assert_eq!(suppressed_lines(this_member), vec![2, 3]);
    let method_ref = "class A { S key; void f() {\n S k = key;\n k = mk();\n run(A::key);\n} }\n";
    assert!(suppressed_lines(method_ref).is_empty());
}

#[test]
fn the_refusal_is_per_target_spelling() {
    // `size` is refused (a `.size()` token); `other` has no token and is still removed.
    let src = "class A { S size; S other; void f(R r) {\n S a = size;\n a = mk();\n S b = other;\n b = mk();\n g(r.size());\n} }\n";
    assert_eq!(suppressed_lines(src), vec![4, 5]);
}
