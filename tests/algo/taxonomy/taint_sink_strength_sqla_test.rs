//! #636 design A: the closed allow-list for Python SQL weak proofs. Weak requires proof;
//! anything unproven stays `warning`. Rows come from the round-1 and round-2 reviewer probes,
//! the closure probes a round-3 reviewer would try (section B), the recall pins taken from the
//! fresh sample and the census (section C), and the value-position contract rows (section D).
//! Models and tables are imported unless a row says otherwise.
//!
//! D1 (contract): a value position binds the Python object it receives. A SQL construct that
//! reaches a value position from outside the function (a parameter, the return of a cross-module
//! function, an attribute set elsewhere) is out of model, as runtime mutation is for `Exact`.
//! D2: a never-rebound parameter annotated as a table or statement is a schema root.
//! D3: local name resolution is capped at 8 hops.

use super::taint_sink_strength_test::*;
use crate::common::*;

#[test]
fn reviewer_rows_round_1_and_2() {
    assert_severities(&[
        py("R2-P1 order_by(ob), ob = literal_column(sort)", "from sqlalchemy import select, literal_column\ndef h(session, req):\n    sort = req.args.get('sort')  # SEED\n    ob = literal_column(sort)\n    return session.execute(select(User).order_by(ob))  # SINK\n", W),
        py("R2-P1b same, statement on its own line", "from sqlalchemy import select, literal_column\ndef h(session, req):\n    sort = req.args.get('sort')  # SEED\n    ob = literal_column(sort)\n    stmt = select(User).order_by(ob)\n    return session.execute(stmt)  # SINK\n", W),
        py("R2-P1c ob = text(sort)", "from sqlalchemy import select, text\ndef h(session, req):\n    sort = req.args.get('sort')  # SEED\n    ob = text(sort)\n    return session.execute(select(User).order_by(ob))  # SINK\n", W),
        py("R2-P2 filter(helper(x)), helper returns text(raw)", "from sqlalchemy import text\ndef build_filter(v):\n    return text(\"name = '\" + v + \"'\")\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.query(User).filter(build_filter(x)).all()  # SINK\n", W),
        py("R2-P2b peewee where(helper(x)), helper returns SQL(raw)", "from peewee import SQL\ndef raw_clause(v):\n    return SQL('name = ' + v)\ndef h(req):\n    x = req.GET['x']  # SEED\n    return User.select().where(raw_clause(x))  # SINK\n", W),
        py("R2-P3 text aliased: filter(T(x))", "from sqlalchemy.sql import text as T\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.query(User).filter(T(x)).all()  # SINK\n", W),
        py("R2-P3b literal_column aliased in order_by", "from sqlalchemy import select, literal_column as lc\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.execute(select(User).order_by(lc(x)))  # SINK\n", W),
        py("R2-P3c literal_column unaliased", "from sqlalchemy import select, literal_column\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.execute(select(User).order_by(literal_column(x)))  # SINK\n", W),
        py("R2-P4 ilike with an f-string value is bound", "def h(session, req):\n    q = req.args.get('q')  # SEED\n    return session.query(User).filter(User.name.ilike(f'%{q}%')).all()  # SINK\n", S),
        py("R2-P4b comparison with a request value (owner control)", "def h(session, req):\n    q = req.args.get('q')  # SEED\n    return session.query(User).filter(User.name == q).all()  # SINK\n", S),
        py("R2-P5 where(cond), cond = text(raw)", "from sqlalchemy.sql import select, text\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    cond = text(\"name = '\" + x + \"'\")\n    stmt = select(User).where(cond)\n    return session.execute(stmt)  # SINK\n", W),
        py("R2-P6 column.op(user operator)", "def h(session, req):\n    op = req.args.get('op')  # SEED\n    return session.query(User).filter(User.age.op(op)(18)).all()  # SINK\n", W),
        py("R2-P7 getattr(func, user name)", "from sqlalchemy import func\ndef h(session, req):\n    fn = req.args.get('fn')  # SEED\n    return session.query(User).filter(getattr(func, fn)(User.name) == 'a').all()  # SINK\n", W),
        py("sol-r2 1a literal_column as fragment in select", "from sqlalchemy import select, literal_column as fragment\ndef h(session, req):\n    x = req.GET[\"expression\"]  # SEED\n    session.execute(select(fragment(x)))  # SINK\n", W),
        py("sol-r2 1b peewee SQL as fragment", "from peewee import SQL as fragment\ndef h(req):\n    x = req.GET['x']  # SEED\n    return Model.select().where(fragment(x))  # SINK\n", W),
        py("sol-r2 1c column.op(x)(1)", "import sqlalchemy as sa\nfrom app.models import t\ndef h(session, req):\n    x = req.GET['x']  # SEED\n    session.execute(sa.select(t.c.id).where(t.c.id.op(x)(1)))  # SINK\n", W),
        py("sol-r2 3a lambda parameter shadows the module alias", "import sqlalchemy as sa\ndef h(cur, req, adapter):\n    x = req.GET[\"x\"]  # SEED\n    return (lambda sa: cur.execute(sa.select(x)))(adapter)  # SINK\n", W),
        py("sol-r2 3b lambda parameter shadows the imported constructor", "from sqlalchemy import select\ndef h(cur, req, adapter):\n    x = req.GET[\"x\"]  # SEED\n    return (lambda select: cur.execute(select(x)))(adapter)  # SINK\n", W),
        py("sol-r1 5 Select-annotated parameter rebound to a string", "import sqlalchemy as sa\ndef h(cur, stmt: sa.Select, req):\n    x = req.GET[\"x\"]  # SEED\n    stmt = \"SELECT * FROM t WHERE id = \" + x\n    cur.execute(stmt)  # SINK\n", W),
        py("opus-r1 P7 Executable annotation is not proof", "from sqlalchemy.sql import Executable\ndef h(session, stmt: Executable, inp):\n    x = inp.v  # SEED\n    session.execute(stmt, {\"a\": x})  # SINK\n", W),
    ]);
}

