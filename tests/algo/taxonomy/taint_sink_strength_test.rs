//! #636 variant matrix: a weak `taint_sink` finding (sensitive operation reached but
//! defused: bound-parameter SQL, ORM expression clause, auto-escaped template render)
//! drops to `suggestion`; everything else stays `warning`. The set of findings never changes.

use super::taint_sink_rules_support::*;
use crate::common::*;

struct Sev {
    label: &'static str,
    path: &'static str,
    lang: Language,
    source: &'static str,
    severity: &'static str,
}

fn sink_severity(path: &str, lang: Language, source: &str) -> Option<String> {
    let sink = marker_line(source, "SINK");
    findings(path, lang, source)
        .findings
        .iter()
        .find(|f| f.category.as_deref() == Some("taint_sink") && f.line == sink)
        .map(|f| f.severity.clone())
}

fn assert_severities(rows: &[Sev]) {
    let wrong: Vec<String> = rows
        .iter()
        .filter_map(|r| {
            let got = sink_severity(r.path, r.lang, r.source);
            let want = (r.severity != NONE).then_some(r.severity);
            (got.as_deref() != want)
                .then(|| format!("{}: expected {} got {:?}", r.label, r.severity, got))
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "{} rows differ:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

const W: &str = "warning";
const S: &str = "suggestion";
/// No finding: the tier is a view, so a shape main never reports stays unreported.
const NONE: &str = "none";

fn go(label: &'static str, source: &'static str, severity: &'static str) -> Sev {
    Sev {
        label,
        path: "a.go",
        lang: Language::Go,
        source,
        severity,
    }
}
fn py(label: &'static str, source: &'static str, severity: &'static str) -> Sev {
    Sev {
        label,
        path: "a.py",
        lang: Language::Python,
        source,
        severity,
    }
}
fn js(label: &'static str, source: &'static str, severity: &'static str) -> Sev {
    Sev {
        label,
        path: "a.js",
        lang: Language::JavaScript,
        source,
        severity,
    }
}

#[test]
fn go_strength_matrix() {
    assert_severities(&[
        go("bound-parameter Query", "package p\nfunc h(db *sql.DB, in Input) {\n\tx := in.V // SEED\n\tdb.Query(\"SELECT a FROM t WHERE id = $1\", x) // SINK\n}\n", S),
        go("concatenated Query", "package p\nfunc h(db *sql.DB, in Input) {\n\tx := in.V // SEED\n\tdb.Query(\"SELECT a FROM t WHERE id = \" + x) // SINK\n}\n", W),
        go("Sprintf Query", "package p\nfunc h(db *sql.DB, in Input) {\n\tx := in.V // SEED\n\tdb.Query(fmt.Sprintf(\"SELECT a FROM t WHERE id = %s\", x)) // SINK\n}\n", W),
        go("const query Exec", "package p\nconst q = \"DELETE FROM t WHERE id = $1\"\nfunc h(db *sql.DB, in Input) {\n\tx := in.V // SEED\n\tdb.Exec(q, x) // SINK\n}\n", S),
        go("raw-string query", "package p\nfunc h(db *sql.DB, in Input) {\n\tx := in.V // SEED\n\tdb.Exec(`DELETE FROM t WHERE id = $1`, x) // SINK\n}\n", S),
        go("ExecContext constant query", "package p\nfunc h(ctx context.Context, db *sql.DB, in Input) {\n\tx := in.V // SEED\n\tdb.ExecContext(ctx, \"DELETE FROM t WHERE id = $1\", x) // SINK\n}\n", S),
        go("ExecContext tainted query", "package p\nfunc h(ctx context.Context, db *sql.DB, in Input) {\n\tx := in.V // SEED\n\tdb.ExecContext(ctx, x) // SINK\n}\n", W),
        go("var (not const) query", "package p\nfunc h(db *sql.DB, in Input) {\n\tx := in.V // SEED\n\tq := \"SELECT \" + x\n\tdb.Query(q) // SINK\n}\n", W),
        go("gin render", "package p\nfunc h(ctx *gin.Context, in Input) {\n\tx := in.V // SEED\n\tctx.HTML(200, \"t.tmpl\", x) // SINK\n}\n", S),
        go("echo raw HTML", "package p\nfunc h(c echo.Context, in Input) {\n\tx := in.V // SEED\n\tc.HTML(200, x) // SINK\n}\n", W),
        go("template.HTML", "package p\nfunc h(in Input) {\n\tx := in.V // SEED\n\ty := template.HTML(x) // SINK\n\t_ = y\n}\n", W),
        go("exec.Command", "package p\nfunc h(in Input) {\n\tx := in.V // SEED\n\texec.Command(x).Run() // SINK\n}\n", W),
        go("weak and strong on one line", "package p\nfunc h(db *sql.DB, in Input) {\n\tx := in.V // SEED\n\tdb.Query(\"SELECT a FROM t WHERE id = $1\", x); exec.Command(x).Run() // SINK\n}\n", W),
    ]);
}

#[test]
fn python_strength_matrix() {
    assert_severities(&[
        py("bound-parameter execute is cleansed (no finding)", "def h(cur, inp):\n    x = inp.v  # SEED\n    cur.execute('SELECT a FROM t WHERE id = %s', (x,))  # SINK\n", NONE),
        py("f-string execute", "def h(cur, inp):\n    x = inp.v  # SEED\n    cur.execute(f'SELECT a FROM t WHERE id = {x}')  # SINK\n", W),
        py("concatenated execute", "def h(cur, inp):\n    x = inp.v  # SEED\n    cur.execute('SELECT a FROM t WHERE id = ' + x)  # SINK\n", W),
        py("percent execute", "def h(cur, inp):\n    x = inp.v  # SEED\n    cur.execute('SELECT a FROM t WHERE id = %s' % x)  # SINK\n", W),
        py("format execute", "def h(cur, inp):\n    x = inp.v  # SEED\n    cur.execute('SELECT {}'.format(x))  # SINK\n", W),
        py("module constant execute", "Q = 'SELECT a FROM t WHERE id = %s'\ndef h(cur, inp):\n    x = inp.v  # SEED\n    cur.execute(Q, (x,))  # SINK\n", S),
        py("rebound constant execute", "Q = 'SELECT a FROM t WHERE id = %s'\nQ = Q + 'x'\ndef h(cur, inp):\n    x = inp.v  # SEED\n    cur.execute(Q, (x,))  # SINK\n", W),
        py("tainted variable execute", "def h(cur, inp):\n    x = inp.v  # SEED\n    cur.execute(x)  # SINK\n", W),
        py("orm filter expression", "def h(q, inp):\n    x = inp.v  # SEED\n    return q.filter(User.id == x)  # SINK\n", S),
        py("sqlalchemy text non-literal", "from sqlalchemy import text\ndef h(session, inp):\n    x = inp.v  # SEED\n    session.execute(text(x))  # SINK\n", W),
        py("os.system", "import os\ndef h(inp):\n    x = inp.v  # SEED\n    os.system(x)  # SINK\n", W),
        py("weak and strong on one line", "import os\nQ = 'SELECT 1 WHERE a = %s'\ndef h(cur, inp):\n    x = inp.v  # SEED\n    cur.execute(Q, (x,)); os.system(x)  # SINK\n", W),
    ]);
}

#[test]
fn js_strength_matrix() {
    assert_severities(&[
        js("bound-parameter query is cleansed (no finding)", "function h(db, input) {\n  const x = input.v; // SEED\n  db.query('SELECT a FROM t WHERE id = $1', [x]); // SINK\n}\n", NONE),
        js("interpolated template query", "function h(db, input) {\n  const x = input.v; // SEED\n  db.query(`SELECT a FROM t WHERE id = ${x}`); // SINK\n}\n", W),
        js("concatenated query", "function h(db, input) {\n  const x = input.v; // SEED\n  db.query('SELECT a FROM t WHERE id = ' + x); // SINK\n}\n", W),
        js("whereRaw literal with bindings", "function h(knex, input) {\n  const x = input.v; // SEED\n  knex('t').whereRaw('id = ?', [x]); // SINK\n}\n", S),
        js("whereRaw template without substitution", "function h(knex, input) {\n  const x = input.v; // SEED\n  knex('t').whereRaw(`id = ?`, [x]); // SINK\n}\n", S),
        js("whereRaw const query", "const Q = 'id = ?';\nfunction h(knex, input) {\n  const x = input.v; // SEED\n  knex('t').whereRaw(Q, [x]); // SINK\n}\n", S),
        js("whereRaw reassigned query", "let Q = 'id = ?';\nQ += ' or 1';\nfunction h(knex, input) {\n  const x = input.v; // SEED\n  knex('t').whereRaw(Q, [x]); // SINK\n}\n", W),
        js("whereRaw interpolated", "function h(knex, input) {\n  const x = input.v; // SEED\n  knex('t').whereRaw(`id = ${x}`); // SINK\n}\n", W),
        js("whereRaw concatenated", "function h(knex, input) {\n  const x = input.v; // SEED\n  knex('t').whereRaw('id = ' + x); // SINK\n}\n", W),
        js("knex.raw interpolated", "function h(knex, input) {\n  const x = input.v; // SEED\n  knex.raw(`select * from t where id = ${x}`); // SINK\n}\n", W),
        js("res.redirect", "function h(req, res) {\n  const x = req.query.v; // SEED\n  res.redirect(x); // SINK\n}\n", W),
        js("child_process.exec", "const child_process = require('child_process');\nfunction h(input) {\n  const x = input.v; // SEED\n  child_process.exec(x); // SINK\n}\n", W),
        js("weak and strong on one line", "function h(knex, res, input) {\n  const x = input.v; // SEED\n  knex('t').whereRaw('id = ?', [x]); res.redirect(x); // SINK\n}\n", W),
    ]);
}

#[test]
fn other_languages_are_always_strong() {
    assert_severities(&[Sev {
        label: "C system",
        path: "a.c",
        lang: Language::C,
        source: "void h(char *in) {\n  char *x = in; // SEED\n  system(x); // SINK\n}\n",
        severity: W,
    }]);
}
