//! #637 Slice A2: Python import-resolved module identity for review-taint sinks.
//!
//! A closed table per module (paramiko, pexpect, fabric / invoke, jinja2) fires, strong with a
//! CWE, when the callee is proven by a lexically resolved binding: an import under any local
//! name, or one hop through a single-binding local, a `with .. as` target or the first
//! parameter of an imported `@task`. Tables are positive evidence only: anything unproven keeps
//! main's result (no finding for these names).

use super::taint_sink_rules_support::*;
use super::taint_sink_strength_test::*;
use crate::common::*;

#[test]
fn py_module_identity_matrix() {
    assert_severities(&[
        py("P1 paramiko SSHClient() local exec_command", "import paramiko\ndef h(inp):\n    x = inp.v  # SEED\n    ssh = paramiko.SSHClient()\n    ssh.exec_command(x)  # SINK\n", W),
        py("P1b from paramiko import SSHClient alias", "from paramiko import SSHClient as C\ndef h(inp):\n    x = inp.v  # SEED\n    c = C()\n    c.exec_command(x)  # SINK\n", W),
        py("P1c paramiko untyped parameter: no fallback in this slice", "def h(ssh, inp):\n    x = inp.v  # SEED\n    ssh.exec_command(x)  # SINK\n", NONE),
        py("P1d import paramiko as pm; with SSHClient() as c", "import paramiko as pm\ndef h(inp):\n    x = inp.v  # SEED\n    with pm.SSHClient() as c:\n        c.exec_command(x)  # SINK\n", W),
        py("P2 pexpect.spawn", "import pexpect\ndef h(inp):\n    x = inp.v  # SEED\n    pexpect.spawn(x)  # SINK\n", W),
        py("P2b from pexpect import spawn as sp", "from pexpect import spawn as sp\ndef h(inp):\n    x = inp.v  # SEED\n    sp(x)  # SINK\n", W),
        py("P2c import pexpect as px; px.run", "import pexpect as px\ndef h(inp):\n    x = inp.v  # SEED\n    px.run(x)  # SINK\n", W),
        py("P2d pexpect.spawnu", "import pexpect\ndef h(inp):\n    x = inp.v  # SEED\n    pexpect.spawnu(x)  # SINK\n", W),
        py("P2e pexpect.runu", "import pexpect\ndef h(inp):\n    x = inp.v  # SEED\n    pexpect.runu(x)  # SINK\n", W),
        py("P2f PopenSpawn named import", "from pexpect.popen_spawn import PopenSpawn\ndef h(inp):\n    x = inp.v  # SEED\n    PopenSpawn(x)  # SINK\n", W),
        py("P2g PopenSpawn through the dotted module path", "import pexpect.popen_spawn\ndef h(inp):\n    x = inp.v  # SEED\n    pexpect.popen_spawn.PopenSpawn(x)  # SINK\n", W),
        py("P2h two imports of one module spelling", "import pexpect\nimport pexpect.popen_spawn\ndef h(inp):\n    x = inp.v  # SEED\n    pexpect.spawn(x)  # SINK\n", W),
        py("P3 fabric Connection with-as c.run", "from fabric import Connection\ndef h(host, inp):\n    x = inp.v  # SEED\n    with Connection(host) as c:\n        c.run(x)  # SINK\n", W),
        py("P3a fabric Connection one-hop local sudo", "from fabric import Connection\ndef h(host, inp):\n    x = inp.v  # SEED\n    c = Connection(host)\n    c.sudo(x)  # SINK\n", W),
        py("P3b fabric @task parameter c.run", "from fabric import task\n@task\ndef deploy(c, inp):\n    x = inp.v  # SEED\n    c.run(x)  # SINK\n", W),
        py("P3c invoke @task parameter c.run", "from invoke import task\n@task\ndef build(c, inp):\n    x = inp.v  # SEED\n    c.run(x)  # SINK\n", W),
        py("P3d invoke @task(..) call form, typed parameter", "from invoke import task, Context\n@task(name='b')\ndef build(c: Context, inp):\n    x = inp.v  # SEED\n    c.local(x)  # SINK\n", W),
        py("P3e fabric.task through the module", "import fabric\n@fabric.task\ndef deploy(c, inp):\n    x = inp.v  # SEED\n    c.run(x)  # SINK\n", W),
        py("P3f from fabric.api import run", "from fabric.api import run\ndef h(inp):\n    x = inp.v  # SEED\n    run(x)  # SINK\n", W),
        py("P3g import fabric.api as fa; fa.sudo", "import fabric.api as fa\ndef h(inp):\n    x = inp.v  # SEED\n    fa.sudo(x)  # SINK\n", W),
        py("P3h from invoke import run as r", "from invoke import run as r\ndef h(inp):\n    x = inp.v  # SEED\n    r(x)  # SINK\n", W),
        py("P3i fabric.api.local through the dotted path", "import fabric.api\ndef h(inp):\n    x = inp.v  # SEED\n    fabric.api.local(x)  # SINK\n", W),
        py("P6 jinja2 Environment().from_string", "import jinja2\ndef h(inp):\n    x = inp.v  # SEED\n    env = jinja2.Environment()\n    env.from_string(x).render()  # SINK\n", W),
        py("P6b from jinja2 import Environment as E", "from jinja2 import Environment as E\ndef h(inp):\n    x = inp.v  # SEED\n    e = E()\n    e.from_string(x)  # SINK\n", W),
        py("P6c jinja2.sandbox SandboxedEnvironment local: weak (sandboxed)", "from jinja2.sandbox import SandboxedEnvironment\ndef h(inp):\n    x = inp.v  # SEED\n    env = SandboxedEnvironment()\n    env.from_string(x)  # SINK\n", S),
        py("P6d sandbox through the dotted module path: weak", "import jinja2.sandbox\ndef h(inp):\n    x = inp.v  # SEED\n    env = jinja2.sandbox.SandboxedEnvironment()\n    env.from_string(x)  # SINK\n", S),
        py("P6e sandbox alias with-as: weak", "from jinja2.sandbox import SandboxedEnvironment as SE\ndef h(inp):\n    x = inp.v  # SEED\n    with SE() as env:\n        env.from_string(x)  # SINK\n", S),
        py("P6f sandbox weak plus a strong sink on the line stays warning", "import os\nfrom jinja2.sandbox import SandboxedEnvironment\ndef h(inp):\n    x = inp.v  # SEED\n    env = SandboxedEnvironment()\n    env.from_string(x); os.system(x)  # SINK\n", W),
        py("P6g jinja2 Environment module-level local used in a function", "import jinja2\nenv = jinja2.Environment()\ndef h(inp):\n    x = inp.v  # SEED\n    env.from_string(x)  # SINK\n", W),
    ]);
}