#[test]
fn closure_probes_stay_strong() {
    assert_severities(&[
        py("V local holding raw SQL", "from sqlalchemy import literal_column\nfrom app.models import User\ndef h(session, req):\n    s = req.args.get('s')  # SEED\n    ob = literal_column(s)\n    return session.query(User).filter(User.a == ob).all()  # SINK\n", W),
        py("literal_column inside in_", "from sqlalchemy import literal_column\nfrom app.models import User\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.query(User).filter(User.a.in_([literal_column(x)])).all()  # SINK\n", W),
        py("literal_column as limit", "from sqlalchemy import select, literal_column\nfrom app.models import User\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.execute(select(User).limit(literal_column(x)))  # SINK\n", W),
        py("literal_column as case else", "from sqlalchemy import select, case, literal_column\nfrom app.models import User\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.execute(select(User).where(case((User.a == 1, 1), else_=literal_column(x)) == 1))  # SINK\n", W),
        py("Flask-SQLAlchemy proxy db.literal_column", "from app import db\nfrom app.models import User\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.query(User).filter(User.a == db.literal_column(x)).all()  # SINK\n", W),
        py("same-file helper returns literal_column", "from sqlalchemy import literal_column\nfrom app.models import User\ndef raw(v):\n    return literal_column(v)\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.query(User).filter(User.a == raw(x)).all()  # SINK\n", W),
        py("alias of the module attribute", "import sqlalchemy as sa\nfrom app.models import User\nlc = sa.literal_column\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.query(User).filter(User.a == lc(x)).all()  # SINK\n", W),
        py("callee passed as a parameter", "from app.models import User\ndef h(session, mk, req):\n    x = req.args.get('x')  # SEED\n    return session.query(User).filter(User.a == mk(x)).all()  # SINK\n", W),
        py("construct stored on a local object", "from sqlalchemy import literal_column\nfrom app.models import User, Box\ndef h(session, req):\n    s = req.args.get('s')  # SEED\n    o = Box()\n    o.v = literal_column(s)\n    return session.query(User).filter(User.a == o.v).all()  # SINK\n", W),
        py("self attribute built from literal_column", "from sqlalchemy import select, literal_column\nfrom app.models import User\nclass Q:\n    def __init__(self, s):\n        self.ob = literal_column(s)\n    def run(self, session, req):\n        x = req.args.get('x')  # SEED\n        return session.execute(select(User).order_by(self.ob).where(User.a == x))  # SINK\n", W),
        py("order_by f-string", "from sqlalchemy import select\nfrom app.models import User\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.execute(select(User).order_by(f\"{x} desc\"))  # SINK\n", W),
        py("order_by identifier from the request", "from sqlalchemy import select\nfrom app.models import User\ndef h(session, req):\n    s = req.args['s']  # SEED\n    return session.execute(select(User).order_by(s))  # SINK\n", W),
        py("where string concatenation", "from app.models import User\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.query(User).filter(\"id = \" + x).all()  # SINK\n", W),
        py("select(s) with a request string", "from sqlalchemy import select\ndef h(session, req):\n    s = req.args['s']  # SEED\n    return session.execute(select(s))  # SINK\n", W),
        py("maximal chain: text in a later order_by", "from sqlalchemy.sql import text\nfrom app.models import User\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    s = req.args.get('s')\n    return session.query(User).filter(User.id == x).order_by(text(s)).all()  # SINK\n", W),
        py("maximal chain: local receiver with a raw order_by", "from sqlalchemy import literal_column\nfrom app.models import User\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    ob = literal_column(req.args.get('s'))\n    q = session.query(User).order_by(ob)\n    return q.filter(User.id == x).all()  # SINK\n", W),
        py("join with a text onclause", "from sqlalchemy import select, text\nfrom app.models import User, Group\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.execute(select(User).join(Group, text(x)))  # SINK\n", W),
        py("prefix_with", "from sqlalchemy import select\nfrom app.models import User\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.execute(select(User).prefix_with(\"/*+ x */\").where(User.a == x))  # SINK\n", W),
        py("with_hint", "from sqlalchemy import select\nfrom app.models import User\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.execute(select(User).with_hint(User, \"USE INDEX\").where(User.a == x))  # SINK\n", W),
        py("getattr(Model, name) in order_by", "from sqlalchemy import select\nfrom app.models import User\ndef h(session, req):\n    sort = req.args.get('sort')  # SEED\n    return session.execute(select(User).order_by(getattr(User, sort)))  # SINK\n", W),
        py("except-as binding shadows the import", "from sqlalchemy import select\nfrom app.models import User\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    try:\n        pass\n    except ValueError as select:\n        pass\n    return session.execute(select(User).where(User.a == x))  # SINK\n", W),
        py("comprehension variable shadows the module alias", "import sqlalchemy as sa\ndef h(session, adapters, req):\n    x = req.args.get('x')  # SEED\n    return [session.execute(sa.select(x)) for sa in adapters]  # SINK\n", W),
        py("global statement rebinds the module alias", "import sqlalchemy as sa\nfrom app.models import User\ndef g(other):\n    global sa\n    sa = other\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.execute(sa.select(User).where(User.a == x))  # SINK\n", W),
        py("function-local import of another module under the same alias", "import sqlalchemy as sa\nfrom app.models import User\ndef g():\n    import other as sa\n    return sa\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.execute(sa.select(User).where(User.a == x))  # SINK\n", W),
        py("sqlalchemy_utils is not sqlalchemy", "import sqlalchemy_utils as sa\nfrom app.models import User\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.execute(sa.select(User).where(User.a == x))  # SINK\n", W),
        py("sqlalchemy_continuum is not sqlalchemy", "from sqlalchemy_continuum import select\nfrom app.models import User\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.execute(select(User).where(User.a == x))  # SINK\n", W),
        py("statement bound twice", "from sqlalchemy import select, text\nfrom app.models import User\ndef h(session, req):\n    s = req.args.get('s')  # SEED\n    stmt = select(User)\n    stmt = stmt.where(text(s))\n    return session.execute(stmt)  # SINK\n", W),
        py("bindparam with a non-literal key", "from sqlalchemy import select, bindparam\nfrom app.models import User\ndef h(session, req):\n    k = req.args.get('k')  # SEED\n    return session.execute(select(User).where(User.a == bindparam(k, 1)))  # SINK\n", W),
        py("like with a non-literal escape", "from sqlalchemy import select\nfrom app.models import User\ndef h(session, req):\n    e = req.args.get('e')  # SEED\n    return session.execute(select(User).where(User.a.like('x', escape=e)))  # SINK\n", W),
        py("label with a non-literal name", "from sqlalchemy import select\nfrom app.models import User\ndef h(session, req):\n    n = req.args.get('n')  # SEED\n    return session.execute(select(User.a.label(n)))  # SINK\n", W),
        py("execute of an unannotated parameter", "def h(session, stmt, req):\n    x = req.args.get('x')  # SEED\n    return session.execute(stmt, {'a': x})  # SINK\n", W),
        py("D2 pin: select(t) with an unannotated parameter t", "from sqlalchemy import select\ndef h(session, t, req):\n    x = req.args.get('x')  # SEED\n    return session.execute(select(t).where(t.c.a == x))  # SINK\n", W),
        py("D3 pin: identifier chain of 9 names", "import sqlalchemy as sa\nfrom app.models import User\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    v0 = sa.select(User).where(User.a == x)\n    v1 = v0\n    v2 = v1\n    v3 = v2\n    v4 = v3\n    v5 = v4\n    v6 = v5\n    v7 = v6\n    v8 = v7\n    return session.execute(v8)  # SINK\n", W),
        py("loop target over an in-place list of raw constructs stays strong (D1 guard)", "from sqlalchemy import literal_column\nfrom app.models import User\ndef h(session, req):\n    s = req.args.get('s')  # SEED\n    for c in [literal_column(s)]:\n        return session.query(User).filter(User.a == c, User.b == s).all()  # SINK\n", W),
        py("Django RawSQL", "from django.db.models.expressions import RawSQL\ndef h(req):\n    s = req.GET['s']  # SEED\n    return M.objects.filter(RawSQL(s, []))  # SINK\n", W),
        py("Django .extra in the chain", "from django.db.models import Q\ndef h(req):\n    s = req.GET['s']  # SEED\n    return M.objects.extra(where=[s]).filter(Q(a=1))  # SINK\n", W),
        py("peewee getattr(fn, name)", "from peewee import fn\ndef h(req):\n    n = req.GET['n']  # SEED\n    return User.select().where(getattr(fn, n)(User.a) == 1)  # SINK\n", W),
        py("module-level raw constructor used in order_by", "from sqlalchemy import select, literal_column\nSORT = literal_column(CFG)\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.execute(select(User).order_by(SORT).where(User.a == x))  # SINK\n", W),
    ]);
}

