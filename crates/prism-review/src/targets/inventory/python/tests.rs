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

fn parse_tied_calls(input: &str) -> ParsedFile {
    let parsed = ParsedFile::parse("input.py", input, Language::Python).unwrap();
    assert!(
        !parsed.tree.root_node().has_error(),
        "{}",
        parsed.tree.root_node().to_sexp()
    );
    parsed
}

fn assert_tied_keys(walk: &Walk<'_>, line: usize, input: &str) {
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
}

fn tied_anchor(input: &str, line: usize) -> [usize; 4] {
    let parsed = parse_tied_calls(input);
    let source = Source::new(&parsed);
    let walk = Walk::new(source.parsed());
    assert_tied_keys(&walk, line, input);
    Span::new(&source, anchor::winners(&walk.calls)[&line].node).range
}

const APPROXIMATE_DEPTH_TIES: &[(&str, &str, usize, &str)] = &[
    (
        "adjacent f-strings in dictionary key",
        "import requests\ndef f(u, obj):\n    return {f'' f\"\"\"{ (obj\n    ).get()}\"\"\": f\"\"\"{requests.get(\n         u)}\"\"\",}\n",
        4,
        "requests.get",
    ),
    (
        "boolean chain in dictionary key",
        "import requests\ndef f(u, b, c):\n    return {(          requests\n.get)(u) and b and c: [requests.post(\n      u)]}\n",
        4,
        "requests.post",
    ),
];

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
        // An enclosing def makes calls in definition defaults injectable too.
        let mut wrapped = String::from("import requests\ndef outer():\n");
        for line in input.lines() {
            wrapped.push_str("    ");
            wrapped.push_str(line);
            wrapped.push('\n');
        }
        assert_eq!(
            http_anchor_label(&wrapped, line + 2, "requests.get"),
            [None],
            "{label}"
        );
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn equal_anchor_keys_match_python_311_ast_walk() {
    let Some(python) = std::env::var_os("PRISM_PYTHON_311_ORACLE")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| {
                Path::new(&home).join("prism-evidence/faultpath-targets/venv/bin/python")
            })
        })
    else {
        eprintln!("skipped AST oracle: configure PRISM_PYTHON_311_ORACLE");
        return;
    };
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
    key = lambda n: (n.func.end_lineno - line, n.func.col_offset, -n.end_col_offset)
    winner = min(candidates, key=key)
    assert sum(key(n) == key(winner) for n in candidates) == 2, source
    winners.append([winner.lineno, winner.col_offset, winner.end_lineno, winner.end_col_offset])
print(json.dumps(winners))
"#;
    let mut fixtures = anchor_tie_inputs();
    for (label, prefix) in [
        ("conditional body versus else", ""),
        ("conditional test versus else", "0 if "),
    ] {
        let col = "    ).get() else ".len();
        let line = format!("    return ({prefix}");
        let pad = " ".repeat(col - line.len());
        let test = if prefix.is_empty() { " if flag" } else { "" };
        let close = "    ).get()";
        // Place the second callee at the first callee's starting column.
        let pad = format!("{pad}{}", " ".repeat(test.len()));
        let source =
            format!("def f():\n{line}{pad}(obj\n{close}{test} else requests.get(\n          ))\n");
        fixtures.push((label, source, 3));
    }
    let listed_count = fixtures.len();
    fixtures.extend(
        APPROXIMATE_DEPTH_TIES
            .iter()
            .map(|&(label, source, line, _)| (label, source.to_owned(), line)),
    );
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
    assert_eq!(expected.len(), fixtures.len());
    let actual: Vec<_> = fixtures[..listed_count]
        .iter()
        .map(|(_, source, line)| tied_anchor(source, *line))
        .collect();
    assert_eq!(actual, expected[..listed_count]);
    for (&(label, _, line, _), winner) in
        APPROXIMATE_DEPTH_TIES.iter().zip(&expected[listed_count..])
    {
        assert_eq!(winner[0], line - 1, "{label}: {winner:?}");
    }
    eprintln!(
        "checked {listed_count} listed ties and {} refused ties against CPython 3.11.11",
        APPROXIMATE_DEPTH_TIES.len()
    );
}

