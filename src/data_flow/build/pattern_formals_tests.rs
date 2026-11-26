use super::{FileRefs, Producer};
use crate::ast::ParsedFile;
use crate::languages::Language;

pub(super) fn refs(source: &str, language: Language) -> FileRefs {
    let file = ParsedFile::parse("test", source, language).unwrap();
    assert!(!file.tree.root_node().has_error(), "{source}");
    Producer::file_refs(&file.path, &file)
}

pub(super) fn evidence(
    source: &str,
    language: Language,
) -> crate::data_flow::s1_evidence::Evidence {
    let file = ParsedFile::parse("test", source, language).unwrap();
    crate::data_flow::DataFlowGraph::collect_s1_evidence(
        &std::collections::BTreeMap::from([(file.path.clone(), file)]),
        "pattern-formals-fixture",
    )
}

#[test]
fn representative_must_share_the_value_version() {
    for (statement, endpoint) in [
        ("x = 0; return <D x={x} />;", "x={"),
        ("return <D x={(x = 0, x)} />;", "x)}"),
        ("return <D x={(x, x = 0)} />;", "x={"),
        ("x = 0; if (x) yes();", "x) yes"),
        ("if (x) yes(); x = 0; if (x) no();", "x) yes"),
        ("if (x) yes(); x = 0;", "  if"),
        ("x = 0; use(x, x);", "x, x"),
        ("use(x, x = 0, x);", "x, x"),
        ("use(x, x); x = 0;", "x, x"),
    ] {
        let source = format!("function C({{x}}) {{\n  {statement}\n}}\n");
        let graph = refs(&source, Language::Tsx);
        let start = source.find(endpoint).unwrap();
        let end = start + usize::from(!endpoint.starts_with(' '));
        let actual: Vec<_> = graph
            .edges
            .iter()
            .filter(|edge| edge.from.start_byte == 12 && edge.to.line == 2)
            .map(|edge| (edge.to.start_byte, edge.to.end_byte))
            .collect();
        assert_eq!(actual, [(start, end)], "{statement}");
    }
    let source = "function C({x}) { return <D x={(x = 0, x)} />; }";
    let graph = refs(source, Language::Tsx);
    let read = source.find("x)}").unwrap();
    assert_eq!(
        graph
            .edges
            .iter()
            .filter(|edge| edge.from.start_byte == 12)
            .map(|edge| (edge.to.start_byte, edge.to.end_byte))
            .collect::<Vec<_>>(),
        [(read, read + 1)]
    );
    for assignment in [
        "[x, t.v = x] = [1]",
        "[[x], t.v = x] = [[1]]",
        "({x, slot: t.v = x} = {x: 1})",
        "({a: {x}, slot: t.v = x} = {a: {x: 1}})",
        "({x, [x]: t.v} = {x: 1})",
    ] {
        let source = format!("function C({{x}}, t) {{ return <D x={{({assignment})}}/>; }}");
        let graph = refs(&source, Language::Tsx);
        assert!(
            graph
                .defs
                .values()
                .flatten()
                .any(|def| def.start_byte == 12),
            "{assignment}"
        );
        assert!(
            graph.edges.iter().all(|edge| edge.from.start_byte != 12),
            "{assignment}"
        );
    }
}

#[test]
fn pattern_projection_uses_bound_normalized_write_paths() {
    for (body, projections) in [
        ("o = {}; return o.k;", vec![]),
        ("[o] = values; return o.k;", vec![]),
        ("[o[key]] = values; return o.k;", vec![]),
        ("[o.other[key]] = values; return o.k;", vec![]),
        (r"\u006f.k = 0; return o.k;", vec![]),
        (r"o.\u006b = 0; return o.k;", vec![]),
        (r"o.k = 0; return o.\u006b;", vec![]),
        ("o /* c */ .k /* c */ .part = 0; return o.k;", vec![]),
        ("o /* c */ .k = {}; return o.k.part;", vec![]),
        ("o /* c */ .other = 0; return o /* c */ .k;", vec![]),
        ("{ let o = {}; o.k = 0; } return o.k;", vec![]),
        ("{ let o; o = {}; } return o.k;", vec!["o.k"]),
        (r"return \u006f.\u006b;", vec!["o.k"]),
    ] {
        let source = format!("function C({{o}}) {{\n  {body}\n}}\n");
        let graph = refs(&source, Language::JavaScript);
        assert!(
            graph
                .defs
                .values()
                .flatten()
                .any(|def| def.start_byte == 12),
            "{body}"
        );
        let actual: Vec<_> = graph
            .edges
            .iter()
            .filter(|edge| edge.from.start_byte == 12 && edge.to.path.has_fields())
            .map(|edge| edge.to.path.to_string())
            .collect();
        assert_eq!(actual, projections, "{body}");
    }
}