#[test]
fn py_module_identity_negatives() {
    assert_severities(&[
        py("N1 re.compile (deny: compile)", "import re\ndef h(inp):\n    x = inp.v  # SEED\n    re.compile(x)  # SINK\n", NONE),
        py("N4 run on a non-process receiver (asyncio.run)", "import asyncio\ndef h(inp):\n    x = inp.v  # SEED\n    asyncio.run(x)  # SINK\n", NONE),
        py("N4a from asyncio import run", "from asyncio import run\ndef h(inp):\n    x = inp.v  # SEED\n    run(x)  # SINK\n", NONE),
        py("N4b Fabric-shaped c.run without a fabric/invoke import", "def deploy(c, inp):\n    x = inp.v  # SEED\n    c.run(x)  # SINK\n", NONE),
        py("N4c @task not imported from fabric/invoke", "from celery import task\n@task\ndef deploy(c, inp):\n    x = inp.v  # SEED\n    c.run(x)  # SINK\n", NONE),
        py("N4d locally defined task decorator", "def task(f):\n    return f\n@task\ndef deploy(c, inp):\n    x = inp.v  # SEED\n    c.run(x)  # SINK\n", NONE),
        py("N4e @task parameter that is not the first", "from fabric import task\n@task\ndef deploy(inp, c):\n    x = inp.v  # SEED\n    c.run(x)  # SINK\n", NONE),
        py("N4f @task parameter reassigned", "from fabric import task\n@task\ndef deploy(c, inp):\n    x = inp.v  # SEED\n    c = make()\n    c.run(x)  # SINK\n", NONE),
        py("N4g lambda parameter shadows the @task parameter", "from fabric import task\n@task\ndef deploy(c, inp):\n    x = inp.v  # SEED\n    go = lambda c: c.run(x)  # SINK\n", NONE),
        py("N4h unrelated function with a parameter named c, task import present", "from fabric import task\ndef other(c, inp):\n    x = inp.v  # SEED\n    c.run(x)  # SINK\n", NONE),
        py("N6 in-repo wrapper named exec_command", "class Runner:\n    def exec_command(self, cmd):\n        return cmd\ndef h(r, inp):\n    x = inp.v  # SEED\n    r.exec_command(x)  # SINK\n", NONE),
        py("N6a paramiko imported, exec_command on an unrelated receiver", "import paramiko\ndef h(r, inp):\n    x = inp.v  # SEED\n    r.exec_command(x)  # SINK\n", NONE),
        py("N6b paramiko local, method not in the table", "import paramiko\ndef h(inp):\n    x = inp.v  # SEED\n    ssh = paramiko.SSHClient()\n    ssh.connect(x)  # SINK\n", NONE),
        py("N6c paramiko local rebound", "import paramiko\ndef h(inp):\n    x = inp.v  # SEED\n    ssh = paramiko.SSHClient()\n    ssh = make()\n    ssh.exec_command(x)  # SINK\n", NONE),
        py("N6d paramiko local second binding in a loop", "import paramiko\ndef h(inp, hosts):\n    x = inp.v  # SEED\n    ssh = paramiko.SSHClient()\n    for ssh in hosts:\n        pass\n    ssh.exec_command(x)  # SINK\n", NONE),
        py("N6e local bound to a non-table call of the module", "import paramiko\ndef h(inp):\n    x = inp.v  # SEED\n    ssh = paramiko.Transport()\n    ssh.exec_command(x)  # SINK\n", NONE),
        py("N6f shadowing parameter of the instance name", "import paramiko\nssh = paramiko.SSHClient()\ndef h(ssh, inp):\n    x = inp.v  # SEED\n    ssh.exec_command(x)  # SINK\n", NONE),
        py("N6g field receiver is out of one-hop scope", "import paramiko\nclass R:\n    def __init__(self):\n        self.ssh = paramiko.SSHClient()\n    def h(self, inp):\n        x = inp.v  # SEED\n        self.ssh.exec_command(x)  # SINK\n", NONE),
        py("N6h annotated parameter is out of scope", "import paramiko\ndef h(ssh: paramiko.SSHClient, inp):\n    x = inp.v  # SEED\n    ssh.exec_command(x)  # SINK\n", NONE),
        py("N9 rebound pexpect name is no proof", "import pexpect\ndef h(inp):\n    pexpect = make_fake()\n    x = inp.v  # SEED\n    pexpect.spawn(x)  # SINK\n", NONE),
        py("N9a shadowing parameter named pexpect", "import pexpect\ndef h(pexpect, inp):\n    x = inp.v  # SEED\n    pexpect.spawn(x)  # SINK\n", NONE),
        py("N9b pexpect function not in the table", "import pexpect\ndef h(inp):\n    x = inp.v  # SEED\n    pexpect.which(x)  # SINK\n", NONE),
        py("N9c spawn from a module that is not pexpect", "from ptyx import spawn\ndef h(inp):\n    x = inp.v  # SEED\n    spawn(x)  # SINK\n", NONE),
        py("N9d same-named import from two modules is no proof", "try:\n    from pexpect import spawn\nexcept ImportError:\n    from other import spawn\ndef h(inp):\n    x = inp.v  # SEED\n    spawn(x)  # SINK\n", NONE),
        py("N11 logger receiver on a fallback name", "def h(logger, inp):\n    x = inp.v  # SEED\n    logger.execute_hook(x)  # SINK\n", NONE),
        py("N13 jinja2 Environment held in a field (out of one-hop scope, D7)", "from jinja2 import Environment\nclass P:\n    def __init__(self):\n        self.env = Environment()\n    def h(self, inp):\n        x = inp.v  # SEED\n        self.env.from_string(x)  # SINK\n", NONE),
        py("N13a jinja2 local rebound", "import jinja2\ndef h(inp):\n    x = inp.v  # SEED\n    env = jinja2.Environment()\n    env = other()\n    env.from_string(x)  # SINK\n", NONE),
        py("N13b Environment local, method not in the table", "import jinja2\ndef h(inp):\n    x = inp.v  # SEED\n    env = jinja2.Environment()\n    env.get_template(x)  # SINK\n", NONE),
        py("N13c from_string on an unknown receiver", "def h(env, inp):\n    x = inp.v  # SEED\n    env.from_string(x)  # SINK\n", NONE),
        py("N14 with Connection(..) as c rebound inside the body", "from fabric import Connection\ndef h(host, inp):\n    x = inp.v  # SEED\n    with Connection(host) as c:\n        c = make()\n        c.run(x)  # SINK\n", NONE),
        py("N14a Connection local used after a for-loop target of the same name", "from fabric import Connection\ndef h(hosts, inp):\n    x = inp.v  # SEED\n    for c in Connection(hosts):\n        c.run(x)  # SINK\n", NONE),
        py("N14b run on a Connection but not a table name", "from fabric import Connection\ndef h(host, inp):\n    x = inp.v  # SEED\n    c = Connection(host)\n    c.put(x)  # SINK\n", NONE),
    ]);
}

