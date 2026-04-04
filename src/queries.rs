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
    /// Assignment expressions and declarations with initializers.
    /// Replaces `collect_assignments`, `collect_assignment_paths`.
    /// Also used (same query, different extraction) for R-values.
    Assignments,
    /// Identifier nodes. Replaces `collect_all_identifiers`, `collect_identifiers_at_row`.
    /// Also used for `collect_variable_refs` with text post-filter.
    Identifiers,
    /// Return statements/expressions. Replaces `collect_returns`.
    Returns,
}

/// Global query cache. Compiled once, reused across all ParsedFile instances.
static QUERY_CACHE: OnceLock<HashMap<(Language, QueryKind), Query>> = OnceLock::new();

/// Get (or initialize) the global query cache.
pub fn query_cache() -> &'static HashMap<(Language, QueryKind), Query> {
    QUERY_CACHE.get_or_init(|| {
        let mut cache = HashMap::new();
        for lang in Language::all() {
            for kind in [
                QueryKind::Functions,
                QueryKind::Calls,
                QueryKind::Assignments,
                QueryKind::Identifiers,
                QueryKind::Returns,
            ] {
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
        QueryKind::Assignments => assignment_query(lang),
        QueryKind::Identifiers => identifier_query(lang),
        QueryKind::Returns => return_query(lang),
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

// ---------------------------------------------------------------------------
// Assignments query — matches assignment expressions and declarations
// ---------------------------------------------------------------------------
// Capture: @assign — the assignment or declaration node

fn assignment_query(lang: Language) -> Option<&'static str> {
    Some(match lang {
        Language::Python => {
            r#"[
                (assignment) @assign
                (augmented_assignment) @assign
                (named_expression) @assign
            ]"#
        }
        Language::JavaScript | Language::TypeScript | Language::Tsx => {
            r#"[
                (assignment_expression) @assign
                (augmented_assignment_expression) @assign
                (variable_declarator) @assign
            ]"#
        }
        Language::Go => {
            r#"[
                (assignment_statement) @assign
                (short_var_declaration) @assign
                (var_declaration) @assign
                (const_declaration) @assign
            ]"#
        }
        Language::Java => {
            r#"[
                (assignment_expression) @assign
                (local_variable_declaration) @assign
                (field_declaration) @assign
            ]"#
        }
        Language::C | Language::Cpp => {
            r#"[
                (assignment_expression) @assign
                (update_expression) @assign
                (declaration) @assign
                (init_declarator) @assign
            ]"#
        }
        Language::Rust => {
            r#"[
                (assignment_expression) @assign
                (compound_assignment_expr) @assign
                (let_declaration) @assign
            ]"#
        }
        Language::Lua => {
            // tree-sitter-lua uses `variable_declaration` for `local x = 1`
            r#"[
                (assignment_statement) @assign
                (variable_declaration) @assign
            ]"#
        }
        Language::Terraform => "(attribute) @assign",
        Language::Bash => "(variable_assignment) @assign",
    })
}

// ---------------------------------------------------------------------------
// Identifiers query — matches identifier nodes
// ---------------------------------------------------------------------------
// Capture: @ident — the identifier node

fn identifier_query(lang: Language) -> Option<&'static str> {
    // Matches core identifier nodes. Used primarily for find_variable_references
    // and find_variable_references_scoped, where we filter by text content anyway.
    //
    // NOTE: This intentionally does NOT cover all types matched by
    // Language::is_identifier_node() (e.g., property_identifier, field_identifier).
    // Methods needing broader matching (identifiers_on_line) use the manual path.
    Some(match lang {
        Language::Python | Language::Go | Language::Java | Language::Lua => "(identifier) @ident",
        Language::JavaScript | Language::TypeScript | Language::Tsx => {
            r#"[
                (identifier) @ident
                (shorthand_property_identifier) @ident
            ]"#
        }
        Language::C | Language::Cpp => "(identifier) @ident",
        Language::Rust => "(identifier) @ident",
        Language::Terraform => {
            r#"[
                (identifier) @ident
                (variable_expr) @ident
            ]"#
        }
        Language::Bash => {
            r#"[
                (variable_name) @ident
                (word) @ident
            ]"#
        }
    })
}

// ---------------------------------------------------------------------------
// Returns query — matches return statements/expressions
// ---------------------------------------------------------------------------
// Capture: @ret — the return node