fn http_anchor_label(input: &str, line: usize, callee: &str) -> Vec<Option<UnsupportedReason>> {
    let parsed = parse_tied_calls(input);
    let mut out = InventoryOutput::default();
    collect_file(
        "input.py",
        &parsed,
        Path::new("."),
        &BTreeSet::from([Family::Http]),
        &mut out,
    );
    out.occurrences
        .iter()
        .filter(|occurrence| {
            occurrence.canonical == callee && occurrence.proof.callee.range[0] == line
        })
        .map(|occurrence| occurrence.reason)
        .collect()
}

#[test]
fn equal_anchor_keys_with_approximate_branch_depth_are_unsupported() {
    let mut failures = Vec::new();
    for &(label, input, line, callee) in APPROXIMATE_DEPTH_TIES {
        let parsed = parse_tied_calls(input);
        let walk = Walk::new(&parsed);
        assert_tied_keys(&walk, line, input);
        let has_winner = anchor::winners(&walk.calls).contains_key(&line);
        let found = http_anchor_label(input, line, callee);
        if has_winner || found != [Some(UnsupportedReason::NotLineAnchor)] {
            failures.push(format!("{label}: expected no winner and not_line_anchor, got winner={has_winner}, label={found:?}"));
        }
    }
    for (index, supported) in [("0", true), ("0,", false), ("0, 1", false)] {
        let prefix = "    return {[";
        let middle = format!("    ).get()][{index}]: requests.get(");
        let pad = " ".repeat(middle.find("requests").unwrap() - prefix.len());
        let input =
            format!("import requests\ndef f():\n{prefix}{pad}(obj\n{middle}\n          )}}\n");
        let parsed = parse_tied_calls(&input);
        let walk = Walk::new(&parsed);
        assert_tied_keys(&walk, 4, &input);
        let has_winner = anchor::winners(&walk.calls).contains_key(&4);
        if has_winner != supported {
            failures.push(format!(
                "subscript [{index}]: expected winner={supported}, got {has_winner}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn unlisted_equal_anchor_keys_are_unsupported() {
    fn tie(prefix: &str, middle: &str, suffix: &str) -> String {
        let close = "            ).get()";
        let col = close.len() + middle.find("requests").unwrap();
        let indent = col - prefix.rsplit('\n').next().unwrap().len();
        format!(
            "{prefix}{}(obj\n{close}{middle}\n{}u){suffix}",
            " ".repeat(indent),
            " ".repeat(close.len() - 2)
        )
    }

    let inputs = [
        ("with header", tie("def f(u):\n    with ", ": return requests.get(", "\n"), 3, "requests.get"),
        ("match guard", tie("def f(u, x):\n    match x:\n        case _ if ", ": return requests.get(", "\n"), 4, "requests.get"),
        ("and chain", "def f(u, c):\n    return (       requests\n.get)(u) and c and requests.post(\n      u)\n".into(), 3, "requests.post"),
        ("or chain", "def f(u, c):\n    return (     requests\n.get)(u) or c or requests.post(\n      u)\n".into(), 3, "requests.post"),
        ("dict comprehension", tie("def f(u, xs):\n    return {\n        ", ": requests.get(", " for x in xs}\n"), 4, "requests.get"),
        ("unlisted tuple siblings", tie("def f(u):\n    return (\n        ", ", requests.get(", ")\n"), 4, "requests.get"),
    ];
    let mut failures = Vec::new();
    for (label, input, line, callee) in inputs {
        let input = format!("import requests\n{input}");
        let parsed = parse_tied_calls(&input);
        let walk = Walk::new(&parsed);
        assert_tied_keys(&walk, line + 1, &input);
        let has_winner = anchor::winners(&walk.calls).contains_key(&(line + 1));
        let found = http_anchor_label(&input, line + 1, callee);
        if has_winner || found != [Some(UnsupportedReason::NotLineAnchor)] {
            failures.push(format!("{label}: expected no winner and not_line_anchor, got winner={has_winner}, label={found:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));

    let unequal = tie(
        "import requests\ndef f(u):\n    with ",
        ": return requests.get(",
        "\n",
    )
    .replace("u)\n", "u )\n");
    assert_eq!(http_anchor_label(&unequal, 4, "requests.get"), [None]);
}
