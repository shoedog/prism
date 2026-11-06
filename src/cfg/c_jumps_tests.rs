//! Unit tests for the C/C++ jump builder: edge provenance and the counters
//! the measurement report reads.
use super::*;
use crate::cfg::{build_cfg_edges_with_arms, EdgeOrigin};
use crate::languages::Language;

fn parse(source: &str, language: Language) -> ParsedFile {
    ParsedFile::parse("t.c", source, language).unwrap()
}

#[test]
fn jump_edges_are_structured_never_sequential() {
    let source = "int f(int x, int n) {\nwhile (n) {\nx = 1;\ncontinue;\n}\nreturn x;\n}\n";
    for language in [Language::C, Language::Cpp] {
        let parsed = parse(source, language);
        let edges = build_cfg_edges_with_arms(&parsed);
        let jump: Vec<_> = edges
            .iter()
            .filter(|(e, _)| e.from_line == 4 && e.to_line == 2)
            .collect();
        assert_eq!(jump.len(), 1, "{language:?}");
        assert_eq!(jump[0].1.origin, EdgeOrigin::Structured);
        assert!(!jump[0].1.crosses_lexical_arm());
    }
}

#[test]
fn counters_report_duff_one_line_do_and_multi_statement_jumps() {
    let duff = parse(
        "int f(int x, int n) {\nswitch (n & 1) {\ncase 0: do {\nx = 1;\ncase 1: x = 2;\n} while (--n);\n}\nreturn x;\n}\n",
        Language::C,
    );
    let s = c_jump_stats(&duff);
    assert_eq!(
        (s.duff_switches, s.dispatch_switches, s.rewired_do_loops),
        (1, 0, 1)
    );

    let one_line = parse(
        "int f(int x, int n) {\ndo { x = 1; } while (n);\nreturn x;\n}\n",
        Language::Cpp,
    );
    let s = c_jump_stats(&one_line);
    assert_eq!((s.one_line_do_loops, s.rewired_do_loops), (1, 0));

    let multi = parse(
        "int f(int x, int n) {\nwhile (n) {\nx = 1; continue;\n}\nreturn x;\n}\n",
        Language::C,
    );
    let s = c_jump_stats(&multi);
    assert_eq!(
        (s.jump_edges, s.jump_edges_on_multi_statement_lines),
        (1, 1)
    );
}

#[test]
fn other_languages_report_nothing() {
    let js = ParsedFile::parse(
        "t.js",
        "function f(x) {\nwhile (x) {\nbreak;\n}\n}\n",
        Language::JavaScript,
    )
    .unwrap();
    assert_eq!(c_jump_stats(&js), CJumpStats::default());
}
