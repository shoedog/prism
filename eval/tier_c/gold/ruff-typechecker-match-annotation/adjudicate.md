# Adjudication — ruff-typechecker-match-annotation

repo: ruff  sha: 44f6d18  symbol: match_annotation
oracle_health: {"lsp": "ok", "prism": "unavailable: prism nav callers failed (3): {\n  \"error\": {\n    \"AmbiguousSymbol\": {\n      \"candidates\": [\n        {\n          \"Function\": {\n            \"end_byte\": 2585,\n            \"end_line\": 89,\n            \"file\": \"crates/ruff_linter/src/rules/flake8_async/rules/blocking_http_call_httpx.rs\",\n            \"name\": \"match_annotation\",\n            \"ordinal\": 0,\n            \"start_byte\": 1625,\n            \"start_line\": 60\n          }\n        },\n        {\n          \"Function\": {\n            \"end_byte\": 5112,\n            \"end_line\": 178,\n            \"file\": \"crates/ruff_linter/src/rules/flake8_async/rules/blocking_path_methods.rs\",\n            \"name\": \"match_annotation\",\n            \"ordinal\": 0,\n            \"start_byte\": 4587,\n            \"start_line\": 162\n          }\n        },\n        {\n          \"Function\": {\n            \"end_byte\": 8495,\n            \"end_line\": 292,\n            \"file\": \"crates/ruff_linter/src/rules/flake8_self/rules/private_member_access.rs\",\n            \"name\": \"match_annotation\",\n            \"ordinal\": 0,\n            \"start_byte\": 8117,\n            \"start_line\": 280\n          }\n        },\n        {\n          \"Function\": {\n            \"end_byte\": 26458,\n            \"end_line\": 747,\n            \"file\": \"crates/ruff_python_semantic/src/analyze/typing.rs\",\n            \"name\": \"match_annotation\",\n            \"ordinal\": 0,\n            \"start_byte\": 26170,\n            \"start_line\": 743\n          }\n        },\n        {\n          \"Function\": {\n            \"end_byte\": 27614,\n            \"end_line\": 775,\n            \"file\": \"crates/ruff_python_semantic/src/analyze/typing.rs\",\n            \"name\": \"match_annotation\",\n            \"ordinal\": 0,\n            \"start_byte\": 27457,\n            \"start_line\": 773\n          }\n        },\n        {\n          \"Function\": {\n            \"end_byte\": 31178,\n            \"end_line\": 884,\n            \"file\": \"crates/ruff_python_semantic/src/analyze/typing.rs\",\n            \"name\": \"match_annotation\",\n            \"ordinal\": 0,\n            \"start_byte\": 29714,\n            \"s"}

## Disagreement band (needs verdict — source-verify; a tool provenance tag is NOT truth)

### crates/ruff_linter/src/rules/flake8_self/rules/private_member_access.rs:312 — match_initializer
provenance: lsp
d_member: none
```
        };

        match &*call.func {
            Expr::Subscript(_) => Self::match_annotation(&call.func, semantic),

            Expr::Name(name) => {
                matches!(&*name.id, "cls" | "mcs") || Self::is_current_class_name(name, semantic)
```
verdict: 
reason: 

### crates/ruff_python_semantic/src/analyze/typing.rs:724 — check_type
provenance: lsp
d_member: none
```
            // ```
            Some(Stmt::FunctionDef(ast::StmtFunctionDef { returns, .. })) => returns
                .as_ref()
                .is_some_and(|return_ann| T::match_annotation(return_ann, semantic)),

            _ => false,
        },
```
verdict: 
reason: 

## Auto-accepted (both — LSP and prism agree; still source-verify before freezing)

(none)
