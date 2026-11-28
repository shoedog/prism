//! I0 syntax inventory against byte expectations frozen from the Q01/Q03/Q05/Q12
//! programs of the T2 matrix (`tests/data/subscript_matrix/cases`). Every frozen
//! range is also checked against its text, so a wrong literal fails loudly.
use super::*;
use crate::ast::ParsedFile;
use crate::languages::Language;

macro_rules! case {
    ($name:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/data/subscript_matrix/cases/",
            $name,
            ".py"
        ))
    };
}

type W = (
    usize,
    usize,
    &'static str,
    Result<WriteKind, WriteRefusal>,
    &'static str,
);
type R = (
    usize,
    usize,
    &'static str,
    Result<Role, OperandRefusal>,
    &'static str,
);

const A: Result<WriteKind, WriteRefusal> = Ok(WriteKind::Assign);
const ARG: Result<Role, OperandRefusal> = Ok(Role::Argument);
const RHS: Result<Role, OperandRefusal> = Ok(Role::AssignmentRhs);

fn scan(src: &str) -> Vec<OwnerScan> {
    let parsed = ParsedFile::parse("t.py", src, Language::Python).unwrap();
    scan_file(&parsed)
}

fn path(p: &StaticPath, bracket: &str) -> String {
    let root = p.root.clone().unwrap_or_default();
    let mut s = root;
    for f in &p.fields {
        s = format!("{s}.{f}");
    }
    s + &bracket.repeat(p.brackets)
}

/// Compare one owner's inventory with frozen `(start, end, text, verdict, path)`
/// rows and the expected `(write index, operand index)` pairs.
fn check(src: &str, owner: &str, writes: &[W], operands: &[R], pairs: &[(usize, usize)]) {
    let scans = scan(src);
    let s = scans.iter().find(|s| s.name == owner).expect(owner);
    let (mut got_w, mut got_r) = (vec![], vec![]);
    for w in &s.writes {
        got_w.push((w.span.start, w.span.end, w.verdict, path(&w.path, "[]")));
    }
    for r in &s.operands {
        got_r.push((r.span.start, r.span.end, r.verdict, path(&r.path, "")));
    }
    let want_w: Vec<_> = writes
        .iter()
        .map(|w| (w.0, w.1, w.3, w.4.to_string()))
        .collect();
    let want_r: Vec<_> = operands
        .iter()
        .map(|r| (r.0, r.1, r.3, r.4.to_string()))
        .collect();
    assert_eq!(got_w, want_w, "writes of {owner}");
    assert_eq!(got_r, want_r, "operands of {owner}");
    assert_eq!(s.pairs(), pairs, "pairs of {owner}");
    for (a, b, text) in writes.iter().map(|w| (w.0, w.1, w.2)) {
        assert_eq!(&src[a..b], text);
    }
    for (a, b, text) in operands.iter().map(|r| (r.0, r.1, r.2)) {
        assert_eq!(&src[a..b], text);
    }
    for other in scans.iter().filter(|o| o.name != owner) {
        assert!(
            other.writes.is_empty() && other.operands.is_empty(),
            "{}",
            other.name
        );
    }
}

fn nth(src: &str, needle: &str, n: usize) -> (usize, usize) {
    let (i, _) = src.match_indices(needle).nth(n).expect(needle);
    (i, i + needle.len())
}

