//! Python module-identity tables (#637 A2): rule, CWE and strength read off the classifier.

use super::*;

/// Classify the callee identifier `name` on the last line of `source`.
fn at_end(source: &str, name: &str) -> Option<SinkHit> {
    let parsed = ParsedFile::parse("t", source, Language::Python).unwrap();
    let line = source.lines().count();
    let ids = parsed.identifiers_on_line(line);
    let id = ids
        .iter()
        .find(|n| parsed.node_text(n) == name)
        .unwrap_or_else(|| panic!("no identifier {name} on line {line}"));
    SinkClassifier::default().classify("t", &parsed, *id, name, &[])
}

fn shape(h: Option<SinkHit>) -> Option<(&'static str, &'static str, Strength)> {
    h.map(|h| (h.rule, h.cwe.unwrap_or("none"), h.strength))
}

const CMD: &str = "py-module-command";
const TPL: &str = "py-module-template";

#[test]
fn python_module_tables_fire_on_proven_identity() {
    let strong = |rule, cwe| Some((rule, cwe, Strength::Strong));
    let cases: &[(&str, &str, Option<(&str, &str, Strength)>)] = &[
        ("import pexpect\npexpect.spawn(x)", "spawn", strong(CMD, "CWE-78")),
        ("import pexpect\npexpect.runu(x)", "runu", strong(CMD, "CWE-78")),
        ("from pexpect import run as r\nr(x)", "r", strong(CMD, "CWE-78")),
        ("from pexpect.popen_spawn import PopenSpawn\nPopenSpawn(x)", "PopenSpawn", strong(CMD, "CWE-78")),
        ("import fabric.api as fa\nfa.sudo(x)", "sudo", strong(CMD, "CWE-78")),
        ("from invoke import run\nrun(x)", "run", strong(CMD, "CWE-78")),
        ("import paramiko\ns = paramiko.SSHClient()\ns.exec_command(x)", "exec_command", strong(CMD, "CWE-78")),
        ("from fabric import Connection\nwith Connection(h) as c:\n    c.run(x)", "run", strong(CMD, "CWE-78")),
        ("from invoke import task\n@task\ndef d(c):\n    c.local(x)", "local", strong(CMD, "CWE-78")),
        ("import jinja2\ne = jinja2.Environment()\ne.from_string(x)", "from_string", strong(TPL, "CWE-1336")),
        ("import jinja2\ne = jinja2.Environment()\ne.from_string(source=x)", "from_string", strong(TPL, "CWE-1336")),
        ("from fabric import Connection\nConnection(h).run(x)", "run", strong(CMD, "CWE-78")),
        ("import jinja2\njinja2.Environment().from_string(x)", "from_string", strong(TPL, "CWE-1336")),
        ("from invoke import *\nfrom invoke import run\nrun(x)", "run", strong(CMD, "CWE-78")),
        ("from invoke import run\ndef a(run):\n    pass\nrun(x)", "run", strong(CMD, "CWE-78")),
        (
            "from jinja2.sandbox import SandboxedEnvironment\nclass T:\n    env = SandboxedEnvironment()\n    t = env.from_string(x)",
            "from_string",
            Some((TPL, "CWE-1336", Strength::Weak)),
        ),
        (
            "from jinja2.sandbox import SandboxedEnvironment\ne = SandboxedEnvironment()\ne.from_string(x)",
            "from_string",
            Some((TPL, "CWE-1336", Strength::Weak)),
        ),
    ];
    for (src, name, want) in cases {
        assert_eq!(shape(at_end(src, name)), *want, "{src}");
    }
}

#[test]
fn python_module_tables_stay_silent_without_proof() {
    let cases: &[(&str, &str)] = &[
        ("import pexpect\npexpect.spawn()", "spawn"),
        ("import pexpect\npexpect.which(x)", "which"),
        (
            "import pexpect\ndef f(pexpect):\n    pexpect.spawn(x)",
            "spawn",
        ),
        ("def d(c):\n    c.run(x)", "run"),
        (
            "from celery import task\n@task\ndef d(c):\n    c.run(x)",
            "run",
        ),
        (
            "from fabric import task\n@task\ndef d(a, c):\n    c.run(x)",
            "run",
        ),
        (
            "from fabric import task\n@task\ndef d(c):\n    go = lambda c: c.run(x)",
            "run",
        ),
        (
            "import paramiko\ns = paramiko.SSHClient()\ns = f()\ns.exec_command(x)",
            "exec_command",
        ),
        (
            "import paramiko\ns = paramiko.SSHClient()\ns.connect(x)",
            "connect",
        ),
        (
            "import paramiko\nfor s in paramiko.SSHClient():\n    s.exec_command(x)",
            "exec_command",
        ),
        (
            "import paramiko\nclass R:\n    def f(self):\n        self.s.exec_command(x)",
            "exec_command",
        ),
        (
            "import jinja2\ne = jinja2.Environment()\ne.compile(x)",
            "compile",
        ),
        ("import jinja2\ne = jinja2.Environment()\ne.from_string('lit')", "from_string"),
        ("import jinja2\ne = jinja2.Environment()\ne.from_string()", "from_string"),
        ("import jinja2\ne = jinja2.Environment()\ne.from_string('lit', globals=g)", "from_string"),
        ("def make():\n    pass\nmake().run(x)", "run"),
        ("from fabric.api import task\n@task\ndef d(c):\n    c.run(x)", "run"),
        ("from invoke import run\nfrom other import *\nrun(x)", "run"),
        ("import paramiko\nfrom paramiko import *\ns = paramiko.SSHClient()\ns.exec_command(x)", "exec_command"),
        ("class H:\n    from invoke import run as len\nlen(x)", "len"),
        ("def f():\n    import pexpect\npexpect.spawn(x)", "spawn"),
        ("from pexpect import spawn\nfrom ptyx import spawn\nspawn(x)", "spawn"),
        ("import jinja2\nfrom jinja2.sandbox import SandboxedEnvironment\nenv = jinja2.Environment()\nclass T:\n    env = SandboxedEnvironment()\n    t = [env.from_string(y) for y in x]", "from_string"),
        ("from pexpect import spawn\nspawn = make()\nspawn(x)", "spawn"),
    ];
    for (src, name) in cases {
        assert_eq!(shape(at_end(src, name)), None, "{src}");
    }
}