#[test]
fn projection_writes_respect_the_parameter_body_seam() {
    for (source, expected) in [
        (
            "function C({o}, f = () => 0) {\n  var o;\n  return o.k;\n}",
            1,
        ),
        (
            "function C({o}, f = () => 0) {\n  var o;\n  o = {};\n  return o.k;\n}",
            0,
        ),
        ("function C({o},\n  f = use(o.k)) {\n  return f;\n}", 0),
        (
            "function C({o},\n  f = (o = {}, use(o.k))) {\n  return f;\n}",
            0,
        ),
    ] {
        let graph = refs(source, Language::JavaScript);
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| edge.from.start_byte == 12 && edge.to.path.has_fields())
                .count(),
            expected,
            "{source}"
        );
    }
    for (init, projection) in [
        ("() => { o.k = 0; }", true),
        ("(() => { o.k = 0; })()", false),
        ("([o, t = o.k] = [{k: 1}])", false),
        ("({a: [o], t = o.k} = {a: [{k: 1}]})", false),
    ] {
        let source =
            format!("function C({{keep}}, {{o}}, t, f = {init}) {{\n  return use(o.k, keep);\n}}");
        let graph = refs(&source, Language::JavaScript);
        let binding = source.find("{o}").unwrap() + 1;
        let projected = graph
            .edges
            .iter()
            .any(|edge| edge.from.start_byte == binding && edge.to.path.to_string() == "o.k");
        assert_eq!(projected, projection, "{init}: {:?}", graph.edges);
        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.from.var_name() == "keep"));
    }
}

#[test]
fn default_execution_regions_preserve_parameter_order() {
    for init in [
        "(function*(){ yield x; })()",
        "[...(function*(){ yield x; })()]",
        "(async () => { Number(await 0); return x; }).call(null)",
        "new (class extends (class { field = x; }) {})()",
        "({ get a() { return x; }, get b() { return this; } })[true ? 'b' : 'a']",
        "(() => x).bind(null)()",
        "(function*(){ yield 1; })().next(x)",
        "(function*(){ yield x; })().next()",
        "(async function*(){ yield x; })()",
        "(async function*(){ yield x; })().next()",
        "(async () => { await ready; return x; })()",
        "(async () => { return (await ready, x); })()",
        "(async () => { return use(await ready, x); })()",
        "(async () => { return (await ready) + x; })()",
        "(async () => { return (await ready) ? x : 0; })()",
        "(async () => { use(x); await ready; })()",
        "(async () => { if (flag) await ready; return x; })()",
        "(async () => { await x; })()",
        "class { field = x; }",
        "class { [x] = 0; }",
        "class { static field = x; }",
        "class { static { use(x); } }",
        "new (class { field = x; })()",
        "new (class { constructor() { use(x); } })()",
        "(() => x)()",
        "(() => x).call(null)",
        "(() => x).apply(null, [])",
        "(() => x).call",
        "({ get value() { return x; } }).value",
        "({ get value() { return x; } }).other",
        "({ get value() { return x; } })",
        "({ get value() { return x; } })['value']",
        "({ get value() { return x; } })['other']",
        "(class { static get value() { return x; } }).value",
        "(class { get value() { return x; } }).value",
        "new (class { get value() { return x; } })().value",
    ] {
        let source =
            format!("function C(it = {init}, {{x, keep}}) {{\n  return use(it, keep);\n}}");
        for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
            let graph = refs(&source, language);
            let binding = source.find("{x, keep}").unwrap() + 1;
            let receipt = evidence(&source, language);
            assert_eq!(
                receipt.counters()["refused_param_list_shape"],
                1,
                "{language:?}: {init}"
            );
            let file = &receipt.files["test"];
            assert!(
                file.attempts.iter().any(|attempt| {
                    attempt.key.binding == [binding, binding + 1]
                        && !attempt.justifies_def
                        && attempt.disposition.as_str() == "refused_param_list_shape"
                }),
                "{language:?}: {init}"
            );
            assert!(
                graph
                    .edges
                    .iter()
                    .all(|edge| edge.from.start_byte != binding),
                "unexpected entry edge for {language:?}: {init}"
            );
            assert!(
                graph
                    .defs
                    .values()
                    .flatten()
                    .all(|def| def.start_byte != binding),
                "unexpected entry Def for {language:?}: {init}"
            );
            assert!(graph
                .edges
                .iter()
                .any(|edge| edge.from.var_name() == "keep"));
        }
    }
}