fn sink_count(source: &str, line_marker: &str) -> Vec<String> {
    let line = marker_line(source, line_marker);
    findings("a.py", Language::Python, source)
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("taint_sink") && f.line == line)
        .map(|f| f.severity.clone())
        .collect()
}

#[test]
fn table_hit_on_a_line_main_reports_stays_one_strong_finding() {
    let src = "import os\nimport pexpect\ndef h(inp):\n    x = inp.v  # SEED\n    os.system(x); pexpect.spawn(x)  # SINK\n";
    assert_eq!(sink_count(src, "SINK"), vec!["warning".to_string()]);
    let src = "import pexpect\ndef h(inp):\n    x = inp.v  # SEED\n    pexpect.spawn(x)  # SINK\n";
    assert_eq!(sink_count(src, "SINK"), vec!["warning".to_string()]);
}

#[test]
fn fold_r1_lexical_identity() {
    assert_severities(&[
        py("R1a class-body import bound to a builtin name", "class Helpers:\n    from invoke import run as len\ndef h(inp):\n    x = inp.v  # SEED\n    return len(x)  # SINK\n", NONE),
        py("R1b import inside another function", "def setup():\n    import pexpect\ndef h(inp):\n    x = inp.v  # SEED\n    pexpect.spawn(x)  # SINK\n", NONE),
        py("R1c import inside a class body", "class K:\n    import pexpect\ndef h(inp):\n    x = inp.v  # SEED\n    pexpect.spawn(x)  # SINK\n", NONE),
        py("R1d unrelated parameter of the same name elsewhere", "from invoke import run\ndef adapter(run, v):\n    return run(v)\ndef h(inp):\n    x = inp.v  # SEED\n    run(x)  # SINK\n", W),
        py("R1e function-local import used in that function", "def h(inp):\n    import pexpect\n    x = inp.v  # SEED\n    pexpect.spawn(x)  # SINK\n", W),
        py("R1f function-local import shadowed by a parameter of the use", "import pexpect\ndef h(pexpect, inp):\n    x = inp.v  # SEED\n    pexpect.spawn(x)  # SINK\n", NONE),
        py("R1g conflicting imports in one scope", "from pexpect import spawn\nfrom ptyx import spawn\ndef h(inp):\n    x = inp.v  # SEED\n    spawn(x)  # SINK\n", NONE),
    ]);
}

