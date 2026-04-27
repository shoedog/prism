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
