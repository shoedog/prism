use super::*;
use crate::languages::Language;
fn parse(source: &str, language: Language) -> ParsedFile {
    ParsedFile::parse("f", source, language).unwrap()
}

/// Resolve at `line` with an empty repo view (no sibling files, no repo on
/// disk) and no finding evidence — the shape most unit cases want.
fn resolve_at(parsed: &ParsedFile, line: usize) -> Option<Resolution> {
    resolve_in(parsed, line, "svc", &[], None)
}

/// Resolve at `line` against a synthetic repo whose file list is
/// `siblings` (repo-relative paths) and with `resolved_name` as the
/// finding's own recorded callee identity.
fn resolve_in(
    parsed: &ParsedFile,
    line: usize,
    file: &str,
    siblings: &[&str],
    resolved_name: Option<&str>,
) -> Option<Resolution> {
    let known: BTreeMap<String, ParsedFile> = siblings
        .iter()
        .map(|path| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, "", Language::Python).unwrap(),
            )
        })
        .collect();
    resolve(
        parsed,
        line,
        &SiteContext {
            file,
            repo_root: Path::new("/nonexistent-prism-test-repo-root"),
            known_files: &known,
            resolved_name,
        },
    )
}

/// The `AstHint` for a site that must resolve to exactly one call.
fn hint_at(parsed: &ParsedFile, line: usize) -> AstHint {
    match resolve_at(parsed, line) {
        Some(Resolution::Hint(hint)) => hint,
        other => panic!("expected a single-call resolution, got {other:?}"),
    }
}

#[test]
fn direct_root_library_call_gets_kind_and_verbatim_callee() {
    let parsed = parse(
        "import requests\n\n\ndef send():\n    requests.post('x')\n",
        Language::Python,
    );
    let hint = hint_at(&parsed, 5);
    assert_eq!(hint.callee, "requests.post");
    assert_eq!(hint.kind, Some("http"));
}

// --- WRONG 1: spelling is not dependency identity -------------------

#[test]
fn library_root_without_an_import_gets_callee_only() {
    let parsed = parse("def send():\n    requests.post('x')\n", Language::Python);
    let hint = hint_at(&parsed, 2);
    assert_eq!(hint.callee, "requests.post");
    assert_eq!(
        hint.kind, None,
        "an unimported `requests` is a local name, not the library"
    );
}

#[test]
fn repo_local_module_of_the_library_name_gets_callee_only() {
    let parsed = parse(
        "import requests\n\n\ndef send():\n    requests.post('x')\n",
        Language::Python,
    );
    let resolved = resolve_in(&parsed, 5, "svc.py", &["requests.py"], None);
    let Some(Resolution::Hint(hint)) = resolved else {
        panic!("expected a hint, got {resolved:?}");
    };
    assert_eq!(hint.callee, "requests.post");
    assert_eq!(
        hint.kind, None,
        "`import requests` resolves to the repo's own requests.py"
    );
}

#[test]
fn repo_local_package_of_the_library_name_gets_callee_only() {
    let parsed = parse(
        "import requests\n\n\ndef send():\n    requests.post('x')\n",
        Language::Python,
    );
    let resolved = resolve_in(
        &parsed,
        5,
        "pkg/svc.py",
        &["pkg/requests/__init__.py"],
        None,
    );
    let Some(Resolution::Hint(hint)) = resolved else {
        panic!("expected a hint, got {resolved:?}");
    };
    assert_eq!(hint.kind, None, "a sibling package shadows the library");
}

#[test]
fn parameter_shadowing_the_library_root_gets_callee_only() {
    let parsed = parse(
        "import requests\n\n\ndef send(requests):\n    requests.post('x')\n",
        Language::Python,
    );
    let hint = hint_at(&parsed, 5);
    assert_eq!(hint.callee, "requests.post");
    assert_eq!(hint.kind, None, "the parameter shadows the import");
}

