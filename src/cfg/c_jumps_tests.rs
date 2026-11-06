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

/// Measurement hook, not a regression test: `PRISM_C_JUMPS_CORPUS=<dir> cargo
/// test --lib c_jumps_tests::corpus_counters -- --ignored --nocapture`.
/// Parses only; prints the builder's counters over every C/C++ file.
#[test]
#[ignore]
fn corpus_counters() {
    let Ok(dir) = std::env::var("PRISM_C_JUMPS_CORPUS") else {
        return;
    };
    let mut total = CJumpStats::default();
    let mut files = 0usize;
    let mut stack = vec![std::path::PathBuf::from(dir)];
    while let Some(path) = stack.pop() {
        if path.is_dir() {
            if let Ok(read) = std::fs::read_dir(&path) {
                stack.extend(read.flatten().map(|e| e.path()));
            }
            continue;
        }
        let Some(name) = path.to_str() else { continue };
        let Some(language) = Language::from_path(name) else {
            continue;
        };
        if !matches!(language, Language::C | Language::Cpp) {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(parsed) = ParsedFile::parse(name, &source, language) else {
            continue;
        };
        files += 1;
        let s = c_jump_stats(&parsed);
        total.jump_edges += s.jump_edges;
        total.jump_edges_on_multi_statement_lines += s.jump_edges_on_multi_statement_lines;
        total.dispatch_switches += s.dispatch_switches;
        total.duff_switches += s.duff_switches;
        total.rewired_do_loops += s.rewired_do_loops;
        total.one_line_do_loops += s.one_line_do_loops;
    }
    println!("CJUMP_COUNTERS files={files} {total:?}");
}
