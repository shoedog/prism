use super::*;
use crate::nested_execution_owner_audit::{
    canonical_raw_rows, compare_expected, nearest_callable_owner, observe_manual, observe_raw,
    resolve_anchor, source_sha256, AuditCase, AuditError, ExecutionRegion, ExpectedToken,
    OwnerAnchor, TokenAnchor,
};

fn anchor(needle: &'static str, occurrence: usize, offset: usize, length: usize) -> TokenAnchor {
    TokenAnchor {
        needle,
        occurrence,
        offset,
        length,
    }
}

#[test]
fn nested_execution_owner_fixture_anchors_are_exact() {
    let source = "const first=value; const second=value; // λ\n";
    assert_eq!(
        resolve_anchor(source, &anchor("value", 0, 12, 5)),
        Ok(12..17)
    );
    assert_eq!(
        resolve_anchor(source, &anchor("value", 1, 32, 5)),
        Ok(32..37)
    );

    assert!(matches!(
        resolve_anchor(source, &anchor("missing", 0, 0, 7)),
        Err(AuditError::MissingSelector { .. })
    ));
    assert!(matches!(
        resolve_anchor(source, &anchor("value", usize::MAX, 12, 5)),
        Err(AuditError::AmbiguousSelector { matches: 2, .. })
    ));
    assert!(matches!(
        resolve_anchor(source, &anchor("value", 0, 13, 5)),
        Err(AuditError::WrongOccurrence { .. })
    ));
    assert!(matches!(
        resolve_anchor(source, &anchor("value", 0, source.len() + 1, 5)),
        Err(AuditError::OutOfRangeOffset { .. })
    ));

    let unicode = "λtoken";
    assert!(matches!(
        resolve_anchor(unicode, &anchor("token", 0, 1, 5)),
        Err(AuditError::InvalidUtf8Boundary { .. })
    ));
    assert!(matches!(
        resolve_anchor(source, &anchor("value", 0, 12, 4)),
        Err(AuditError::WrongLength { .. })
    ));
}

const ASSIGNMENT: &str = "function outer(seed){let cb;cb=function inner(nestedParam){sinkNested(nestedRead);return nestedReturn;};let local=seed;return local;}";
const INITIALIZER: &str = "function outer(seed){const cb=function inner(nestedParam){sinkNested(nestedRead);return nestedReturn;};let local=seed;return local;}";
const CALL: &str = "function outer(seed){register(eager,function inner(nestedParam){sinkNested(nestedRead);return nestedReturn;});let local=seed;return local;}";
const RETURNED: &str = "function outer(seed){return (eager,function inner(nestedParam){sinkNested(nestedRead);return nestedReturn;});}";
const DEFAULTED: &str =
    "function outer(){const cb=function inner(p=initDefault()){sinkNested(p);};use(cb);}";
const PRESERVED: &str = "function outer(){let kept;kept=outerRead;return kept;}";

fn expected(
    needle: &'static str,
    occurrence: usize,
    offset: usize,
    region: ExecutionRegion,
) -> ExpectedToken {
    ExpectedToken {
        anchor: anchor(needle, occurrence, offset, needle.len()),
        region,
        owner: None,
    }
}