#[test]
fn assignment_shadowing_the_library_root_gets_callee_only() {
    let parsed = parse(
        "import requests\n\n\ndef send():\n    requests = Stub()\n    requests.post('x')\n",
        Language::Python,
    );
    let hint = hint_at(&parsed, 6);
    assert_eq!(hint.kind, None, "the local rebind shadows the import");
}

#[test]
fn aliased_import_resolves_through_the_library_root() {
    let parsed = parse(
        "import requests as rq\n\n\ndef send():\n    rq.post('x')\n",
        Language::Python,
    );
    let hint = hint_at(&parsed, 5);
    assert_eq!(hint.callee, "rq.post", "callee stays source-verbatim");
    assert_eq!(hint.kind, Some("http"), "the alias binds the library root");
}

#[test]
fn submodule_import_resolves_the_single_purpose_submodule() {
    let parsed = parse(
        "import urllib.request\n\n\ndef send():\n    urllib.request.urlopen('x')\n",
        Language::Python,
    );
    let hint = hint_at(&parsed, 5);
    assert_eq!(hint.callee, "urllib.request.urlopen");
    assert_eq!(hint.kind, Some("http"));
}

#[test]
fn same_named_local_import_path_gets_callee_only() {
    let parsed = parse(
        "from . import requests\n\n\ndef send():\n    requests.post('x')\n",
        Language::Python,
    );
    let hint = hint_at(&parsed, 5);
    assert_eq!(
        hint.kind, None,
        "a relative import never names an external library"
    );
}

#[test]
fn go_import_path_naming_a_local_package_gets_callee_only() {
    let source = "package main\n\nimport \"example.com/repo/internal/http\"\n\nfunc send() {\n\thttp.Get(\"x\")\n}\n";
    let parsed = parse(source, Language::Go);
    let hint = hint_at(&parsed, 6);
    assert_eq!(hint.callee, "http.Get");
    assert_eq!(
        hint.kind, None,
        "a repo-local package spelled `http` is not net/http"
    );
}

#[test]
fn js_relative_import_of_the_library_name_gets_callee_only() {
    let source =
        "import axios from './axios';\n\nasync function run() {\n  await axios.get('x');\n}\n";
    let parsed = parse(source, Language::JavaScript);
    let hint = hint_at(&parsed, 4);
    assert_eq!(hint.callee, "axios.get");
    assert_eq!(hint.kind, None, "./axios is a local module");
}

#[test]
fn js_bare_import_of_a_cataloged_package_resolves_kind() {
    let source =
        "import axios from 'axios';\n\nasync function run() {\n  await axios.get('x');\n}\n";
    let parsed = parse(source, Language::JavaScript);
    let hint = hint_at(&parsed, 4);
    assert_eq!(hint.callee, "axios.get");
    assert_eq!(hint.kind, Some("http"));
}

#[test]
fn js_require_binding_of_a_cataloged_package_resolves_kind() {
    let source =
        "const kafka = require('kafkajs');\n\nfunction run() {\n  kafka.producer('x');\n}\n";
    let parsed = parse(source, Language::JavaScript);
    let hint = hint_at(&parsed, 4);
    assert_eq!(hint.kind, Some("queue"));
}

#[test]
fn bare_identifier_callee_with_unmapped_root_has_no_kind() {
    let parsed = parse("def send():\n    fetch('x')\n", Language::Python);
    let hint = hint_at(&parsed, 2);
    assert_eq!(hint.callee, "fetch");
    assert_eq!(hint.kind, None);
}

// --- WRONG 2: receiver resolution stays inside its lexical owner ----

#[test]
fn receiver_shape_resolves_kind_via_same_owner_constructor() {
    let source = "import requests\n\n\nclass C:\n    def __init__(self):\n        self.client = requests.Session()\n\n    def send(self):\n        self.client.get('x')\n";
    let parsed = parse(source, Language::Python);
    let hint = hint_at(&parsed, 9);
    assert_eq!(hint.callee, "self.client.get");
    assert_eq!(hint.kind, Some("http"));
}

