//! #636 round 1: weak needs proof. Exactly-one-binding rules, shadowed imports, SQLAlchemy chain
//! and clause allowlists, structured non-SQL sinks that mask a weak call, `createQueryBuilder`,
//! and the Go HTML render shape. Anything unproven stays `warning`.

use super::taint_sink_strength_test::*;
use crate::common::*;

#[test]
fn go_weak_needs_proof() {
    assert_severities(&[
        go("redeclared by := with a new name", "package p\nfunc h(db *sql.DB, in Input) {\n\tq := \"SELECT a FROM t WHERE 1=1\"\n\tx := in.V // SEED\n\tq, n := addFilter(q, x)\n\t_ = n\n\tdb.Query(q) // SINK\n}\n", W),
        go("redeclared by := direct concat", "package p\nfunc h(db *sql.DB, in Input) {\n\tq := \"SELECT a FROM t WHERE 1=1\"\n\tx := in.V // SEED\n\tq, n := q+\" AND b = '\"+x+\"'\", 1\n\t_ = n\n\tdb.Query(q) // SINK\n}\n", W),
        go("redeclared by var", "package p\nfunc h(db *sql.DB, in Input) {\n\tx := in.V // SEED\n\tvar q = \"SELECT a FROM t\"\n\tvar q = \"x\" + x\n\tdb.Query(q) // SINK\n}\n", W),
        go("multi-assign", "package p\nfunc h(db *sql.DB, in Input) {\n\tx := in.V // SEED\n\tq := \"SELECT a FROM t\"\n\tq, y = \"z\"+x, 1\n\tdb.Query(q) // SINK\n}\n", W),
        go("same name literal in two functions is still one binding each", "package p\nfunc other(db *sql.DB) {\n\tq := \"SELECT 1\"\n\tdb.Query(q)\n}\nfunc h(db *sql.DB, in Input) {\n\tx := in.V // SEED\n\tq := \"SELECT a FROM t WHERE id = $1\"\n\tdb.Query(q, x) // SINK\n}\n", S),
        go("iris printf-form HTML", "package p\nimport \"github.com/kataras/iris/v12\"\nfunc h(ctx iris.Context, in Input) {\n\tx := in.V // SEED\n\tctx.HTML(\"<b>%s</b> %s\", x, \"!\") // SINK\n}\n", W),
        go("HTML with http.Status first arg on an unknown receiver", "package p\nfunc h(c Ctx, in Input) {\n\tx := in.V // SEED\n\tc.HTML(http.StatusOK, \"t.tmpl\", x) // SINK\n}\n", W),
        go("HTML with unknown first arg and receiver", "package p\nfunc h(c Ctx, code int, in Input) {\n\tx := in.V // SEED\n\tc.HTML(code, \"t.tmpl\", x) // SINK\n}\n", W),
    ]);
}

#[test]
fn js_weak_needs_proof() {
    assert_severities(&[
        js("var redeclared in a branch", "function h(db, input) {\n  var sql = 'SELECT a FROM t';\n  const x = input.v; // SEED\n  if (x) {\n    var sql = 'SELECT a FROM t WHERE id = ' + x;\n  }\n  knex('t').whereRaw(sql); // SINK\n}\n", W),
        js("var redeclared in the same block", "function h(db, input) {\n  var sql = 'SELECT a FROM t';\n  const x = input.v; // SEED\n  var sql = sql + ' WHERE id = ' + x;\n  knex('t').whereRaw(sql); // SINK\n}\n", W),
        js("sol: var redeclared with initializer", "function h(db, input) {\n  const x = input.v; // SEED\n  var query = 'id = ?';\n  var query = 'id = ' + x;\n  knex('t').whereRaw(query); // SINK\n}\n", W),
        js("single const, same name in another function", "function other(knex) {\n  const q = \"id = 1\";\n  knex(\"t\").whereRaw(q);\n}\nfunction h(knex, input) {\n  const x = input.v; // SEED\n  const q = \"id = ?\";\n  knex(\"t\").whereRaw(q, [x]); // SINK\n}\n", S),
        js("typeorm createQueryBuilder with template where", "async function h(repo, input) {\n  const x = input.v; // SEED\n  return repo.createQueryBuilder('user').where(`user.name = '${x}'`).getMany(); // SINK\n}\n", W),
        js("typeorm createQueryBuilder with concat where", "async function h(repo, input) {\n  const x = input.v; // SEED\n  return repo.createQueryBuilder('user').where(\"user.name = '\" + x + \"'\").getMany(); // SINK\n}\n", W),
        js("readFileSync interior arg of a weak multi-line query", "const fs = require('fs');\nfunction h(db, req) {\n  const p = req.query.p; // SEED\n  db.query('INSERT INTO t VALUES (?)', [\n    fs.readFileSync(p), // SINK\n  ]);\n}\n", W),
        js("axios.get interior arg of a weak multi-line query", "const axios = require('axios');\nfunction h(db, req) {\n  const u = req.query.u; // SEED\n  db.query('INSERT INTO t VALUES (?)', [\n    axios.get(u), // SINK\n  ]);\n}\n", W),
        js("axios.get on the same line as a weak query", "const axios = require('axios');\nfunction h(db, req) {\n  const u = req.query.u; // SEED\n  db.query('INSERT INTO t VALUES (?)', [await axios.get(u)]); // SINK\n}\n", W),
    ]);
}

