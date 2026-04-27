#[path = "../common/mod.rs"]
mod common;
use common::*;

fn parse_python(source: &str) -> ParsedFile {
    ParsedFile::parse("app.py", source, Language::Python).unwrap()
}

#[test]
fn test_fastapi_route_positive() {
    let source = r#"from fastapi import FastAPI

app = FastAPI()

@app.post("/items")
def create_item(item: Item):
    pass
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("fastapi"));
}

#[test]
fn test_fastapi_router_positive() {
    let source = r#"from fastapi import APIRouter

router = APIRouter()

@router.get("/items")
def list_items():
    pass
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("fastapi"));
}

#[test]
fn test_fastapi_type_annotated_receiver_positive() {
    let source = r#"from fastapi import FastAPI

app: FastAPI = FastAPI()

@app.get("/items")
def list_items():
    pass
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("fastapi"));
}

#[test]
fn test_fastapi_tuple_receiver_positive() {
    let source = r#"from fastapi import APIRouter, FastAPI

app, router = FastAPI(), APIRouter()

@router.post("/items")
def create_item():
    pass
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("fastapi"));
}

#[test]
fn test_fastapi_negative_unbound_app_get() {
    let source = r#"from fastapi import FastAPI

class App:
    def get(self, path):
        return lambda f: f

app = App()

@app.get("/items")
def helper():
    pass
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), None);
}

#[test]
fn test_fastapi_negative_docstring_comment_receivers() {
    let source = r#"from fastapi import FastAPI

"""
shadow = FastAPI()
@shadow.get("/items")
"""
# app = FastAPI()

class App:
    def get(self, path):
        return lambda f: f

app = App()

@app.get("/items")
def helper():
    pass
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), None);
}

#[test]
fn test_fastapi_attribute_lhs_receiver_negative() {
    // Route receivers must be bare local identifiers. `holder.app = FastAPI()`
    // should not register either `holder` or nested child identifier `app`.
    let source = r#"from fastapi import FastAPI

class Holder:
    pass

holder = Holder()
holder.app = FastAPI()

@app.get("/items")
def helper():
    pass
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), None);
}

#[test]
fn test_fastapi_scans_past_unregistered_route_decorator_positive() {
    // Multiple route-shaped decorators should scan all decorators, not stop at
    // the first unregistered receiver.
    let source = r#"from fastapi import FastAPI

class Other:
    def get(self, path):
        return lambda f: f

other = Other()
app = FastAPI()

@other.get("/shadow")
@app.get("/items")
def list_items():
    pass
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("fastapi"));
}

#[test]
fn test_fastapi_module_qualified_constructor_positive() {
    // `import fastapi; app = fastapi.FastAPI()` resolves the namespace via the
    // import map.
    let source = r#"import fastapi

app = fastapi.FastAPI()

@app.get("/items")
def list_items():
    pass
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("fastapi"));
}

#[test]
fn test_fastapi_aliased_module_constructor_positive() {
    // `import fastapi as fa; app = fa.FastAPI()` — the alias resolves to the
    // fastapi module per `extract_imports`.
    let source = r#"import fastapi as fa

app = fa.FastAPI()

@app.get("/items")
def list_items():
    pass
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("fastapi"));
}

#[test]
fn test_fastapi_unrelated_namespace_negative() {
    // `unrelated.FastAPI()` — basename matches but the namespace does not
    // resolve to fastapi, so the binding does not register as a receiver.
    let source = r#"from fastapi import FastAPI as _Real
import unrelated

app = unrelated.FastAPI()

@app.get("/items")
def helper():
    pass
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), None);
}

#[test]
fn test_fastapi_local_class_shadow_negative() {
    // A locally-defined `FastAPI` class shadows the import. The bare basename
    // matches, but the import map's entry for `FastAPI` is absent (we only
    // `import fastapi`, not `from fastapi import FastAPI`), so the local class
    // is not treated as a constructor.
    let source = r#"import fastapi

class FastAPI:
    def get(self, path):
        return lambda f: f

app = FastAPI()

@app.get("/items")
def helper():
    pass
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), None);
}

#[test]
fn test_fastapi_parenthesized_tuple_receiver_positive() {
    // `(app, router) = FastAPI(), APIRouter()` — non-canonical but valid Python.
    // `sequence_elements` unwraps `parenthesized_expression` before zipping.
    let source = r#"from fastapi import APIRouter, FastAPI

(app, router) = FastAPI(), APIRouter()

@router.post("/items")
def create_item():
    pass
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("fastapi"));
}
