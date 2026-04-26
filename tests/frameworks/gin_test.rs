#[path = "../common/mod.rs"]
mod common;
use common::*;

fn parse_go(source: &str) -> ParsedFile {
    ParsedFile::parse("test.go", source, Language::Go).unwrap()
}

#[test]
fn test_gin_positive_context_param() {
    let source = r#"
package main

import "github.com/gin-gonic/gin"

func handler(c *gin.Context) {
    c.JSON(200, gin.H{"hello": "world"})
}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("gin"));
}

#[test]
fn test_gin_negative_wrong_import() {
    let source = r#"
package main

import "github.com/some-other/gin-like-package"

func handler() {}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), None);
}

#[test]
fn test_gin_negative_vendored_unused() {
    let source = r#"
package main

import (
    _ "github.com/gin-gonic/gin"
)

func main() {}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), None);
}

#[test]
fn test_gin_disambiguation_wins_over_nethttp() {
    // File imports both gin and net/http; gin should win per ordered registry.
    let source = r#"
package main

import (
    "net/http"

    "github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
    c.String(http.StatusOK, "ok")
}
"#;
    let parsed = parse_go(source);
    assert_eq!(
        parsed.framework().map(|f| f.name),
        Some("gin"),
        "gin should win over net/http when both are detected"
    );
}
