use super::{FileRefs, Producer};
use crate::ast::ParsedFile;
use crate::languages::Language;

fn refs(source: &str, language: Language) -> FileRefs {
    let file = ParsedFile::parse("test", source, language).unwrap();
    assert!(!file.tree.root_node().has_error(), "{source}");
    Producer::file_refs(&file.path, &file)
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
}

#[test]
fn pattern_projection_uses_bound_normalized_write_paths() {
    for (body, projections) in [
        ("o = {}; return o.k;", vec![]),
        ("[o] = values; return o.k;", vec![]),
        (r"\u006f.k = 0; return o.k;", vec![]),
        (r"o.\u006b = 0; return o.k;", vec![]),
        (r"o.k = 0; return o.\u006b;", vec![]),
        ("o /* c */ .k /* c */ .part = 0; return o.k;", vec![]),
        ("o /* c */ .k = {}; return o.k.part;", vec![]),
        ("o /* c */ .other = 0; return o /* c */ .k;", vec!["o.k"]),
        ("{ let o = {}; o.k = 0; } return o.k;", vec!["o.k"]),
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
        ("function C({o},\n  f = use(o.k)) {\n  return f;\n}", 1),
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
}

#[test]
fn default_execution_regions_preserve_parameter_order() {
    for (init, capture) in [
        ("(function*(){ yield x; })()", true),
        ("(function*(){ yield 1; })().next(x)", false),
        ("(function*(){ yield x; })().next()", false),
        ("(async function*(){ yield x; })()", true),
        ("(async function*(){ yield x; })().next()", false),
        ("(async () => { await ready; return x; })()", true),
        ("(async () => { return (await ready, x); })()", true),
        ("(async () => { return use(await ready, x); })()", true),
        ("(async () => { return (await ready) + x; })()", true),
        ("(async () => { return (await ready) ? x : 0; })()", true),
        ("(async () => { use(x); await ready; })()", false),
        (
            "(async () => { if (flag) await ready; return x; })()",
            false,
        ),
        ("(async () => { await x; })()", false),
        ("class { field = x; }", true),
        ("class { [x] = 0; }", false),
        ("class { static field = x; }", false),
        ("class { static { use(x); } }", false),
        ("new (class { field = x; })()", false),
        ("new (class { constructor() { use(x); } })()", false),
        ("(() => x)()", false),
        ("(() => x).call(null)", false),
        ("(() => x).apply(null, [])", false),
        ("(() => x).call", true),
        ("({ get value() { return x; } }).value", false),
        ("({ get value() { return x; } }).other", true),
        ("({ get value() { return x; } })", true),
        ("({ get value() { return x; } })['value']", false),
        ("({ get value() { return x; } })['other']", true),
        ("(class { static get value() { return x; } }).value", false),
        ("(class { get value() { return x; } }).value", true),
        ("new (class { get value() { return x; } })().value", false),
    ] {
        let source =
            format!("function C(it = {init}, {{x, keep}}) {{\n  return use(it, keep);\n}}");
        for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
            let graph = refs(&source, language);
            let binding = source.find("{x, keep}").unwrap() + 1;
            let edge = graph
                .edges
                .iter()
                .find(|edge| edge.from.start_byte == binding);
            assert_eq!(edge.is_some(), capture, "{language:?}: {init}");
            if let Some(edge) = edge {
                assert_eq!(
                    &source[edge.to.start_byte..edge.to.end_byte],
                    "x",
                    "{language:?}: {init}"
                );
            }
            assert_eq!(
                graph
                    .defs
                    .values()
                    .flatten()
                    .any(|def| def.start_byte == binding),
                capture,
                "{language:?}: {init}"
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
            true,
        ),
        (
            "function C({k}) { return class { m(k: number, @dec(k) value: number) {} }; }",
            Language::TypeScript,
            "k",
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
        let binding = source.find('{').unwrap();
        let binding = binding + source[binding..].find(&format!("{{{name}")).unwrap() + 1;
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
fn body_seam_preserves_legacy_anchor_and_keeps_signature_fenced() {
    for statement in ["if (x) return 1;", "return use(x);"] {
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
        let legacy = body_edges(&base);
        let expected: Vec<_> = legacy
            .into_iter()
            .filter(|(line, _, _)| *line == 3)
            .collect();
        assert_eq!(expected.len(), 1, "{statement}");
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
