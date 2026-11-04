//! #636 design A, review round 1: each row fails on 451eae9f (or pins presence/recall). Weak
//! requires proof; the finding set never changes.

use super::taint_sink_strength_test::*;

#[test]
fn python_round_1_folds() {
    assert_severities(&[
        py("W1 collate(non-literal) as a comparison value", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\ndef h(session, req):\n    c = req.args['c']  # SEED\n    return session.execute(select(User).where(User.id == User.name.collate(c)))  # SINK\n", W),
        py("W1 collate inside func argument", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\ndef h(session, req):\n    c = req.args['c']  # SEED\n    return session.execute(select(User).where(func.lower(User.name.collate(c)) == 'a'))  # SINK\n", W),
        py("control: collate in order_by", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\ndef h(session, req):\n    c = req.args['c']  # SEED\n    return session.execute(select(User).order_by(User.name.collate(c)))  # SINK\n", W),
        py("W2 with_hint in an in_ value", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\ndef h(session, req):\n    hint = req.args['h']  # SEED\n    return session.execute(select(User).where(User.id.in_(select(User.id).with_hint(User, hint))))  # SINK\n", W),
        py("W2 with_hint on an opaque statement parameter", "from sqlalchemy import select\nfrom app.models import User\ndef h(base, req):\n    hint = req.args['h']  # SEED\n    return base.with_hint(User, hint).where(User.id == 1)  # SINK\n", W),
        py("W2 with_hint on an opaque Query attribute", "from sqlalchemy import select\nfrom app.models import User\ndef h(req):\n    hint = req.args['h']  # SEED\n    return User.query.with_hint(User, hint).filter(User.id == 1).all()  # SINK\n", W),
        py("W3 sqlmodel text in a comparison value", "from sqlalchemy import select\nfrom sqlmodel import text\nfrom app.models import User\ndef h(session, req):\n    s = req.args['s']  # SEED\n    return session.execute(select(User).where(User.a == text(s)))  # SINK\n", W),
        py("W3 text imported from two SQLAlchemy paths, raw via local", "from sqlalchemy import select\nfrom sqlalchemy import text\nfrom app.models import User\ndef g():\n    from sqlalchemy.sql import text\n    return text('1')\ndef h(session, req):\n    s = req.args['s']\n    ob = text(s)\n    x = req.args['x']  # SEED\n    return session.execute(select(User).where(User.a == ob, User.id == x))  # SINK\n", W),
        py("W4 list.append of a raw construct", "from sqlalchemy import select, insert, func, literal_column, text\nfrom app.models import User, t\ndef h(session, req):\n    s = req.args['s']\n    vals = []\n    vals.append(literal_column(s))\n    x = req.args['x']  # SEED\n    return session.execute(select(User).where(User.a.in_(vals), User.id == x))  # SINK\n", W),
        py("W4 dict.update of a raw construct", "from sqlalchemy import select, insert, func, literal_column, text\nfrom app.models import User, t\ndef h(session, req):\n    s = req.args['s']\n    row = {}\n    row.update(a=literal_column(s))\n    x = req.args['x']  # SEED\n    return session.execute(insert(t).values(row).values(b=x))  # SINK\n", W),
        py("control: subscript store", "from sqlalchemy import select, insert, func, literal_column, text\nfrom app.models import User, t\ndef h(session, req):\n    s = req.args['s']\n    row = {}\n    row['a'] = literal_column(s)\n    x = req.args['x']  # SEED\n    return session.execute(insert(t).values(row).values(b=x))  # SINK\n", W),
        py("W5 conditional receiver with raw order_by", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\ndef h(session, req, flag):\n    s = req.args['s']\n    q = session.query(User).order_by(text(s)) if flag else session.query(User)\n    x = req.args['x']  # SEED\n    return q.filter(User.id == x).all()  # SINK\n", W),
        py("W5 walrus receiver with raw order_by", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\ndef h(session, req):\n    s = req.args['s']\n    if (q := session.query(User).order_by(text(s))):\n        x = req.args['x']  # SEED\n        return q.filter(User.id == x).all()  # SINK\n", W),
        py("control: plain local receiver", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\ndef h(session, req):\n    s = req.args['s']\n    q = session.query(User).order_by(text(s))\n    x = req.args['x']  # SEED\n    return q.filter(User.id == x).all()  # SINK\n", W),
        py("D1a self attribute built in __init__", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\nclass R:\n    def __init__(self, s):\n        self.ob = literal_column(s)\n    def h(self, session, req):\n        x = req.args['x']  # SEED\n        return session.execute(select(User).where(User.a == self.ob, User.id == x))  # SINK\n", W),
        py("D1b loop target over a local list of raw constructs", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\ndef h(session, req):\n    s = req.args['s']\n    cols = [literal_column(s)]\n    for c in cols:\n        x = req.args['x']  # SEED\n        session.execute(select(User).where(User.a == c, User.id == x))  # SINK\n", W),
        py("sol1 unrelated parameter named like the aliased import", "from sqlalchemy import literal_column as fragment\nfrom app.models import User\ndef unrelated(fragment):\n    pass\ndef h(session, req):\n    x = req.GET[\"x\"]  # SEED\n    ob = fragment(x)\n    return session.query(User).filter(User.a == ob, User.b == x).all()  # SINK\n", W),
        py("W4 recall: appended request values stay weak", "from sqlalchemy import select\nfrom app.models import User\ndef h(session, req, rows):\n    x = req.args.get('x')  # SEED\n    ids = []\n    for r in rows:\n        ids.append(r.id)\n    return session.execute(select(User).where(User.a.in_(ids), User.id == x))  # SINK\n", S),
        py("D1a recall: self attribute built from data stays weak", "from sqlalchemy import select\nfrom app.models import User\nclass R:\n    def __init__(self, s):\n        self.ob = s\n    def h(self, session, req):\n        x = req.args.get('x')  # SEED\n        return session.execute(select(User).where(User.a == self.ob, User.id == x))  # SINK\n", S),
        py("D1b recall: loop over a local list of data stays weak", "from sqlalchemy import select\nfrom app.models import User\ndef h(session, req):\n    s = req.args.get('s')\n    cols = [s, 2]\n    for c in cols:\n        x = req.args.get('x')  # SEED\n        session.execute(select(User).where(User.a == c, User.id == x))  # SINK\n", S),
        py("W5 recall: conditional receiver of proven branches stays weak", "from app.models import User\ndef h(session, req, flag):\n    x = req.args.get('x')  # SEED\n    q = session.query(User) if flag else session.query(User).limit(5)\n    return q.filter(User.id == x).all()  # SINK\n", S),
    ]);
}

#[test]
fn js_round_1_folds() {
    assert_severities(&[
        js("sol2 shorthand destructuring redeclares the query", "function h(knex, input) {\n  const x = input.v; // SEED\n  var q = \"id = ?\";\n  var {q} = {q: `id = ${x} OR note = ?`};\n  knex(\"t\").whereRaw(q, [x]); // SINK\n}\n", W),
        js("sol2/S4 catch parameter shadows a constant", "function h(knex, input) {\n  var q = 'id = ?';\n  try { run(); } catch (q) {\n    const x = input.v; // SEED\n    knex('t').whereRaw(q, [x]); // SINK\n  }\n}\n", W),
    ]);
}

#[test]
fn go_round_1_folds() {
    assert_severities(&[
        go("sol3 receive assignment writes the query", "package p\nfunc h(db *sql.DB, input string) {\n	x := input // SEED\n	q := \"SELECT * FROM t WHERE note = $1\"\n	ch := make(chan string, 1)\n	ch <- \"SELECT * FROM t WHERE id = \" + x + \" OR note = $1\"\n	select {\n	case q = <-ch:\n		db.Query(q, x) // SINK\n	}\n}\n", W),
        go("sol3 receive declaration shadows an outer constant", "package p\nconst q = \"SELECT * FROM t WHERE note = $1\"\nfunc h(db *sql.DB, input string, ch chan string) {\n	x := input // SEED\n	select {\n	case q := <-ch:\n		db.Query(q, x) // SINK\n	}\n}\n", W),
        go("W6/G1 multi-name var, file constructor: presence as on main", "package main\nimport (\n	\"net/http\"\n	\"os\"\n)\nfunc h(w http.ResponseWriter, r *http.Request) {\n	m := r.URL.Query().Get(\"m\") // SEED\n	var cache, err = os.Create(\"/tmp/x\")\n	_ = err\n	cache.Chmod(os.FileMode(len(m))) // SINK\n}\n", W),
        go("W6/G2 var a, run = 1, db.Query: no finding on main", "package main\nimport (\n	\"database/sql\"\n	\"net/http\"\n)\nfunc h(db *sql.DB, r *http.Request) {\n	q := r.URL.Query().Get(\"q\") // SEED\n	var a, run = 1, db.Query\n	_ = a\n	run(\"SELECT * FROM t WHERE a = \" + q) // SINK\n}\n", NONE),
        go("W6/G3 var run, other = db.Query, fmtx: finding on main", "package main\nimport (\n	\"database/sql\"\n	\"net/http\"\n)\nfunc h(db *sql.DB, r *http.Request) {\n	q := r.URL.Query().Get(\"q\") // SEED\n	var run, other = db.Query, fmtx\n	_ = run\n	other(\"SELECT * FROM t WHERE a = \" + q) // SINK\n}\n", W),
        go("sol4a label and run, run is db.Query: no finding on main", "package p\nfunc h(db *sql.DB, in Input) {\n	x := in.V // SEED\n	var label, run = \"label\", db.Query\n	_ = label\n	run(x) // SINK\n}\n", NONE),
        go("sol4b unused, other = db.Query, TrimSpace: finding on main", "package p\nfunc h(db *sql.DB, in Input) {\n	x := in.V // SEED\n	var unused, other = db.Query, strings.TrimSpace\n	_ = unused\n	other(x) // SINK\n}\n", W),
    ]);
}
