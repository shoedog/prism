use super::*;
use std::cell::Cell;

thread_local! {
    static VISITS: Cell<usize> = const { Cell::new(0) };
}

pub(super) fn record_visit() {
    VISITS.with(|visits| visits.set(visits.get() + 1));
}

// Original, unpruned pre-order walk. This module is compiled only under cfg(test).
fn collect_identifiers_at_row_full<'a>(
    parsed: &ParsedFile,
    node: Node<'a>,
    row: usize,
    out: &mut Vec<Node<'a>>,
) {
    if node.start_position().row == row && parsed.language.is_identifier_node(node.kind()) {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_identifiers_at_row_full(parsed, child, row, out);
    }
}

fn assert_every_line(parsed: &ParsedFile) {
    // Include line zero (the API aliases it to line one), trailing empty lines,
    // and an out-of-file line as well as every actual source line.
    let lines = parsed.source.bytes().filter(|&b| b == b'\n').count() + 1;
    for line in 0..=lines + 1 {
        let mut expected = Vec::new();
        collect_identifiers_at_row_full(
            parsed,
            parsed.tree.root_node(),
            line.saturating_sub(1),
            &mut expected,
        );
        let actual = parsed.identifiers_on_line(line);
        // Node equality pins identities and order, including zero-width nodes.
        assert_eq!(actual, expected, "{:?}, line {line}", parsed.language);
    }
}

fn descendants(node: Node<'_>, predicate: impl Fn(Node<'_>) -> bool + Copy) -> usize {
    let mut count = usize::from(predicate(node));
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        count += descendants(child, predicate);
    }
    count
}

#[test]
fn every_line_matches_full_walk_in_six_languages() {
    for (language, source) in [
        (
            Language::JavaScript,
            r#"const first = seed;
const result = invoke(
    alpha, beta,
    (value) => ({
        property: value,
        other: alpha + beta,
    }),
);
const text = `plain
    ${alpha + beta}
    tail`;

const last = result;"#,
        ),
        (
            Language::TypeScript,
            r#"const first: number = seed;
const result = invoke(
    alpha, beta,
    (value: number) => ({
        property: value,
        other: alpha + beta,
    }),
);
const text: string = `plain
    ${alpha + beta}
    tail`;

const last = result;"#,
        ),
        (
            Language::Python,
            r#"first = seed
result = invoke(
    alpha, beta, gamma,
    {
        "property": alpha,
        "other": alpha + beta,
    },
)
text = f"""plain
    {alpha + beta}
    tail"""

last = result"#,
        ),
        (
            Language::Go,
            r#"package fixture
var first = seed
func run() {
    result := invoke(
        alpha, beta,
        map[string]int{
            "property": alpha,
            "other": alpha + beta,
        },
    )
    text := `plain
        alpha beta
        tail`

    consume(result, text)
}
var last = first"#,
        ),
        (
            Language::Rust,
            r##"const FIRST: i32 = SEED;
fn run() {
    let result = invoke(
        alpha, beta,
        |value| {
            alpha + beta + value
        },
    );
    let text = r#"plain
        alpha beta
        tail"#;

    consume(result, text);
}
const LAST: i32 = FIRST;"##,
        ),
        (
            Language::C,
            "int first = 1;\n\
int run(int alpha, int beta) {\n\
    int result = invoke(\n\
        alpha, beta,\n\
        (struct Pair){\n\
            .left = alpha,\n\
            .right = beta,\n\
        }\n\
    );\n\
    const char *text = \"plain\\\n\
        alpha beta\\\n\
        tail\";\n\n\
    return consume(result, text);\n\
}\n\
int last = 2;",
        ),
    ] {
        let parsed = ParsedFile::parse("fixture", source, language).unwrap();
        assert_eq!(parsed.parse_error_count, 0, "{language:?}");
        assert_every_line(&parsed);
        assert!(
            (1..=source.lines().count()).any(|line| parsed.identifiers_on_line(line).is_empty()),
            "{language:?}: require a line without identifiers"
        );
        assert!(
            (1..=source.lines().count()).any(|line| parsed.identifiers_on_line(line).len() >= 3),
            "{language:?}: require several identifiers on one line"
        );
    }
}

#[test]
fn every_line_matches_full_walk_with_error_nodes() {
    for (language, source) in [
        (
            Language::JavaScript,
            "const first = seed;\n@@ alpha + beta;\nconst last = first;\n",
        ),
        (
            Language::TypeScript,
            "const first: number = seed;\n@@ alpha + beta;\nconst last = first;\n",
        ),
        (
            Language::Python,
            "first = seed\n@@ alpha + beta\nlast = first\n",
        ),
        (
            Language::Go,
            "package fixture\nfunc run() {\n@@ alpha + beta\n}\n",
        ),
        (Language::Rust, "fn run() {\n@@ alpha + beta;\n}\n"),
        (Language::C, "int run(void) {\n@@ alpha + beta;\n}\n"),
    ] {
        let parsed = ParsedFile::parse("broken", source, language).unwrap();
        assert!(
            descendants(parsed.tree.root_node(), |n| n.is_error()) > 0,
            "{language:?}"
        );
        assert_every_line(&parsed);
    }
}

#[test]
fn every_line_matches_full_walk_with_zero_width_missing_node() {
    let parsed = ParsedFile::parse(
        "missing.c",
        "int run(void) {\n    return value\n}\n",
        Language::C,
    )
    .unwrap();
    assert!(
        descendants(parsed.tree.root_node(), |n| n.is_missing()
            && n.start_byte() == n.end_byte())
            > 0
    );
    assert_every_line(&parsed);
}

#[test]
fn unrelated_subtrees_are_not_visited() {
    // A structural work assertion, not a timing measurement. Keep the target
    // between unrelated siblings to exercise pruning on both sides.
    let parsed = ParsedFile::parse(
        "work.js",
        "const before = old + other;\nconst target = wanted + input;\nconst after = next + value;\n",
        Language::JavaScript,
    )
    .unwrap();
    assert_every_line(&parsed);
    let root = parsed.tree.root_node();
    for line in [1, 2, 3, 4, 100] {
        VISITS.with(|visits| visits.set(0));
        parsed.identifiers_on_line(line);
        let visits = VISITS.with(Cell::get);
        let row = line - 1;
        let relevant = descendants(root, |n| {
            n.id() == root.id() || (n.start_position().row <= row && n.end_position().row >= row)
        });
        assert_eq!(
            visits, relevant,
            "line {line}: visited an unrelated subtree"
        );
    }
}
