//! #636 design A, review round 2 (post-cap closed folds): each Strong row fails on bda2b9cc.

use super::taint_sink_strength_test::*;
use crate::common::*;

#[test]
fn python_round_2_folds() {
    assert_severities(&[
        py("WR1 multi-line chain with_hint on Query attribute", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\ndef h(req):\n    hint = req.args['h']\n    x = req.args['x']  # SEED\n    return (\n        User.query.with_hint(User, hint)\n        .filter(User.id == x)  # SINK\n        .all()\n    )\n", W),
        py("WR1 multi-line chain prefix_with", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\ndef h(req):\n    s = req.args['s']\n    x = req.args['x']  # SEED\n    return (\n        User.query.prefix_with(s)\n        .filter(User.id == x)  # SINK\n        .all()\n    )\n", W),
        py("WR1 local-bound with_hint", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\ndef h(req):\n    hint = req.args['h']\n    q = User.query.with_hint(User, hint)\n    x = req.args['x']  # SEED\n    return q.filter(User.id == x).all()  # SINK\n", W),
        py("WR1 local-bound prefix_with on a parameter", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\ndef h(base, req):\n    s = req.args['s']\n    q = base.prefix_with(s)\n    x = req.args['x']  # SEED\n    return q.where(User.id == x)  # SINK\n", W),
        py("control: same-line with_hint", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\ndef h(req):\n    hint = req.args['h']\n    x = req.args['x']  # SEED\n    return User.query.with_hint(User, hint).filter(User.id == x).all()  # SINK\n", W),
        py("recall: multi-line chain without raw", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\ndef h(req):\n    x = req.args['x']  # SEED\n    return (\n        User.query.order_by(User.id)\n        .filter(User.id == x)  # SINK\n        .all()\n    )\n", S),
        py("WR2 dynamic column collate value", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\ndef h(session, req, col):\n    c = req.args['c']  # SEED\n    return session.execute(select(User).where(User.id == User.__table__.c[col].collate(c)))  # SINK\n", W),
        py("WR2 local dynamic column collate", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\ndef h(session, req, col):\n    cc = User.__table__.c[col]\n    c = req.args['c']  # SEED\n    return session.execute(select(User).where(User.id == cc.collate(c)))  # SINK\n", W),
        py("WR3 loop over a mutator-filled local list", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\ndef h(session, req):\n    s = req.args['s']\n    cols = []\n    cols.append(literal_column(s))\n    for c in cols:\n        x = req.args['x']  # SEED\n        session.execute(select(User).where(User.a == c, User.id == x))  # SINK\n", W),
        py("WR4 tuple target self.ob, self.n", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\nclass R:\n    def __init__(self, s):\n        self.ob, self.n = literal_column(s), 1\n    def h(self, session, req):\n        x = req.args['x']  # SEED\n        return session.execute(select(User).where(User.a == self.ob, User.id == x))  # SINK\n", W),
        py("WR4 setattr(self, ob, raw)", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\nclass R:\n    def __init__(self, s):\n        setattr(self, 'ob', literal_column(s))\n    def h(self, session, req):\n        x = req.args['x']  # SEED\n        return session.execute(select(User).where(User.a == self.ob, User.id == x))  # SINK\n", W),
        py("WR4 mutator on self.cols", "from sqlalchemy import select, func, literal_column, text\nfrom app.models import User\nclass R:\n    def __init__(self, s):\n        self.cols = []\n        self.cols.append(literal_column(s))\n    def h(self, session, req):\n        x = req.args['x']  # SEED\n        return session.execute(select(User).where(User.a.in_(self.cols), User.id == x))  # SINK\n", W),
        py("sol1/S1 untrusted sa root Function(s)", "import sqlalchemy as sa\nfrom sqlalchemy import select\nfrom app.models import User\ndef g(sa):\n    pass\ndef h(session, req):\n    s = req.args['s']\n    ob = sa.sql.functions.Function(s)\n    x = req.args['x']  # SEED\n    return session.execute(select(User).where(User.a == ob, User.id == x))  # SINK\n", W),
        py("sol1 untrusted peewee module root with an unrelated pw parameter", "import peewee as pw\nfrom app.models import User\ndef unrelated(pw):\n    pass\ndef h(req):\n    op = req.GET[\"op\"]\n    ob = pw.Expression(User.id, op, 1)\n    x = req.GET[\"x\"]  # SEED\n    return User.select().where(User.id == ob, User.other == x)  # SINK\n", W),
        py("sol3 unpacking target with an unused name", "from sqlalchemy import select, literal_column\nfrom app.models import User\nclass R:\n    def __init__(self, sql):\n        self.ob, unused = literal_column(sql), 0\n    def h(self, session, req):\n        x = req.args[\"x\"]  # SEED\n        return session.execute(select(User).where(User.a == self.ob, User.id == x))  # SINK\n", W),
        py("recall: unpacking target of data stays weak", "from sqlalchemy import select\nfrom app.models import User\nclass R:\n    def __init__(self, s):\n        self.ob, unused = s, 0\n    def h(self, session, req):\n        x = req.args.get('x')  # SEED\n        return session.execute(select(User).where(User.a == self.ob, User.id == x))  # SINK\n", S),
        py("recall: mutator of data on self.cols stays weak", "from sqlalchemy import select\nfrom app.models import User\nclass R:\n    def __init__(self, ids):\n        self.cols = []\n        self.cols.append(ids)\n    def h(self, session, req):\n        x = req.args.get('x')  # SEED\n        return session.execute(select(User).where(User.a.in_(self.cols), User.id == x))  # SINK\n", S),
        py("recall: loop over a list filled with data stays weak", "from sqlalchemy import select\nfrom app.models import User\ndef h(session, req, rows):\n    cols = []\n    for r in rows:\n        cols.append(r.id)\n    for c in cols:\n        x = req.args.get('x')  # SEED\n        session.execute(select(User).where(User.a == c, User.id == x))  # SINK\n", S),
    ]);
}

#[test]
fn js_round_2_folds() {
    assert_severities(&[
        js("presence: catch parameter shadows a child_process binding (finding exists as on main)", "const {exec: run} = require(\"child_process\");\nfunction h(input) {\n  try {\n    throw run;\n  } catch (run) {\n    const x = input.v; // SEED\n    run(x); // SINK\n  }\n}\n", W),
    ]);
}

#[test]
fn go_round_2_folds() {
    assert_severities(&[
        go("WR5 address-of write via Unmarshal", "package p\nimport (\"database/sql\"; \"encoding/json\")\nfunc h(db *sql.DB, input string, body []byte) {\n	x := input // SEED\n	q := \"SELECT * FROM t WHERE note = $1\"\n	json.Unmarshal(body, &q)\n	db.Query(q, x) // SINK\n}\n", W),
        go("WR5 address-of write via Sscan", "package p\nimport (\"database/sql\"; \"fmt\")\nfunc h(db *sql.DB, input string) {\n	x := input // SEED\n	q := \"SELECT * FROM t WHERE note = $1\"\n	fmt.Sscan(x, &q)\n	db.Query(q, x) // SINK\n}\n", W),
        go("WR5 type switch shadows a constant", "package p\nimport \"database/sql\"\nconst q = \"SELECT * FROM t WHERE note = $1\"\nfunc h(db *sql.DB, input string, v any) {\n	x := input // SEED\n	switch q := v.(type) {\n	case string:\n		db.Query(q, x) // SINK\n	}\n}\n", W),
        go("recall: constant query", "package p\nimport \"database/sql\"\nfunc h(db *sql.DB, input string) {\n	x := input // SEED\n	q := \"SELECT * FROM t WHERE note = $1\"\n	db.Query(q, x) // SINK\n}\n", S),
    ]);
}
