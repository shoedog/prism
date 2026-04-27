#[path = "../common/mod.rs"]
mod common;
use common::*;

fn parse_python(source: &str) -> ParsedFile {
    ParsedFile::parse("views.py", source, Language::Python).unwrap()
}

#[test]
fn test_drf_api_view_positive() {
    let source = r#"from rest_framework.decorators import api_view

@api_view(["GET"])
def view(request):
    return Response({})
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("drf"));
}

#[test]
fn test_drf_viewset_positive() {
    let source = r#"from rest_framework.viewsets import ViewSet

class Users(ViewSet):
    def list(self, request):
        pass
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("drf"));
}

#[test]
fn test_django_positive() {
    let source = r#"from django.shortcuts import render

def view(request):
    return render(request, "index.html")
"#;
    let parsed = parse_python(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("django"));
}
