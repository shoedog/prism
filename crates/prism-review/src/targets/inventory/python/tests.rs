use super::{anchor, collect_file, function_bounds, source::Source, walk::Walk};
use crate::targets::inventory::{evidence::Span, Family, InventoryOutput, UnsupportedReason};
use prism::ast::ParsedFile;
use prism::languages::Language;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[test]
fn review_focus_6_line_anchor_occurrence() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/targets_inventory/py");
    for (name, entire_call, callee, target_span, supported) in [
        (
            "as4-07",
            [3, 11, 4, 35],
            [3, 11, 3, 35],
            [3, 19, 3, 34],
            false,
        ),
        (
            "tr6-02",
            [3, 11, 3, 28],
            [3, 11, 3, 23],
            [3, 11, 3, 28],
            true,
        ),
        (
            "tr7-02",
            [3, 11, 4, 31],
            [3, 11, 3, 33],
            [3, 11, 3, 28],
            false,
        ),
    ] {
        let file = format!("http_root/{name}.py");
        let input = fs::read_to_string(repo.join(&file)).unwrap();
        let parsed = ParsedFile::parse(&file, &input, Language::Python).unwrap();
        let source = Source::new(&parsed);
        let walk = Walk::new(source.parsed());
        let anchor = anchor::winners(&walk.calls)[&3];
        assert_eq!(Span::new(&source, anchor.node).range, entire_call, "{name}");
        assert_eq!(Span::new(&source, anchor.callee).range, callee, "{name}");
        let mut result = InventoryOutput::default();
        collect_file(
            &file,
            &parsed,
            &repo,
            &BTreeSet::from([Family::Http]),
            &mut result,
        );
        let [occurrence] = result.occurrences.as_slice() else {
            panic!("expected one proven occurrence for {name}");
        };
        assert_eq!(occurrence.proof.call.range, target_span, "{name}");
        assert_eq!(
            occurrence.reason,
            (!supported).then_some(UnsupportedReason::NotLineAnchor)
        );
    }
}

#[test]
fn normalized_bounds_and_original_crlf_span() {
    let input =
        "import requests\r\nasync def f(u):\r\n    (requests\r\n     .get(u))\r\n    # ignored\r\n";
    let parsed = ParsedFile::parse("input.py", input, Language::Python).unwrap();
    let bounds = function_bounds(&parsed);
    assert_eq!((bounds[0].start_line, bounds[0].end_line), (2, 4));
    let mut result = InventoryOutput::default();
    collect_file(
        "input.py",
        &parsed,
        Path::new("."),
        &BTreeSet::from([Family::Http]),
        &mut result,
    );
    assert_eq!(
        result.unsupported[0].reason,
        UnsupportedReason::MultilineCallee
    );
    let evidence = &result.occurrences[0].proof;
    assert_eq!(evidence.callee.text, "requests\r\n     .get");
    assert_eq!(evidence.callee.range, [3, 5, 4, 9]);
}

#[test]
fn function_bounds_recovers_after_starred_string_tuple() {
    for tuple in ["1, *x", "1, *\"ten\""] {
        let input = format!("g = {tuple}\n\ndef f():\n    pass\n");
        let parsed = ParsedFile::parse("input.py", &input, Language::Python).unwrap();
        let bounds = function_bounds(&parsed);
        assert_eq!(
            bounds,
            [crate::targets::inventory::FunctionBounds {
                name: "f".to_owned(),
                start_line: 3,
                end_line: 4,
            }],
            "{tuple}: {}",
            parsed.tree.root_node().to_sexp()
        );
    }
}

