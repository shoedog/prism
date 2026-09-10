//! Erased import options are syntax, not runtime argument/value occurrences.
use prism::{ast::ParsedFile, data_flow::DataFlowGraph, languages::Language};
use std::collections::{BTreeMap, BTreeSet};

const IMPORT: &str = "typeof import(\"./m\", { with: { \"resolution-mode\": \"import\" } })";

fn parse(source: &str, language: Language) -> ParsedFile {
    let parsed = ParsedFile::parse("values.ts", source, language).unwrap();
    assert_eq!(parsed.parse_error_count, 0, "{language:?}: {source}");
    assert_eq!(parsed.source, source);
    parsed
}

fn erased_sources() -> Vec<String> {
    [
        "function owner() { type T = TYPE; }",
        "function owner(value: TYPE) {}",
        "function owner<T extends TYPE>() {}",
        "function owner<T = TYPE>() {}",
        "function owner() { interface Props { value: TYPE; } }",
        "function owner() { const result = runtime as TYPE; }",
        "function owner() { const result = runtime satisfies TYPE; }",
        "function owner() { result = runtime as TYPE; }",
        "function owner() { result = runtime satisfies TYPE; }",
        "function owner() { sink<TYPE>(runtime); }",
        "function owner() { sink(runtime as TYPE); }",
        "function owner() { sink(runtime satisfies TYPE); }",
        "function owner() { const result = { value: runtime as TYPE }; }",
        "function owner() { const result = runtime /* value */ as /* type */ TYPE; }",
        "function owner() { const result = runtime /* value */ satisfies /* type */ TYPE; }",
        "function owner() { return runtime as TYPE; }",
    ]
    .into_iter()
    .map(|source| source.replace("TYPE", IMPORT))
    .collect()
}

