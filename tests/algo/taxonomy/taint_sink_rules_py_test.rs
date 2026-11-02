//! #622 S1 variant matrix, Python: review sinks match by call position and receiver.

use super::taint_sink_rules_support::*;
use crate::common::*;

#[rustfmt::skip]
const POSITIVES: &[Case] = &[
    Case { label: "eval", source: "def h(inp):\n    x = inp.v  # SEED\n    eval(x)  # SINK\n" },
    Case { label: "exec", source: "def h(inp):\n    x = inp.v  # SEED\n    exec(x)  # SINK\n" },
    Case { label: "os.system", source: "import os\ndef h(inp):\n    x = inp.v  # SEED\n    os.system(x)  # SINK\n" },
    Case { label: "os.popen", source: "import os\ndef h(inp):\n    x = inp.v  # SEED\n    os.popen(x)  # SINK\n" },
    Case { label: "subprocess.run", source: "import subprocess\ndef h(inp):\n    x = inp.v  # SEED\n    subprocess.run(x, shell=True)  # SINK\n" },
    Case { label: "aliased subprocess module", source: "import subprocess as sp\ndef h(inp):\n    x = inp.v  # SEED\n    sp.run(x, shell=True)  # SINK\n" },
    Case { label: "from subprocess import run", source: "from subprocess import run\ndef h(inp):\n    x = inp.v  # SEED\n    run(x, shell=True)  # SINK\n" },
    Case { label: "Popen", source: "import subprocess\ndef h(inp):\n    x = inp.v  # SEED\n    subprocess.Popen(x, shell=True)  # SINK\n" },
    Case { label: "check_output", source: "import subprocess\ndef h(inp):\n    x = inp.v  # SEED\n    subprocess.check_output(x, shell=True)  # SINK\n" },
    Case { label: "cursor.execute", source: "def h(cur, inp):\n    x = inp.v  # SEED\n    cur.execute(x)  # SINK\n" },
    Case { label: "objects.raw", source: "def h(inp):\n    x = inp.v  # SEED\n    return Model.objects.raw(x)  # SINK\n" },
    Case { label: "orm .filter clause", source: "def h(q, inp):\n    x = inp.v  # SEED\n    return q.filter(x)  # SINK\n" },
    Case { label: "orm .filter with an expression", source: "def h(q, inp):\n    x = inp.v  # SEED\n    return q.filter(User.id == x)  # SINK\n" },
    Case { label: "orm .where clause", source: "def h(q, inp):\n    x = inp.v  # SEED\n    return q.where(x)  # SINK\n" },
    Case { label: "sqlalchemy text with f-string", source: "from sqlalchemy import text\ndef h(inp):\n    x = inp.v  # SEED\n    return text(f\"select {x}\")  # SINK\n" },
    Case { label: "sa.text with concatenation", source: "import sqlalchemy as sa\ndef h(inp):\n    x = inp.v  # SEED\n    return sa.text('select ' + x)  # SINK\n" },
    Case { label: "mark_safe", source: "def h(inp):\n    x = inp.v  # SEED\n    return mark_safe(x)  # SINK\n" },
    Case { label: "Markup", source: "def h(inp):\n    x = inp.v  # SEED\n    return Markup(x)  # SINK\n" },
    Case { label: "render_template_string", source: "def h(inp):\n    x = inp.v  # SEED\n    return render_template_string(x)  # SINK\n" },
    Case { label: "getattr with a computed name", source: "def h(obj, inp):\n    x = inp.v  # SEED\n    return getattr(obj, x)  # SINK\n" },
    Case { label: "redirect", source: "def h(inp):\n    x = inp.v  # SEED\n    return redirect(x)  # SINK\n" },
    Case { label: "open call", source: "def h(inp):\n    x = inp.v  # SEED\n    return open(x)  # SINK\n" },
    Case { label: "os.remove", source: "import os\ndef h(inp):\n    x = inp.v  # SEED\n    os.remove(x)  # SINK\n" },
];

