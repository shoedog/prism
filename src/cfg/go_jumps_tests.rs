//! Boundary and provenance tests for the Go-only builder.
use super::*;
use crate::cfg::{build_cfg_edges_with_arms, EdgeOrigin};
use crate::languages::Language;

fn added(source: &str) -> EdgeSink {
    let parsed = ParsedFile::parse("jump.go", source, Language::Go).unwrap();
    let mut sink = EdgeSink::new();
    for owner in parsed.all_functions() {
        let lines: Vec<_> = parsed
            .statements_in_function(&owner)
            .iter()
            .map(|s| s.0)
            .collect();
        build(owner, &parsed, &lines, &mut sink);
    }
    sink
}

#[test]
fn go_jump_unresolved_targets_never_fall_back_to_another_scope() {
    for source in [
        "package p\nfunc f() {\ngoto Missing\n}\n",
        "package p\nfunc f() {\nfor {\nbreak Missing\n}\nafter()\n}\n",
        "package p\nfunc f() {\nfor {\nL: switch {\ndefault:\ncontinue L\n}\n}\n}\n",
        "package p\nfunc f() {\nf := func() {\nL: use(1)\n}\ngoto L\n}\n",
        "package p\nfunc f() {\nL: use(1)\nfor {\nbreak L\n}\nafter()\n}\n",
    ] {
        assert!(added(source).is_empty(), "invented target: {source}");
    }
}

#[test]
fn go_jump_unused_label_and_final_exit_have_no_added_edges() {
    for source in [
        "package p\nfunc f() {\nUnused: use(1)\nafter()\n}\n",
        "package p\nfunc f() {\nfor {\nbreak\n}\n}\n",
    ] {
        assert!(added(source).is_empty(), "unexpected exit node: {source}");
    }
}

#[test]
fn go_jump_edges_have_structural_provenance_across_arms() {
    let source = "package p\nfunc f(c bool) {\nif c {\ngoto L\n}\nL: use(1)\n}\n";
    let parsed = ParsedFile::parse("jump.go", source, Language::Go).unwrap();
    let edges = build_cfg_edges_with_arms(&parsed);
    let jumps: Vec<_> = edges.iter().filter(|(e, _)| e.from_line == 4).collect();
    assert_eq!(jumps.len(), 1);
    let (edge, provenance) = jumps[0];
    assert_eq!(edge.to_line, 6);
    assert_eq!(provenance.origin, EdgeOrigin::Structured);
    assert!(!provenance.crosses_lexical_arm());
    // Ordinary jumps are structural; function-exit defer dispatch is flagged.
    let all = added(include_str!("../../tests/fixtures/go_jumps/jumps.go"));
    assert_eq!(all.edges.len(), 15);
    assert_eq!(
        all.origins
            .iter()
            .filter(|o| **o == EdgeOrigin::Structured)
            .count(),
        14
    );
    let exit = all
        .edges
        .iter()
        .zip(&all.origins)
        .find(|(e, _)| e.from_line == 155)
        .unwrap();
    assert_eq!(exit.0.to_line, 153);
    assert_eq!(*exit.1, EdgeOrigin::GoDeferExit);
}

#[test]
fn go_round2_successor_shapes() {
    // The first statement after the wrapper is the successor, even when the
    // textual next statement is in an exclusive arm or a sibling case.
    let wrappers = [
        ("if c {", "} else {\nwrong()\n}"),
        ("if c {\nwrong()\n} else {", "}"),
        ("switch c {\ncase true:", "case false:\nwrong()\n}"),
        ("switch v.(type) {\ncase int:", "default:\nwrong()\n}"),
        ("select {\ncase <-ch:", "default:\nwrong()\n}"),
        ("select {\ndefault:", "}"),
        ("{", "}"),
        ("L: {", "}"),
    ];
    let mut failures = Vec::new();
    for (prefix, suffix) in wrappers {
        let source = format!("package p\nfunc f(c bool, v interface{{}}, ch chan int) {{\n{prefix}\nfor {{\nbreak\n}}\n{suffix}\nafter()\n}}\n");
        let from = source.lines().position(|l| l == "break").unwrap() + 1;
        let to = source.lines().position(|l| l == "after()").unwrap() + 1;
        let sink = added(&source);
        let actual: Vec<_> = sink
            .edges
            .iter()
            .filter(|e| e.from_line == from)
            .map(|e| e.to_line)
            .collect();
        if actual != [to] {
            failures.push(format!("{prefix}: {actual:?} != {to}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn go_round2_terminal_switch_resumes_loop_instead_of_dispatching_defer() {
    let source = "package p\nfunc f() {\ndefer use()\nfor {\nswitch {\ndefault:\nbreak\n}\n}\n}\n";
    let sink = added(source);
    let rows: Vec<_> = sink
        .edges
        .iter()
        .zip(&sink.origins)
        .map(|(e, o)| (e.from_line, e.to_line, *o))
        .collect();
    assert_eq!(rows, [(7, 4, EdgeOrigin::Structured)]);
}

#[test]
fn go_round2_unknown_successor_is_not_a_guessed_edge_or_function_exit() {
    // Inc/dec are absent from the statement universe. Do not skip across one
    // to invent a proof edge, or mistake an unmappable successor for exit.
    let source = "package p\nfunc f(x int) {\ndefer use(x)\nfor {\nbreak\n}\nx++\nuse(x)\n}\n";
    assert!(added(source).is_empty());
}
