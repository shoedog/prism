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
    // Calling framework() twice should return the same value without re-detecting.
    let source = r#"
package main

import "github.com/gin-gonic/gin"

func handler(c *gin.Context) {}
"#;
    let parsed = parse_go(source);
    let first = parsed.framework().map(|f| f.name);
    let second = parsed.framework().map(|f| f.name);
    assert_eq!(first, Some("gin"));
    assert_eq!(first, second);
}

#[test]
fn test_registry_iteration_order() {
    // Sanity-check that ALL_FRAMEWORKS is in the expected order: gin, gorilla/mux, nethttp.
    let names: Vec<&str> = frameworks::ALL_FRAMEWORKS.iter().map(|f| f.name).collect();
    assert_eq!(
        names,
        vec!["gin", "gorilla/mux", "net/http"],
        "registry order must be: gin, gorilla/mux, net/http (more-specific first)"
    );
}