fn case(route: &'static str, language: Language) -> AuditCase {
    let (source, expected) = match route {
        "assignment" => (
            ASSIGNMENT,
            vec![
                expected("seed", 0, 15, ExecutionRegion::ParameterBinding),
                expected("inner", 0, 40, ExecutionRegion::Unsupported),
                expected("nestedParam", 0, 46, ExecutionRegion::ParameterBinding),
                expected("sinkNested", 0, 59, ExecutionRegion::NestedBody),
                expected("nestedRead", 0, 70, ExecutionRegion::NestedBody),
                expected("nestedReturn", 0, 89, ExecutionRegion::NestedBody),
                expected("seed", 1, 114, ExecutionRegion::ImmediateBody),
                expected("local", 1, 126, ExecutionRegion::ImmediateBody),
            ],
        ),
        "initializer" => (
            INITIALIZER,
            vec![
                expected("seed", 0, 15, ExecutionRegion::ParameterBinding),
                expected("inner", 0, 39, ExecutionRegion::Unsupported),
                expected("nestedParam", 0, 45, ExecutionRegion::ParameterBinding),
                expected("sinkNested", 0, 58, ExecutionRegion::NestedBody),
                expected("nestedRead", 0, 69, ExecutionRegion::NestedBody),
                expected("nestedReturn", 0, 88, ExecutionRegion::NestedBody),
                expected("seed", 1, 113, ExecutionRegion::ImmediateBody),
                expected("local", 1, 125, ExecutionRegion::ImmediateBody),
            ],
        ),
        "call" => (
            CALL,
            vec![
                expected("seed", 0, 15, ExecutionRegion::ParameterBinding),
                expected("eager", 0, 30, ExecutionRegion::EagerDefinitionExpression),
                expected("inner", 0, 45, ExecutionRegion::Unsupported),
                expected("nestedParam", 0, 51, ExecutionRegion::ParameterBinding),
                expected("sinkNested", 0, 64, ExecutionRegion::NestedBody),
                expected("nestedRead", 0, 75, ExecutionRegion::NestedBody),
                expected("nestedReturn", 0, 94, ExecutionRegion::NestedBody),
                expected("seed", 1, 120, ExecutionRegion::ImmediateBody),
                expected("local", 1, 132, ExecutionRegion::ImmediateBody),
            ],
        ),
        "returned" => (
            RETURNED,
            vec![
                expected("seed", 0, 15, ExecutionRegion::ParameterBinding),
                expected("eager", 0, 29, ExecutionRegion::EagerDefinitionExpression),
                expected("inner", 0, 44, ExecutionRegion::Unsupported),
                expected("nestedParam", 0, 50, ExecutionRegion::ParameterBinding),
                expected("sinkNested", 0, 63, ExecutionRegion::NestedBody),
                expected("nestedRead", 0, 74, ExecutionRegion::NestedBody),
                expected("nestedReturn", 0, 93, ExecutionRegion::NestedBody),
            ],
        ),
        _ => unreachable!(),
    };
    AuditCase {
        id: route,
        source: source.to_string(),
        language,
        queried_owner: anchor("outer", 0, 9, 5),
        expected,
    }
}

fn default_case(language: Language) -> AuditCase {
    AuditCase {
        id: "default",
        source: DEFAULTED.to_string(),
        language,
        queried_owner: anchor("outer", 0, 9, 5),
        expected: vec![expected(
            "initDefault",
            0,
            43,
            ExecutionRegion::ParameterDefault,
        )],
    }
}

fn named_owner<'a>(parsed: &'a ParsedFile, owner: &TokenAnchor) -> Node<'a> {
    let range = resolve_anchor(&parsed.source, owner).unwrap();
    parsed
        .all_functions()
        .into_iter()
        .find(|node| {
            parsed.language.function_name(node).is_some_and(|name| {
                name.start_byte() == range.start && name.end_byte() == range.end
            })
        })
        .expect("fixture owner must be independently identified by exact name bytes")
}

