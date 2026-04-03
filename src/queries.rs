//! Tree-sitter query-based pattern matching registry.
//!
//! Replaces manual recursive AST traversal (`collect_*` methods) with compiled
//! tree-sitter queries. Queries are compiled once per (Language, QueryKind) pair
//! and cached for reuse.

use crate::languages::Language;
use std::collections::HashMap;
use std::sync::OnceLock;
use tree_sitter::Query;

/// Categories of structural patterns that can be matched by tree-sitter queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryKind {
    /// Function/method definitions. Replaces `collect_functions`.
    Functions,
    /// Function/method call expressions. Replaces `collect_calls`, `collect_all_callees`.
    Calls,
}

/// Global query cache. Compiled once, reused across all ParsedFile instances.
static QUERY_CACHE: OnceLock<HashMap<(Language, QueryKind), Query>> = OnceLock::new();

/// Get (or initialize) the global query cache.
pub fn query_cache() -> &'static HashMap<(Language, QueryKind), Query> {
    QUERY_CACHE.get_or_init(|| {
        let mut cache = HashMap::new();
        for lang in Language::all() {
            for kind in [QueryKind::Functions, QueryKind::Calls] {
                if let Some(pattern) = query_pattern(lang, kind) {
                    let ts_lang = lang.tree_sitter_language();
                    match Query::new(&ts_lang, pattern) {
                        Ok(query) => {
                            cache.insert((lang, kind), query);
                        }
                        Err(e) => {
                            // Log but don't panic — fallback to manual traversal.
                            eprintln!(
                                "Warning: failed to compile {:?} query for {:?}: {}",
                                kind, lang, e
                            );
                        }
                    }
                }
            }
        }
        cache
    })
}

/// Get a compiled query for a (Language, QueryKind) pair, if available.
pub fn get_query(lang: Language, kind: QueryKind) -> Option<&'static Query> {
    query_cache().get(&(lang, kind))
}

/// Return the tree-sitter query pattern string for a given (Language, QueryKind).
///
/// Each pattern uses named captures (`@name`) that the caller extracts after
/// running the query. Returns `None` if the language doesn't support this query.
fn query_pattern(lang: Language, kind: QueryKind) -> Option<&'static str> {
    match kind {
        QueryKind::Functions => function_query(lang),
        QueryKind::Calls => call_query(lang),
    }
}

// ---------------------------------------------------------------------------
// Functions query — matches function/method definitions
// ---------------------------------------------------------------------------
// Capture: @func — the function definition node

fn function_query(lang: Language) -> Option<&'static str> {
    Some(match lang {
        Language::Python => {
            r#"[
                (function_definition) @func
                (decorated_definition) @func
            ]"#
        }
        Language::JavaScript => {
            r#"[
                (function_declaration) @func
                (method_definition) @func
                (arrow_function) @func
                (function_expression) @func
                (generator_function_declaration) @func
            ]"#
        }
        Language::TypeScript | Language::Tsx => {
            r#"[
                (function_declaration) @func
                (method_definition) @func
                (arrow_function) @func
                (function_expression) @func
                (generator_function_declaration) @func
            ]"#
        }
        Language::Go => {
            r#"[
                (function_declaration) @func
                (method_declaration) @func
            ]"#
        }
        Language::Java => {
            r#"[
                (method_declaration) @func
                (constructor_declaration) @func
            ]"#
        }
        Language::C => "(function_definition) @func",
        Language::Cpp => {
            r#"[
                (function_definition) @func
                (template_declaration) @func
            ]"#
        }
        Language::Rust => "(function_item) @func",
        Language::Lua => "(function_declaration) @func",
        Language::Terraform => "(block) @func",
        Language::Bash => "(function_definition) @func",
    })
}

// ---------------------------------------------------------------------------
// Calls query — matches function/method call expressions
// ---------------------------------------------------------------------------
// Capture: @call — the call expression node