#[test]
fn recall_pins_stay_weak() {
    assert_severities(&[
        py("F072 select with is_/like", "from sqlalchemy import select\nfrom app.models import M\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.execute(select(M.id, M.meta).where(M.deleted.is_(None), M.meta.like(\"%x%\"), M.id == x))  # SINK\n", S),
        py("F002 execution_options on a local select", "from sqlalchemy import select\nfrom app.models import M\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    candidates = select(M).where(M.id == x)\n    return session.execute(candidates.execution_options(yield_per=1000))  # SINK\n", S),
        py("F115 in_ of an annotated Select parameter", "import sqlalchemy as sa\nfrom app.models import M\ndef h(session, view_ids: sa.Select, req):\n    x = req.args.get('x')  # SEED\n    return session.execute(sa.select(M.id).where(M.k == \"lit\", M.k.in_(view_ids), M.id == x))  # SINK\n", S),
        py("F084 in_ of a local select and a same-file helper", "from sqlalchemy import select, or_\nfrom app.models import A\ndef pick(v):\n    return set()\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    local_sel = select(A.id).where(A.k == x)\n    return session.execute(select(A).where(or_(A.id.in_(local_sel), A.id.in_(pick(x)))))  # SINK\n", S),
        py("F122 union_all with literal().label() and func.coalesce", "from sqlalchemy import select, literal, func, union_all\nfrom app.models import M\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    u = union_all(select(literal(\"c\").label(\"t\"), func.coalesce(M.n, str(_(\"U\")))).where(M.k == x)).subquery()\n    return session.execute(select(u.c.t))  # SINK\n", S),
        py("F003 count/sum/case over a local clause", "import sqlalchemy as sa\ndef h(session, dependents: sa.Subquery, req):\n    x = req.args.get('x')  # SEED\n    visibility = sa.or_(sa.and_(dependents.c.a == x, dependents.c.b == 1))\n    return session.execute(sa.select(sa.func.count(), sa.func.sum(sa.case((visibility, 1), else_=0))).select_from(dependents))  # SINK\n", S),
        py("F056 identifier clause in where", "import sqlalchemy as sa\ndef h(session, dependents: sa.Subquery, req):\n    x = req.args.get('x')  # SEED\n    visibility = sa.or_(sa.and_(dependents.c.a == x, dependents.c.b == 1))\n    sub = sa.select(dependents).where(visibility).subquery()  # SINK\n    return session.execute(sa.select(sub.c.a))\n", S),
        py("F095 order_by case then limit over a local subquery", "from sqlalchemy import select, case\nfrom app.models import M\nN = 10\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    v = select(M.a, M.t).where(M.k == x).subquery()\n    return session.execute(select(v.c.a).order_by(case((v.c.t == \"c\", 0), else_=2), v.c.t).limit(N))  # SINK\n", S),
        py("F096 query(model_cls) with inspect().primary_key", "from sqlalchemy import inspect\ndef h(session, model_cls: type[Model], req):\n    pks = req.args.getlist('p')  # SEED\n    return session.query(model_cls).execution_options(**{K: {model_cls}}).filter(inspect(model_cls).primary_key[0].in_(pks))  # SINK\n", S),
        py("F045 in_ of request ids", "from app.models import Slice\ndef h(session, req):\n    ids = req.args.getlist('i')  # SEED\n    return session.query(Slice).filter(Slice.id.in_(ids)).all()  # SINK\n", S),
        py("invert of a column", "from app.models import M\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.query(M).filter(~M.flag, M.id == x).all()  # SINK\n", S),
        py("arithmetic of columns then ilike", "from app.models import U\ndef h(session, req):\n    v = req.args.get('v')  # SEED\n    return session.query(U).filter((U.first + \" \" + U.last).ilike(v)).all()  # SINK\n", S),
        py("in_ of typing.cast(Select, ...) is a value (D1)", "import typing\nimport sqlalchemy as sa\nfrom app.models import M\ndef h(session, q, req):\n    x = req.args.get('x')  # SEED\n    return session.query(M).filter(M.id.in_(typing.cast(sa.Select, X().apply(q).statement)), M.k == x).all()  # SINK\n", S),
        py("conditional between desc and asc", "from sqlalchemy import select\nfrom app.models import M\ndef h(session, d, req):\n    x = req.args.get('x')  # SEED\n    return session.execute(select(M).where(M.k == x).order_by(M.a.desc() if d else M.a.asc()))  # SINK\n", S),
        py("star import supplies the model", "from models import *\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.query(User).filter(User.id == x).all()  # SINK\n", S),
        py("existing row, t imported: sa.select(t.c.id).where(t.c.a == x)", "import sqlalchemy as sa\nfrom app.models import t\ndef h(session, inp):\n    x = inp.v  # SEED\n    session.execute(sa.select(t.c.id).where(t.c.a == x))  # SINK\n", S),
        py("existing row, t imported: multi-line limit", "import sqlalchemy as sa\nfrom app.models import t\ndef h(session, inp):\n    x = inp.v  # SEED\n    session.execute(\n        sa.select(t.c.id).order_by(t.c.id).limit(x)  # SINK\n    )\n", S),
        py("F095 shape: annotated visible subquery reused across case, order_by, limit, cast wrapper", "from typing import cast\nimport sqlalchemy as sa\nfrom app import db\nfrom app.models import Slice, Dashboard, Report\n_LIMIT: int = 20\ndef f(view_ids: sa.Select, req):\n    x = req.args.get('x')  # SEED\n    chart_ids: sa.Select = sa.select(Slice.id).where(Slice.k.in_(view_ids), Slice.n == x)\n    dash_ids: sa.Select = sa.select(Dashboard.id).where(Dashboard.id.in_(chart_ids))\n    dependents: sa.Subquery = sa.union_all(\n        sa.select(sa.literal(\"chart\").label(\"type\"), Slice.id.label(\"id\")).where(Slice.id.in_(chart_ids)),\n        sa.select(sa.literal(\"dash\").label(\"type\"), Dashboard.id.label(\"id\")).where(Dashboard.id.in_(dash_ids)),\n        sa.select(sa.literal(\"rep\").label(\"type\"), Report.id.label(\"id\")).where(Report.c.in_(chart_ids), Report.d.in_(dash_ids)),\n    ).subquery()\n    visibility: sa.ColumnElement[bool] = sa.or_(\n        sa.and_(dependents.c.type == \"chart\", dependents.c.id.in_(chart_ids)),\n        sa.and_(dependents.c.type == \"dash\", dependents.c.id.in_(dash_ids)),\n        sa.and_(dependents.c.type.notin_((\"chart\", \"dash\")), dependents.c.id.in_(dash_ids)),\n    )\n    visible: sa.Subquery = sa.select(dependents).where(visibility).subquery()\n    rows = cast(\n        list[tuple[str, int]],\n        db.session.execute(\n            sa.select(visible.c.type, visible.c.id, visible.c.name)\n            .where(visible.c.id == x)  # SINK\n            .order_by(\n                sa.case((visible.c.type == \"chart\", 0), (visible.c.type == \"dash\", 1), else_=2),\n                visible.c.type,\n                visible.c.id,\n            )\n            .limit(_LIMIT)\n        ).all(),\n    )\n    return rows\n", S),
        py("F083 shape: in_ of a set comprehension over loop targets", "from sqlalchemy import select\nfrom app import db\nfrom app.models import SemanticView\ndef f(model, req):\n    x = req.args.get('x')  # SEED\n    controls = [c for key in ('a', 'b') for c in model.get(key) or []]\n    semantic_ids = {id(c): {t.get('id') for t in c.get('targets') or []} for c in controls}\n    requested_ids: set[int] = {i for ids in semantic_ids.values() for i in ids if i is not None}\n    return set(db.session.scalars(select(SemanticView.id).where(SemanticView.id.in_(requested_ids), SemanticView.n == x)))  # SINK\n", S),
        py("Django Q | Q", "from django.db.models import Q\ndef h(req):\n    x = req.GET['x']  # SEED\n    return M.objects.filter(Q(a=x) | Q(b=x))  # SINK\n", S),
        py("Django Exists with OuterRef", "from django.db.models import Exists, OuterRef\ndef h(req):\n    x = req.GET['x']  # SEED\n    return M.objects.filter(Exists(M.objects.filter(pk=OuterRef(\"pk\"), a=x)))  # SINK\n", S),
        py("D3 pin: identifier chain of 7 names (8 hops with the request value)", "import sqlalchemy as sa\nfrom app.models import User\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    v0 = sa.select(User).where(User.a == x)\n    v1 = v0\n    v2 = v1\n    v3 = v2\n    v4 = v3\n    v5 = v4\n    v6 = v5\n    return session.execute(v6)  # SINK\n", S),
    ]);
}