#[test]
fn element_syntax_recognizes_whole_operands() {
    let w = |s, e, t, p| -> W { (s, e, t, A, p) };
    let q = |s, e, t, v, p| -> R { (s, e, t, v, p) };
    let d = |s, e| q(s, e, "d", ARG, "d");
    let one = &[(0, 0)][..];
    check(
        case!("Q01.argument"),
        "case",
        &[w(50, 54, "d[k]", "d[]")],
        &[d(68, 69)],
        one,
    );
    let rhs = q(76, 77, "d", RHS, "d");
    check(
        case!("Q01.assignment"),
        "case",
        &[w(57, 63, "d[\"a\"]", "d[]")],
        &[rhs],
        one,
    );
    let fs = w(50, 64, "d[f\"slot-{k}\"]", "d[]");
    check(case!("Q01.fstring"), "case", &[fs], &[d(78, 79)], one);
    check(
        case!("Q01.integer"),
        "case",
        &[w(45, 49, "d[0]", "d[]")],
        &[d(63, 64)],
        one,
    );
    let ke = w(57, 72, "d[(key := \"a\")]", "d[]");
    check(case!("Q01.key-effect"), "case", &[ke], &[d(86, 87)], one);
    let kw = q(83, 84, "d", Ok(Role::Keyword), "d");
    check(
        case!("Q01.keyword"),
        "case",
        &[w(57, 63, "d[\"a\"]", "d[]")],
        &[kw],
        one,
    );
    let ri = w(57, 76, "d[(d := {}) or \"a\"]", "d[]");
    check(case!("Q01.rebind-index"), "case", &[ri], &[d(90, 91)], one);
    let rr = w(57, 63, "d[\"a\"]", "d[]");
    check(case!("Q01.rebind-rhs"), "case", &[rr], &[d(92, 93)], one);
    let ret = q(113, 114, "d", Ok(Role::Return), "d");
    check(
        case!("Q01.return"),
        "make",
        &[w(87, 93, "d[\"a\"]", "d[]")],
        &[ret],
        one,
    );
    // Transparent parentheses: the byte join uses the inner operand, `outer` keeps them.
    for (src, inner, outer) in [
        (case!("Q01.parenthesized"), (78, 79), "(d)"),
        (case!("Q03.parenthesized"), (78, 79), "( d )"),
    ] {
        let scans = scan(src);
        let o = &scans.iter().find(|s| s.name == "case").unwrap().operands[0];
        assert_eq!((o.span.start, o.span.end), inner);
        assert_eq!(&src[o.outer.start..o.outer.end], outer);
    }
    let p = q(78, 79, "d", RHS, "d");
    check(case!("Q03.parenthesized"), "case", &[rr], &[p], one);
    // Static prefix: the root and a selector prefix of the container match, a sibling does not.
    let ow = w(84, 97, "o.items[0][0]", "o.items[][]");
    let o1 = q(111, 112, "o", ARG, "o");
    check(case!("Q05.root"), "case", &[ow], &[o1], one);
    let o2 = q(111, 118, "o.items", ARG, "o.items");
    check(case!("Q05.prefix"), "case", &[ow], &[o2], one);
    let o3 = q(128, 129, "o", ARG, "o");
    for name in [case!("Q05.write-sibling"), case!("Q05.replace-prefix")] {
        check(name, "case", &[ow], &[o3], one);
    }
    let oo = q(111, 118, "o.other", ARG, "o.other");
    check(case!("Q05.other-prefix"), "case", &[ow], &[oo], &[]);
    let whole = q(79, 80, "d", RHS, "d");
    let proj = q(89, 90, "d", Err(OperandRefusal::Projected), "d");
    check(
        case!("Q03.projected-with-whole"),
        "case",
        &[rr],
        &[whole, proj],
        one,
    );
}