fn return_query(lang: Language) -> Option<&'static str> {
    match lang {
        Language::Python
        | Language::JavaScript
        | Language::TypeScript
        | Language::Tsx
        | Language::Go
        | Language::Java
        | Language::C
        | Language::Cpp
        | Language::Lua => Some("(return_statement) @ret"),
        Language::Rust => Some("(return_expression) @ret"),
        // Terraform and Bash don't have structural return statements
        Language::Terraform | Language::Bash => None,
    }
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
        // 12 languages × 5 query kinds = 60, minus 2 (Returns is None for Terraform, Bash)
        let expected_count = Language::all().len() * 5 - 2;
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

    #[test]
    fn test_assignment_query_matches_fixture() {
        let fixtures: Vec<(Language, &str, usize)> = vec![
            (Language::Python, "def f():\n    x = 1\n    y += x\n", 2),
            (
                Language::JavaScript,
                "function f() { let x = 1; x += 2; }\n",
                2,
            ),
            (Language::Go, "package main\nfunc f() { x := 1 }\n", 1),
            (Language::C, "void f() { int x = 1; x = 2; }\n", 2),
            (Language::Rust, "fn f() { let x = 1; }\n", 1),
            (Language::Lua, "function f() local x = 1 end\n", 1),
            (Language::Bash, "foo() { x=1; }\n", 1),
        ];

        for (lang, source, expected_min) in fixtures {
            let n = count_matches(lang, QueryKind::Assignments, source);
            assert!(
                n >= expected_min,
                "Assignments query for {:?} should match at least {}, got {}",
                lang,
                expected_min,
                n
            );
        }
    }

    #[test]
    fn test_identifier_query_matches_fixture() {
        let fixtures: Vec<(Language, &str)> = vec![
            (Language::Python, "def f():\n    x = y\n"),
            (Language::JavaScript, "function f() { let x = y; }\n"),
            (Language::Go, "package main\nfunc f() { x := y }\n"),
            (Language::C, "void f() { int x = y; }\n"),
            (Language::Rust, "fn f() { let x = y; }\n"),
            (Language::Bash, "foo() { x=$y; }\n"),
        ];

        for (lang, source) in fixtures {
            let n = count_matches(lang, QueryKind::Identifiers, source);
            assert!(
                n > 0,
                "Identifiers query for {:?} should match at least one node, got 0",
                lang
            );
        }
    }

    #[test]
    fn test_return_query_matches_fixture() {
        let fixtures: Vec<(Language, &str)> = vec![
            (Language::Python, "def f():\n    return 1\n"),
            (Language::JavaScript, "function f() { return 1; }\n"),
            (Language::Go, "package main\nfunc f() int { return 1 }\n"),
            (Language::C, "int f() { return 1; }\n"),
            (Language::Rust, "fn f() -> i32 { return 1; }\n"),
            (Language::Lua, "function f() return 1 end\n"),
        ];

        for (lang, source) in fixtures {
            let n = count_matches(lang, QueryKind::Returns, source);
            assert!(
                n > 0,
                "Returns query for {:?} should match at least one node, got 0",
                lang
            );
        }

        // Terraform and Bash should not have return queries
        assert!(get_query(Language::Terraform, QueryKind::Returns).is_none());
        assert!(get_query(Language::Bash, QueryKind::Returns).is_none());
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

    /// Verify Assignments query captures a superset of variable names found by manual walk.
    ///
    /// The query path may produce slightly different match counts than manual traversal
    /// (e.g., query matches `variable_declarator` directly while manual also matches its
    /// parent `lexical_declaration`), but the *set of variables and lines* must match.
    #[test]
    fn test_assignment_query_vs_manual_consistency() {
        use crate::ast::ParsedFile;
        use std::collections::BTreeSet;

        let fixtures: Vec<(&str, Language, &str)> = vec![
            (
                "test.py",
                Language::Python,
                "def foo():\n    x = 1\n    y += x\n    z = bar()\n",
            ),
            (
                "test.js",
                Language::JavaScript,
                "function foo() { let x = 1; x += 2; }\n",
            ),
            (
                "test.go",
                Language::Go,
                "package main\nfunc foo() { x := 1; x = 2 }\n",
            ),
            ("test.c", Language::C, "void foo() { int x = 1; x = 2; }\n"),
            ("test.rs", Language::Rust, "fn foo() { let x = 1; }\n"),
        ];

        for (path, lang, source) in &fixtures {
            let parsed = ParsedFile::parse(path, source, *lang).unwrap();

            for func_node in parsed.all_functions() {
                let all_lines: std::collections::BTreeSet<usize> = {
                    let (start, end) = parsed.node_line_range(&func_node);
                    (start..=end).collect()
                };

                let query_result = parsed.assignment_lvalues_on_lines(&func_node, &all_lines);

                let mut manual_result = Vec::new();
                parsed.collect_assignments_manual(func_node, &all_lines, &mut manual_result);

                // Compare as sets of (name, line) — query and manual should surface
                // the same variable names on the same lines, even if counts differ.
                let q: BTreeSet<_> = query_result.iter().map(|(n, l)| (n.as_str(), *l)).collect();
                let m: BTreeSet<_> = manual_result
                    .iter()
                    .map(|(n, l)| (n.as_str(), *l))
                    .collect();

                assert_eq!(
                    q, m,
                    "Assignments mismatch for {:?} ({}): query={:?} manual={:?}",
                    lang, path, q, m
                );
            }
        }
    }
}