#[test]
fn python_weak_needs_proof() {
    assert_severities(&[
        py("Select-annotated param rebound to a string", "import sqlalchemy as sa\ndef h(session, stmt: sa.Select, inp):\n    x = inp.v  # SEED\n    stmt = \"SELECT a FROM t WHERE id = \" + x\n    session.execute(stmt)  # SINK\n", W),
        py("sol: Select-annotated param reassigned, cursor.execute", "import sqlalchemy as sa\ndef h(cur, stmt: sa.Select, req):\n    x = req.GET[\"x\"]  # SEED\n    stmt = \"SELECT * FROM t WHERE id = \" + x\n    cur.execute(stmt)  # SINK\n", W),
        py("Executable annotation is not proof", "from sqlalchemy.sql import Executable\ndef h(session, stmt: Executable, inp):\n    x = inp.v  # SEED\n    session.execute(stmt, {\"a\": x})  # SINK\n", W),
        py("text from sqlalchemy.sql with an f-string inside select", "from sqlalchemy.sql import select, text\ndef h(session, t, inp):\n    x = inp.v  # SEED\n    session.execute(select(t).where(text(f\"a = '{x}'\")))  # SINK\n", W),
        py("text from sqlalchemy.sql, statement built on its own line", "from sqlalchemy.sql import select, text\ndef h(session, t, inp):\n    x = inp.v  # SEED\n    stmt = select(t).where(text(f\"a = '{x}'\"))  # SINK\n    session.execute(stmt)\n", W),
        py("literal_column(non-literal) in order_by", "from sqlalchemy import select, literal_column\ndef h(session, t, inp):\n    x = inp.v  # SEED\n    session.execute(select(t).order_by(literal_column(x)))  # SINK\n", W),
        py("text from sqlalchemy.sql in filter", "from sqlalchemy.sql import text\ndef h(session, inp):\n    x = inp.v  # SEED\n    return session.query(User).filter(text(f\"name = '{x}'\")).all()  # SINK\n", W),
        py("text from sqlalchemy.sql.expression in where", "from sqlalchemy.sql.expression import text\ndef h(q, inp):\n    x = inp.v  # SEED\n    return q.where(text(\"name = '\" + x + \"'\"))  # SINK\n", W),
        py("suffix_with takes raw SQL", "import sqlalchemy as sa\ndef h(session, t, req):\n    x = req.GET[\"x\"]  # SEED\n    session.execute(sa.select(t.c.id).suffix_with(x))  # SINK\n", W),
        py("peewee SQL() in where", "from peewee import SQL\ndef h(req):\n    x = req.GET[\"x\"]  # SEED\n    return Model.select().where(SQL(\"id = \" + x))  # SINK\n", W),
        py("sa parameter shadows the import", "import sqlalchemy as sa\ndef h(cur, sa, req):\n    x = req.GET[\"x\"]  # SEED\n    cur.execute(sa.select(x))  # SINK\n", W),
        py("select parameter shadows the imported constructor", "from sqlalchemy import select\ndef h(cur, select, req):\n    x = req.GET[\"x\"]  # SEED\n    cur.execute(select(x))  # SINK\n", W),
        py("percent string bound once", "def h(cur, inp):\n    x = inp.v  # SEED\n    q = \"SELECT a FROM t WHERE id = %s\" % x\n    cur.execute(q)  # SINK\n", W),
        py("safe chain with filter and order_by", "import sqlalchemy as sa\nfrom app.models import t\ndef h(session, inp):\n    x = inp.v  # SEED\n    session.execute(sa.select(t).filter(t.c.a == x).order_by(t.c.id).limit(5))  # SINK\n", S),
        py("text literal with bindparams is cleansed (no finding)", "from sqlalchemy import text\ndef h(session, inp):\n    x = inp.v  # SEED\n    session.execute(text(\"select 1 where a = :a\").bindparams(a=x))  # SINK\n", NONE),
        py("requests.get inside a weak execute (same line)", "import requests\nQ = \"INSERT INTO t VALUES (%s)\"\ndef h(cur, inp):\n    u = inp.v  # SEED\n    cur.execute(Q, (requests.get(u).text,))  # SINK\n", W),
        py("requests.get interior of a weak execute", "import requests\nQ = \"INSERT INTO t VALUES (%s)\"\ndef h(cur, inp):\n    u = inp.v  # SEED\n    cur.execute(Q, (\n        requests.get(u).text,  # SINK\n    ))\n", W),
        py("open() on the same line as a weak execute", "Q = \"INSERT INTO t VALUES (%s)\"\ndef h(cur, inp):\n    u = inp.v  # SEED\n    cur.execute(Q, (open(u).read(),))  # SINK\n", W),
        py("ORM filter with a comparison stays weak", "def h(q, inp):\n    x = inp.v  # SEED\n    return q.filter(User.id == x)  # SINK\n", S),
        py("ORM filter with a bare identifier is unproven", "def h(q, inp):\n    x = inp.v  # SEED\n    return q.filter(x)  # SINK\n", W),
        py("ORM filter with text(f-string)", "from sqlalchemy import text\ndef h(q, inp):\n    x = inp.v  # SEED\n    return q.filter(text(f\"a = {x}\"))  # SINK\n", W),
    ]);
}
