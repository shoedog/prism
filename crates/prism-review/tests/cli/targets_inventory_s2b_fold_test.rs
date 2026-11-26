use super::targets_inventory_constructors_test::{identity_warnings, source_document};

type Outcome<'a> = (&'a [(&'a str, &'a str)], &'a [&'a str]);

const SILENT: Outcome<'_> = (&[], &[]);
const ROOT: Outcome<'_> = (&[("requests.get", "exact")], &[]);
const SESSION: Outcome<'_> = (&[("requests.Session.get", "exact")], &[]);
const HELPER: Outcome<'_> = (&[("requests.Session.get", "nameonly")], &[]);
const REBOUND: Outcome<'_> = (&[], &["1 (member_rebound: 1)"]);
const UNPROVEN: Outcome<'_> = (&[], &["1 (receiver_value_unproven: 1)"]);
const NOT_ADMISSIBLE: Outcome<'_> = (&[], &["1 (helper_not_admissible: 1)"]);
const UNANNOTATED: Outcome<'_> = (&[], &["1 (helper_unannotated: 1)"]);
const NONE: Outcome<'_> = (&[], &["1 (receiver_is_none: 1)"]);
const CONFLICT: Outcome<'_> = (&[], &["1 (annotation_conflict: 1)"]);

#[derive(Default)]
struct Cases(Vec<String>);

impl Cases {
    fn check(&mut self, label: &str, source: &str, (rows, warnings): Outcome<'_>) {
        let doc = source_document(source);
        let mut actual: Vec<_> = doc["targets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                assert_eq!(row["kind"], "external_call", "{label}: {row:#}");
                (
                    row["dependency_hint"]["callee"].as_str().unwrap(),
                    row["confidence"].as_str().unwrap(),
                )
            })
            .collect();
        actual.sort_unstable();
        let actual_warnings = identity_warnings(&doc);
        let expected_warnings: Vec<_> = warnings
            .iter()
            .map(|detail| format!("inventory: identity unproven: probe.py: {detail}"))
            .collect();
        if actual != rows || actual_warnings != expected_warnings {
            self.0.push(format!(
                "{label}: expected {rows:?}, {expected_warnings:?}; got {actual:?}, {actual_warnings:?}"
            ));
        }
    }

    fn finish(self) {
        assert!(self.0.is_empty(), "\n{}", self.0.join("\n"));
    }
}

#[test]
fn unproven_origins_keep_s2a_write_vetoes() {
    let mut cases = Cases::default();
    for (id, source) in [
        ("i1", "def helper():\n    return requests\ndef f(u):\n    m = helper()\n    m.KEY = len"),
        ("i4", "from x import thing\ndef f(u):\n    m: requests.Session = thing()\n    m.KEY = len"),
        ("i5", "def f(m: requests.Session, u):\n    m.KEY = len"),
        ("i9", "class A:\n    def m(self, u):\n        self.KEY = len"),
        ("i10", "def f(m: 'requests.Session' = None, u=''):\n    m.KEY = len"),
        ("i11", "def helper(x):\n    return x\ndef f(u):\n    m = helper(requests)\n    setattr(m, 'KEY', len)"),
        ("i12", "class A:\n    def __init__(self):\n        self.client = requests\n    def m(self, u):\n        self.client.KEY = len"),
    ] {
        let indent = if source.starts_with("class") { "        " } else { "    " };
        for (route, call) in [
            ("root", format!("{indent}requests.get(u)")),
            ("capture", format!("{indent}from requests import get\n{indent}get(u)")),
        ] {
            let program = format!("import requests\n{source}\n{call}\n");
            cases.check(&format!("{id}/{route}/post"), &program.replace("KEY", "post"), ROOT);
            cases.check(&format!("{id}/{route}/get"), &program.replace("KEY", "get"), REBOUND);
        }
    }
    for (key, want) in [("post", SESSION), ("get", REBOUND)] {
        cases.check(
            &format!("i13/{key}"),
            &format!("import requests\nS = requests.Session()\ndef helper():\n    return S\ndef f(u):\n    m = helper()\n    m.{key} = len\n    S.get(u)\n"),
            want,
        );
    }
    cases.check(
        "i6/proven construction",
        "import requests\ndef helper() -> requests.Session:\n    return requests.Session()\ndef f(u):\n    m = helper()\n    m.get = len\n    requests.get(u)\n",
        ROOT,
    );
    cases.finish();
}

fn returned_local(statement: &str) -> String {
    format!("from requests import Session\ndef make(flag=None, obj=None) -> Session:\n    v = Session()\n    {statement}\n    return v\ndef use():\n    c = make()\n    c.get('u')\n")
}

#[test]
fn class_pattern_heads_are_returned_local_uses() {
    let mut cases = Cases::default();
    for (pattern, expected) in [
        ("v()", UNPROVEN),
        ("(v())", UNPROVEN),
        ("v() as matched", UNPROVEN),
        ("None | v()", UNPROVEN),
        ("[v()]", UNPROVEN),
        ("(v(),)", UNPROVEN),
        ("{'k': v()}", UNPROVEN),
        ("list(v())", UNPROVEN),
        ("str(ex=v())", UNPROVEN),
        ("v.cookies", HELPER),
        ("v.cookies()", HELPER),
        ("str(v=_)", HELPER),
    ] {
        cases.check(
            pattern,
            &returned_local(&format!(
                "if flag:\n        match obj:\n            case {pattern}:\n                pass"
            )),
            expected,
        );
    }
    cases.finish();
}

