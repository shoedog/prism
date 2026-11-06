//! #645 fold 1 (PR #682 round 1): the do back edge, the shared-line guards and
//! the matrix gaps the reviewers' probes found. Inline sources; line numbers
//! are 1-indexed from the first source line.
use crate::common::*;
use prism::cpg::{FlowConfidence, FlowDoubt};

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

// ---- F1: the do back edge ------------------------------------------------

const OPUS_DO: &str = "int f(int c, int x) {\nc = 0;\ndo {\nx = c;\nc = 1;\n} while ((c = g()) != 0);\nreturn x;\n}\n";
const SOL_DO: &str = "int f(int again) {\nint x = 0;\ndo {\nint y = x;\nx = 1;\n} while ((x = 2), again);\nreturn x;\n}\n";
const COMPOUND_DO: &str =
    "int f(int c, int x) {\nc = 0;\ndo {\nx = c;\nc = 1;\n} while ((c += 1) < 3);\nreturn x;\n}\n";
const IF_END_DO: &str = "int f(int c, int x) {\nc = 0;\ndo {\nx = c;\nif (x)\nc = 1;\n} while ((c = g()) != 0);\nreturn x;\n}\n";

#[test]
fn rewired_do_has_no_back_edge_and_completes_through_the_condition() {
    both(OPUS_DO, |s, l| {
        let e = edges(s, l);
        assert!(!e.contains(&(5, 3)), "{l:?} {e:?}");
        assert!(e.contains(&(5, 6)) && e.contains(&(6, 4)), "{l:?} {e:?}");
        assert_eq!(label(s, l, "c", 5, 4), "killed@6", "{l:?}");
    });
    both(SOL_DO, |s, l| {
        let e = edges(s, l);
        assert!(!e.contains(&(5, 3)), "{l:?} {e:?}");
        assert!(e.contains(&(5, 6)), "{l:?} {e:?}");
        // the condition's `x = 2` (line 6) intervenes: anything but `exact`
        assert_ne!(label(s, l, "x", 5, 4), "exact", "{l:?}");
    });
    both(COMPOUND_DO, |s, l| {
        let e = edges(s, l);
        assert!(!e.contains(&(5, 3)), "{l:?} {e:?}");
        assert_eq!(label(s, l, "c", 5, 4), "killed@6", "{l:?}");
    });
    both(IF_END_DO, |s, l| {
        let e = edges(s, l);
        assert!(!e.contains(&(6, 3)) && !e.contains(&(5, 3)), "{l:?} {e:?}");
        assert!(e.contains(&(6, 7)) && e.contains(&(5, 7)), "{l:?} {e:?}");
    });
}

/// (source, from-lines that must reach the condition line, condition line)
const ENDINGS: &[(&str, &[usize], usize)] = &[
    // plain statement
    ("int f(int x) {\ndo {\nx = 1;\n} while (x);\nreturn x;\n}\n", &[3], 4),
    // braced if/else
    ("int f(int x) {\ndo {\nif (x) {\nx = 1;\n} else {\nx = 2;\n}\n} while (x);\nreturn x;\n}\n", &[4, 6], 8),
    // unbraced if/else
    ("int f(int x) {\ndo {\nif (x)\nx = 1;\nelse\nx = 2;\n} while (x);\nreturn x;\n}\n", &[4, 6], 7),
    // unbraced if without else
    ("int f(int x) {\ndo {\nx = 3;\nif (x)\nx = 1;\n} while (x);\nreturn x;\n}\n", &[4, 5], 6),
    // nested loop
    ("int f(int x, int n) {\ndo {\nwhile (n) {\nx = 1;\n}\n} while (x);\nreturn x;\n}\n", &[3], 6),
    // switch with default as last statement
    ("int f(int x, int n) {\ndo {\nswitch (n) {\ncase 1:\nx = 1;\nbreak;\ndefault:\nx = 2;\n}\n} while (x);\nreturn x;\n}\n", &[6, 8], 10),
    // labeled statement
    ("int f(int x) {\ndo {\nx = 1;\nl:\nx = 2;\n} while (x);\nreturn x;\n}\n", &[5], 6),
    // empty body
    ("int f(int x) {\ndo {\n} while (x);\nreturn x;\n}\n", &[2], 3),
    // nested do as last statement
    ("int f(int x) {\ndo {\ndo {\nx = 1;\n} while (x);\n} while (x);\nreturn x;\n}\n", &[5], 6),
];

#[test]
fn every_structured_body_ending_reaches_the_condition_without_a_back_edge() {
    for (source, froms, cond) in ENDINGS {
        both(source, |s, l| {
            let e = edges(s, l);
            for from in *froms {
                assert!(
                    e.contains(&(*from, *cond)),
                    "{l:?} {from}->{cond}: {e:?}\n{s}"
                );
            }
            // the `do` header (line 2) is never re-entered from the body
            assert!(
                !e.iter().any(|&(f, t)| t == 2 && f > 2),
                "{l:?} back edge to the do header: {e:?}\n{s}"
            );
            assert!(e.contains(&(*cond, cond + 1)), "{l:?} exit: {e:?}\n{s}");
        });
    }
}

