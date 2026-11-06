//! #645 fold 2 (PR #682 round 2): no condition bypass for any rewired `do`
//! (including an Unknown body entry), explicit completion rules, the generic
//! condition-line collision guard, and exact predecessor sets for every
//! completion row. Inline sources; lines are 1-indexed from the first line.
use crate::common::*;
use prism::cpg::{FlowConfidence, FlowDoubt};
use std::collections::BTreeSet;

type Edge = (usize, usize);

fn edges(source: &str, language: Language) -> Vec<Edge> {
    let parsed = ParsedFile::parse("t.src", source, language).unwrap();
    let mut edges: Vec<Edge> = prism::cfg::build_cfg_edges(&parsed)
        .iter()
        .map(|e| (e.from_line, e.to_line))
        .collect();
    edges.sort();
    edges.dedup();
    edges
}

fn preds(e: &[Edge], line: usize) -> BTreeSet<usize> {
    e.iter().filter(|x| x.1 == line).map(|x| x.0).collect()
}

fn label(source: &str, language: Language, var: &str, def: usize, use_: usize) -> String {
    let parsed = ParsedFile::parse("t.src", source, language).unwrap();
    let cpg = CodePropertyGraph::build(&BTreeMap::from([("t.src".to_owned(), parsed)]));
    let rows: Vec<String> = cpg
        .dfg
        .labels()
        .filter(|(e, _)| e.from.path.base == var && e.from.line == def && e.to.line == use_)
        .map(|(_, l)| match l {
            FlowConfidence::Exact => "exact".to_string(),
            FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line }) => {
                format!("killed@{kill_line}")
            }
            FlowConfidence::NameOnly(d) => format!("{d:?}"),
        })
        .collect();
    assert_eq!(rows.len(), 1, "{var} {def}->{use_}: {rows:?}");
    rows[0].clone()
}

fn both(source: &str, check: impl Fn(&str, Language)) {
    for language in [Language::C, Language::Cpp] {
        check(source, language);
    }
}

// ---- 1: no condition bypass when the body entry is Unknown ----------------

const IFDEF_DO: &str = "int f(int c, int x) {\nc = 0;\ndo {\n#ifdef A\nx = 0;\n#endif\nx = c;\nc = 1;\n} while ((c = g()) != 0);\nreturn x;\n}\n";
const PRAGMA_DO: &str = "int f(int c, int x) {\nc = 0;\ndo {\n#pragma once\nx = c;\nc = 1;\n} while ((c = g()) != 0);\nreturn x;\n}\n";
const SOL_IFDEF_DO: &str = "int f(int c, int again) {\nint x = 0;\ndo {\n#ifdef A\n(void)0;\n#endif\nx = c;\nc = 1;\n} while ((c = 2), again);\nreturn x;\n}\n";

#[test]
fn unknown_entry_do_has_no_back_edge_and_the_condition_reenters_through_the_header() {
    both(IFDEF_DO, |s, l| {
        let e = edges(s, l);
        assert!(!e.contains(&(8, 3)), "{l:?} {e:?}");
        assert!(e.contains(&(8, 9)) && e.contains(&(9, 3)), "{l:?} {e:?}");
        assert_eq!(preds(&e, 9), BTreeSet::from([8]), "{l:?} {e:?}");
        assert_eq!(label(s, l, "c", 8, 7), "killed@9", "{l:?}");
    });
    both(PRAGMA_DO, |s, l| {
        let e = edges(s, l);
        assert!(!e.contains(&(6, 3)), "{l:?} {e:?}");
        assert!(e.contains(&(7, 3)), "{l:?} {e:?}");
        assert_eq!(label(s, l, "c", 6, 5), "killed@7", "{l:?}");
    });
    both(SOL_IFDEF_DO, |s, l| {
        let e = edges(s, l);
        assert!(!e.contains(&(8, 3)), "{l:?} {e:?}");
        assert_ne!(label(s, l, "c", 8, 7), "exact", "{l:?}");
    });
}

// ---- 2: explicit completion rules; 4: exact predecessor sets --------------