#[test]
fn fold_r1_star_imports() {
    assert_severities(&[
        py("S12 star import can replace the name", "import pexpect\nfrom fake import *\ndef h(inp):\n    x = inp.v  # SEED\n    pexpect.spawn(x)  # SINK\n", NONE),
        py("R2a star import after the proving import (direct call)", "from invoke import run\nfrom safe_helpers import *\ndef h(inp):\n    x = inp.v  # SEED\n    run(x)  # SINK\n", NONE),
        py("R2b star import of the proving module is harmless", "from invoke import *\nfrom invoke import run\ndef h(inp):\n    x = inp.v  # SEED\n    run(x)  # SINK\n", W),
        py("R2c star import can replace a module-level one-hop local", "import paramiko\nfrom paramiko import *\nssh = paramiko.SSHClient()\ndef h(inp):\n    x = inp.v  # SEED\n    ssh.exec_command(x)  # SINK\n", NONE),
        py("R2d function-local binding is out of the star import's reach", "from fake import *\ndef h(inp):\n    import paramiko\n    ssh = paramiko.SSHClient()\n    x = inp.v  # SEED\n    ssh.exec_command(x)  # SINK\n", W),
    ]);
}

#[test]
// Class-body taint does not reach the sink end to end, so the class-comprehension rows live in the
// classifier unit tests (`tests_py_modules`); this row pins the function-scope control.
fn fold_r1_class_comprehension() {
    assert_severities(&[
        py("R3c comprehension in a function sees the function local", "from jinja2.sandbox import SandboxedEnvironment\ndef h(inp):\n    env = SandboxedEnvironment()\n    x = inp.v  # SEED\n    return [env.from_string(x) for _ in range(2)]  # SINK\n", S),
    ]);
}