#[test]
fn shared_line_do_keeps_the_back_edge() {
    // condition on the last body line: main's edges, back edge included
    let s = "int f(int x, int n) {\nx = 0;\ndo {\nx = 1; } while (n);\nreturn x;\n}\n";
    both(s, |s, l| {
        let e = edges(s, l);
        assert!(e.contains(&(4, 3)), "{l:?} {e:?}");
    });
    let one = "int f(int x, int n) {\ndo { x = 1; } while (n);\nreturn x;\n}\n";
    both(one, |s, l| assert!(edges(s, l).contains(&(2, 2)), "{l:?}"));
}

// ---- F2: a statement after the do on the condition line ------------------

#[test]
fn statement_after_while_on_the_condition_line_keeps_main_edges() {
    let s = "int f(int x, int c) {\ndo {\nx = 1;\n} while (c); return x;\nint y;\n}\n";
    both(s, |s, l| {
        let e = edges(s, l);
        // main: back edge, header exit; no condition node
        assert!(e.contains(&(3, 2)), "{l:?} {e:?}");
        assert!(!e.contains(&(4, 2)), "{l:?} {e:?}");
        // the `return` keeps no fall-through
        assert!(!e.iter().any(|&(f, t)| f == 4 && t == 5), "{l:?} {e:?}");
        let parsed = ParsedFile::parse("t.src", s, l).unwrap();
        let kinds: Vec<_> = parsed
            .all_functions()
            .iter()
            .flat_map(|f| parsed.statements_in_function(f))
            .filter(|(line, _)| *line == 4)
            .map(|(_, k)| k)
            .collect();
        assert_eq!(kinds, vec!["return_statement".to_string()], "{l:?}");
    });
}

// ---- F4 (tests): probes ---------------------------------------------------

#[test]
fn loop_exit_inside_a_try_or_preprocessor_wrapper_gets_no_edge() {
    // `after` must stop at a parent it does not model
    let try_src = "int f(int x, int n) {\ntry {\nwhile (n) {\nx = 1;\nbreak;\n}\n} catch (...) {\nx = 2;\n}\nreturn x;\n}\n";
    let e = edges(try_src, Language::Cpp);
    assert!(!e.contains(&(5, 10)), "{e:?}");
    let ifdef =
        "int f(int x, int n) {\n#ifdef A\nwhile (n) {\nx = 1;\nbreak;\n}\n#endif\nreturn x;\n}\n";
    for l in [Language::C, Language::Cpp] {
        let e = edges(ifdef, l);
        assert!(!e.contains(&(5, 8)), "{l:?} {e:?}");
    }
    let attr = "int f(int x, int n) {\n[[gsl::suppress(\"a\")]] while (n) {\nx = 1;\nbreak;\n}\nreturn x;\n}\n";
    let e = edges(attr, Language::Cpp);
    assert!(!e.contains(&(4, 6)), "{e:?}");
}

#[test]
fn loop_at_the_end_of_an_else_arm_exits_past_the_if() {
    let s = "int f(int x, int n) {\nif (x) {\nx = 1;\n} else {\nwhile (n) {\nx = 2;\nbreak;\n}\n}\nreturn x;\n}\n";
    both(s, |s, l| {
        let e = edges(s, l);
        assert!(e.contains(&(7, 10)), "{l:?} {e:?}");
    });
}

#[test]
fn break_ending_a_range_for_body_returns_to_the_range_for_header() {
    let s = "int f(int x, int n) {\nfor (auto v : xs) {\nwhile (n) {\nx = v;\nbreak;\n}\n}\nreturn x;\n}\n";
    let e = edges(s, Language::Cpp);
    assert!(e.contains(&(5, 2)), "{e:?}");
    assert!(!e.contains(&(5, 8)), "{e:?}");
}

#[test]
fn dispatch_never_emits_a_self_edge() {
    let s = "int f(int x, int n) {\nswitch (n) { case 1: x = 1; }\nreturn x;\n}\n";
    both(s, |s, l| {
        let e = edges(s, l);
        assert!(e.contains(&(2, 3)), "{l:?} {e:?}");
        assert!(!e.contains(&(2, 2)), "{l:?} {e:?}");
    });
}

#[test]
fn nested_do_conditions_on_one_line_emit_no_self_edge() {
    let s = "int f(int x) {\ndo {\ndo { x = 1;\n} while (x); } while (x);\nreturn x;\n}\n";
    both(s, |s, l| {
        let e = edges(s, l);
        assert!(!e.contains(&(4, 4)), "{l:?} {e:?}");
    });
}
