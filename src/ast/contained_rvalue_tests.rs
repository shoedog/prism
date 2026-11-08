use super::*;

const BODY: &str = "let local; local=rhs; local+=delta; sink(arg); return returned;";

#[test]
fn contained_rvalue_query_capture_boundaries() {
    let mut failures = Vec::new();
    for lang in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for (style, source) in [
            (
                "assignment-function",
                format!("let holder; holder=function inner(unusedHeader){{{BODY}}};"),
            ),
            (
                "assignment-arrow",
                format!("let holder; holder=(unusedHeader)=>{{{BODY}}};"),
            ),
            (
                "assignment-sibling",
                format!("let holder; holder=(outside, function inner(unusedHeader){{{BODY}}});"),
            ),
            (
                "call-function",
                format!("register(outside, function inner(unusedHeader){{{BODY}}});"),
            ),
            (
                "call-arrow",
                format!("register(outside, (unusedHeader)=>{{{BODY}}});"),
            ),
            (
                "call-multiline",
                format!("register(outside, function inner(unusedHeader){{\n{BODY}\n}});"),
            ),
            (
                "return-function",
                format!(
                    "function outer(){{return (outside, function inner(unusedHeader){{{BODY}}});}}"
                ),
            ),
            (
                "return-arrow",
                format!("function outer(){{return (outside, (unusedHeader)=>{{{BODY}}});}}"),
            ),
        ] {
            let parsed = ParsedFile::parse("fixture", &source, lang).unwrap();
            assert_eq!(parsed.parse_error_count, 0, "{lang:?}/{style}");
            let function = parsed
                .all_functions()
                .into_iter()
                .find(|n| {
                    parsed
                        .function_parameter_occurrences(n)
                        .iter()
                        .any(|(name, _, _)| name == "unusedHeader")
                })
                .unwrap();
            let (start, end) = parsed.node_line_range(&function);
            let lines = (start..=end).collect();
            let names = parsed.rvalue_identifiers_on_lines(&function, &lines);
            let paths = parsed.rvalue_identifier_paths_on_lines(&function, &lines);
            let spans = parsed.rvalue_identifier_spans_on_lines(&function, &lines);
            for (kind, values) in [
                (
                    "names",
                    names
                        .iter()
                        .map(|(n, _)| n.clone())
                        .collect::<BTreeSet<_>>(),
                ),
                ("paths", paths.iter().map(|(p, _)| p.to_string()).collect()),
                ("spans", spans.iter().map(|s| s.path.to_string()).collect()),
            ] {
                let bad: Vec<_> = ["unusedHeader", "outside"]
                    .into_iter()
                    .filter(|s| values.contains(*s))
                    .collect();
                if !bad.is_empty() {
                    failures.push(format!("{lang:?}/{style}/{kind}: {bad:?}"));
                }
                for required in ["rhs", "delta", "sink", "arg"] {
                    assert!(
                        values.contains(required),
                        "lost {required}: {lang:?}/{style}/{kind} {values:?}"
                    );
                }
                println!("CONTAINED_RAW {lang:?}/{style}/{kind} {values:?}");
            }
            assert!(spans.iter().any(|s| s.path.to_string() == "returned"));
            assert!(spans.iter().any(|s| s.path.to_string() == "local"
                && s.start_byte == source.find("local+=").unwrap()));
            assert_eq!(parsed.source, source);
        }
    }
    assert!(
        failures.is_empty(),
        "outside-callable/signature Uses: {failures:?}"
    );
}

#[test]
fn contained_rvalue_whole_file_root_keeps_accepted_capture_behavior() {
    for lang in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let parsed = ParsedFile::parse(
            "fixture",
            "register(outside, function inner(unusedHeader){sink(arg);});",
            lang,
        )
        .unwrap();
        assert_eq!(parsed.parse_error_count, 0);
        let root = parsed.tree.root_node();
        let lines = BTreeSet::from([1]);
        let names = parsed.rvalue_identifiers_on_lines(&root, &lines);
        let paths = parsed.rvalue_identifier_paths_on_lines(&root, &lines);
        let spans = parsed.rvalue_identifier_spans_on_lines(&root, &lines);
        // Whole-node containment is not a nested execution-scope rewrite.
        for value in ["outside", "unusedHeader", "arg"] {
            assert!(names.iter().any(|(n, _)| n == value));
            assert!(paths.iter().any(|(p, _)| p.to_string() == value));
            assert!(spans.iter().any(|s| s.path.to_string() == value));
        }
    }
}

#[test]
fn contained_rvalue_non_js_immediate_reads_are_unchanged() {
    for (lang, source) in [
        (
            Language::Python,
            "def run(input):\n    local = input\n    sink(input)\n    return input\n",
        ),
        (
            Language::Rust,
            "fn run(input:i32){let mut local=input;local+=input;sink(input);}",
        ),
        (
            Language::Go,
            "package p\nfunc run(input int){local:=input;local+=input;sink(input)}",
        ),
        (
            Language::C,
            "void run(int input){int local;local=input;local+=input;sink(input);}",
        ),
        (
            Language::Java,
            "class A { void run(int input){int local;local=input;local+=input;sink(input);} }",
        ),
        (
            Language::Lua,
            "function run(input)\n local value=input\n sink(input)\n return input\nend",
        ),
    ] {
        let parsed = ParsedFile::parse("fixture", source, lang).unwrap();
        assert_eq!(parsed.parse_error_count, 0, "{lang:?}");
        let function = parsed.all_functions()[0];
        let (start, end) = parsed.node_line_range(&function);
        let lines = (start..=end).collect();
        assert!(
            parsed
                .rvalue_identifiers_on_lines(&function, &lines)
                .iter()
                .any(|(n, _)| n == "input"),
            "{lang:?}"
        );
        assert!(
            parsed
                .rvalue_identifier_paths_on_lines(&function, &lines)
                .iter()
                .any(|(p, _)| p.to_string() == "input"),
            "{lang:?}"
        );
        assert!(
            parsed
                .rvalue_identifier_spans_on_lines(&function, &lines)
                .iter()
                .any(|s| s.path.to_string() == "input"),
            "{lang:?}"
        );
    }
}
