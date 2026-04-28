//! Python Phase 2 sanitizer and safe-sink tests.

#[path = "../../common/mod.rs"]
mod common;
use common::*;

fn run_taint_python_single(
    source: &str,
    path: &str,
    diff_lines: BTreeSet<usize>,
) -> prism::slice::SliceResult {
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines,
        }],
    };
    algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap()
}

fn has_taint_sink(result: &prism::slice::SliceResult) -> bool {
    result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink"))
}

fn has_taint_sink_on(result: &prism::slice::SliceResult, line: usize) -> bool {
    result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink") && f.line == line)
}

#[test]
fn test_python_html_escape_suppresses_mark_safe() {
    let source = r#"from flask import Flask, request
from django.utils.safestring import mark_safe
import html

app = Flask(__name__)

@app.route("/profile")
def profile():
    name = request.args.get("name")
    escaped = html.escape(name)
    return mark_safe(escaped)
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "html.escape result should suppress mark_safe XSS finding"
    );
}

#[test]
fn test_python_mark_safe_without_escape_fires() {
    let source = r#"from flask import Flask, request
from django.utils.safestring import mark_safe

app = Flask(__name__)

@app.route("/profile")
def profile():
    name = request.args.get("name")
    return mark_safe(name)
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(has_taint_sink(&result), "unsanitized mark_safe should fire");
}

#[test]
fn test_python_format_html_result_suppresses_mark_safe() {
    let source = r#"from flask import Flask, request
from django.utils.html import format_html
from django.utils.safestring import mark_safe

app = Flask(__name__)

@app.route("/profile")
def profile():
    name = request.args.get("name")
    safe_html = format_html("<b>{}</b>", name)
    return mark_safe(safe_html)
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "format_html with a literal format string should cleanse its assigned result"
    );
}

#[test]
fn test_python_format_html_tainted_format_does_not_suppress_downstream_mark_safe() {
    let source = r#"from flask import Flask, request
from django.utils.html import format_html
from django.utils.safestring import mark_safe

app = Flask(__name__)

@app.route("/profile")
def profile():
    fmt = request.args.get("fmt")
    safe_html = format_html(fmt, "value")
    return mark_safe(safe_html)
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 11),
        "format_html with a tainted format string must not cleanse the downstream result"
    );
}

#[test]
fn test_python_format_html_result_cleansing_is_variable_scoped() {
    let source = r#"from flask import Flask, request
from django.utils.html import format_html
from django.utils.safestring import mark_safe

app = Flask(__name__)

@app.route("/profile")
def profile():
    name = request.args.get("name")
    safe_html = format_html("<b>{}</b>", "literal")
    return mark_safe(name)
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 11),
        "safe format_html result must not cleanse unrelated tainted variables"
    );
}

#[test]
fn test_python_sql_parametrized_execute_suppresses() {
    let source = r#"from flask import Flask, request

app = Flask(__name__)

@app.route("/items")
def items(cursor):
    name = request.args.get("name")
    cursor.execute("SELECT * FROM items WHERE name = %s", (name,))
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "DB-API placeholder query with separate params should suppress"
    );
}

#[test]
fn test_python_sql_f_string_execute_fires() {
    let source = r#"from flask import Flask, request

app = Flask(__name__)

@app.route("/items")
def items(cursor):
    name = request.args.get("name")
    cursor.execute(f"SELECT * FROM items WHERE name = '{name}'")
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(has_taint_sink(&result), "raw f-string SQL should fire");
}

#[test]
fn test_python_sqlalchemy_text_bindparams_suppresses() {
    let source = r#"from flask import Flask, request
from sqlalchemy import text

app = Flask(__name__)

@app.route("/items")
def items(session):
    name = request.args.get("name")
    session.execute(text("SELECT * FROM items WHERE name = :name").bindparams(name=name))
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "SQLAlchemy text(...).bindparams(...) should suppress"
    );
}

#[test]
fn test_python_ssrf_url_allowlist_reject_branch_suppresses() {
    let source = r#"from flask import Flask, request
from urllib.parse import urlparse
import requests

ALLOWED_HOSTS = {"example.com"}
app = Flask(__name__)

@app.route("/fetch")
def fetch():
    url = request.args.get("url")
    parsed = urlparse(url)
    if parsed.hostname not in ALLOWED_HOSTS:
        return "blocked"
    return requests.get(url)
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "urlparse hostname allowlist reject branch should suppress SSRF"
    );
}

#[test]
fn test_python_ssrf_inverted_allowlist_fires() {
    let source = r#"from flask import Flask, request
from urllib.parse import urlparse
import requests

ALLOWED_HOSTS = {"example.com"}
app = Flask(__name__)

@app.route("/fetch")
def fetch():
    url = request.args.get("url")
    parsed = urlparse(url)
    if parsed.hostname in ALLOWED_HOSTS:
        return "blocked"
    return requests.get(url)
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "inverted URL allowlist guard must not suppress SSRF"
    );
}

