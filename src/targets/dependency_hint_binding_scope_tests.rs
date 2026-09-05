use super::*;
use crate::languages::Language;

fn hint_at_language(path: &str, source: &str, line: usize, language: Language) -> AstHint {
    let parsed = ParsedFile::parse(path, source, language).unwrap();
    let known_files = BTreeMap::new();
    let resolved = resolve(
        &parsed,
        line,
        &SiteContext {
            file: path,
            repo_root: Path::new("/nonexistent-prism-test-repo-root"),
            known_files: &known_files,
            resolved_name: None,
        },
    );
    match resolved {
        Some(Resolution::Hint(hint)) => hint,
        other => panic!("expected a single-call resolution, got {other:?}"),
    }
}

fn hint_at(source: &str, line: usize) -> AstHint {
    hint_at_language("svc.py", source, line, Language::Python)
}

#[test]
fn sibling_function_import_does_not_bind_at_the_site() {
    let source = concat!(
        "def unrelated():\n",
        "    import requests as rq\n",
        "\n",
        "def run():\n",
        "    from .store import db as rq\n",
        "    rq.get('key')\n",
    );
    let hint = hint_at(source, 6);
    assert_eq!(hint.callee, "rq.get");
    assert_eq!(
        hint.kind, None,
        "a sibling function's import is not visible"
    );
}

#[test]
fn nested_callable_assignment_does_not_type_the_outer_receiver() {
    let source = concat!(
        "import requests\n",
        "\n",
        "def send(client):\n",
        "    def never_called():\n",
        "        client = requests.Session()\n",
        "        return client\n",
        "    client.get('key')\n",
    );
    let hint = hint_at(source, 7);
    assert_eq!(hint.callee, "client.get");
    assert_eq!(hint.kind, None, "a nested callable's local is opaque");
}

#[test]
fn sibling_dotted_import_cannot_extend_the_bound_module_path() {
    let source = concat!(
        "import urllib.request\n",
        "import urllib.parse\n",
        "\n",
        "def run(url):\n",
        "    urllib.parse.urljoin(url, 'child')\n",
    );
    let hint = hint_at(source, 5);
    assert_eq!(hint.callee, "urllib.parse.urljoin");
    assert_eq!(
        hint.kind, None,
        "urllib.parse cannot extend the urllib.request import binding"
    );
}

#[test]
fn module_import_alias_is_visible_in_an_enclosed_function() {
    let hint = hint_at("import requests as rq\n\ndef run():\n    rq.post('x')\n", 4);
    assert_eq!(hint.callee, "rq.post");
    assert_eq!(hint.kind, Some("http"));
}

#[test]
fn dotted_import_chain_extending_the_exact_module_path_gets_kind() {
    let hint = hint_at(
        "import urllib.request\n\ndef run():\n    urllib.request.urlopen('x')\n",
        4,
    );
    assert_eq!(hint.callee, "urllib.request.urlopen");
    assert_eq!(hint.kind, Some("http"));
}

#[test]
fn self_receiver_uses_a_constructor_in_the_same_class() {
    let source = concat!(
        "import requests\n",
        "\n",
        "class Service:\n",
        "    def __init__(self):\n",
        "        self.client = requests.Session()\n",
        "\n",
        "    def run(self):\n",
        "        self.client.get('x')\n",
    );
    let hint = hint_at(source, 8);
    assert_eq!(hint.callee, "self.client.get");
    assert_eq!(hint.kind, Some("http"));
}

#[test]
fn enclosing_function_import_is_visible_in_a_nested_callable() {
    let source = concat!(
        "def outer():\n",
        "    import requests as rq\n",
        "    def inner():\n",
        "        rq.post('x')\n",
    );
    let hint = hint_at(source, 4);
    assert_eq!(hint.callee, "rq.post");
    assert_eq!(hint.kind, Some("http"));
}

#[test]
fn outer_local_receiver_does_not_type_a_nested_callable_receiver() {
    let source = concat!(
        "import requests\n",
        "\n",
        "def outer():\n",
        "    client = requests.Session()\n",
        "    def inner():\n",
        "        client.get('x')\n",
    );
    let hint = hint_at(source, 6);
    assert_eq!(hint.callee, "client.get");
    assert_eq!(hint.kind, None, "receiver bindings do not cross callables");
}

#[test]
fn sibling_commonjs_require_does_not_bind_at_the_site() {
    let source = concat!(
        "function unrelated() {\n",
        "  const axios = require('axios');\n",
        "  return axios;\n",
        "}\n",
        "function run() {\n",
        "  axios.get('x');\n",
        "}\n",
    );
    let hint = hint_at_language("svc.js", source, 6, Language::JavaScript);
    assert_eq!(hint.callee, "axios.get");
    assert_eq!(hint.kind, None, "a sibling function's require is opaque");
}