#[test]
fn element_syntax_refuses_wrappers_and_non_value_targets() {
    let ok = |s, e| -> W { (s, e, "d[\"a\"]", A, "d[]") };
    let refused = |s, e, t, r, p| -> W { (s, e, t, Err(r), p) };
    let q = |s, e, t, r, p| -> R { (s, e, t, Err(r), p) };
    use OperandRefusal as O;
    for (src, a, b, why) in [
        (case!("Q03.identity"), 63, 64, O::ExcludedCallee),
        (case!("Q03.length"), 64, 65, O::ExcludedCallee),
        (case!("Q03.shadowed-len"), 83, 84, O::ExcludedCallee),
        (case!("Q03.literal-leaf"), 61, 62, O::Wrapped),
        (case!("Q03.membership"), 67, 68, O::Wrapped),
        (case!("Q03.projected"), 60, 61, O::Projected),
        (case!("Q03.receiver"), 60, 61, O::Receiver),
    ] {
        check(src, "case", &[ok(41, 47)], &[q(a, b, "d", why, "d")], &[]);
    }
    let bad = refused(
        66,
        81,
        "o.items[0].name",
        WriteRefusal::DotAfterBracket,
        "o.items[]",
    );
    let sink = (95, 96, "o", ARG, "o");
    check(case!("Q05.field-after-index"), "case", &[bad], &[sink], &[]);
    let ow = (84, 97, "o.items[0][0]", A, "o.items[][]");
    let ir = q(110, 117, "o.items", O::Projected, "o.items");
    check(case!("Q05.indexed-read"), "case", &[ow], &[ir], &[]);
    let d = |s, e| -> R { (s, e, "d", ARG, "d") };
    let call = refused(57, 68, "make()[\"a\"]", WriteRefusal::NonNameRoot, "[]");
    check(case!("Q12.call-root"), "case", &[call], &[], &[]);
    use WriteRefusal as X;
    let w = refused(57, 63, "d[::2]", X::Slice, "d[]");
    check(case!("Q12.extended-slice"), "case", &[w], &[d(85, 86)], &[]);
    let w = refused(45, 50, "d[:1]", X::Slice, "d[]");
    check(case!("Q12.slice-write"), "case", &[w], &[d(66, 67)], &[]);
    let w = refused(57, 63, "d[\"a\"]", X::Unpack, "d[]");
    check(case!("Q12.unpack"), "case", &[w], &[d(87, 88)], &[]);
    let w = refused(61, 67, "d[\"a\"]", X::BindingTarget, "d[]");
    check(case!("Q12.loop-head"), "case", &[w], &[d(98, 99)], &[]);
    let w = (45, 49, "d[0]", A, "d[]");
    check(
        case!("Q12.slice-read"),
        "case",
        &[w],
        &[q(62, 63, "d", O::Slice, "d")],
        &[],
    );
    check(case!("Q12.initializer"), "case", &[], &[], &[]);
    inline_refusals();
}

/// Sources written for this test: spans come from needles, the verdicts are the claim.
fn inline_refusals() {
    let verdicts = |body: &str| {
        let src = format!("def f(d, t, c, e, p, x):\n{body}\n    sink(d)\n");
        let s = scan(&src).remove(0);
        let w: Vec<_> = s
            .writes
            .iter()
            .map(|w| (w.span.start, w.span.end, w.verdict))
            .collect();
        let r: Vec<_> = s
            .operands
            .iter()
            .map(|r| (r.span.start, r.span.end, r.verdict))
            .collect();
        (src, w, r)
    };
    let (src, w, _) = verdicts("    d[1, 2] = t");
    let at = nth(&src, "d[1, 2]", 0);
    assert_eq!(w, [(at.0, at.1, Err(WriteRefusal::MultiIndex))]);
    let (src, w, r) = verdicts("    del d[\"a\"]");
    assert_eq!(w[0].2, Err(WriteRefusal::Delete));
    assert_eq!(r.len(), 1, "only the sink operand: {src}");
    let (_, w, _) = verdicts("    d[\"a\"]: int");
    assert_eq!(w[0].2, Err(WriteRefusal::NoValue));
    let (_, w, _) = verdicts("    with p as d[\"a\"]:\n        pass");
    assert_eq!(w[0].2, Err(WriteRefusal::BindingTarget));
    let (_, w, _) = verdicts("    global d\n    d[\"a\"] = t");
    assert_eq!(w[0].2, Err(WriteRefusal::Redirected));
    let (_, w, _) = verdicts("    d[\"a\"] += t");
    assert_eq!(w[0].2, Ok(WriteKind::Augmented));
    let (_, w, _) = verdicts("    d[\"a\"]: int = t");
    assert_eq!(w[0].2, A);
    let (_, w, _) = verdicts("    d[\"a\"] = d[\"b\"] = t\n    d.x = 1");
    assert_eq!(w.iter().map(|w| w.2).collect::<Vec<_>>(), [A, A]);
    let (_, w, _) = verdicts("    if c:\n        while e:\n            d[\"a\"] = t");
    assert_eq!(w.len(), 1, "compound statements are scanned");
    // Operand positions of the root of an admitted store.
    let operand = |use_line: &str| {
        let (src, w, r) = verdicts(&format!("    d[\"a\"] = t\n    {use_line}"));
        assert_eq!(w.len(), 1);
        let (at, end) = nth(&src, use_line, 0);
        let r: Vec<_> = r.into_iter().filter(|r| r.0 >= at && r.1 <= end).collect();
        assert_eq!(r.len(), 1, "{use_line}: {r:?}");
        r[0].2
    };
    use OperandRefusal as O;
    assert_eq!(operand("f(*d)"), Err(O::Wrapped));
    assert_eq!(operand("f(**d)"), Err(O::Wrapped));
    assert_eq!(operand("f(d if c else e)"), Err(O::Wrapped));
    assert_eq!(operand("x = d + e"), Err(O::Wrapped));
    assert_eq!(operand("x += d"), Err(O::Wrapped));
    assert_eq!(operand("x = d, e"), Err(O::Wrapped));
    assert_eq!(operand("x = e[d]"), Err(O::Key));
    assert_eq!(operand("d()"), Err(O::Callee));
    assert_eq!(operand("d.clear()"), Err(O::Receiver));
    assert_eq!(operand("len(x=d)"), Err(O::ExcludedCallee));
    assert_eq!(operand("x = clear(d)"), Err(O::ExcludedCallee));
    assert_eq!(operand("return (d)"), Ok(Role::Return));
    assert_eq!(operand("x = y = d"), RHS);
    assert_eq!(operand("g(h(d))"), ARG);
}