#[test]
fn receiver_shape_without_resolvable_constructor_keeps_callee_omits_kind() {
    let source =
        "class C:\n    def send(self):\n        self.client = make_client()\n        self.client.get('x')\n";
    let parsed = parse(source, Language::Python);
    let hint = hint_at(&parsed, 4);
    assert_eq!(hint.callee, "self.client.get");
    assert_eq!(hint.kind, None);
}

#[test]
fn receiver_kind_never_crosses_class_owners() {
    let source = concat!(
        "import requests\n",
        "import sqlalchemy\n",
        "\n",
        "\n",
        "class A:\n",
        "    def __init__(self):\n",
        "        self.client = requests.Session()\n",
        "\n",
        "\n",
        "class B:\n",
        "    def __init__(self):\n",
        "        self.client = sqlalchemy.Session()\n",
        "\n",
        "    def load(self, model, key):\n",
        "        self.client.get(model, key)\n",
    );
    let parsed = parse(source, Language::Python);
    let hint = hint_at(&parsed, 15);
    assert_eq!(hint.callee, "self.client.get");
    assert_eq!(
        hint.kind,
        Some("db"),
        "B's own constructor owns B's receiver; A's must not reach it"
    );
}

#[test]
fn receiver_with_no_constructor_in_its_owner_gets_callee_only() {
    let source = concat!(
        "import requests\n",
        "\n",
        "\n",
        "class A:\n",
        "    def __init__(self):\n",
        "        self.client = requests.Session()\n",
        "\n",
        "\n",
        "class B:\n",
        "    def load(self, model, key):\n",
        "        self.client.get(model, key)\n",
    );
    let parsed = parse(source, Language::Python);
    let hint = hint_at(&parsed, 11);
    assert_eq!(hint.callee, "self.client.get");
    assert_eq!(
        hint.kind, None,
        "B never constructs self.client; A's constructor is another owner's"
    );
}

#[test]
fn reassigned_receiver_in_one_function_gets_callee_only() {
    let source = concat!(
        "import requests\n",
        "import sqlalchemy\n",
        "\n",
        "\n",
        "def send(url):\n",
        "    client = requests.Session()\n",
        "    client = sqlalchemy.Session()\n",
        "    client.get(url)\n",
    );
    let parsed = parse(source, Language::Python);
    let hint = hint_at(&parsed, 8);
    assert_eq!(hint.callee, "client.get");
    assert_eq!(
        hint.kind, None,
        "two assignments to the receiver leave its type unproven"
    );
}

#[test]
fn local_receiver_resolves_from_its_own_function() {
    let source = concat!(
        "import requests\n",
        "\n",
        "\n",
        "def send(url):\n",
        "    client = requests.Session()\n",
        "    client.get(url)\n",
    );
    let parsed = parse(source, Language::Python);
    let hint = hint_at(&parsed, 6);
    assert_eq!(hint.kind, Some("http"));
}

#[test]
fn local_receiver_constructed_in_another_function_gets_callee_only() {
    let source = concat!(
        "import requests\n",
        "\n",
        "\n",
        "def build():\n",
        "    client = requests.Session()\n",
        "    return client\n",
        "\n",
        "\n",
        "def send(client, url):\n",
        "    client.get(url)\n",
    );
    let parsed = parse(source, Language::Python);
    let hint = hint_at(&parsed, 10);
    assert_eq!(
        hint.kind, None,
        "another function's local is not this one's receiver"
    );
}

#[test]
fn unknown_root_emits_callee_without_kind() {
    let parsed = parse(
        "def send():\n    unknownlib.frobnicate('x')\n",
        Language::Python,
    );
    let hint = hint_at(&parsed, 2);
    assert_eq!(hint.callee, "unknownlib.frobnicate");
    assert_eq!(hint.kind, None);
}

#[test]
fn go_net_http_selector_call_resolves_http() {
    let source = "package main\n\nimport \"net/http\"\n\nfunc send() {\n\thttp.Get(\"x\")\n}\n";
    let parsed = parse(source, Language::Go);
    let hint = hint_at(&parsed, 6);
    assert_eq!(hint.callee, "http.Get");
    assert_eq!(hint.kind, Some("http"));
}