#[test]
fn test_python_ssrf_unrelated_urlparse_does_not_suppress() {
    let source = r#"from flask import Flask, request
from urllib.parse import urlparse
import requests

ALLOWED_HOSTS = {"example.com"}
app = Flask(__name__)

@app.route("/fetch")
def fetch():
    url = request.args.get("url")
    other = urlparse("https://example.com")
    if other.hostname not in ALLOWED_HOSTS:
        return "blocked"
    return requests.get(url)
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "allowlist check on unrelated URL must not suppress SSRF"
    );
}

#[test]
fn test_python_ssrf_aiohttp_client_session_get_fires() {
    let source = r#"from flask import Flask, request
import aiohttp

app = Flask(__name__)

@app.route("/fetch")
async def fetch():
    url = request.args.get("url")
    async with aiohttp.ClientSession() as session:
        return await session.get(url)
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 10),
        "aiohttp ClientSession.get should fire when its URL argument is tainted"
    );
}

#[test]
fn test_python_ssrf_aiohttp_client_session_allowlist_suppresses() {
    let source = r#"from flask import Flask, request
from urllib.parse import urlparse
import aiohttp

ALLOWED_HOSTS = {"example.com"}
app = Flask(__name__)

@app.route("/fetch")
async def fetch():
    url = request.args.get("url")
    parsed = urlparse(url)
    if parsed.hostname not in ALLOWED_HOSTS:
        return "blocked"
    async with aiohttp.ClientSession() as session:
        return await session.get(url)
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "urlparse hostname allowlist should suppress aiohttp SSRF"
    );
}

#[test]
fn test_python_ssrf_aiohttp_top_level_request_alias_fires() {
    let source = r#"from flask import Flask, request
import aiohttp as ah

app = Flask(__name__)

@app.route("/fetch")
async def fetch():
    url = request.args.get("url")
    return await ah.request("GET", url)
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 9),
        "aiohttp.request should treat arg1 as the tainted URL"
    );
}

#[test]
fn test_python_ssrf_aiohttp_direct_client_session_post_fires() {
    let source = r#"from flask import Flask, request
import aiohttp

app = Flask(__name__)

@app.route("/fetch")
async def fetch():
    url = request.args.get("url")
    return await aiohttp.ClientSession().post(url)
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 9),
        "direct aiohttp ClientSession().post should fire on tainted URL"
    );
}

#[test]
fn test_python_ssrf_unrelated_get_method_does_not_fire_with_aiohttp_import() {
    let source = r#"from flask import Flask, request
import aiohttp

app = Flask(__name__)

@app.route("/fetch")
def fetch(cache):
    url = request.args.get("url")
    return cache.get(url)
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "aiohttp support must not turn unrelated .get calls into SSRF sinks"
    );
}

#[test]
fn test_python_yaml_safe_loader_suppresses() {
    let source = r#"from flask import Flask, request
import yaml

app = Flask(__name__)

@app.route("/load")
def load_config():
    payload = request.get_data()
    return yaml.load(payload, Loader=yaml.SafeLoader)
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "yaml.load with SafeLoader should suppress deserialization finding"
    );
}

#[test]
fn test_python_yaml_load_without_safe_loader_fires() {
    let source = r#"from flask import Flask, request
import yaml

app = Flask(__name__)

@app.route("/load")
def load_config():
    payload = request.get_data()
    return yaml.load(payload)
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "yaml.load without SafeLoader should fire"
    );
}

#[test]
fn test_python_json_loads_does_not_fire_deserialization() {
    let source = r#"from flask import Flask, request
import json

app = Flask(__name__)

@app.route("/load")
def load_config():
    payload = request.get_data()
    return json.loads(payload)
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "json.loads parses data but is not a CWE-502 code-execution sink"
    );
}

#[test]
fn test_python_explicit_unsafe_deserializers_still_fire() {
    let cases = [
        ("import cPickle", "cPickle.loads(payload)"),
        ("import cPickle", "cPickle.load(payload)"),
        ("import cloudpickle", "cloudpickle.loads(payload)"),
        ("import cloudpickle", "cloudpickle.load(payload)"),
        ("import marshal", "marshal.loads(payload)"),
        ("import marshal", "marshal.load(payload)"),
        ("import dill", "dill.loads(payload)"),
        ("import dill", "dill.load(payload)"),
        ("import jsonpickle", "jsonpickle.decode(payload)"),
    ];

    for (import_line, call) in cases {
        let source = format!(
            r#"from flask import Flask, request
{}

app = Flask(__name__)

@app.route("/load")
def load_config():
    payload = request.get_data()
    return {}
"#,
            import_line, call
        );
        let result = run_taint_python_single(&source, "app.py", BTreeSet::from([1]));
        assert!(
            has_taint_sink(&result),
            "{call} should remain a CWE-502 sink"
        );
    }
}
