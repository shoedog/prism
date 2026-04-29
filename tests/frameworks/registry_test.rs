#[path = "../common/mod.rs"]
mod common;
use common::*;
use prism::frameworks;

fn parse_go(source: &str) -> ParsedFile {
    ParsedFile::parse("test.go", source, Language::Go).unwrap()
}

#[test]
fn test_no_framework_plain_go_file() {
    // A plain Go file with no web imports. Pins ACK §3 Q5 quiet-mode default.
    let source = r#"
package main

import "fmt"

func main() {
    fmt.Println("hi")
}
"#;
    let parsed = parse_go(source);
    assert_eq!(
        parsed.framework().map(|f| f.name),
        None,
        "plain Go file with no web imports should detect no framework"
    );
}

#[test]
fn test_framework_caching_via_oncecell() {
    let source = r#"
package main

import "github.com/gin-gonic/gin"

func handler(c *gin.Context) {}
"#;
    let parsed = parse_go(source);
    // Cell empty before first call.
    assert!(
        parsed.framework.get().is_none(),
        "cache empty before first call"
    );
    // First call populates.
    assert_eq!(parsed.framework().map(|f| f.name), Some("gin"));
    // Cell now populated; second call hits cache.
    assert!(
        parsed.framework.get().is_some(),
        "cache populated after first call"
    );
    assert_eq!(parsed.framework().map(|f| f.name), Some("gin"));
}

#[test]
fn test_registry_iteration_order() {
    // Sanity-check that ALL_FRAMEWORKS is in the expected order: JS/TS and
    // Python framework-specific entries first, then Go's more-specific entries.
    let names: Vec<&str> = frameworks::ALL_FRAMEWORKS.iter().map(|f| f.name).collect();
    assert_eq!(
        names,
        vec![
            "nestjs",
            "fastify",
            "express",
            "koa",
            "fastapi",
            "drf",
            "flask",
            "django",
            "gin",
            "gorilla/mux",
            "net/http"
        ],
        "registry order must keep more-specific frameworks first"
    );
}
