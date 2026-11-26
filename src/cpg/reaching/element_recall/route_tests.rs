//! I1 route phases against the Q06 programs of the T2 matrix. Expectations are frozen by hand
//! from the programs and cross-checked against the matrix's own `rows.jsonl` (`group`
//! `recall-py` = the marked pair is a recall positive, `boundary` = no edge expected), so the
//! route code never judges itself.
use super::route::Phase::*;
use super::route::*;
use super::*;
use crate::ast::ParsedFile;
use crate::languages::Language;
use serde_json::Value;
use std::collections::BTreeSet;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/subscript_matrix/");

#[derive(Clone, Copy, Debug)]
enum Exp {
    Hit,
    /// Every route died at a kill.
    Killed,
    Unreachable,
    /// R precedes W in bytes inside a shared loop: only another body entry could reach it.
    ReentryNotAfter,
    /// The route reaches the loop header and is refused there.
    Reentry,
    /// No exit of any kind reaches R.
    NoRoute,
    Unproven(Unsupported),
}
use Exp::*;

fn rows() -> Vec<Value> {
    let text = std::fs::read_to_string(format!("{DIR}rows.jsonl")).unwrap();
    text.lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn pair_of(row: &Value, key: &str) -> (usize, usize) {
    let b = &row[key];
    (
        b[0].as_u64().unwrap() as usize,
        b[1].as_u64().unwrap() as usize,
    )
}

struct Case {
    src: String,
    group: String,
    w: (usize, usize),
    route: Route,
}

fn run(name: &str) -> Case {
    let row = rows()
        .into_iter()
        .find(|r| r["id"] == format!("{name}.py"))
        .unwrap_or_else(|| panic!("no row {name}"));
    let src = std::fs::read_to_string(format!("{DIR}cases/{name}.py")).unwrap();
    let (w, r) = (pair_of(&row, "w_bytes"), pair_of(&row, "r_bytes"));
    let parsed = ParsedFile::parse("t.py", &src, Language::Python).unwrap();
    let mut found = None;
    for scan in scan_file(&parsed) {
        for ((wi, ri), route) in routes(&parsed, &scan) {
            let (ws, rs) = (scan.writes[wi].span, scan.operands[ri].span);
            if (ws.start, ws.end) == w && (rs.start, rs.end) == r {
                found = Some(route);
            }
        }
    }
    let route = found.unwrap_or_else(|| panic!("{name}: marked pair not admitted"));
    Case {
        src,
        group: row["group"].as_str().unwrap().to_string(),
        w,
        route,
    }
}

fn phases(route: &Route) -> Vec<Phase> {
    match route {
        Route::Reached(steps) => steps.iter().map(|s| s.phase).collect(),
        Route::Refused(_) => panic!("not reached: {route:?}"),
    }
}

fn check(name: &str, exp: Exp) -> Case {
    let case = run(name);
    let hit = matches!(exp, Hit);
    let oracle = case.group == "recall-py";
    assert_eq!(
        oracle, hit,
        "{name}: matrix group {} vs expectation",
        case.group
    );
    let refused = match (&case.route, exp) {
        (Route::Reached(_), Hit) => return case,
        (Route::Refused(r), e) if !matches!(e, Hit) => r,
        (route, e) => panic!("{name}: expected {e:?}, got {route:?}"),
    };
    let none = refused.unproven.is_empty();
    let ok = match exp {
        Hit => unreachable!(),
        Killed => refused.why == Why::NoRoute && refused.killed && none,
        Unreachable => refused.why == Why::UnreachableW,
        ReentryNotAfter => refused.why == Why::NotAfter && refused.reentry_cut,
        Reentry => refused.why == Why::NoRoute && refused.reentry_cut && !refused.killed && none,
        NoRoute => refused.why == Why::NoRoute && !refused.reentry_cut && !refused.killed && none,
        Unproven(k) => refused.why == Why::NoRoute && refused.unproven.iter().all(|u| u.0 == k),
    };
    assert!(
        ok && (!matches!(exp, Unproven(_)) || !none),
        "{name}: expected {exp:?}, got {refused:?}"
    );
    case
}

#[test]
fn element_preview_respects_branch_rebind_and_loop_order() {
    let mut table = vec![
        ("branch-join", Hit),
        ("correlated", Hit),
        ("elif-preserve", Hit),
        ("elif-replace", Killed),
        ("root-replace", Killed),
        ("read-before-rebind", Hit),
        ("read-after-rebind", Killed),
        ("list-iadd", Killed),
        ("dict-ior", Killed),
        ("prefix-iadd", Killed),
        ("unrelated-compound", Hit),
        ("early-return", Unreachable),
        ("exclusive", Unreachable),
        ("loop-target-replace", Killed),
        ("loop-target-skipped", Hit),
        ("loop-iterable-replace", Killed),
        ("loop-test-replace", Killed),
    ];
    for kind in ["range", "iterator", "while"] {
        table.push((leak(format!("loop-{kind}-skip-suffix")), Hit));
    }
    for depth in [1, 2] {
        for (verb, exp) in [("preserve", Hit), ("replace", Killed)] {
            table.push((leak(format!("compound-branch-{depth}-{verb}")), exp));
            for (kind, part) in [
                ("range", "body"),
                ("range", "else"),
                ("while", "body"),
                ("while", "else"),
            ] {
                table.push((leak(format!("compound-{kind}-{part}-{depth}-{verb}")), exp));
            }
        }
    }
    for (name, exp) in table {
        check(&format!("Q06.{name}"), exp);
    }
    // the surviving arm's phases, frozen from the program text
    let join = check("Q06.branch-join", Hit).route;
    assert_eq!(phases(&join), [Join, Read]);
    let elif = check("Q06.elif-preserve", Hit).route;
    assert_eq!(phases(&elif), [Test, Test, Arm(1), Join, Read]);
    let skip = check("Q06.loop-range-skip-suffix", Hit).route;
    assert_eq!(
        phases(&skip),
        [Continue, Resolve, Advance, Exhaust, Exit, Read]
    );
    // a kill that may not run keeps the route; one that always runs ends it
    let inline = |between: &str| -> Route {
        let src =
            format!("def case(t, c):\n    d = {{}}\n    d['a'] = t\n    {between}\n    sink(d)\n");
        let parsed = ParsedFile::parse("t.py", &src, Language::Python).unwrap();
        let scans = scan_file(&parsed);
        let mut all = routes(&parsed, scans.iter().find(|s| s.name == "case").unwrap());
        assert_eq!(all.len(), 1, "{src}");
        all.remove(0).1
    };
    for (between, reached) in [
        ("x = (d := {}) if c else None", true),
        ("x = None if c else (d := {})", true),
        ("x = (d := {}) if (d := t) else None", false),
        ("x = c and (d := {})", true),
        ("x = c or (d := {})", true),
        ("x = (d := {}) and c", false),
        ("x = [(d := {}) for _ in t]", true),
        ("x = lambda: (d := {})", true),
    ] {
        assert_eq!(inline(between).is_reached(), reached, "{between}");
    }
    // literal-true tests enter one arm, and each nesting level joins
    let nested = check("Q06.compound-branch-2-preserve", Hit).route;
    assert_eq!(
        phases(&nested),
        [Test, Arm(0), Test, Arm(0), Join, Join, Join, Read]
    );
}

/// Routes of the single pair of `def case(t, c)`: a store, `between`, then a whole read.
fn between_reached(between: &str) -> bool {
    let src =
        format!("def case(t, c):\n    d = {{}}\n    d['a'] = t\n    {between}\n    sink(d)\n");
    let parsed = ParsedFile::parse("t.py", &src, Language::Python).unwrap();
    let scans = scan_file(&parsed);
    let all = routes(&parsed, scans.iter().find(|s| s.name == "case").unwrap());
    assert_eq!(all.len(), 1, "{src}");
    all[0].1.is_reached()
}

#[test]
fn element_preview_follows_literal_boolean_and_ternary_operands() {
    // a literal left operand decides which operand runs: the taken one is unconditional, so a
    // clear or rebind in it ends the route (`d` is `{}` afterwards); the other never runs
    for (between, reached) in [
        // taken operand runs: the kill is certain
        ("True and d.clear()", false),
        ("1 and d.clear()", false),
        ("'x' and d.clear()", false),
        ("(True) and d.clear()", false),
        ("not False and d.clear()", false),
        ("False or d.clear()", false),
        ("0 or d.clear()", false),
        ("None or d.clear()", false),
        ("'' or d.clear()", false),
        ("True and (d := {})", false),
        ("False or (d := {})", false),
        ("d.clear() if True else None", false),
        ("None if False else d.clear()", false),
        ("(d := {}) if 1 else None", false),
        // the operand that never runs keeps the route (controls)
        ("False and d.clear()", true),
        ("0 and d.clear()", true),
        ("None and d.clear()", true),
        ("'' and d.clear()", true),
        ("not True and d.clear()", true),
        ("True or d.clear()", true),
        ("1 or d.clear()", true),
        ("'x' or d.clear()", true),
        ("False and (d := {})", true),
        ("True or (d := {})", true),
        ("None if True else d.clear()", true),
        ("d.clear() if False else None", true),
        ("None if 1 else (d := {})", true),
        // non-literal operands stay deferred, as before
        ("c and d.clear()", true),
        ("c or d.clear()", true),
        ("d.clear() if c else None", true),
        ("(c and True) and d.clear()", true),
        // a literal on the right, or a call as the test, decides nothing
        ("c and True and d.clear()", true),
        ("d.clear() if t else None", true),
    ] {
        assert_eq!(between_reached(between), reached, "{between}");
    }
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

#[test]
fn element_preview_exits_loop_without_reentering_body() {
    let mut table = vec![
        ("loop-nested-jump", Hit),
        ("reentry-immediate-break", Hit),
        ("reentry-checked-break", Reentry),
        ("reentry-exclusive", ReentryNotAfter),
        ("loop-no-exit", NoRoute),
    ];
    for kind in ["range", "iterator", "while"] {
        table.push((leak(format!("loop-{kind}-same-iteration")), Hit));
        table.push((leak(format!("loop-{kind}-exhaustion")), Hit));
        table.push((leak(format!("loop-{kind}-break-else-replace")), Hit));
        table.push((leak(format!("loop-{kind}-continue-else-replace")), Killed));
    }
    for (name, exp) in table {
        check(&format!("Q06.{name}"), exp);
    }
    let same = check("Q06.loop-range-same-iteration", Hit).route;
    assert_eq!(phases(&same), [Read]);
    let exhaust = check("Q06.loop-range-exhaustion", Hit).route;
    assert_eq!(phases(&exhaust), [Advance, Exhaust, Exit, Read]);
    let exhaust = check("Q06.loop-while-exhaustion", Hit).route;
    assert_eq!(phases(&exhaust), [Test, Exhaust, Exit, Read]);
    // a break skips the loop else, even one that would replace the root
    let brk = check("Q06.loop-while-break-else-replace", Hit).route;
    assert_eq!(phases(&brk), [Break, Resolve, Read]);
    // the refused re-entry is recorded with the loop it would enter
    let case = run("Q06.reentry-checked-break");
    let Route::Refused(r) = case.route else {
        panic!()
    };
    assert!(r.reentry_cut && r.why == Why::NoRoute);
}

#[test]
fn element_preview_refuses_unsupported_completion() {
    let table = [
        ("try-else", Unproven(Unsupported::Try)),
        ("try-else-finally", Unproven(Unsupported::Try)),
        ("try-handled", Unproven(Unsupported::Try)),
        ("try-handled-finally", Unproven(Unsupported::Try)),
        ("unsupported-finally", Unproven(Unsupported::Try)),
        ("with", Unproven(Unsupported::With)),
        ("async-with", Unproven(Unsupported::With)),
        ("match-guarded", Unproven(Unsupported::Match)),
        ("match-unmatched", Unproven(Unsupported::Match)),
        ("unsupported-alternate", Hit),
    ];
    for (name, exp) in table {
        check(&format!("Q06.{name}"), exp);
    }
    // an unsupported construct charges its own kind and span
    let case = run("Q06.try-else");
    let Route::Refused(r) = &case.route else {
        panic!()
    };
    let (kind, at) = r.unproven[0];
    assert_eq!(kind, Unsupported::Try);
    assert_eq!(&case.src[at.start..at.start + 3], "try");
    // inline programs: W and R inside one try body need no completion; W inside it with R after,
    // W before an `async for`, and a `try` after W inside a loop body
    let inline = |body: &str| -> Route {
        let src = format!("def case(t, xs):\n    d = {{}}\n{body}");
        let parsed = ParsedFile::parse("t.py", &src, Language::Python).unwrap();
        let scans = scan_file(&parsed);
        let scan = scans.iter().find(|s| s.name == "case").unwrap();
        let mut all = routes(&parsed, scan);
        assert_eq!(all.len(), 1, "{src}");
        all.remove(0).1
    };
    let same_try = "    try:\n        d['a'] = t\n        sink(d)\n    finally:\n        pass\n";
    assert!(inline(same_try).is_reached());
    let after_try =
        "    try:\n        d['a'] = t\n    except ValueError:\n        pass\n    sink(d)\n";
    let kinds = |r: Route| match r {
        Route::Refused(r) => r.unproven.iter().map(|u| u.0).collect::<BTreeSet<_>>(),
        Route::Reached(_) => panic!("reached"),
    };
    assert_eq!(kinds(inline(after_try)), BTreeSet::from([Unsupported::Try]));
    let in_with = "    with open(t) as f:\n        d['a'] = t\n    sink(d)\n";
    assert_eq!(kinds(inline(in_with)), BTreeSet::from([Unsupported::With]));
    let async_for = "    d['a'] = t\n    async for x in xs:\n        pass\n    sink(d)\n";
    let src = format!("async def case(t, xs):\n    d = {{}}\n{async_for}");
    let parsed = ParsedFile::parse("t.py", &src, Language::Python).unwrap();
    let scans = scan_file(&parsed);
    let route = routes(&parsed, &scans[0]).remove(0).1;
    assert_eq!(kinds(route), BTreeSet::from([Unsupported::AsyncFor]));
    let loop_try = "    for x in xs:\n        d['a'] = t\n        try:\n            pass\n        finally:\n            pass\n    sink(d)\n";
    assert_eq!(kinds(inline(loop_try)), BTreeSet::from([Unsupported::Try]));
}

/// 1-indexed line of a byte offset.
fn line_of(src: &str, at: usize) -> usize {
    src[..at].matches('\n').count() + 1
}

/// Lines reachable from `from` along public CFG edges.
fn reach(edges: &[(usize, usize)], from: usize) -> BTreeSet<usize> {
    let mut seen = BTreeSet::from([from]);
    let mut work = vec![from];
    while let Some(l) = work.pop() {
        for (_, to) in edges.iter().filter(|(f, _)| *f == l) {
            if seen.insert(*to) {
                work.push(*to);
            }
        }
    }
    seen
}

/// Q06 rows that cross a loop jump with the store before it: the syntax walk certifies the
/// route, while the public CFG has no transition out of the jump line, so its reachability from
/// W's line never gets to R's line. These are graph-eligible, public-CFG-stopped rows.
#[test]
fn element_preview_pins_graph_eligible_public_cfg_stopped_q06() {
    let mut count = 0;
    for kind in ["range", "iterator", "while"] {
        for jump in ["break", "continue"] {
            for els in ["", "-else"] {
                for place in ["body", "pre-loop"] {
                    let name = format!("Q06.cfg-{kind}-{jump}{els}-{place}");
                    let case = check(&name, Hit);
                    let want = match (jump, kind == "while", els) {
                        ("break", _, _) => vec![Break, Resolve, Read],
                        ("continue", false, "") => {
                            vec![Continue, Resolve, Advance, Exhaust, Exit, Read]
                        }
                        ("continue", false, _) => {
                            vec![Continue, Resolve, Advance, Exhaust, LoopElse, Read]
                        }
                        (_, _, "") => vec![Continue, Resolve, Test, Exhaust, Exit, Read],
                        _ => vec![Continue, Resolve, Test, Exhaust, LoopElse, Read],
                    };
                    assert_eq!(phases(&case.route), want, "{name}");
                    let parsed = ParsedFile::parse("t.py", &case.src, Language::Python).unwrap();
                    let edges: Vec<_> = crate::cfg::build_cfg_edges(&parsed)
                        .into_iter()
                        .map(|e| (e.from_line, e.to_line))
                        .collect();
                    let row = rows()
                        .into_iter()
                        .find(|r| r["id"] == format!("{name}.py"))
                        .unwrap();
                    let (w_line, r_line) = (
                        line_of(&case.src, case.w.0),
                        line_of(&case.src, pair_of(&row, "r_bytes").0),
                    );
                    let jump_line = case.src.lines().position(|l| l.trim() == jump).unwrap() + 1;
                    assert!(jump_line > w_line && r_line > jump_line, "{name}");
                    // the missing public transition: nothing leaves the jump line
                    assert!(
                        edges.iter().all(|(f, _)| *f != jump_line),
                        "{name}: {edges:?}"
                    );
                    assert!(
                        !reach(&edges, w_line).contains(&r_line),
                        "{name}: {edges:?}"
                    );
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, 24);
    // control: the same routes inside one body iteration stay public-CFG reachable
    let case = check("Q06.loop-range-same-iteration", Hit);
    let parsed = ParsedFile::parse("t.py", &case.src, Language::Python).unwrap();
    let edges: Vec<_> = crate::cfg::build_cfg_edges(&parsed)
        .into_iter()
        .map(|e| (e.from_line, e.to_line))
        .collect();
    let w_line = line_of(&case.src, case.w.0);
    assert!(reach(&edges, w_line).contains(&(w_line + 1)));
}