/// D1: these assert Weak. A SQL construct that reaches a value position from outside the
/// function is out of model, as runtime mutation is for `Exact`.
#[test]
fn contract_rows_d1_assert_weak() {
    assert_severities(&[
        py("D1: a construct passed in as a parameter", "from sqlalchemy import select\nfrom app.models import User\ndef h(s, ob, req):\n    x = req.args.get('x')  # SEED\n    return s.execute(select(User).where(User.a == ob, User.b == x))  # SINK\n", S),
        py("D1: cross-module helper return", "from sqlalchemy import select\nfrom helpers import make_expr\nfrom app.models import User\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.execute(select(User).where(User.a == make_expr(x)))  # SINK\n", S),
        py("D1: attribute of an imported module", "from sqlalchemy import select\nimport cfg\nfrom app.models import User\ndef h(session, req):\n    x = req.args.get('x')  # SEED\n    return session.execute(select(User).where(User.a == cfg.value, User.b == x))  # SINK\n", S),
    ]);
}

#[test]
fn go_multi_name_var_and_html_identity() {
    assert_severities(&[
        go("multi-name var: constant first, concatenated second", "package p\nfunc h(db *sql.DB, in Input) {\n\tx := in.V // SEED\n\tvar label, query = \"lookup\", \"SELECT * FROM t WHERE id = \" + x + \" OR note = $1\"\n\t_ = label\n\tdb.Query(query, x) // SINK\n}\n", W),
        go("multi-name var: concatenated first, constant second", "package p\nfunc h(db *sql.DB, in Input) {\n\tx := in.V // SEED\n\tvar query, label = \"SELECT * FROM t WHERE id = \" + x + \" OR note = $1\", \"lookup\"\n\t_ = label\n\tdb.Query(query, x) // SINK\n}\n", W),
        go("multi-name var: each name has its own constant initializer", "package p\nfunc h(db *sql.DB, in Input) {\n\tx := in.V // SEED\n\tvar label, query = \"lookup\", \"SELECT a FROM t WHERE id = $1\"\n\t_ = label\n\tdb.Query(query, x) // SINK\n}\n", S),
        go("multi-name var: arity mismatch from a call", "package p\nfunc h(db *sql.DB, in Input) {\n\tx := in.V // SEED\n\tvar query, n = build(x)\n\t_ = n\n\tdb.Query(query, x) // SINK\n}\n", W),
        go("HTML on plugin.Context is not gin", "package p\nfunc h(c plugin.Context, in Input) {\n\tx := in.V // SEED\n\tc.HTML(200, \"t.tmpl\", x) // SINK\n}\n", W),
        go("HTML on origin.Context is not gin", "package p\nfunc h(c *origin.Context, in Input) {\n\tx := in.V // SEED\n\tc.HTML(200, \"t.tmpl\", x) // SINK\n}\n", W),
        go("HTML on *gin.Context with a variable status", "package p\nfunc h(ctx *gin.Context, code int, in Input) {\n\tx := in.V // SEED\n\tctx.HTML(code, \"t.tmpl\", x) // SINK\n}\n", S),
        go("HTML on gin.Context by value", "package p\nfunc h(ctx gin.Context, in Input) {\n\tx := in.V // SEED\n\tctx.HTML(200, \"t.tmpl\", x) // SINK\n}\n", S),
        go("HTML on an unknown receiver with http.StatusOK", "package p\nfunc h(c Ctx, in Input) {\n\tx := in.V // SEED\n\tc.HTML(http.StatusOK, \"t.tmpl\", x) // SINK\n}\n", W),
        go("HTML raw renderer type with an int status", "package p\ntype Raw struct{ w http.ResponseWriter }\nfunc (c Raw) HTML(code int, body string, _ any) {}\nfunc h(c Raw, in Input) {\n\tx := in.V // SEED\n\tc.HTML(200, x, nil) // SINK\n}\n", W),
    ]);
}

#[test]
fn js_parameter_counts_as_a_declaration() {
    assert_severities(&[
        js("R2-J2c parameter read before a later var of the same name", "function h(knex, q, input) {\n  const x = input.v + q; // SEED\n  knex('t').whereRaw(q, [x]); // SINK\n  var q = 'id = ?';\n}\n", W),
        js("arrow parameter read before a later var", "const h = (knex, q, input) => {\n  const x = input.v + q; // SEED\n  knex('t').whereRaw(q, [x]); // SINK\n  var q = 'id = ?';\n};\n", W),
        js("a different parameter name keeps the constant weak", "function h(knex, other, input) {\n  const x = input.v; // SEED\n  var q = 'id = ?';\n  knex('t').whereRaw(q, [x]); // SINK\n}\n", S),
    ]);
}