#[test]
fn fold_r1_jinja_literal_template() {
    assert_severities(&[
        py("L1 literal template text", "import jinja2\nenv = jinja2.Environment()\ndef h(inp):\n    x = inp.v  # SEED\n    return env.from_string('Hi {{ n }}').render(n=x)  # SINK\n", NONE),
        py("L1b literal template text, sandboxed", "from jinja2.sandbox import SandboxedEnvironment\nenv = SandboxedEnvironment()\ndef h(inp):\n    x = inp.v  # SEED\n    return env.from_string('Hi {{ n }}').render(n=x)  # SINK\n", NONE),
        py("L1c literal text with a tainted globals keyword", "import jinja2\nenv = jinja2.Environment()\ndef h(inp):\n    x = inp.v  # SEED\n    t = env.from_string('Hi {{ n }}', globals={'n': x})  # SINK\n", NONE),
        py("L1d literal source= keyword", "import jinja2\nenv = jinja2.Environment()\ndef h(inp):\n    x = inp.v  # SEED\n    t = env.from_string(source='Hi {{ n }}', globals={'n': x})  # SINK\n", NONE),
        py("L4 template text held in a constant name stays reported", "import jinja2\nT = 'Hi {{ n }}'\nenv = jinja2.Environment()\ndef h(inp):\n    x = inp.v  # SEED\n    return env.from_string(T).render(n=x)  # SINK\n", W),
        py("L4b tainted source= keyword", "import jinja2\nenv = jinja2.Environment()\ndef h(inp):\n    x = inp.v  # SEED\n    t = env.from_string(source=x)  # SINK\n", W),
        py("L4c interpolated f-string template text", "import jinja2\nenv = jinja2.Environment()\ndef h(inp):\n    x = inp.v  # SEED\n    t = env.from_string(f'Hi {x}')  # SINK\n", W),
    ]);
}

#[test]
fn fold_r1_constructor_call_receiver() {
    assert_severities(&[
        py("Z1 Connection(host).run", "from fabric import Connection\ndef h(host, inp):\n    x = inp.v  # SEED\n    Connection(host).run(x)  # SINK\n", W),
        py("Z2 jinja2.Environment().from_string", "import jinja2\ndef h(inp):\n    x = inp.v  # SEED\n    return jinja2.Environment().from_string(x).render()  # SINK\n", W),
        py("Z3 SandboxedEnvironment().from_string: weak", "from jinja2.sandbox import SandboxedEnvironment\ndef h(inp):\n    x = inp.v  # SEED\n    return SandboxedEnvironment().from_string(x).render()  # SINK\n", S),
        py("Z4 paramiko.SSHClient().exec_command", "import paramiko\ndef h(inp):\n    x = inp.v  # SEED\n    paramiko.SSHClient().exec_command(x)  # SINK\n", W),
        py("Z5 unknown constructor receiver", "def h(inp):\n    x = inp.v  # SEED\n    make().run(x)  # SINK\n", NONE),
        py("Z6 listed class, method not in the table", "from fabric import Connection\ndef h(host, inp):\n    x = inp.v  # SEED\n    Connection(host).put(x)  # SINK\n", NONE),
        py("Z7 constructor of an unlisted class of a listed module", "import paramiko\ndef h(inp):\n    x = inp.v  # SEED\n    paramiko.Transport().exec_command(x)  # SINK\n", NONE),
    ]);
}