#[test]
fn optional_requires_a_scalar_in_each_declaration_position() {
    let mut cases = Cases::default();
    let imports = "from __future__ import annotations\nimport typing\nfrom typing import Optional as Maybe\nfrom requests import Session";
    for (annotation, valid) in [
        ("Maybe[Session,]", false),
        ("typing.Optional[Session,]", false),
        ("'Maybe[Session,]'", false),
        ("'typing.Optional[Session,]'", false),
        ("Maybe[(Session),]", false),
        ("Maybe[\"Session\",]", false),
        ("Maybe[(Session,)]", false),
        ("typing.Optional[Session, str]", false),
        ("Maybe[*[Session]]", false),
        ("Maybe[Session]", true),
        ("Maybe[(Session)]", true),
        ("'typing.Optional[(Session)]'", true),
        ("Maybe[\"Session\"]", true),
    ] {
        for (position, source, want) in [
            ("return", format!("def make() -> {annotation}:\n    return Session()\ndef use():\n    c = make()\n    c.get('u')"), if valid { HELPER } else { UNANNOTATED }),
            ("parameter", format!("def use(c: {annotation}):\n    c.get('u')"), if valid { UNPROVEN } else { SILENT }),
            ("local", format!("def use():\n    c: {annotation} = None\n    c.get('u')"), if valid { NONE } else { SILENT }),
        ] {
            cases.check(&format!("{position}/{annotation}"), &format!("{imports}\n{source}\n"), want);
        }
    }
    cases.finish();
}

#[test]
fn reflection_requires_a_scalar_string_subscript() {
    let mut cases = Cases::default();
    const FIELD: Outcome<'_> = (&[], &["1 (receiver_value_unproven: 1)"]);
    const NON_UNIQUE: Outcome<'_> = (&[], &["1 (receiver_not_unique: 1)"]);
    for (route, source, good, bad) in [
        ("root", "import requests\ndef use():\n    requests.__dict__[KEY] = {}.get\n    requests.get('u')", ROOT, REBOUND),
        ("named", "from requests import Session\ndef use():\n    v = Session()\n    vars(v)[KEY] = {}.get\n    v.get('u')", SESSION, REBOUND),
        ("field", "from requests import Session\nclass Owner:\n    def __init__(self):\n        self.get = Session()\n    def use(self):\n        vars(self)[KEY] = {}.get\n        self.get.get('u')", FIELD, NON_UNIQUE),
        ("annotated", "from requests import Session\ndef make() -> Session:\n    return Session()\ndef use():\n    v: Session = make()\n    vars(v)[KEY] = {}.get\n    v.get('u')", HELPER, REBOUND),
        ("helper", "from requests import Session\ndef make() -> Session:\n    return Session()\ndef use():\n    v = make()\n    v.__dict__[KEY] = {}.get\n    v.get('u')", HELPER, REBOUND),
        ("returned local", "from requests import Session\ndef make() -> Session:\n    v = Session()\n    vars(v)[KEY] = {}.get\n    return v\ndef use():\n    v = make()\n    v.get('u')", HELPER, REBOUND),
        ("globals", "from requests import Session\nget = Session()\nglobals()[KEY] = {}\ndef use():\n    get.get('u')", SESSION, NON_UNIQUE),
    ] {
        for (key, want) in [
            ("\"get\",", good),
            ("\"get\", \"other\"", good),
            ("(\"get\",)", good),
            ("\"get\"", bad),
            ("(\"get\")", bad),
        ] {
            cases.check(&format!("{route}/{key}"), &format!("{}\n", source.replace("KEY", key)), want);
        }
    }
    cases.finish();
}

#[test]
fn returned_local_uses_share_modeled_write_targets() {
    let mut cases = Cases::default();
    for statement in [
        "(vars(v))['KEY'] = {}",
        "(vars(v)['KEY']) = {}",
        "(vars(v))['KEY'] |= {}",
        "(vars(v)['KEY']) += {}",
        "vars(v)['KEY'], _ = {}, 0",
        "[*vars(v)['KEY']] = items",
        "for vars(v)['KEY'] in [{}]: pass",
        "with open('foo') as vars(v)['KEY']: pass",
        "del (vars(v)['KEY'])",
        "del vars(v)['KEY'], v.headers",
        "vars((v))['KEY'] = {}",
        "setattr((v), 'KEY', {})",
    ] {
        for (key, want) in [("cookies", HELPER), ("get", REBOUND)] {
            cases.check(
                statement,
                &returned_local(&statement.replace("KEY", key)),
                want,
            );
        }
    }
    for statement in [
        "use(vars(v))",
        "x = vars(v)['cookies']",
        "box[vars(v)] = {}",
        "vars(v)['cookies']: dict",
        "def vars(x): return x\n    vars(v)['cookies'] = {}",
    ] {
        cases.check(statement, &returned_local(statement), UNPROVEN);
    }
    cases.finish();
}