#[rustfmt::skip]
const NEGATIVES: &[Case] = &[
    Case { label: "get_user_id call (kernel pattern)", source: "def h(inp):\n    x = inp.v  # SEED\n    uid = get_user_id(x)  # SINK\n" },
    Case { label: "get_user call outside C", source: "def h(inp):\n    x = inp.v  # SEED\n    u = get_user(x)  # SINK\n" },
    Case { label: "def get_user_id signature", source: "def get_user_id(x):  # SEED SINK\n    return x\n" },
    Case { label: "getattr with a literal name", source: "def h(obj, inp):\n    x = inp.v  # SEED\n    return getattr(obj, 'name', x)  # SINK\n" },
    Case { label: "setattr with a literal name", source: "def h(obj, inp):\n    x = inp.v  # SEED\n    setattr(obj, 'name', x)  # SINK\n" },
    Case { label: "compile", source: "import re\ndef h(inp):\n    x = inp.v  # SEED\n    p = re.compile(x)  # SINK\n" },
    Case { label: "run name without subprocess binding", source: "def h(inp):\n    x = inp.v  # SEED\n    run = x  # SINK\n" },
    Case { label: "run call without subprocess binding", source: "def h(runner, inp):\n    x = inp.v  # SEED\n    runner.run(x)  # SINK\n" },
    Case { label: "variable named source", source: "def h(inp):\n    x = inp.v  # SEED\n    source = x  # SINK\n" },
    Case { label: "variable named raw", source: "def h(inp):\n    x = inp.v  # SEED\n    raw = x  # SINK\n" },
    Case { label: "revalidate (eval substring)", source: "def h(inp):\n    x = inp.v  # SEED\n    revalidate(x)  # SINK\n" },
    Case { label: "evaluate (eval substring)", source: "def h(inp):\n    x = inp.v  # SEED\n    evaluate(x)  # SINK\n" },
    Case { label: "render helper", source: "def h(request, inp):\n    x = inp.v  # SEED\n    return render(request, 't.html', x)  # SINK\n" },
    Case { label: "list.remove", source: "def h(items, inp):\n    x = inp.v  # SEED\n    items.remove(x)  # SINK\n" },
    Case { label: "data.find", source: "def h(data, inp):\n    x = inp.v  # SEED\n    return data.find(x)  # SINK\n" },
    Case { label: "socket send", source: "def h(sock, inp):\n    x = inp.v  # SEED\n    sock.send(x)  # SINK\n" },
    Case { label: "file write", source: "def h(f, inp):\n    x = inp.v  # SEED\n    f.write(x)  # SINK\n" },
    Case { label: "session.query (ORM entry, not raw SQL)", source: "def h(session, inp):\n    x = inp.v  # SEED\n    return session.query(x)  # SINK\n" },
    Case { label: "text with a literal", source: "from sqlalchemy import text\ndef h(inp):\n    x = inp.v  # SEED\n    return text('select 1'), x  # SINK\n" },
    Case { label: "text without sqlalchemy binding", source: "def h(inp):\n    x = inp.v  # SEED\n    return text(x)  # SINK\n" },
    Case { label: "execution_result variable", source: "def h(inp):\n    x = inp.v  # SEED\n    execution_result = x  # SINK\n" },
    Case { label: "django filter with keyword lookups", source: "def h(inp):\n    x = inp.v  # SEED\n    return Model.objects.filter(pk=x)  # SINK\n" },
    Case { label: "filter_by with keyword lookups", source: "def h(q, inp):\n    x = inp.v  # SEED\n    return q.filter_by(id=x)  # SINK\n" },
    Case { label: "lookup_where helper", source: "def h(inp):\n    x = inp.v  # SEED\n    return lookup_where(x)  # SINK\n" },
    Case { label: "redirect_url variable", source: "def h(inp):\n    x = inp.v  # SEED\n    redirect_url = x  # SINK\n" },
    Case { label: "freeze (free substring)", source: "def h(inp):\n    x = inp.v  # SEED\n    freeze(x)  # SINK\n" },
];

#[test]
fn python_genuine_call_sinks_are_reported() {
    assert_all_sinks("a.py", Language::Python, POSITIVES);
}

#[test]
fn python_name_fragments_are_not_sinks() {
    assert_no_sinks("a.py", Language::Python, NEGATIVES);
}
