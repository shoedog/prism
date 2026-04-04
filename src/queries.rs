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
        // 12 languages x 2 query kinds = 24 expected. All must compile.
        let expected_count = Language::all().len() * 2;
        assert_eq!(
            cache.len(),
            expected_count,
            "all {} queries should compile, but only {} did",
            expected_count,
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
            (
                Language::Lua,
                "local function foo() end\nfunction bar() end\n",
            ),
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

        // Lua-specific: verify both `local function` and `function` are detected.
        let lua_count = count_matches(
            Language::Lua,
            QueryKind::Functions,
            "local function foo() end\nfunction bar() end\n",
        );
        assert_eq!(
            lua_count, 2,
            "Lua Functions query should match both local and global functions"
        );
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

    /// Verify query-based and manual traversal produce identical results.
    ///
    /// Runs both code paths on multi-language fixtures and compares the set of
    /// matched node line ranges. This catches drift if queries are refined but
    /// the manual fallback isn't updated (or vice versa).
    #[test]
    fn test_query_vs_manual_consistency() {
        use crate::ast::ParsedFile;

        let fixtures: Vec<(&str, Language, &str)> = vec![
            (
                "test.py",
                Language::Python,
                "def foo():\n    bar()\n\ndef baz(x):\n    return x\n",
            ),
            (
                "test.js",
                Language::JavaScript,
                "function foo() { bar(); }\nconst baz = (x) => x;\n",
            ),
            (
                "test.go",
                Language::Go,
                "package main\nfunc foo() { bar() }\nfunc baz(x int) int { return x }\n",
            ),
            (
                "test.c",
                Language::C,
                "void foo() { bar(); }\nint baz(int x) { return x; }\n",
            ),
            (
                "test.rs",
                Language::Rust,
                "fn foo() { bar(); }\nfn baz(x: i32) -> i32 { x }\n",
            ),
            (
                "test.lua",
                Language::Lua,
                "local function foo() bar() end\nfunction baz(x) return x end\n",
            ),
        ];

        for (path, lang, source) in &fixtures {
            let parsed = ParsedFile::parse(path, source, *lang).unwrap();

            // --- Functions: query path vs manual path ---
            let query_funcs: Vec<(usize, usize)> = parsed
                .all_functions()
                .iter()
                .map(|n| (n.start_position().row, n.end_position().row))
                .collect();

            let mut manual_funcs_nodes = Vec::new();
            parsed.collect_functions_manual(parsed.tree.root_node(), &mut manual_funcs_nodes);
            let manual_funcs: Vec<(usize, usize)> = manual_funcs_nodes
                .iter()
                .map(|n| (n.start_position().row, n.end_position().row))
                .collect();

            assert_eq!(
                query_funcs, manual_funcs,
                "Functions mismatch for {:?} ({}): query={:?} manual={:?}",
                lang, path, query_funcs, manual_funcs
            );

            // --- Calls: query path vs manual path ---
            // Run callees_in_function on each detected function, comparing both paths.
            for func_node in parsed.all_functions() {
                let all_lines: std::collections::BTreeSet<usize> = {
                    let (start, end) = parsed.node_line_range(&func_node);
                    (start..=end).collect()
                };

                // Query-based path (used by function_calls_on_lines)
                let query_calls = parsed.function_calls_on_lines(&func_node, &all_lines);

                // Manual fallback path
                let mut manual_calls = Vec::new();
                parsed.collect_calls_manual(func_node, &all_lines, &mut manual_calls);

                // Compare sorted (name, line) pairs
                let mut q: Vec<_> = query_calls.iter().map(|(n, l)| (n.as_str(), *l)).collect();
                let mut m: Vec<_> = manual_calls.iter().map(|(n, l)| (n.as_str(), *l)).collect();
                q.sort();
                m.sort();

                assert_eq!(
                    q,
                    m,
                    "Calls mismatch for {:?} ({}) in function at row {}: query={:?} manual={:?}",
                    lang,
                    path,
                    func_node.start_position().row,
                    q,
                    m
                );
            }
        }
    }
}