fn validate_authored_ast_relationships(case: &mut AuditCase, parsed: &ParsedFile) {
    let outer = named_owner(parsed, &case.queried_owner);
    let outer_anchor = OwnerAnchor {
        start_byte: outer.start_byte(),
        end_byte: outer.end_byte(),
        kind: outer.kind().to_string(),
    };
    for token in &mut case.expected {
        let range = resolve_anchor(&case.source, &token.anchor).unwrap();
        let node = parsed
            .tree
            .root_node()
            .descendant_for_byte_range(range.start, range.end)
            .unwrap();
        assert_eq!(node.start_byte()..node.end_byte(), range, "{}", case.id);
        assert_eq!(parsed.node_text(&node), token.anchor.needle, "{}", case.id);
        let actual_owner = nearest_callable_owner(parsed, node.start_byte(), node.end_byte());
        match token.region {
            ExecutionRegion::ImmediateBody | ExecutionRegion::EagerDefinitionExpression => {
                assert_eq!(actual_owner.as_ref(), Some(&outer_anchor), "{}", case.id);
            }
            ExecutionRegion::NestedBody => {
                assert_ne!(actual_owner.as_ref(), Some(&outer_anchor), "{}", case.id);
                let nested = parsed
                    .all_functions()
                    .into_iter()
                    .find(|f| {
                        Some(OwnerAnchor {
                            start_byte: f.start_byte(),
                            end_byte: f.end_byte(),
                            kind: f.kind().to_string(),
                        }) == actual_owner
                    })
                    .unwrap();
                let body = nested.child_by_field_name("body").unwrap();
                assert!(body.start_byte() <= range.start && range.end <= body.end_byte());
            }
            ExecutionRegion::ParameterBinding | ExecutionRegion::ParameterDefault => {
                let owner = actual_owner.as_ref().unwrap();
                let function = parsed
                    .all_functions()
                    .into_iter()
                    .find(|f| f.start_byte() == owner.start_byte && f.end_byte() == owner.end_byte)
                    .unwrap();
                if token.region == ExecutionRegion::ParameterBinding {
                    assert!(parsed
                        .function_parameter_occurrences(&function)
                        .iter()
                        .any(|(_, start, end)| *start == range.start && *end == range.end));
                } else {
                    let parameters = function.child_by_field_name("parameters").unwrap();
                    assert!(
                        parameters.start_byte() <= range.start
                            && range.end <= parameters.end_byte()
                    );
                }
            }
            _ => {}
        }
        token.owner = actual_owner;
    }
}

fn observations_for(
    case: &AuditCase,
    parsed: &ParsedFile,
) -> Vec<crate::nested_execution_owner_audit::ObservedOccurrence> {
    let outer = named_owner(parsed, &case.queried_owner);
    let (start, end) = parsed.node_line_range(&outer);
    let lines: BTreeSet<_> = (start..=end).collect();
    let mut rows = observe_raw(parsed, outer, &lines);
    let mut names = Vec::new();
    let mut paths = Vec::new();
    let mut spans = Vec::new();
    parsed.collect_rvalues_manual(outer, &lines, &mut names);
    parsed.collect_rvalue_paths_manual(outer, &lines, &mut paths);
    parsed.collect_rvalue_spans_manual(outer, &lines, &mut spans);
    rows.extend(observe_manual(parsed, names, paths, spans));
    rows.sort();
    rows
}

#[test]
fn nested_execution_owner_query_and_manual_classification() {
    let frozen = [
        (
            "assignment",
            2,
            "5f30c85ddfba5e8b26edaca20b6d30b64934fa2bf3f8fc51604abfcea26dad33",
        ),
        (
            "initializer",
            2,
            "cf865636a6b5d9de878ce93611535e1fee9d22e719240c25e3cb40ef42283cb7",
        ),
        (
            "call",
            14,
            "f39736873146f7dec5041393de025fdbdfb3003355e2dc93f064516f0ba0ee03",
        ),
        (
            "returned",
            2,
            "f58a0dec38da8d8edbd35946d89ec5d9d5a43b9fa83f2f9e2bbc0c2a883f05f2",
        ),
    ];
    for route in ["assignment", "initializer", "call", "returned"] {
        let mut language_rows = Vec::new();
        for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
            let mut case = case(route, language);
            let parsed = ParsedFile::parse("owner-fixture", &case.source, language).unwrap();
            validate_authored_ast_relationships(&mut case, &parsed);
            let rows = observations_for(&case, &parsed);
            let canonical = canonical_raw_rows(&rows);
            if language == Language::JavaScript {
                println!("OWNER_RAW {route} {}", canonical.join(" || "));
            }
            let (_, expected_len, expected_sha) =
                frozen.iter().find(|(id, _, _)| id == &route).unwrap();
            assert_eq!(canonical.len(), *expected_len, "{route}/{language:?}");
            assert_eq!(source_sha256(&canonical.join("\n")), *expected_sha);
            language_rows.push(canonical);
        }
        assert_eq!(language_rows[0], language_rows[1], "{route}/js-ts");
        assert_eq!(language_rows[0], language_rows[2], "{route}/js-tsx");
    }
}