fn anchor_tie_inputs() -> Vec<(&'static str, String, usize)> {
    [
        (
            "dict",
            "def f():\n    return {\n        0: ",
            ", requests.get(",
            ": 0}\n",
        ),
        (
            "conditional",
            "def f():\n    return (\n",
            " if requests.get(",
            " else 0)\n",
        ),
        (
            "def defaults",
            "def f(x=",
            ", *, y=requests.get(",
            "):\n    pass\n",
        ),
        (
            "lambda defaults",
            "f = (lambda x=",
            ", *, y=requests.get(",
            ": 0)\n",
        ),
        (
            "chained targets",
            "def f():\n    [",
            "][0] = requests.get(",
            "[0] = 0\n",
        ),
    ]
    .into_iter()
    .map(|(label, prefix, middle, suffix)| {
        let line = prefix.lines().count() + usize::from(prefix.ends_with('\n')) + 1;
        let col = "    ).get()".len() + middle.find("requests").unwrap();
        let used = prefix.rsplit('\n').next().unwrap().len();
        assert!(used <= col, "{label}");
        let source = format!(
            "{prefix}{}(obj\n    ).get(){middle}\n          ){suffix}",
            " ".repeat(col - used)
        );
        (label, source, line)
    })
    .collect()
}

fn tied_anchor(input: &str, line: usize) -> [usize; 4] {
    let parsed = ParsedFile::parse("input.py", input, Language::Python).unwrap();
    assert!(
        !parsed.tree.root_node().has_error(),
        "{}",
        parsed.tree.root_node().to_sexp()
    );
    let source = Source::new(&parsed);
    let walk = Walk::new(source.parsed());
    let competitors: Vec<_> = walk
        .calls
        .iter()
        .filter(|call| {
            call.callee.start_position().row < line && call.callee.end_position().row + 1 == line
        })
        .collect();
    assert_eq!(competitors.len(), 2, "{input}");
    assert_eq!(
        competitors[0].callee.start_position().column,
        competitors[1].callee.start_position().column,
        "{input}"
    );
    assert_eq!(
        competitors[0].node.end_position().column,
        competitors[1].node.end_position().column,
        "{input}"
    );
    Span::new(&source, anchor::winners(&walk.calls)[&line].node).range
}

#[test]
fn equal_anchor_keys_follow_python_field_order_and_depth() {
    let mut failures = Vec::new();
    for (label, input, line) in anchor_tie_inputs() {
        let actual = tied_anchor(&input, line);
        let col = input
            .lines()
            .nth(line - 1)
            .unwrap()
            .find("requests")
            .unwrap();
        let expected = [line, col, line + 1, 11];
        if actual != expected {
            failures.push(format!("{label}: expected {expected:?}, got {actual:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn equal_anchor_keys_match_python_311_ast_walk() {
    let Some(home) = std::env::var_os("HOME") else {
        eprintln!("skipped AST oracle: HOME is unavailable");
        return;
    };
    let python = Path::new(&home).join("prism-evidence/faultpath-targets/venv/bin/python");
    if !python.is_file() {
        eprintln!("skipped AST oracle: {} is unavailable", python.display());
        return;
    }
    let script = r#"
import ast, json, sys
assert sys.version_info[:3] == (3, 11, 11), sys.version
fixtures = json.loads(sys.argv[1])
winners = []
for _, source, line in fixtures:
    candidates = [n for n in ast.walk(ast.parse(source)) if isinstance(n, ast.Call)
                  and n.func.lineno <= line <= n.func.end_lineno]
    winner = min(candidates, key=lambda n: (n.func.end_lineno - line, n.func.col_offset, -n.end_col_offset))
    winners.append([winner.lineno, winner.col_offset, winner.end_lineno, winner.end_col_offset])
print(json.dumps(winners))
"#;
    let fixtures = anchor_tie_inputs();
    let output = std::process::Command::new(python)
        .args(["-c", script, &serde_json::to_string(&fixtures).unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let expected: Vec<[usize; 4]> = serde_json::from_slice(&output.stdout).unwrap();
    let actual: Vec<_> = fixtures
        .iter()
        .map(|(_, source, line)| tied_anchor(source, *line))
        .collect();
    assert_eq!(actual, expected);
}
