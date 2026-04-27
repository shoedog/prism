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