#[test]
fn only_runtime_references_in_the_formals_environment_supply_defs() {
    for (source, language, name, present) in [
        (
            "function C({arguments}) { return function () { return arguments; }; }",
            Language::JavaScript,
            "arguments",
            false,
        ),
        (
            "function C({arguments}) { return () => arguments; }",
            Language::JavaScript,
            "arguments",
            true,
        ),
        (
            "function C({arguments}) { return { [arguments]() { return 0; } }; }",
            Language::JavaScript,
            "arguments",
            true,
        ),
        (
            "function C({arguments}) { return function () { return () => arguments; }; }",
            Language::JavaScript,
            "arguments",
            false,
        ),
        (
            "function C({arguments}) { return function () { return { [arguments]: 0 }; }; }",
            Language::JavaScript,
            "arguments",
            false,
        ),
        (
            "function C({k}: {k: 'field'}) { interface I { [k]: number } }",
            Language::TypeScript,
            "k",
            false,
        ),
        (
            "function C({k}: {k: 'field'}) { return { [k]: 0 }; }",
            Language::TypeScript,
            "k",
            true,
        ),
        (
            "class C { m({k}: {k: number}, @dec(k) other: number) {} }",
            Language::TypeScript,
            "k",
            false,
        ),
        (
            "class C { m({k}: {k: number}, @dec(k) other = use(k)) { return other; } }",
            Language::TypeScript,
            "k",
            false,
        ),
        (
            "function C({k}) { return class { m(k: number, @dec(k) value: number) {} }; }",
            Language::TypeScript,
            "k",
            false,
        ),
        (
            "function C({x}) { return class { a(@d(class { b(x: number, @d(x) y: number) {} }) z: number) {} }; }",
            Language::TypeScript,
            "x",
            false,
        ),
        (
            "function C({k, live}) { abstract class K { declare [k]: number; abstract [k]: number; [live] = 0; } }",
            Language::TypeScript,
            "k",
            false,
        ),
        (
            "function C({k, live}) { abstract class K { declare [k]: number; abstract [k]: number; [live] = 0; } }",
            Language::TypeScript,
            "live",
            true,
        ),
        (
            r"function C({live}, [\u007a]) { return live; }",
            Language::JavaScript,
            "live",
            true,
        ),
    ] {
        let graph = refs(source, language);
        let binding = source.match_indices(name).find(|(at, _)| {
            source[..*at].ends_with('{') || source[..*at].ends_with(", ")
        }).unwrap().0;
        if source.contains("@dec(") || source.contains("@d(class") {
            let receipt = evidence(source, language);
            assert_eq!(receipt.counters()["refused_param_list_decorator"], 1, "{source}");
            if source.contains("other = use(k)") {
                assert_eq!(receipt.counters()["refused_param_list_shape"], 1, "{source}");
            }
        }
        if name == "k" && source.contains("declare [k]") {
            let receipt = evidence(source, language);
            assert!(receipt.files["test"].attempts.iter().all(|attempt| attempt.key.binding[0] != binding));
        }
        assert_eq!(
            graph
                .defs
                .values()
                .flatten()
                .any(|def| def.start_byte == binding),
            present,
            "{source}"
        );
    }
}

#[test]
fn body_seam_preserves_legacy_signature_and_body_anchors() {
    for (statement, end_bytes) in [("if (x) return 1;", (44, 44)), ("return use(x);", (57, 58))] {
        let source =
            format!("function C({{x = 0}}, f = () => x) {{\n  var x;\n  {statement}\n}}\n");
        let graph = refs(&source, Language::JavaScript);
        let file = ParsedFile::parse("test", &source, Language::JavaScript).unwrap();
        let base = Producer::file_refs_frozen_with_cap(&file.path, &file, None);
        let body_edges = |graph: &FileRefs| {
            graph
                .edges
                .iter()
                .filter(|edge| edge.from.var_name() == "x" && edge.from.line == 2)
                .map(|edge| (edge.to.line, edge.to.start_byte, edge.to.end_byte))
                .collect::<Vec<_>>()
        };
        let expected = [(1, 0, 0), (3, end_bytes.0, end_bytes.1)];
        assert_eq!(body_edges(&base), expected, "{statement}");
        assert_eq!(body_edges(&graph), expected, "{statement}");
    }
}

