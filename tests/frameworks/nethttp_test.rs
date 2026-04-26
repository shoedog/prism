#[path = "../common/mod.rs"]
mod common;
use common::*;

fn parse_go(source: &str) -> ParsedFile {
    ParsedFile::parse("test.go", source, Language::Go).unwrap()
}

#[test]
fn test_nethttp_positive_handler_signature() {
    let source = r#"
package main

import "net/http"

func handler(w http.ResponseWriter, r *http.Request) {
    _, _ = w.Write([]byte("hello"))
}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("net/http"));
}

#[test]
fn test_nethttp_positive_handlefunc() {
    let source = r#"
package main

import "net/http"

func main() {
    http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {})
    http.ListenAndServe(":8080", nil)
}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("net/http"));
}

#[test]
fn test_nethttp_negative_wrong_import() {
    // Imports something that mentions "net" but isn't net/http.
    let source = r#"
package main

import "net"

func main() {
    _ = net.Dial("tcp", "localhost:8080")
}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), None);
}

#[test]
fn test_nethttp_negative_vendored_unused() {
    // Imports net/http but never uses *http.Request, http.HandleFunc, or http.Handle.
    let source = r#"
package main

import (
    _ "net/http"
)

func main() {}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), None);
}