#[test]
fn nested_execution_owner_desired_contract() {
    let mut mismatches = Vec::new();
    for route in ["assignment", "initializer", "call", "returned"] {
        for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
            let mut case = case(route, language);
            let parsed = ParsedFile::parse("owner-fixture", &case.source, language).unwrap();
            validate_authored_ast_relationships(&mut case, &parsed);
            let rows = observations_for(&case, &parsed);
            mismatches.extend(compare_expected(&case, &rows));
        }
    }
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let mut case = default_case(language);
        let parsed = ParsedFile::parse("owner-default", &case.source, language).unwrap();
        validate_authored_ast_relationships(&mut case, &parsed);
        let rows = observations_for(&case, &parsed);
        mismatches.extend(compare_expected(&case, &rows));
    }
    let mut preserved = AuditCase {
        id: "preserved",
        source: PRESERVED.to_string(),
        language: Language::JavaScript,
        queried_owner: anchor("outer", 0, 9, 5),
        expected: vec![
            expected("outerRead", 0, 31, ExecutionRegion::ImmediateBody),
            expected("kept", 2, 48, ExecutionRegion::ImmediateBody),
        ],
    };
    let parsed =
        ParsedFile::parse("owner-preserved", &preserved.source, preserved.language).unwrap();
    validate_authored_ast_relationships(&mut preserved, &parsed);
    let required = canonical_raw_rows(&observations_for(&preserved, &parsed));
    assert_eq!(
        required,
        [
            "ManualNames|outerRead|1|-|-|-",
            "ManualPaths|outerRead|1|-|-|-",
            "ManualSpans|kept|1|48|52|function_declaration:0-54",
            "ManualSpans|outerRead|1|31|40|function_declaration:0-54",
            "QueryNames|outerRead|1|-|-|-",
            "QueryPaths|outerRead|1|-|-|-",
            "QuerySpans|kept|1|48|52|function_declaration:0-54",
            "QuerySpans|outerRead|1|31|40|function_declaration:0-54",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>()
    );
    println!("OWNER_DESIRED_REQUIRED {}", required.join(" || "));
    for mismatch in &mismatches {
        println!("OWNER_DESIRED_MISMATCH {mismatch:?}");
    }
    assert!(
        mismatches.is_empty(),
        "{} wrong-owner rows",
        mismatches.len()
    );
}