#[test]
fn element_syntax_fences_owners() {
    // Lambda: the read is deferred, so it is recorded as fenced, not as an operand.
    let src = case!("Q12.deferred");
    let w = (57, 63, "d[\"a\"]", A, "d[]");
    let lam = (84, 85, "d", Err(OperandRefusal::Fenced), "d");
    check(src, "case", &[w], &[lam], &[]);
    let lines = [
        "d[\"x\"] = 1",
        "print(d)",
        "def outer(t):",
        "    d = {}",
        "    d[\"a\"] = t",
        "    def inner():",
        "        return d",
        "    k = [d for _ in t]",
        "    g = (d for _ in t)",
        "    class C:",
        "        x = d",
        "    return d",
    ];
    let src = &(lines.join("\n") + "\n");
    let scans = scan(src);
    let names: Vec<_> = scans.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["outer", "inner"], "module level is no owner");
    let outer = &scans[0];
    assert_eq!(outer.writes.len(), 1);
    let fenced = outer
        .operands
        .iter()
        .filter(|o| o.verdict == Err(OperandRefusal::Fenced));
    assert_eq!(
        fenced.count(),
        4,
        "inner, comprehension, generator, class body"
    );
    let last = outer.operands.last().unwrap();
    assert_eq!(last.verdict, Ok(Role::Return));
    assert_eq!(last.span.start, src.rfind("return d").unwrap() + 7);
    assert_eq!(outer.pairs().len(), 1, "only the unfenced return pairs");
    assert!(scans[1].writes.is_empty() && scans[1].operands.is_empty());
    // A store inside a nested function belongs to that owner alone.
    let src =
        "def outer(t):\n    def inner(u):\n        d[\"a\"] = u\n        return d\n    return t\n";
    let scans = scan(src);
    assert!(scans[0].writes.is_empty() && scans[0].operands.is_empty());
    assert_eq!(scans[1].name, "inner");
    assert_eq!(scans[1].pairs(), [(0, 0)]);
    // Decorated and async owners are the parsed owners; the owner span includes the decorator.
    let src = "@deco\nasync def a(t):\n    d = {}\n    d[\"k\"] = t\n    return d\n";
    let scans = scan(src);
    assert_eq!(
        (scans.len(), scans[0].name.as_str(), scans[0].span.line),
        (1, "a", 1)
    );
    assert_eq!(scans[0].pairs(), [(0, 0)]);
    // One scan per parsed owner, in the parser's order.
    let parsed = ParsedFile::parse("t.py", case!("Q01.keyword"), Language::Python).unwrap();
    assert_eq!(scan_file(&parsed).len(), parsed.all_functions().len());
}