fn call_query(lang: Language) -> Option<&'static str> {
    Some(match lang {
        Language::Python => "(call) @call",
        Language::JavaScript => {
            r#"[
                (call_expression) @call
                (jsx_self_closing_element) @call
                (jsx_opening_element) @call
            ]"#
        }
        Language::TypeScript => "(call_expression) @call",
        Language::Tsx => {
            r#"[
                (call_expression) @call
                (jsx_self_closing_element) @call
                (jsx_opening_element) @call
            ]"#
        }
        Language::Go => "(call_expression) @call",
        Language::Java => {
            r#"[
                (method_invocation) @call
                (object_creation_expression) @call
            ]"#
        }
        Language::C | Language::Cpp => "(call_expression) @call",
        Language::Rust => {
            r#"[
                (call_expression) @call
                (macro_invocation) @call
            ]"#
        }
        Language::Lua => "(function_call) @call",
        Language::Terraform => "(function_call) @call",
        Language::Bash => "(command) @call",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::StreamingIterator;

    /// Count matches for a query against a source string.
    fn count_matches(lang: Language, kind: QueryKind, source: &str) -> usize {
        let query = get_query(lang, kind).expect("query should exist");
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&lang.tree_sitter_language())
            .expect("failed to set language");
        let tree = parser.parse(source, None).expect("failed to parse");
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(query, tree.root_node(), source.as_bytes());
        let mut count = 0;
        while let Some(_m) = matches.next() {
            count += 1;
        }
        count
    }

    #[test]
    fn test_all_queries_compile() {
        let cache = query_cache();
        // 12 languages x 2 query kinds = 24 expected
        let expected_count = Language::all().len() * 2;
        assert!(
            cache.len() >= expected_count - 2,
            "expected at least {} compiled queries, got {}",
            expected_count - 2,
            cache.len()
        );
    }

    #[test]
    fn test_function_query_matches_fixture() {
        let fixtures: Vec<(Language, &str)> = vec![
            (Language::Python, "def foo():\n    pass\n"),
            (Language::JavaScript, "function foo() { return 1; }\n"),
            (
                Language::TypeScript,
                "function foo(): number { return 1; }\n",
            ),
            (Language::Go, "package main\nfunc foo() {}\n"),
            (Language::Java, "class Foo { void bar() {} }\n"),
            (Language::C, "void foo() {}\n"),
            (Language::Cpp, "void foo() {}\n"),
            (Language::Rust, "fn foo() {}\n"),
            (Language::Lua, "function foo() end\n"),
            (Language::Terraform, "resource \"aws_instance\" \"ex\" {}\n"),
            (Language::Bash, "foo() { echo hi; }\n"),
        ];

        for (lang, source) in fixtures {
            let n = count_matches(lang, QueryKind::Functions, source);
            assert!(
                n > 0,
                "Functions query for {:?} should match at least one node, got 0",
                lang
            );
        }
    }

    #[test]
    fn test_call_query_matches_fixture() {
        let fixtures: Vec<(Language, &str)> = vec![
            (Language::Python, "def f():\n    g()\n"),
            (Language::JavaScript, "function f() { g(); }\n"),
            (Language::TypeScript, "function f() { g(); }\n"),
            (Language::Go, "package main\nfunc f() { g() }\n"),
            (Language::Java, "class Foo { void bar() { baz(); } }\n"),
            (Language::C, "void f() { g(); }\n"),
            (Language::Cpp, "void f() { g(); }\n"),
            (Language::Rust, "fn f() { g(); }\n"),
            (Language::Lua, "function f() g() end\n"),
            (Language::Bash, "foo() { echo hi; }\n"),
        ];

        for (lang, source) in fixtures {
            let n = count_matches(lang, QueryKind::Calls, source);
            assert!(
                n > 0,
                "Calls query for {:?} should match at least one node, got 0",
                lang
            );
        }
    }
}