#[test]
fn go_database_sql_root_resolves_db() {
    let source =
        "package main\n\nimport \"database/sql\"\n\nfunc open() {\n\tsql.Open(\"pg\", \"dsn\")\n}\n";
    let parsed = parse(source, Language::Go);
    let hint = hint_at(&parsed, 6);
    assert_eq!(hint.callee, "sql.Open");
    assert_eq!(hint.kind, Some("db"));
}

#[test]
fn os_system_override_resolves_process() {
    let parsed = parse(
        "import os\n\n\ndef run():\n    os.system('ls')\n",
        Language::Python,
    );
    let hint = hint_at(&parsed, 5);
    assert_eq!(hint.kind, Some("process"));
}

#[test]
fn time_sleep_override_resolves_clock() {
    let parsed = parse(
        "import time\n\n\ndef run():\n    time.sleep(1)\n",
        Language::Python,
    );
    let hint = hint_at(&parsed, 5);
    assert_eq!(hint.kind, Some("clock"));
}

#[test]
fn no_call_at_line_returns_none() {
    let parsed = parse("def run():\n    pass\n", Language::Python);
    assert!(resolve_at(&parsed, 2).is_none());
}

// --- WRONG 3: same-line candidates follow the finding's evidence ----

#[test]
fn sibling_call_on_the_site_line_follows_the_findings_own_callee() {
    let source = concat!(
        "import requests\n",
        "\n",
        "\n",
        "def run(db, url):\n",
        "    db.commit(); requests.get(url)\n",
    );
    let parsed = parse(source, Language::Python);
    let resolved = resolve_in(&parsed, 5, "svc.py", &[], Some("commit"));
    let Some(Resolution::Hint(hint)) = resolved else {
        panic!("expected the finding's own call, got {resolved:?}");
    };
    assert_eq!(
        hint.callee, "db.commit",
        "the finding is about db.commit(), not the http call beside it"
    );
    assert_eq!(hint.kind, None);
}

#[test]
fn nested_call_argument_never_displaces_the_outer_callee() {
    let source = "import requests\n\n\ndef run():\n    requests.post(str(1))\n";
    let parsed = parse(source, Language::Python);
    let resolved = resolve_in(&parsed, 5, "svc.py", &[], Some("post"));
    let Some(Resolution::Hint(hint)) = resolved else {
        panic!("expected the outer call, got {resolved:?}");
    };
    assert_eq!(hint.callee, "requests.post", "never the str() argument");
    assert_eq!(hint.kind, Some("http"));
}

#[test]
fn indistinguishable_same_line_calls_are_reported_ambiguous() {
    let source = "def run(a, b):\n    a.send(); b.send()\n";
    let parsed = parse(source, Language::Python);
    assert_eq!(
        resolve_in(&parsed, 2, "svc.py", &[], Some("send")),
        Some(Resolution::Ambiguous(2)),
        "two `send` calls on one line: keep the existing hint, warn"
    );
}

#[test]
fn same_line_calls_without_evidence_are_reported_ambiguous() {
    let source = "def run(a, b):\n    a.send(); b.store()\n";
    let parsed = parse(source, Language::Python);
    assert_eq!(
        resolve_in(&parsed, 2, "svc.py", &[], None),
        Some(Resolution::Ambiguous(2))
    );
}

// --- WRONG 4: a table entry must be single-purpose ------------------

#[test]
fn multipurpose_library_receiver_gets_callee_only() {
    let source = concat!(
        "import redis\n",
        "\n",
        "\n",
        "def run(payload):\n",
        "    r = redis.Redis()\n",
        "    r.publish('jobs', payload)\n",
    );
    let parsed = parse(source, Language::Python);
    let hint = hint_at(&parsed, 6);
    assert_eq!(hint.callee, "r.publish");
    assert_eq!(
        hint.kind, None,
        "redis spans cache and queue; publish is not a cache operation"
    );
}

