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

fn has_taint_sink_on(result: &prism::slice::SliceResult, line: usize) -> bool {
    result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink") && f.line == line)
}

#[test]
fn test_python_target_scoped_pydantic_field_only_source_reaches_sql() {
    let source = r#"from fastapi import FastAPI
from pydantic import BaseModel

app = FastAPI()

class Item(BaseModel):
    filter_field: str

@app.post("/items")
def create_item(item: Item):
    cursor.execute(f"SELECT * FROM x WHERE f = {item.filter_field}")
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 11),
        "FastAPI Pydantic model used only through item.field should reach SQL sink"
    );
}

#[test]
fn test_python_fastapi_type_annotated_receiver_taints_pydantic_source() {
    let source = r#"from fastapi import FastAPI
from pydantic import BaseModel

app: FastAPI = FastAPI()

class Item(BaseModel):
    filter_field: str

@app.post("/items")
def create_item(item: Item):
    cursor.execute(f"SELECT * FROM x WHERE f = {item.filter_field}")
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 11),
        "type-annotated FastAPI receiver should still mark route params as tainted"
    );
}

#[test]
fn test_python_target_scoped_source_does_not_taint_same_line_db_param() {
    let source = r#"from fastapi import FastAPI
from pydantic import BaseModel

app = FastAPI()

class Item(BaseModel):
    filter_field: str

@app.post("/items")
def create_item(item: Item, db: Session):
    cursor.execute(db)
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        !has_taint_sink_on(&result, 11),
        "line-scoped handler source must not taint unrelated db parameter"
    );
}

#[test]
fn test_python_fastapi_request_body_reaches_pickle_loads() {
    let source = r#"from fastapi import FastAPI, Request
import pickle

app = FastAPI()

@app.post("/upload")
async def upload(request: Request):
    return pickle.loads(await request.body())
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "request: Request seed should taint request.body() at pickle.loads sink"
    );
}

#[test]
fn test_python_render_template_string_autoescape_safe_no_flat_leak() {
    let source = r#"from flask import Flask, request, render_template_string

app = Flask(__name__)

@app.route("/profile")
def profile():
    name = request.args.get("name")
    return render_template_string("Hello {{ name }}", name=name)
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        !has_taint_sink_on(&result, 8),
        "default-autoescaped render_template_string should not leak through flat fallback"
    );
}

#[test]
fn test_python_render_template_string_safe_filter_fires() {
    let source = r#"from flask import Flask, request, render_template_string

app = Flask(__name__)

@app.route("/profile")
def profile():
    name = request.args.get("name")
    return render_template_string("Hello {{ name | safe }}", name=name)
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "inline | safe disables autoescape for tainted keyword value"
    );
}

#[test]
fn test_python_render_template_string_safe_filter_keyword_coupling() {
    let source = r#"from flask import Flask, request, render_template_string

app = Flask(__name__)

@app.route("/profile")
def profile():
    y = request.args.get("name")
    return render_template_string("Hello {{ x | safe }}", x="literal", y=y)
"#;
    let result = run_taint_python_single(source, "app.py", BTreeSet::from([1]));
    assert!(
        !has_taint_sink_on(&result, 8),
        "tainted y should not fire when only x is rendered with | safe"
    );
}

#[test]
fn test_python_format_html_arg0_only_policy() {
    let source = r#"from django.utils.html import format_html

def unsafe(fmt, value):
    return format_html(fmt, value)

def safe(value):
    return format_html("<b>{}</b>", value)
"#;
    let unsafe_result = run_taint_python_single(source, "views.py", BTreeSet::from([3]));
    assert!(has_taint_sink_on(&unsafe_result, 4));

    let safe_result = run_taint_python_single(source, "views.py", BTreeSet::from([6]));
    assert!(
        !has_taint_sink_on(&safe_result, 7),
        "format_html with literal format string must not fire for tainted arg1"
    );
}
