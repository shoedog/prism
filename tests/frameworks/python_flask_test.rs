#[path = "../common/mod.rs"]
mod common;
use common::*;

fn parse_python(source: &str) -> ParsedFile {
    ParsedFile::parse("app.py", source, Language::Python).unwrap()
}

#[test]
fn test_flask_positive() {
    let source = r#"from flask import Flask

app = Flask(__name__)

@app.route("/")
def index():
    return "ok"
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("flask"));
}

#[test]
fn test_flask_negative_import_only() {
    let source = r#"from flask import request

def helper():
    return request.args
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), None);
}

#[test]
fn test_flask_aliased_module_constructor_positive() {
    let source = r#"import flask as fl

app = fl.Flask(__name__)

@app.route("/")
def index():
    return "ok"
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("flask"));
}

#[test]
fn test_flask_blueprint_receiver_positive() {
    let source = r#"from flask import Blueprint

bp = Blueprint("bp", __name__)

@bp.route("/")
def index():
    return "ok"
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("flask"));
}

#[test]
fn test_flask_local_class_shadow_negative() {
    let source = r#"class Flask:
    pass

app = Flask(__name__)

@app.route("/")
def index():
    return "ok"
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), None);
}
