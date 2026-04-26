#[path = "../common/mod.rs"]
mod common;
use common::*;

fn parse_go(source: &str) -> ParsedFile {
    ParsedFile::parse("test.go", source, Language::Go).unwrap()
}

#[test]
fn test_gorilla_mux_positive() {
    let source = r#"
package main

import (
    "net/http"

    "github.com/gorilla/mux"
)

func handler(w http.ResponseWriter, r *http.Request) {
    vars := mux.Vars(r)
    _ = vars["id"]
}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("gorilla/mux"));
}

#[test]
fn test_gorilla_mux_negative_no_vars_call() {
    // Imports gorilla/mux but never calls mux.Vars.
    let source = r#"
package main

import (
    _ "github.com/gorilla/mux"
)

func main() {}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), None);
}

#[test]
fn test_gorilla_mux_disambiguation_wins_over_nethttp() {
    // File uses both *http.Request and mux.Vars; mux wins per ordered registry.
    let source = r#"
package main

import (
    "net/http"

    "github.com/gorilla/mux"
)

func handler(w http.ResponseWriter, r *http.Request) {
    _ = mux.Vars(r)
}
"#;
    let parsed = parse_go(source);
    assert_eq!(parsed.framework().map(|f| f.name), Some("gorilla/mux"));
}