#[test]
fn fold_r1_fabric1_task_paths() {
    assert_severities(&[
        py("T4 Fabric 1 task parameter is a CLI string", "from fabric.api import task\n@task\ndef deploy(branch, inp):\n    x = inp.v  # SEED\n    branch.local(x)  # SINK\n", NONE),
        py("T4b fabric.decorators.task", "from fabric.decorators import task\n@task\ndef deploy(branch, inp):\n    x = inp.v  # SEED\n    branch.run(x)  # SINK\n", NONE),
        py("T6 stacked decorators keep the invoke task", "from invoke import task\nimport functools\n@functools.lru_cache\n@task(help={})\ndef build(ctx, inp):\n    x = inp.v  # SEED\n    ctx.sudo(x)  # SINK\n", W),
    ]);
}

#[test]
fn fold_r1_scope_variants() {
    assert_severities(&[
        py("S1 global rebind elsewhere", "import paramiko\nssh = paramiko.SSHClient()\ndef reset():\n    global ssh\n    ssh = object()\ndef h(inp):\n    x = inp.v  # SEED\n    ssh.exec_command(x)  # SINK\n", NONE),
        py("S2 nonlocal rebind", "import paramiko\ndef outer(inp):\n    ssh = paramiko.SSHClient()\n    def inner():\n        nonlocal ssh\n        ssh = object()\n    x = inp.v  # SEED\n    ssh.exec_command(x)  # SINK\n", NONE),
        py("S3 except-as on the instance name", "import paramiko\ndef h(inp):\n    ssh = paramiko.SSHClient()\n    try:\n        pass\n    except Exception as ssh:\n        pass\n    x = inp.v  # SEED\n    ssh.exec_command(x)  # SINK\n", NONE),
        py("S4 del then use", "import paramiko\ndef h(inp):\n    ssh = paramiko.SSHClient()\n    del ssh\n    x = inp.v  # SEED\n    ssh.exec_command(x)  # SINK\n", NONE),
        py("S5 walrus rebind", "import paramiko\ndef h(inp):\n    ssh = paramiko.SSHClient()\n    if (ssh := other()):\n        pass\n    x = inp.v  # SEED\n    ssh.exec_command(x)  # SINK\n", NONE),
        py("S6 class-body local is not visible to a method", "import paramiko\nssh = object()\nclass R:\n    ssh = paramiko.SSHClient()\n    def h(self, inp):\n        x = inp.v  # SEED\n        ssh.exec_command(x)  # SINK\n", NONE),
        py("S7 positive: closure over the @task parameter", "from invoke import task\n@task\ndef deploy(c, inp):\n    def go():\n        x = inp.v  # SEED\n        c.run(x)  # SINK\n    go()\n", W),
        py("S8 comprehension variable of the instance name", "import paramiko\ndef h(inp, hosts):\n    x = inp.v  # SEED\n    [ssh.exec_command(x) for ssh in hosts]  # SINK\n", NONE),
        py("S10 with target rebound by a for loop", "from fabric import Connection\ndef h(host, hosts, inp):\n    x = inp.v  # SEED\n    with Connection(host) as c:\n        for c in hosts:\n            pass\n        c.run(x)  # SINK\n", NONE),
        py("S11 except-as on the module name", "import pexpect\ndef h(inp):\n    try:\n        pass\n    except Exception as pexpect:\n        pass\n    x = inp.v  # SEED\n    pexpect.spawn(x)  # SINK\n", NONE),
        py("S13 match capture rebinds the instance", "import paramiko\ndef h(inp, v):\n    ssh = paramiko.SSHClient()\n    match v:\n        case ssh:\n            pass\n    x = inp.v  # SEED\n    ssh.exec_command(x)  # SINK\n", NONE),
        py("X5 @task on a method: self is the first parameter", "from invoke import task\nclass K:\n    @task\n    def m(self, c, inp):\n        x = inp.v  # SEED\n        c.run(x)  # SINK\n", NONE),
        py("T1 try/except import of the same class from two paths is no proof", "try:\n    from fabric import Connection\nexcept ImportError:\n    from fabric.connection import Connection\ndef h(host, inp):\n    x = inp.v  # SEED\n    c = Connection(host)\n    c.run(x)  # SINK\n", NONE),
        py("T9 paramiko.client.SSHClient", "from paramiko.client import SSHClient\ndef h(inp):\n    x = inp.v  # SEED\n    c = SSHClient()\n    c.exec_command(x)  # SINK\n", W),
    ]);
}