/// (source, exact predecessors of the condition line, condition line, C++ only)
const PREDS: &[(&str, &[usize], usize, bool)] = &[
    ("int f(int x) {\ndo {\nx = 1;\n} while (x);\nreturn x;\n}\n", &[3], 4, false),
    ("int f(int x) {\ndo {\nif (x) {\nx = 1;\n} else {\nx = 2;\n}\n} while (x);\nreturn x;\n}\n", &[4, 6], 8, false),
    ("int f(int x) {\ndo {\nif (x)\nx = 1;\nelse\nx = 2;\n} while (x);\nreturn x;\n}\n", &[4, 6], 7, false),
    ("int f(int x) {\ndo {\nx = 3;\nif (x)\nx = 1;\n} while (x);\nreturn x;\n}\n", &[4, 5], 6, false),
    // nested loop: (4, 6) is main's sequential edge, the header (3) its false exit
    ("int f(int x, int n) {\ndo {\nwhile (n) {\nx = 1;\n}\n} while (x);\nreturn x;\n}\n", &[3, 4], 6, false),
    // switch with `default`: the header never exits directly
    ("int f(int x, int n) {\ndo {\nswitch (n) {\ncase 1:\nx = 1;\nbreak;\ndefault:\nx = 2;\n}\n} while (x);\nreturn x;\n}\n", &[6, 8], 10, false),
    // switch without `default`: the header is a predecessor
    ("int f(int x, int n) {\ndo {\nswitch (n) {\ncase 1:\nx = 1;\nbreak;\ncase 2:\nx = 2;\n}\n} while (x);\nreturn x;\n}\n", &[3, 6, 8], 10, false),
    ("int f(int x) {\ndo {\nx = 1;\nl:\nx = 2;\n} while (x);\nreturn x;\n}\n", &[5], 6, false),
    // empty body: the header and the condition's own re-entry
    ("int f(int x) {\ndo {\n} while (x);\nreturn x;\n}\n", &[2, 3], 3, false),
    ("int f(int x) {\ndo {\ndo {\nx = 1;\n} while (x);\n} while (x);\nreturn x;\n}\n", &[5], 6, false),
    // terminal try/catch: no completion from the try header (line 3)
    ("int f(int x) {\ndo {\ntry {\nx = 1;\n} catch (...) {\nx = 2;\n}\n} while (x);\nreturn x;\n}\n", &[6], 8, true),
];

#[test]
fn every_completion_row_has_exactly_the_expected_predecessors_of_the_condition() {
    for (source, expect, cond, cpp_only) in PREDS {
        let langs: &[Language] = if *cpp_only {
            &[Language::Cpp]
        } else {
            &[Language::C, Language::Cpp]
        };
        for l in langs {
            let e = edges(source, *l);
            let want: BTreeSet<usize> = expect.iter().copied().collect();
            assert_eq!(preds(&e, *cond), want, "{l:?}: {e:?}\n{source}");
        }
    }
}

#[test]
fn switch_with_default_ending_the_body_kills_an_earlier_definition() {
    let s = "int f(int n, int again) {\nint x = 0;\ndo {\nswitch (n) {\ncase 1:\nx = 1;\nbreak;\ndefault:\nx = 2;\n}\n} while (again);\nreturn x;\n}\n";
    both(s, |s, l| {
        let e = edges(s, l);
        assert!(!e.contains(&(4, 11)), "{l:?} {e:?}");
        assert_ne!(label(s, l, "x", 2, 12), "exact", "{l:?}");
    });
    // without `default` the header exits and the definition can survive
    let nd = "int f(int n, int again) {\nint x = 0;\ndo {\nswitch (n) {\ncase 1:\nx = 1;\nbreak;\ncase 2:\nx = 2;\n}\n} while (again);\nreturn x;\n}\n";
    both(nd, |s, l| {
        assert!(edges(s, l).contains(&(4, 11)), "{l:?}");
    });
}

/// `Parser/lexer/number.c` shape: the inner `do` overwrites `c` before the
/// next outer iteration, so the outer def is killed at the inner write.
#[test]
fn nested_do_body_overwrite_kills_the_outer_definition() {
    let s = "int f(int c, int x) {\nc = 0;\ndo {\nx = c;\nc = 1;\ndo {\nc = 2;\n} while (c);\n} while (x);\nreturn x;\n}\n";
    both(s, |s, l| {
        let e = edges(s, l);
        assert_eq!(preds(&e, 8), BTreeSet::from([7]), "{l:?} {e:?}");
        assert_eq!(label(s, l, "c", 5, 4), "killed@7", "{l:?}");
    });
}

// ---- 3: the condition-line collision guard, generically -------------------

fn kinds_at(s: &str, l: Language, line: usize) -> Vec<String> {
    let parsed = ParsedFile::parse("t.src", s, l).unwrap();
    parsed
        .all_functions()
        .iter()
        .flat_map(|f| parsed.statements_in_function(f))
        .filter(|(n, _)| *n == line)
        .map(|(_, k)| k)
        .collect()
}

#[test]
fn a_statement_starting_on_the_condition_line_anywhere_keeps_main_edges() {
    // (source, condition line, next line, main's kept back edge)
    let rows = [
        // unbraced `if` parent
        ("int f(int x, int c, int a) {\nif (a)\ndo {\nx = 1;\n} while (c); return x;\nx = 2;\n}\n", 5, 6, (4, 3)),
        // after an enclosing block's closing brace
        ("int f(int x, int c, int a) {\nif (a) {\ndo {\nx = 1;\n} while (c); } return x;\nx = 2;\n}\n", 5, 6, (4, 3)),
        // a labeled do
        ("int f(int x, int c) {\nl: do {\nx = 1;\n} while (c); return x;\nx = 2;\n}\n", 4, 5, (3, 2)),
    ];
    for (s, cond, next, back) in rows {
        both(s, |s, l| {
            let e = edges(s, l);
            assert!(
                !e.iter().any(|&(f, t)| f == cond && t == next),
                "{l:?} {e:?}"
            );
            assert_eq!(
                kinds_at(s, l, cond),
                vec!["return_statement".to_string()],
                "{l:?}"
            );
            // no condition node: main's back edge from the body stays
            assert!(e.contains(&back), "{l:?} {e:?}");
        });
    }
}