#[test]
fn nested_execution_owner_eager_and_own_callable_controls() {
    let parsed = ParsedFile::parse("controls.js", CALL, Language::JavaScript).unwrap();
    let outer = named_owner(&parsed, &anchor("outer", 0, 9, 5));
    let inner = named_owner(&parsed, &anchor("inner", 0, 45, 5));
    let outer_lines: BTreeSet<_> = (1..=parsed.node_line_range(&outer).1).collect();
    let inner_lines: BTreeSet<_> = (1..=parsed.node_line_range(&inner).1).collect();
    let outer_spans = parsed.rvalue_identifier_spans_on_lines(&outer, &outer_lines);
    let inner_spans = parsed.rvalue_identifier_spans_on_lines(&inner, &inner_lines);
    assert!(outer_spans
        .iter()
        .any(|span| span.path.to_string() == "eager"));
    assert!(inner_spans
        .iter()
        .any(|span| span.path.to_string() == "nestedRead"));
    assert!(!inner_spans
        .iter()
        .any(|span| span.path.to_string() == "eager"));
    let root_spans =
        parsed.rvalue_identifier_spans_on_lines(&parsed.tree.root_node(), &BTreeSet::from([1]));
    assert!(root_spans
        .iter()
        .any(|span| span.path.to_string() == "nestedParam"));

    let computed_source = "function outer(){const obj={[keyExpr()](p){sinkNested(p);}};use(obj);}";
    let computed = ParsedFile::parse("computed.js", computed_source, Language::JavaScript).unwrap();
    assert_eq!(computed.parse_error_count, 0);
    let key = computed_source.find("keyExpr").unwrap();
    let lexical = nearest_callable_owner(&computed, key, key + "keyExpr".len()).unwrap();
    assert_eq!(lexical.kind, "method_definition");
    let outer = computed
        .all_functions()
        .into_iter()
        .find(|node| {
            computed
                .language
                .function_name(node)
                .is_some_and(|name| computed.node_text(&name) == "outer")
        })
        .unwrap();
    let spans = computed.rvalue_identifier_spans_on_lines(&outer, &BTreeSet::from([1]));
    assert!(spans
        .iter()
        .any(|span| { span.path.to_string() == "keyExpr" && span.start_byte == key }));

    let default_source =
        "function outer(){const cb=function inner(p=initDefault()){sinkNested(p);};use(cb);}";
    let defaulted = ParsedFile::parse("default.ts", default_source, Language::TypeScript).unwrap();
    let nested = defaulted
        .all_functions()
        .into_iter()
        .find(|node| {
            defaulted
                .language
                .function_name(node)
                .is_some_and(|name| defaulted.node_text(&name) == "inner")
        })
        .unwrap();
    let init = default_source.find("initDefault").unwrap();
    let parameters = nested.child_by_field_name("parameters").unwrap();
    assert!(parameters.start_byte() <= init && init < parameters.end_byte());
    let roles = [
        ("initDefault", ExecutionRegion::ParameterDefault),
        ("TypeToken", ExecutionRegion::ErasedType),
    ];
    assert_eq!(roles[0].1, ExecutionRegion::ParameterDefault);
    let outer = defaulted
        .all_functions()
        .into_iter()
        .max_by_key(|node| node.end_byte() - node.start_byte())
        .unwrap();
    let default_spans = defaulted.rvalue_identifier_spans_on_lines(&outer, &BTreeSet::from([1]));
    let nested_default_spans =
        defaulted.rvalue_identifier_spans_on_lines(&nested, &BTreeSet::from([1]));
    let outer_default_count = default_spans
        .iter()
        .filter(|span| span.start_byte == init && span.path.to_string() == "initDefault")
        .count();
    let own_default_count = nested_default_spans
        .iter()
        .filter(|span| span.start_byte == init && span.path.to_string() == "initDefault")
        .count();
    println!("OWNER_DEFAULT_REPAIR outer={outer_default_count} own_callable={own_default_count}");
    assert_eq!((outer_default_count, own_default_count), (0, 0));

    let erased_source =
        "function typed(seed: TypeToken){let local;local=seed as ErasedCast;return local;}";
    let erased = ParsedFile::parse("erased.ts", erased_source, Language::TypeScript).unwrap();
    let typed = erased.all_functions()[0];
    let erased_spans = erased.rvalue_identifier_spans_on_lines(&typed, &BTreeSet::from([1]));
    assert!(erased_spans
        .iter()
        .any(|span| span.path.to_string() == "seed"));
    assert!(!erased_spans
        .iter()
        .any(|span| { matches!(span.path.to_string().as_str(), "TypeToken" | "ErasedCast") }));
    assert_eq!(roles[1].1, ExecutionRegion::ErasedType);

    let augmented_source = "function outer(){total+=delta;}";
    let augmented =
        ParsedFile::parse("augmented.js", augmented_source, Language::JavaScript).unwrap();
    let spans = augmented
        .rvalue_identifier_spans_on_lines(&augmented.all_functions()[0], &BTreeSet::from([1]));
    assert!(spans.iter().any(|span| {
        span.path.to_string() == "total"
            && span.start_byte == augmented_source.find("total").unwrap()
    }));

    let arrow =
        ParsedFile::parse("arrow.js", "const cb=p=>sink(p);", Language::JavaScript).unwrap();
    let arrow_function = arrow.all_functions()[0];
    assert_eq!(arrow_function.kind(), "arrow_function");
    assert!(arrow
        .function_parameter_occurrences(&arrow_function)
        .is_empty());

    let shapes = "function outer(){function named(p){}const a=function expr(p){};const b=function(p){};const c=(p)=>{};const d=p=>p;const e=function*(p){};function* gen(p){}async function af(p){}const aa=async(p)=>p;const o={method(p){},get value(){return 1;}};}";
    let shape_file = ParsedFile::parse("shapes.js", shapes, Language::JavaScript).unwrap();
    assert_eq!(shape_file.parse_error_count, 0);
    let kinds: BTreeSet<_> = shape_file
        .all_functions()
        .iter()
        .map(|node| node.kind())
        .collect();
    println!("OWNER_CALLABLE_KINDS {kinds:?}");
    for required in [
        "function_declaration",
        "function_expression",
        "arrow_function",
        "generator_function_declaration",
        "method_definition",
    ] {
        assert!(kinds.contains(required), "missing {required}: {kinds:?}");
    }
    assert!(!kinds.contains("generator_function"));
    fn tree_has_kind(node: Node<'_>, kind: &str) -> bool {
        if node.kind() == kind {
            return true;
        }
        let mut cursor = node.walk();
        let found = node
            .children(&mut cursor)
            .any(|child| tree_has_kind(child, kind));
        found
    }
    assert!(tree_has_kind(
        shape_file.tree.root_node(),
        "generator_function"
    ));
    assert!(shape_file
        .language
        .callable_boundary_node_types()
        .contains(&"generator_function"));

    let phase = "class C extends heritageExpr(){[classKey()](){}static field=staticInit();instance=instanceInit();static{blockInit();}}";
    let phase_file = ParsedFile::parse("phase.js", phase, Language::JavaScript).unwrap();
    assert_eq!(phase_file.parse_error_count, 0);
    let phase_paths: Vec<_> = phase_file
        .rvalue_identifier_spans_on_lines(&phase_file.tree.root_node(), &BTreeSet::from([1]))
        .into_iter()
        .map(|span| (span.path.to_string(), span.start_byte, span.end_byte))
        .collect();
    println!("OWNER_PHASE_EXCLUSIONS {phase_paths:?}");

    let class_source = "function outer(){const C=class extends heritageExpr(){[classKey()](){methodRead();}static field=staticInit();instance=instanceInit();static{blockInit();}};use(C);}";
    let class_file = ParsedFile::parse("class.js", class_source, Language::JavaScript).unwrap();
    let class_outer = named_owner(&class_file, &anchor("outer", 0, 9, 5));
    let class_spans =
        class_file.rvalue_identifier_spans_on_lines(&class_outer, &BTreeSet::from([1]));
    for preserved in [
        "heritageExpr",
        "classKey",
        "methodRead",
        "staticInit",
        "instanceInit",
        "blockInit",
    ] {
        assert!(
            class_spans
                .iter()
                .any(|span| span.path.to_string() == preserved),
            "class-phase exclusion lost {preserved}: {class_spans:?}"
        );
    }

    let ambiguous_source = "function outer(){const a=function same(){return firstRead;};const b=function same(){return secondRead;};}";
    let ambiguous =
        ParsedFile::parse("ambiguous.js", ambiguous_source, Language::JavaScript).unwrap();
    let same_ranges: BTreeSet<_> = ambiguous
        .all_functions()
        .into_iter()
        .filter(|node| {
            ambiguous
                .language
                .function_name(node)
                .is_some_and(|name| ambiguous.node_text(&name) == "same")
        })
        .map(|node| {
            (
                node.start_byte(),
                node.end_byte(),
                node.start_position().row + 1,
            )
        })
        .collect();
    assert_eq!(same_ranges.len(), 2);
    assert!(same_ranges.iter().all(|(_, _, line)| *line == 1));

    let unicode_source = "function outer(){const λvalue=1;return λvalue;}";
    let unicode_offset = unicode_source.find("λvalue").unwrap();
    assert_eq!(
        resolve_anchor(
            unicode_source,
            &anchor("λvalue", 0, unicode_offset, "λvalue".len())
        ),
        Ok(unicode_offset..unicode_offset + "λvalue".len())
    );

    let signatures = "function outer(){const a=function inner(required,optional?:T,inert=0,effect=init(),{x},...rest){return required;};const b=p=>p;}";
    let signature_file =
        ParsedFile::parse("signatures.ts", signatures, Language::TypeScript).unwrap();
    assert_eq!(signature_file.parse_error_count, 0);
    let parameter_rows: Vec<_> = signature_file
        .all_functions()
        .iter()
        .map(|node| {
            (
                node.kind(),
                signature_file.function_parameter_occurrences(node),
            )
        })
        .collect();
    println!("OWNER_SIGNATURE_ROWS {parameter_rows:?}");
    let recovery = ParsedFile::parse(
        "recovery.js",
        "function outer(){const cb=function broken( {",
        Language::JavaScript,
    )
    .unwrap();
    assert!(recovery.parse_error_count > 0);
}

