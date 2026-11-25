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