#[test]
fn erased_import_options_do_not_become_rvalue_identifiers() {
    let mut failures = Vec::new();
    for language in [Language::TypeScript, Language::Tsx] {
        for source in erased_sources() {
            let parsed = parse(&source, language);
            let values =
                parsed.rvalue_identifiers_on_lines(&parsed.tree.root_node(), &BTreeSet::from([1]));
            if values
                .iter()
                .any(|(name, _)| name == "with" || name == "import")
            {
                failures.push(format!("{language:?} {source}: {values:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn erased_import_options_do_not_become_rvalue_paths() {
    let mut failures = Vec::new();
    for language in [Language::TypeScript, Language::Tsx] {
        for source in erased_sources() {
            let parsed = parse(&source, language);
            let values = parsed
                .rvalue_identifier_paths_on_lines(&parsed.tree.root_node(), &BTreeSet::from([1]));
            if values
                .iter()
                .any(|(path, _)| path.base == "with" || path.base == "import")
            {
                failures.push(format!("{language:?} {source}: {values:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn erased_import_options_do_not_become_rvalue_spans() {
    let mut failures = Vec::new();
    for language in [Language::TypeScript, Language::Tsx] {
        for source in erased_sources() {
            let parsed = parse(&source, language);
            let erased_start = source.find(IMPORT).unwrap();
            let erased_end = erased_start + IMPORT.len();
            let values = parsed
                .rvalue_identifier_spans_on_lines(&parsed.tree.root_node(), &BTreeSet::from([1]));
            if values
                .iter()
                .any(|span| span.end_byte > erased_start && span.start_byte < erased_end)
            {
                failures.push(format!("{language:?} {source}: {values:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn nested_value_casts_keep_runtime_identifier_spans_and_selected_lines() {
    for language in [Language::TypeScript, Language::Tsx] {
        let source = format!("function owner() {{\n type T = {IMPORT};\n const answer = sink(runtime as {IMPORT});\n return runtime satisfies {IMPORT};\n}}");
        let parsed = parse(&source, language);
        let root = parsed.tree.root_node();
        assert!(parsed
            .rvalue_identifiers_on_lines(&root, &BTreeSet::from([2]))
            .is_empty());
        assert!(parsed
            .rvalue_identifier_paths_on_lines(&root, &BTreeSet::from([2]))
            .is_empty());
        assert!(parsed
            .rvalue_identifier_spans_on_lines(&root, &BTreeSet::from([2]))
            .is_empty());
        for (line, marker) in [(3, "runtime as"), (4, "runtime satisfies")] {
            let values = parsed.rvalue_identifier_spans_on_lines(&root, &BTreeSet::from([line]));
            let start = source.find(marker).unwrap();
            assert!(values.iter().any(|span| span.path.base == "runtime"
                && span.start_byte == start
                && span.end_byte == start + 7));
            assert!(
                values
                    .iter()
                    .all(|span| ["sink", "runtime"].contains(&span.path.base.as_str())),
                "{values:?}"
            );
        }
    }
}

#[test]
fn condition_value_side_survives_while_its_type_options_are_erased() {
    for language in [Language::TypeScript, Language::Tsx] {
        let source = format!("function owner() {{ if (runtime /*value*/ satisfies /*type*/ {IMPORT}) {{ sink(runtime); }} }}");
        let parsed = parse(&source, language);
        assert_eq!(
            parsed.condition_variables_on_lines(&parsed.tree.root_node(), &BTreeSet::from([1])),
            vec![("runtime".into(), 1)]
        );
    }
}

#[test]
fn real_dynamic_import_options_and_runtime_typeof_match_javascript() {
    for source in [
        "function owner() { const value = import(moduleId, { with: { type: kind } }); }",
        "function owner() { const value = typeof import(moduleId, { with: { type: kind } }); }",
        "function owner() { value = { with: kind, type: moduleId }; }",
    ] {
        let js = parse(source, Language::JavaScript);
        let lines = BTreeSet::from([1]);
        let names = js.rvalue_identifiers_on_lines(&js.tree.root_node(), &lines);
        let paths = js.rvalue_identifier_paths_on_lines(&js.tree.root_node(), &lines);
        let spans = js.rvalue_identifier_spans_on_lines(&js.tree.root_node(), &lines);
        assert!(
            names.iter().any(|(name, _)| name == "with"),
            "{source}: {names:?}"
        );
        for language in [Language::TypeScript, Language::Tsx] {
            let parsed = parse(source, language);
            assert_eq!(
                parsed.rvalue_identifiers_on_lines(&parsed.tree.root_node(), &lines),
                names
            );
            assert_eq!(
                parsed.rvalue_identifier_paths_on_lines(&parsed.tree.root_node(), &lines),
                paths
            );
            assert_eq!(
                parsed.rvalue_identifier_spans_on_lines(&parsed.tree.root_node(), &lines),
                spans
            );
            assert_eq!(
                parsed.call_names_on_lines(&[1]),
                js.call_names_on_lines(&[1])
            );
        }
    }
}

#[test]
fn mixed_same_line_erased_and_real_options_keep_only_real_option_spans() {
    for language in [Language::TypeScript, Language::Tsx] {
        let source = format!("function owner() {{ /* Ω */ type T = {IMPORT}; import(moduleId, {{ with: {{ type: kind }} }}); }}");
        let parsed = parse(&source, language);
        let spans =
            parsed.rvalue_identifier_spans_on_lines(&parsed.tree.root_node(), &BTreeSet::from([1]));
        let real = source.rfind("with").unwrap();
        let with: Vec<_> = spans
            .iter()
            .filter(|span| span.path.base == "with")
            .collect();
        assert_eq!(with.len(), 1, "{spans:?}");
        assert_eq!((with[0].start_byte, with[0].end_byte), (real, real + 4));
    }
}

#[test]
fn full_and_subset_dfg_do_not_materialize_erased_option_uses() {
    for language in [Language::TypeScript, Language::Tsx] {
        let source = format!("function owner() {{\n type T = {IMPORT};\n import(moduleId, {{ with: {{ type: kind }} }});\n}}");
        let files = BTreeMap::from([("values.ts".into(), parse(&source, language))]);
        let full = DataFlowGraph::build(&files);
        let subset = DataFlowGraph::build_subset(&files, &BTreeSet::from(["values.ts".into()]));
        for graph in [&full, &subset] {
            let uses: Vec<_> = graph
                .uses
                .values()
                .flatten()
                .filter(|usage| usage.path.base == "with")
                .collect();
            assert!(!uses.is_empty(), "real runtime option control disappeared");
            assert!(uses.iter().all(|usage| usage.line == 3), "{uses:?}");
        }
        assert_eq!(full.uses, subset.uses);
    }
}

#[test]
fn semantic_value_use_refuses_erased_spans_without_refusing_real_options() {
    for language in [Language::TypeScript, Language::Tsx] {
        let source = format!("function owner() {{ type T = {IMPORT}; import(moduleId, {{ with: {{ type: kind }} }}); }}");
        let parsed = parse(&source, language);
        let erased = source.find("with").unwrap();
        let real = source.rfind("with").unwrap();
        assert!(!parsed.is_semantic_value_use(erased, erased + 4));
        assert!(parsed.is_semantic_value_use(real, real + 4));
        let runtime = source.find("moduleId").unwrap();
        assert!(parsed.is_semantic_value_use(runtime, runtime + 8));
    }
}

#[test]
fn angle_assertion_prunes_only_typescript_type_side() {
    let source = format!("function owner() {{ result = <{IMPORT}>runtime; }}");
    let parsed = parse(&source, Language::TypeScript);
    let values =
        parsed.rvalue_identifier_spans_on_lines(&parsed.tree.root_node(), &BTreeSet::from([1]));
    assert!(!values.is_empty());
    assert!(
        values.iter().all(|span| span.path.base == "runtime"),
        "{values:?}"
    );
}