#[test]
fn nested_execution_owner_unindexed_and_recovery_refuse() {
    fn first_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
        if node.kind() == kind {
            return Some(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = first_kind(child, kind) {
                return Some(found);
            }
        }
        None
    }

    let source =
        "function outer(){const hidden=function*(p=initDefault()){sinkNested(p);};use(hidden);}";
    let parsed = ParsedFile::parse("unindexed.js", source, Language::JavaScript).unwrap();
    let generator = first_kind(parsed.tree.root_node(), "generator_function").unwrap();
    assert!(!parsed
        .all_functions()
        .iter()
        .any(|node| byte_range_eq(node, &generator)));
    let lines = BTreeSet::from([1]);
    assert!(parsed
        .rvalue_identifiers_on_lines(&generator, &lines)
        .is_empty());
    assert!(parsed
        .rvalue_identifier_paths_on_lines(&generator, &lines)
        .is_empty());
    assert!(parsed
        .rvalue_identifier_spans_on_lines(&generator, &lines)
        .is_empty());
    let mut manual_names = Vec::new();
    let mut manual_paths = Vec::new();
    let mut manual_spans = Vec::new();
    parsed.collect_rvalues_manual(generator, &lines, &mut manual_names);
    parsed.collect_rvalue_paths_manual(generator, &lines, &mut manual_paths);
    parsed.collect_rvalue_spans_manual(generator, &lines, &mut manual_spans);
    assert_eq!(
        (manual_names, manual_paths, manual_spans),
        Default::default()
    );

    let root_spans = parsed.rvalue_identifier_spans_on_lines(&parsed.tree.root_node(), &lines);
    for retained in ["initDefault", "sinkNested"] {
        assert!(root_spans
            .iter()
            .any(|span| span.path.to_string() == retained));
    }

    let recovery = ParsedFile::parse(
        "recovery.js",
        "function outer(){const cb=function broken( {",
        Language::JavaScript,
    )
    .unwrap();
    assert!(recovery.parse_error_count > 0);
    assert!(first_kind(recovery.tree.root_node(), "function_expression").is_none());
    assert!(recovery.all_functions().is_empty());
}