#[test]
fn multipurpose_library_root_gets_callee_only() {
    let parsed = parse(
        "import redis\n\n\ndef run():\n    redis.Redis().publish('jobs', 1)\n",
        Language::Python,
    );
    let resolved = resolve_in(&parsed, 5, "svc.py", &[], Some("publish"));
    let Some(Resolution::Hint(hint)) = resolved else {
        panic!("expected a hint, got {resolved:?}");
    };
    assert_eq!(hint.kind, None);
}

#[test]
fn os_path_join_is_not_an_external_call() {
    let parsed = parse(
        "import os\n\n\ndef run():\n    os.path.join('a', 'b')\n",
        Language::Python,
    );
    let hint = hint_at(&parsed, 5);
    assert_eq!(hint.callee, "os.path.join");
    assert_eq!(
        hint.kind, None,
        "os.path.join is pure string work, and bare `os` spans three kinds"
    );
}

#[test]
fn urllib_parse_is_not_http() {
    let parsed = parse(
        "import urllib.parse\n\n\ndef run(u):\n    urllib.parse.urljoin(u, 'x')\n",
        Language::Python,
    );
    let hint = hint_at(&parsed, 5);
    assert_eq!(hint.kind, None, "urllib.parse does no I/O");
}

#[test]
fn subprocess_run_resolves_process() {
    let parsed = parse(
        "import subprocess\n\n\ndef run():\n    subprocess.run(['ls'])\n",
        Language::Python,
    );
    let hint = hint_at(&parsed, 5);
    assert_eq!(hint.kind, Some("process"));
}

#[test]
fn builtin_open_resolves_filesystem_unless_shadowed() {
    let parsed = parse("def run(p):\n    open(p)\n", Language::Python);
    let hint = hint_at(&parsed, 2);
    assert_eq!(hint.callee, "open");
    assert_eq!(hint.kind, Some("filesystem"));

    let shadowed = parse("def run(p, open):\n    open(p)\n", Language::Python);
    assert_eq!(hint_at(&shadowed, 2).kind, None);
}

// --- WRONG 5: the callee is the source-verbatim receiver chain ------

#[test]
fn java_qualified_call_keeps_the_receiver_chain() {
    let source = "class S {\n  void run() {\n    this.client.get(\"x\");\n  }\n}\n";
    let parsed = parse(source, Language::Java);
    let hint = hint_at(&parsed, 3);
    assert_eq!(hint.callee, "this.client.get");
    assert_eq!(hint.kind, None, "no Java root-library table");
}

#[test]
fn java_unqualified_call_keeps_the_bare_name() {
    let source = "class S {\n  void run() {\n    helper(\"x\");\n  }\n}\n";
    let parsed = parse(source, Language::Java);
    assert_eq!(hint_at(&parsed, 3).callee, "helper");
}

#[test]
fn java_static_qualified_call_keeps_the_type_qualifier() {
    let source = "class S {\n  void run() {\n    HttpClient.newHttpClient();\n  }\n}\n";
    let parsed = parse(source, Language::Java);
    assert_eq!(hint_at(&parsed, 3).callee, "HttpClient.newHttpClient");
}

#[test]
fn go_receiver_call_keeps_the_receiver_chain() {
    let source = "package main\n\nfunc run(obj *T) {\n\tobj.Method(\"x\")\n}\n";
    let parsed = parse(source, Language::Go);
    assert_eq!(hint_at(&parsed, 4).callee, "obj.Method");
}

#[test]
fn js_receiver_call_keeps_the_receiver_chain() {
    let source = "class C {\n  run() {\n    this.client.get('x');\n  }\n}\n";
    let parsed = parse(source, Language::JavaScript);
    assert_eq!(hint_at(&parsed, 3).callee, "this.client.get");
}

#[test]
fn js_optional_chaining_keeps_the_source_text() {
    let source = "function run(client) {\n  client?.get('x');\n}\n";
    let parsed = parse(source, Language::JavaScript);
    assert_eq!(hint_at(&parsed, 2).callee, "client?.get");
}