#[test]
fn helper_failures_use_cause_precedence() {
    let mut cases = Cases::default();
    for (label, annotation, body, want) in [
        (
            "returns absent first",
            "Session",
            "if flag: return requests.Session()\n    return httpx.Client()",
            "shadowed_root",
        ),
        (
            "returns shadowed first",
            "Session",
            "if flag: return httpx.Client()\n    return requests.Session()",
            "shadowed_root",
        ),
        (
            "signature absent",
            "requests.Session",
            "return httpx.Client()",
            "shadowed_root",
        ),
        (
            "one absent",
            "Session",
            "return requests.Session()",
            "not_imported",
        ),
        (
            "locals absent first",
            "Session",
            "v = requests.Session()\n    v = httpx.Client()\n    return v",
            "shadowed_root",
        ),
        (
            "locals shadowed first",
            "Session",
            "v = httpx.Client()\n    v = requests.Session()\n    return v",
            "shadowed_root",
        ),
        (
            "locally rebound",
            "Session",
            "v = Session()\n    v.get = len\n    v = other\n    return v",
            "member_rebound",
        ),
    ] {
        let source = format!("from requests import Session\nhttpx = object()\ndef make() -> {annotation}:\n    {body}\ndef use():\n    c = make()\n    c.get('u')\n");
        let warning = format!("1 ({want}: 1)");
        cases.check(label, &source, (&[], &[&warning]));
    }
    cases.check("independent local constructor", "from requests import Session\nhttpx = object()\ndef make() -> Session:\n    return httpx.Client()\ndef use():\n    c: httpx.Client = Session()\n    c.get('u')\n", SESSION);
    cases.finish();
}

#[test]
fn grouped_simple_targets_keep_assignment_values() {
    let mut cases = Cases::default();
    for (assignment, want) in [
        ("v = Session()", SESSION),
        ("(v) = Session()", SESSION),
        ("((v)) = Session()", SESSION),
        ("(v) = w = Session()", SESSION),
        ("w = (v) = Session()", SESSION),
        ("(v): Session = Session()", SESSION),
        ("(v) = make()", HELPER),
        ("(v,) = Session()", SILENT),
        ("[v] = Session()", SILENT),
        ("with Session() as (v): pass", SESSION),
    ] {
        cases.check(assignment, &format!("from requests import Session\ndef make() -> Session:\n    return Session()\ndef use():\n    {assignment}\n    v.get('u')\n"), want);
    }
    cases.check(
        "grouped returned binding",
        &returned_local("((v)) = Session()").replace("    v = Session()\n", ""),
        HELPER,
    );
    cases.finish();
}

#[test]
fn comprehension_displays_are_annotation_conflicts() {
    let mut cases = Cases::default();
    for (expr, want) in [
        ("[x for x in ()]", CONFLICT),
        ("{x for x in ()}", CONFLICT),
        ("{x:x for x in ()}", CONFLICT),
        ("[]", CONFLICT),
        ("{1}", CONFLICT),
        ("{}", CONFLICT),
        ("factory()", UNPROVEN),
        ("(x for x in ())", UNPROVEN),
    ] {
        for (position, source) in [
            ("return", format!("def make() -> Session:\n    return {expr}\ndef use():\n    c = make()\n    c.get('u')")),
            ("parameter", format!("def use(c: Session = {expr}):\n    c.get('u')")),
            ("local", format!("def use():\n    c: Session = {expr}\n    c.get('u')")),
        ] {
            cases.check(&format!("{position}/{expr}"), &format!("from requests import Session\n{source}\n"), want);
        }
    }
    cases.finish();
}

#[test]
fn helper_diagnostics_include_hidden_lexical_ancestors() {
    let mut cases = Cases::default();
    for (label, source, want) in [
        ("hidden parameter", "def outer():\n    def make() -> Session:\n        return Session()\n    def use(make):\n        c = make()\n        c.get('u')", NOT_ADMISSIBLE),
        ("hidden grandparent", "def outer():\n    def make() -> Session:\n        return Session()\n    def middle():\n        make = other\n        def use():\n            c = make()\n            c.get('u')", NOT_ADMISSIBLE),
        ("visible enclosing", "def outer():\n    def make() -> Session:\n        return Session()\n    def use():\n        c = make()\n        c.get('u')", NOT_ADMISSIBLE),
        ("unrelated sibling", "def outer():\n    def sibling():\n        def make() -> Session:\n            return Session()\n    def use(make):\n        c = make()\n        c.get('u')", SILENT),
        ("skipped class", "class Owner:\n    def make() -> Session:\n        return Session()\n    def use(self, make):\n        c = make()\n        c.get('u')", SILENT),
        ("visible module", "def make() -> Session:\n    return Session()\ndef use():\n    c = make()\n    c.get('u')", HELPER),
    ] {
        cases.check(label, &format!("from requests import Session\n{source}\n"), want);
    }
    cases.finish();
}
