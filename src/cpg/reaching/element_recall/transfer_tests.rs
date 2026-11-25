//! I1 binding and storage transfer (lane 10, #847) against the Q01/Q02/Q05-Q08/Q13 programs of
//! the T2 matrix. Expectations are frozen by hand from the program text; `rows.jsonl` (`group`
//! `recall-py` = a recall positive) cross-checks the rows whose only discriminator is the transfer.
use super::route::{Route, Why};
use super::*;
use crate::ast::ParsedFile;
use crate::languages::Language;
use serde_json::Value;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/subscript_matrix/");

#[derive(Clone, Copy, Debug, PartialEq)]
enum Exp {
    Hit,
    /// Every route died at a kill (key-blind delete/clear, root or prefix replacement).
    Killed,
    /// The store's receiver was captured, then its binding or a captured link was replaced.
    Capture,
    /// The syntax inventory admits no (W, R) pair for the marked bytes.
    NoPair,
    Try,
    /// Any other refusal, kept apart so an assertion names the program.
    Other(Why),
}
use Exp::*;

fn rows() -> Vec<Value> {
    let text = std::fs::read_to_string(format!("{DIR}rows.jsonl")).unwrap();
    text.lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn bytes(row: &Value, key: &str) -> (usize, usize) {
    let b = &row[key];
    (
        b[0].as_u64().unwrap() as usize,
        b[1].as_u64().unwrap() as usize,
    )
}

/// Routes of every admitted pair of every owner, with W and R bytes.
fn all_routes(src: &str) -> Vec<((Span, Span), Route)> {
    let parsed = ParsedFile::parse("t.py", src, Language::Python).unwrap();
    let mut out = Vec::new();
    for scan in scan_file(&parsed) {
        for ((w, r), route) in route::routes(&parsed, &scan) {
            out.push(((scan.writes[w].span, scan.operands[r].span), route));
        }
    }
    out
}

fn verdict(route: &Route) -> Exp {
    match route {
        Route::Reached(_) => Hit,
        Route::Refused(r) if r.why == Why::Capture => Capture,
        Route::Refused(r) if r.unproven.len() == 1 && r.why == Why::NoRoute => Try,
        Route::Refused(r) if r.why == Why::NoRoute && r.killed => Killed,
        Route::Refused(r) => Other(r.why),
    }
}

/// The marked pair of a matrix row, with `oracle` = whether the row says "recall positive".
fn marked(name: &str) -> (Option<Exp>, bool) {
    let row = rows()
        .into_iter()
        .find(|r| r["id"] == format!("{name}.py"))
        .unwrap_or_else(|| panic!("no row {name}"));
    let src = std::fs::read_to_string(format!("{DIR}cases/{name}.py")).unwrap();
    let (w, r) = (bytes(&row, "w_bytes"), bytes(&row, "r_bytes"));
    let found = all_routes(&src)
        .into_iter()
        .find(|((ws, rs), _)| (ws.start, ws.end) == w && (rs.start, rs.end) == r)
        .map(|(_, route)| verdict(&route));
    (found, row["group"] == "recall-py")
}

/// Check a table of rows; `oracle` rows must also agree with the matrix's own group.
fn table(rows: &[(&str, Exp)], oracle: bool) {
    for (name, exp) in rows {
        let (got, recall) = marked(name);
        let got = got.unwrap_or(NoPair);
        assert_eq!(got, *exp, "{name}");
        if oracle {
            assert_eq!(recall, *exp == Hit, "{name}: matrix group vs expectation");
        }
    }
}

/// Routes of the pairs of owner `case` in an inline program, in source order of W then R.
fn inline(body: &str) -> Vec<Exp> {
    let src = format!("def case(t, c, take, sink):\n{body}");
    // an operand that precedes its store (the statement's own right side) is no candidate
    let found = all_routes(&src);
    let live = found
        .iter()
        .filter(|(_, r)| !matches!(verdict(r), Other(Why::NotAfter)));
    live.map(|(_, r)| verdict(r)).collect()
}

fn one(body: &str) -> Exp {
    let v = inline(body);
    assert_eq!(v.len(), 1, "{body}: {v:?}");
    v[0]
}

#[test]
fn element_preview_refuses_rebind_after_receiver_capture() {
    // receiver captured first, then the index rebinds the root or detaches a captured child
    table(
        &[
            ("Q01.rebind-index", Capture),
            ("Q13.detach-child", Capture),
            // a rebind before capture starts a new lifetime, a clear of a still-bound root
            // before a one-level store detaches nothing
            ("Q01.rebind-rhs", Hit),
            ("Q13.clear-before-commit", Hit),
            ("Q01.key-effect", Hit),
        ],
        true,
    );
    // the refusal is its own reason: no kill, no unsupported completion
    let (found, _) = marked("Q01.rebind-index");
    assert_eq!(found, Some(Capture));
    for body in [
        // augmented: target, then RHS, then store
        "    d = {}\n    d['a'] += (d := {})\n    sink(d)\n",
        // a nested store holds the child `d[0]` while the second index clears `d`
        "    d = {0: {}}\n    d[0][(d.pop(0), 0)[1]] = t\n    sink(d)\n",
        "    d = {0: {}}\n    d[0][(d.update({}), 0)[1]] = t\n    sink(d)\n",
        // fields: `o.items` is loaded before the index, clearing `o` drops that link
        "    o = take()\n    o.items[(o.clear(), 0)[1]] = t\n    sink(o)\n",
        // the augmented RHS also runs after the whole receiver is captured
        "    o = take()\n    o.items[0][1] += (o.clear(), 1)[1]\n    sink(o)\n",
    ] {
        assert_eq!(one(body), Capture, "{body}");
    }
    for body in [
        // chained assignment: the left target rebinds the root before this store captures it
        "    d = {}\n    d = d['a'] = t\n    sink(d)\n",
        // plain assignment evaluates the RHS before the target
        "    d = {}\n    d['a'] = (d := {})\n    sink(d)\n",
        "    d = {}\n    d['a'] = (d.clear(), t)[1]\n    sink(d)\n",
        // the receiver itself may be cleared by its own index
        "    o = take()\n    o.items[(o.items.clear(), 0)[1]] = t\n    sink(o)\n",
        "    d = {0: {}}\n    d[(d.clear(), 0)[1]][1] = t\n    sink(d)\n",
        // a rebind that may not run, in a deferred arm or a comprehension, detaches nothing certain
        "    d = {}\n    d[(d := {}) if c else 'a'] = t\n    sink(d)\n",
        "    d = {}\n    d[[(d := {}) for _ in t][0]] = t\n    sink(d)\n",
        "    d = {}\n    d[c and (d := {})] = t\n    sink(d)\n",
        // another name, and a store into a sibling field
        "    d = {}\n    e = {}\n    d[(e := {}) or 'a'] = t\n    sink(d)\n",
        "    o = take()\n    o.items[(o.other.clear(), 0)[1]] = t\n    sink(o)\n",
    ] {
        assert_eq!(one(body), Hit, "{body}");
    }
    // a chained later target replacing the root kills the store just made
    assert_eq!(one("    d = {}\n    d['a'] = d = t\n    sink(d)\n"), Killed);
}

#[test]
fn element_preview_keeps_earlier_sibling_and_cuts_delete() {
    // other indexed stores generate and never kill, even for the same slot (key-blind)
    table(
        &[
            ("Q13.other-slot", Hit),
            ("Q13.same-slot", Hit),
            ("Q13.replace-indexed-ancestor", Hit),
            ("Q13.delete-same", Killed),
            ("Q13.delete-sibling", Killed),
            ("Q13.clear", Killed),
            ("Q05.root", Hit),
            ("Q05.prefix", Hit),
            ("Q05.write-sibling", Hit),
            ("Q05.replace-prefix", Killed),
            ("Q08.unaffected-sibling", Hit),
            ("Q08.alias-overwrite", Hit),
            ("Q08.alias-after", Hit),
            ("Q08.mutator", Hit),
            ("Q08.item-override", Hit),
        ],
        true,
    );
    table(
        &[
            ("Q05.other-prefix", NoPair),
            ("Q05.indexed-read", NoPair),
            ("Q05.field-after-index", NoPair),
            ("Q08.alias-before", NoPair),
            ("Q08.name-as-key", NoPair),
        ],
        false,
    );
    // Q02: both earlier stores of one container reach the whole read, the overwritten slot too;
    // a store into another name never does
    for name in [
        "Q02.both",
        "Q02.replace-a",
        "Q02.N01-mixed",
        "Q02.O01-mixed",
    ] {
        let src = std::fs::read_to_string(format!("{DIR}cases/{name}.py")).unwrap();
        let found = all_routes(&src);
        assert!(found.len() >= 2, "{name}");
        assert!(found.iter().all(|(_, r)| r.is_reached()), "{name}");
    }
    let src = std::fs::read_to_string(format!("{DIR}cases/Q02.neither.py")).unwrap();
    assert!(all_routes(&src).is_empty());
    let src = std::fs::read_to_string(format!("{DIR}cases/Q02.only-a.py")).unwrap();
    assert_eq!(all_routes(&src).len(), 1);
    // both siblings survive a third store and a sibling field write
    let v = inline("    d = {}\n    d['a'] = t\n    d['b'] = t\n    d.x = 1\n    sink(d)\n");
    assert_eq!(v, [Hit, Hit]);
    // key-blind cuts: the prefix before the first bracket is the affected subtree
    for (between, exp) in [
        ("del d['z']", Killed),
        ("del d", Killed),
        ("del (d['z'])", Killed),
        ("del e, d[0]", Killed),
        ("d.clear()", Killed),
        ("d[1][2].clear()", Killed),
        ("x = [d.clear()]", Killed),
        ("take(d.clear())", Killed),
        // unrelated names, other methods and a deferred or conditional cut
        ("del e['z']", Hit),
        ("e.clear()", Hit),
        ("d.get(1)", Hit),
        ("d.pop(1)", Hit),
        ("x = None if c else d.clear()", Hit),
        ("x = c and d.clear()", Hit),
        ("x = lambda: d.clear()", Hit),
        ("if c:\n        d.clear()", Hit),
        (
            "if c:\n        del d['z']\n    else:\n        del d['y']",
            Killed,
        ),
    ] {
        let body =
            format!("    d = {{}}\n    e = {{}}\n    d['a'] = t\n    {between}\n    sink(d)\n");
        // the `else` row above deletes on both arms
        assert_eq!(one(&body), exp, "{between}");
    }
    // field prefixes: clearing or deleting a prefix of W's storage cuts, a sibling field does not
    for (between, exp) in [
        ("o.items.clear()", Killed),
        ("o.items[3].clear()", Killed),
        ("del o.items[3]", Killed),
        ("del o.items", Killed),
        ("o.clear()", Killed),
        ("del o['k']", Killed),
        ("o.other.clear()", Hit),
        ("del o.other[3]", Hit),
        ("del o.items.sub", Hit),
        ("o.items.sub.clear()", Hit),
    ] {
        let body = format!("    o = take()\n    o.items[0][1] = t\n    {between}\n    sink(o)\n");
        assert_eq!(one(&body), exp, "{between}");
    }
    // the cut and the read share one statement: the read comes first, or it does not
    assert_eq!(
        one("    d = {}\n    d['a'] = t\n    take(d, d.clear())\n"),
        Hit
    );
    assert_eq!(
        one("    d = {}\n    d['a'] = t\n    take(d.clear(), d)\n"),
        Killed
    );
}

#[test]
fn element_preview_reads_before_assignment_rebind() {
    table(
        &[
            ("Q06.read-before-rebind", Hit),
            ("Q06.read-after-rebind", Killed),
            ("Q06.root-replace", Killed),
        ],
        true,
    );
    let prog = |stmt: &str| format!("    d = {{}}\n    d['a'] = t\n    {stmt}\n");
    for (stmt, exp) in [
        // the target is bound after the RHS has been evaluated
        ("d = take(d)", Hit),
        ("d = d", Hit),
        ("d += take(d)", Hit),
        ("d, e = take(d), 1", Hit),
        ("d = e = take(d)", Hit),
        // eager siblings run in operand order
        ("take(d, (d := {}))", Hit),
        ("take((d := {}), d)", Killed),
        ("take(d.clear(), d)", Killed),
        ("take(d, d.clear())", Hit),
        ("(d := {}) and take(d)", Killed),
        ("c and (d := {}) and take(d)", Hit),
        ("take((d := take(d)))", Hit),
        ("take(key=d, other=(d := {}))", Hit),
        // the target's own index runs after the RHS read; a condition runs before its arms
        ("d[(d := {})] = d", Hit),
        ("x = take(d) if (d := {}) else None", Killed),
    ] {
        let found = inline(&prog(stmt));
        let hit = found.first().is_some_and(|e| *e == Hit);
        assert_eq!(hit, exp == Hit, "{stmt}: {found:?}");
        if exp == Killed {
            assert!(found.is_empty() || found[0] == Killed, "{stmt}: {found:?}");
        }
    }
}

#[test]
fn element_preview_requires_independent_local_binding() {
    let prog = |head: &str| format!("{head}\n    d['a'] = t\n    sink(d)\n");
    // a bare local or formal: any binding construct of the owner's own scope
    for head in [
        "    d = {}",
        "    d: dict = {}",
        "    d: dict",
        "    d += take()",
        "    (d := {})",
        "    for d in take(): pass",
        "    with take() as d: pass",
        "    try:\n        pass\n    except ValueError as d:\n        pass",
        "    import d",
        "    import x.y as d",
        "    from m import d",
        "    del d",
        "    def d(): pass",
        "    class d: pass",
        "    x, (d, *y) = take()",
        "    if c:\n        d = {}",
    ] {
        assert_eq!(one(&prog(head)), Hit, "{head}");
    }
    // a formal is bound without a statement, whatever its spelling
    for sig in [
        "d",
        "d=None",
        "d: dict",
        "d: dict = None",
        "*d",
        "**d",
        "*, d",
        "x, /, d",
        "x, *d",
    ] {
        let src = format!("def f(t, {sig}):\n    d['a'] = t\n    sink(d)\n");
        let v: Vec<_> = all_routes(&src).iter().map(|(_, r)| verdict(r)).collect();
        assert_eq!(v, [Hit], "{sig}");
    }
    let src = "class A:\n    def m(self, t):\n        self.items[0] = t\n        return self\n";
    let v: Vec<_> = all_routes(src).iter().map(|(_, r)| verdict(r)).collect();
    assert_eq!(v, [Hit]);
    // free, global, nested-scope and comprehension-only names are not owner locals
    for (src, why) in [
        // free variable of the enclosing function
        (
            "def f(t):\n    d = {}\n    def g():\n        d['a'] = t\n        sink(d)\n",
            Why::Binding,
        ),
        // module global read from the owner
        ("def f(t):\n    d['a'] = t\n    sink(d)\n", Why::Binding),
        // bound only inside a comprehension, a lambda or a nested def
        (
            "def f(t):\n    x = [d for d in t]\n    d['a'] = t\n    sink(d)\n",
            Why::Binding,
        ),
        (
            "def f(t):\n    g = lambda d: d\n    d['a'] = t\n    sink(d)\n",
            Why::Binding,
        ),
        (
            "def f(t):\n    def g(d): pass\n    d['a'] = t\n    sink(d)\n",
            Why::Binding,
        ),
        (
            "def f(t):\n    class K:\n        d = 1\n    d['a'] = t\n    sink(d)\n",
            Why::Binding,
        ),
    ] {
        let found = all_routes(src);
        assert_eq!(found.len(), 1, "{src}");
        let Route::Refused(r) = &found[0].1 else {
            panic!("{src}: reached")
        };
        assert_eq!(r.why, why, "{src}");
    }
    // `global`/`nonlocal` redirection never reaches the transfer: the inventory refuses it
    for decl in ["global d", "nonlocal d"] {
        let src = format!("def f(t):\n    {decl}\n    d = 1\n    d['a'] = t\n    sink(d)\n");
        assert!(all_routes(&src).is_empty(), "{decl}");
    }
    // Q07: nothing is paired across owners or through fenced scopes
    table(
        &[
            ("Q07.closure", NoPair),
            ("Q07.shadowed", NoPair),
            ("Q07.deferred", NoPair),
            ("Q07.free", NoPair),
            ("Q07.global", NoPair),
            // the `raise` after W has no normal exit and handlers are not entered: no route
            ("Q07.exceptional", Other(Why::NoRoute)),
        ],
        false,
    );
    assert_eq!(marked("Q07.exceptional").1, false);
}
