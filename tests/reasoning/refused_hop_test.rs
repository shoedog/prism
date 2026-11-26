use std::fmt::Write;

use super::taint_reaches_test::fixture;
use prism::navigation::types::{Evidence, Reachability, ReasoningWarning, SymbolRef, WarningKind};
use prism::reasoning::seeds::SeedSpec;
use prism::reasoning::taint_reaches::taint_reaches;

fn line(source: &str, text: &str) -> usize {
    source[..source.find(text).expect("fixture marker")]
        .bytes()
        .filter(|&byte| byte == b'\n')
        .count()
        + 1
}

fn query(file: &str, source: &str, roots: &[&str], sink: &str) -> Evidence {
    let fixture = fixture(&[(file, source)]);
    let seed = |text| SeedSpec::Loc {
        file: file.into(),
        line: line(source, text),
    };
    let roots: Vec<_> = roots.iter().map(|root| seed(root)).collect();
    taint_reaches(&fixture.session, &roots, Some(&[seed(sink)])).unwrap()
}

fn assert_distal_miss(file: &str, source: &str, root: &str, sink: &str) {
    let evidence = query(file, source, &[root], sink);
    let summary = evidence.reasoning.unwrap();
    assert!(!summary.per_sink.is_empty(), "missing operand at {sink}");
    assert_eq!(summary.reachability, Some(Reachability::NotReached));
    for operand in &summary.per_sink {
        assert_eq!(operand.reachability, Reachability::NotReached);
        assert!(!operand.sources.is_empty());
        for pair in &operand.sources {
            assert_eq!(pair.reachability, Reachability::NotReached);
        }
    }
}

#[test]
fn same_line_assignment_keeps_parent_verdict() {
    let original = include_str!("../taint_parity/f01/a.js")
        .replace("  d(z);", "  observe(z); const q = 'fixed';\n  d(q);");
    let severed = original.replace("observe(z)", "observe('fixed')");
    for source in [&original, &severed] {
        assert_distal_miss("a.js", source, "const t =", "exec(w)");
    }
}

#[test]
fn discarded_return_input_keeps_parent_verdict() {
    let original = include_str!("../taint_parity/s3-js-refused-ascent-one-more/a.js")
        .replace("return x;", "return (x, 'fixed');");
    let severed = original.replace("(x, 'fixed')", "('other', 'fixed')");
    for source in [&original, &severed] {
        assert_distal_miss("a.js", source, "const t =", "exec(x)");
    }
}

#[test]
fn literal_member_pairing_keeps_parent_verdict() {
    let mut original = include_str!("../taint_parity/s2-ts-literal-dynamic-sibling/a.ts")
        .replace("exec(p[k]);", "tail(p[k]);");
    original.push_str("\nfunction tail(x: string) { exec(x); }\n");
    let severed = original.replace("{cmd: t, safe:", "{cmd: '', safe:");
    for source in [&original, &severed] {
        assert_distal_miss("a.ts", source, "const t =", "exec(x)");
    }
}

#[test]
fn dispatch_keeps_parent_verdict() {
    let mut original = include_str!("../taint_parity/s2-js-mixed-bound-alias/a.js")
        .replace("exec(x);", "tail(x);");
    original.push_str("\nfunction tail(x) { exec(x); }\n");
    let severed = original.replace("other.run(t)", "other.run('other')");
    for source in [&original, &severed] {
        assert_distal_miss("a.js", source, "const t =", "exec(x)");
    }
}

#[test]
fn option_b_distal_fanout_stays_deferred_in_both_definition_orders() {
    for callees in [
        (0..130).collect::<Vec<_>>(),
        [129].into_iter().chain(0..129).collect(),
    ] {
        let mut source = String::from(
            "function entry() { const t = acquire(); h1(t); }\n\
             function h1(x) { h2(x); }\nfunction h2(y) {\n",
        );
        for callee in 0..130 {
            writeln!(source, "  g{callee}(y);").unwrap();
        }
        source.push_str("}\n");
        for callee in callees {
            let body = if callee == 129 {
                "tail(p);"
            } else {
                "observe(p);"
            };
            writeln!(source, "function g{callee}(p) {{ {body} }}").unwrap();
        }
        source.push_str("function tail(q) {\n  exec(q);\n}\n");
        assert_distal_miss("a.js", &source, "const t =", "exec(q)");
    }
}

#[test]
fn option_b_distal_sink_stays_deferred_with_competing_routes() {
    let source = r#"function entry() { const t = acquire(); h1(t); }
function h1(x) { h2(x); }
function h2(y) {
  short(y);
  long(y);
}
function short(s) {
  tail({cmd: s});
}
function long(p) {
  relay(p);
}
function relay(p) {
  tail(p);
}
function tail(p) {
  exec(p.cmd);
}"#;
    assert_distal_miss("a.js", source, "const t =", "exec(p.cmd)");
}

#[test]
fn shared_boundary_keeps_each_roots_reason_on_the_wire() {
    let source = r#"function entry() {
  const t = acquire();
  h1(t);
  const s = acquire();
  g(s) + g(1);
}
function h1(x) { h2(x); }
function h2(y) { g(y); }
function g(p) {
  exec(p);
}
function unrelated(q) {
  exec(q);
}"#;
    for roots in [["const t", "const s"], ["const s", "const t"]] {
        let evidence = query("a.js", source, &roots, "exec(p)");
        let warnings: Vec<_> = evidence
            .warnings
            .iter()
            .filter(|warning| {
                matches!(
                    warning.kind,
                    WarningKind::Reasoning(ReasoningWarning::InterproceduralBoundary { .. })
                )
            })
            .collect();
        assert_eq!(warnings.len(), 1);
        assert_eq!(
            warnings[0].location.as_ref().unwrap().start_line,
            line(source, "function g(p)")
        );
        let summary = evidence.reasoning.unwrap();
        assert_eq!(summary.reachability, Some(Reachability::BoundaryExited));
        let operand = summary
            .per_sink
            .iter()
            .find(
                |operand| matches!(&operand.sink, SymbolRef::Variable { path, .. } if path == "p"),
            )
            .expect("p operand");
        assert_eq!(operand.sources.len(), 2);
        for pair in &operand.sources {
            let SymbolRef::Variable { path, .. } = &pair.source else {
                panic!("expected a variable root");
            };
            let expected = match path.as_str() {
                "t" => "depth_cap",
                "s" => "ambiguous_site",
                _ => panic!("unexpected root {path}"),
            };
            assert_eq!(pair.reachability, Reachability::BoundaryExited);
            assert_eq!(serde_json::to_value(pair).unwrap()["reason"], expected);
        }
        assert_distal_miss("a.js", source, "const t", "exec(q)");
        assert_distal_miss("a.js", source, "const s", "exec(q)");
    }
}