#[test]
fn body_seam_preserves_alias_twin_endpoint_and_fence() {
    let source = "function C({x = 0}, f = () => x.k) {\n  var x;\n  const alias = x;\n  alias.k = 0;\n  if (x.k) return 1;\n}\n";
    let graph = refs(source, Language::JavaScript);
    let file = ParsedFile::parse("test", source, Language::JavaScript).unwrap();
    let base = Producer::file_refs_frozen_with_cap(&file.path, &file, None);
    let twin_edges = |graph: &FileRefs| {
        graph
            .edges
            .iter()
            .filter(|edge| edge.from.line == 4 && edge.from.path.to_string() == "x.k")
            .map(|edge| (edge.to.line, edge.to.start_byte, edge.to.end_byte))
            .collect::<Vec<_>>()
    };
    let legacy = twin_edges(&base);
    let expected: Vec<_> = legacy
        .into_iter()
        .filter(|(line, _, _)| *line == 5)
        .collect();
    assert_eq!(
        expected,
        [(5, file.line_start_byte(5), file.line_start_byte(5))]
    );
    assert_eq!(twin_edges(&graph), expected);
}

#[test]
fn body_seam_preserves_scope_without_pattern_def() {
    let source = "function C({x=0}, f=()=>\nsee(x)) { if (x) return 1;\nvar x = 0;\n}";
    let condition = source.find("if (x)").unwrap() + 4;
    let binding = source.find("var x").unwrap() + 4;
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let graph = refs(source, language);
        assert!(graph
            .defs
            .values()
            .flatten()
            .all(|def| def.start_byte != 12));
        let endpoints: Vec<_> = graph
            .edges
            .iter()
            .filter(|edge| edge.from.start_byte == binding && edge.to.line == 2)
            .map(|edge| (edge.to.start_byte, edge.to.end_byte))
            .collect();
        assert_eq!(endpoints, [(condition, condition + 1)], "{language:?}");
    }
}

#[test]
fn opaque_signature_call_can_invoke_a_stored_writer_through_an_alias() {
    for (default, entry_reaches_body) in [("0", true), ("invoke(alias)", false)] {
        let source = format!("function C({{a, keep}}, writer = () => {{ a = 1; }}, alias = writer, ignored = {default}) {{\n  return [a, keep];\n}}\n");
        let graph = refs(&source, Language::JavaScript);
        let read = source.find("return [a").unwrap() + "return [".len();
        let entry = graph
            .edges
            .iter()
            .any(|edge| edge.from.start_byte == 12 && edge.to.line == 2);
        assert_eq!(entry, entry_reaches_body, "{default}");
        let evidence = evidence(&source, Language::JavaScript);
        let attempt = evidence.files["test"]
            .attempts
            .iter()
            .find(|attempt| attempt.key.operand == [read, read + 1])
            .unwrap();
        let disposition = if entry_reaches_body {
            "emitted"
        } else {
            "refused_param_list_write_effect"
        };
        assert_eq!(attempt.disposition.as_str(), disposition, "{default}");
        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.from.var_name() == "keep" && edge.to.line == 2));
    }
}

#[test]
fn jsx_attribute_representative_contains_its_own_read() {
    for (jsx, endpoint) in [
        (r#"<D x="fixed" y={x}/>"#, "x}"),
        ("<D x={other} y={x}/>", "x}"),
        ("<D x={other} {...x}/>", "x}"),
        (r#"<D x="fixed" ns:y={x}/>"#, "x}"),
        (r#"<D ns:x="fixed" y={x}/>"#, "x}"),
        ("<D x={x}/>", "x={"),
    ] {
        let source = format!("function C({{x}}) {{\n  return {jsx};\n}}\n");
        let body = source.find("return ").unwrap();
        let start = body + source[body..].find(endpoint).unwrap();
        let actual: Vec<_> = refs(&source, Language::Tsx)
            .edges
            .iter()
            .filter(|edge| edge.from.start_byte == 12)
            .map(|edge| (edge.to.start_byte, edge.to.end_byte))
            .collect();
        assert_eq!(actual, [(start, start + 1)], "{jsx}");
    }
}

#[test]
fn optional_eval_in_a_formal_is_indirect() {
    for (language, default, direct) in [
        (Language::JavaScript, r#"eval?.("typeof a")"#, false),
        (Language::JavaScript, r#"(eval)?.("typeof a")"#, false),
        (
            Language::TypeScript,
            r#"(eval as any)?.("typeof a")"#,
            false,
        ),
        (Language::JavaScript, r#"(0, eval)("typeof a")"#, false),
        (Language::JavaScript, r#"eval("typeof a")"#, true),
        (Language::JavaScript, r#"(eval)("typeof a")"#, true),
        (Language::TypeScript, r#"(eval as any)("typeof a")"#, true),
    ] {
        let source = format!("function C({{a}}, t = {default}) {{ return [a,t] }}");
        let receipt = evidence(&source, language);
        let attempt = &receipt.files["test"].attempts[0];
        assert_eq!(
            attempt.disposition.as_str(),
            if direct {
                "refused_param_list_write_effect"
            } else {
                "emitted"
            },
            "{default}",
        );
    }
}
