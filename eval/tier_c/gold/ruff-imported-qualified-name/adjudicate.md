# Adjudication — ruff-imported-qualified-name

repo: ruff  sha: 44f6d18  symbol: qualified_name
oracle_health: {"lsp": "ok", "prism": "unavailable: prism nav callers failed (3): {\n  \"error\": {\n    \"AmbiguousSymbol\": {\n      \"candidates\": [\n        {\n          \"Function\": {\n            \"end_byte\": 24510,\n            \"end_line\": 742,\n            \"file\": \"crates/ruff_python_semantic/src/binding.rs\",\n            \"name\": \"qualified_name\",\n            \"ordinal\": 0,\n            \"start_byte\": 24427,\n            \"start_line\": 740\n          }\n        },\n        {\n          \"Function\": {\n            \"end_byte\": 25125,\n            \"end_line\": 763,\n            \"file\": \"crates/ruff_python_semantic/src/binding.rs\",\n            \"name\": \"qualified_name\",\n            \"ordinal\": 0,\n            \"start_byte\": 25042,\n            \"start_line\": 761\n          }\n        },\n        {\n          \"Function\": {\n            \"end_byte\": 25752,\n            \"end_line\": 784,\n            \"file\": \"crates/ruff_python_semantic/src/binding.rs\",\n            \"name\": \"qualified_name\",\n            \"ordinal\": 0,\n            \"start_byte\": 25669,\n            \"start_line\": 782\n          }\n        },\n        {\n          \"Function\": {\n            \"end_byte\": 26869,\n            \"end_line\": 816,\n            \"file\": \"crates/ruff_python_semantic/src/binding.rs\",\n            \"name\": \"qualified_name\",\n            \"ordinal\": 0,\n            \"start_byte\": 26586,\n            \"start_line\": 810\n          }\n        },\n        {\n          \"Function\": {\n            \"end_byte\": 2106,\n            \"end_line\": 69,\n            \"file\": \"crates/ruff_python_semantic/src/definition.rs\",\n            \"name\": \"qualified_name\",\n            \"ordinal\": 0,\n            \"start_byte\": 1917,\n            \"start_line\": 63\n          }\n        },\n        {\n          \"Function\": {\n            \"end_byte\": 2324,\n            \"end_line\": 69,\n            \"file\": \"crates/ruff_python_semantic/src/imports.rs\",\n            \"name\": \"qualified_name\",\n            \"ordinal\": 0,\n            \"start_byte\": 1922,\n            \"start_line\": 60\n          }\n        },\n        {\n          \"Function\": {\n            \"end_byte\": 28690,\n            \"end_l"}

## Disagreement band (needs verdict — source-verify; a tool provenance tag is NOT truth)

### crates/ruff_linter/src/rules/airflow/rules/variable_get_outside_task.rs:116 — is_dag_file
provenance: lsp
d_member: D2
```
            .as_any_import()
            .is_some_and(|import| {
                matches!(
                    import.qualified_name().segments(),
                    ["airflow", .., "DAG" | "dag"]
                )
            })
```
verdict: 
reason: 

### crates/ruff_linter/src/rules/flake8_import_conventions/rules/unconventional_import_alias.rs:68 — unconventional_import_alias
provenance: lsp
d_member: D2
```
    let Some(import) = binding.as_any_import() else {
        return;
    };
    let qualified_name = import.qualified_name().to_string();
    let Some(expected_alias) = conventions.get(qualified_name.as_str()) else {
        return;
    };
```
verdict: 
reason: 

### crates/ruff_linter/src/rules/flake8_pyi/rules/unaliased_collections_abc_set_import.rs:65 — unaliased_collections_abc_set_import
provenance: lsp
d_member: D2
```
        return;
    };
    if !matches!(
        import.qualified_name().segments(),
        ["collections", "abc", "Set"]
    ) {
        return;
```
verdict: 
reason: 

### crates/ruff_linter/src/rules/flake8_type_checking/rules/runtime_import_in_type_checking_block.rs:271 — runtime_import_in_type_checking_block
provenance: lsp
d_member: D2
```
                {
                    let mut diagnostic = checker.report_diagnostic(
                        RuntimeImportInTypeCheckingBlock {
                            qualified_name: import.qualified_name().to_string(),
                            strategy: Strategy::MoveImport,
                        },
                        range,
```
verdict: 
reason: 

### crates/ruff_linter/src/rules/flake8_type_checking/rules/typing_only_runtime_import.rs:445 — typing_only_runtime_import
provenance: lsp
d_member: D2
```
            let mut diagnostic = diagnostic_for(
                checker,
                import_type,
                import.qualified_name().to_string(),
                range,
            );
            if let Some(range) = parent_range {
```
verdict: 
reason: 

### crates/ruff_linter/src/rules/pandas_vet/helpers.rs:53 — test_expression
provenance: lsp
d_member: D2
```
                        | BindingKind::Global(_)
                        | BindingKind::Nonlocal(_, _) => Resolution::RelevantLocal,
                        BindingKind::Import(import)
                            if matches!(import.qualified_name().segments(), ["pandas"]) =>
                        {
                            Resolution::PandasModule
                        }
```
verdict: 
reason: 

### crates/ruff_linter/src/rules/pyflakes/rules/redefined_while_unused.rs:340 — bindings_in_different_forks
provenance: lsp
d_member: D2
```
        if left_binding.scope == right_binding.scope
            && let (Some(left_import), Some(right_import)) =
                (left_binding.as_any_import(), right_binding.as_any_import())
            && left_import.qualified_name() == right_import.qualified_name()
        {
            let (runtime_import, type_checking_import) =
                if left_ { (right, left) } else { (left, right) };
```
verdict: 
reason: 

### crates/ruff_linter/src/rules/pyflakes/rules/redefined_while_unused.rs:230 — redefined_while_unused
provenance: lsp
d_member: D2
```
            .filter_map(|info| {
                if let Some(shadowed_import) = info.shadowed.as_any_import() {
                    if let Some(import) = info.binding.as_any_import() {
                        if shadowed_import.qualified_name() == import.qualified_name() {
                            return Some(import.member_name());
                        }
                    }
```
verdict: 
reason: 

### crates/ruff_linter/src/rules/pyflakes/rules/unused_import.rs:274 — is_first_party
provenance: lsp
d_member: D2
```
    let source_name = import.source_name().join(".");
    let category = isort::categorize(
        &source_name,
        import.qualified_name().is_unresolved_import(),
        &checker.settings().src,
        checker.package(),
        checker.settings().isort.detect_same_package,
```
verdict: 
reason: 

### crates/ruff_linter/src/rules/pyflakes/rules/unused_import.rs:866 — mark_uses_of_qualified_name
provenance: lsp
d_member: D2
```

        if binding
            .as_any_import()
            .is_some_and(|imp| imp.qualified_name() == best_name)
        {
            *is_used = true;
        }
```
verdict: 
reason: 

### crates/ruff_linter/src/rules/pyflakes/rules/unused_import.rs:887 — rank_matches
provenance: lsp
d_member: D2
```
    let Some(import) = binding.as_any_import() else {
        unreachable!()
    };
    let qname = import.qualified_name();
    let left = qname
        .segments()
        .iter()
```
verdict: 
reason: 

### crates/ruff_linter/src/rules/pyflakes/rules/unused_import.rs:493 — unused_import
provenance: lsp
d_member: D2
```
    for binding in ignored.into_values().flatten() {
        let mut diagnostic = checker.report_diagnostic(
            UnusedImport {
                name: binding.import.qualified_name().to_string(),
                module: binding.import.member_name().to_string(),
                binding: binding.name.to_string(),
                context: UnusedImportContext::Other,
```
verdict: 
reason: 

### crates/ruff_linter/src/rules/pyflakes/rules/unused_import.rs:749 — unused_imports_from_binding
provenance: lsp
d_member: D2
```
            let first = *binding
                                .as_any_import()
                                .expect("binding to be import binding since current function called after restricting to these in `unused_imports_in_scope`")
                                .qualified_name()
                                .segments().first().expect("import binding to have nonempty qualified name");
            mark_uses_of_qualified_name(&mut marked, &QualifiedName::user_defined(first));
            marked_dunder_all = true;
```
verdict: 
reason: 

### crates/ruff_linter/src/rules/pylint/rules/import_private_name.rs:196 — from
provenance: lsp
d_member: D2
```
    fn from(import: &'a Import) -> Self {
        let module_name = import.module_name();
        let member_name = import.member_name();
        let qualified_name = import.qualified_name();
        Self {
            module_name,
            member_name,
```
verdict: 
reason: 

### crates/ruff_python_semantic/src/binding.rs:772 — member_name
provenance: lsp
d_member: D2
```

    /// For example, given `import foo.bar`, returns `"foo.bar"`.
    fn member_name(&self) -> Cow<'a, str> {
        Cow::Owned(self.qualified_name().to_string())
    }

    fn source_name(&self) -> &[&'a str] {
```
verdict: 
reason: 

### crates/ruff_python_semantic/src/binding.rs:814 — qualified_name
provenance: lsp
d_member: D2
```
        match self {
            Self::Import(import) => import.qualified_name(),
            Self::SubmoduleImport(import) => import.qualified_name(),
            Self::FromImport(import) => import.qualified_name(),
        }
    }

```
verdict: 
reason: 

### crates/ruff_python_semantic/src/imports.rs:75 — matches
provenance: lsp
d_member: D2
```
impl NameImport {
    /// Returns `true` if the [`NameImport`] matches the specified name and binding.
    pub fn matches(&self, name: &str, binding: &AnyImport) -> bool {
        name == self.bound_name() && self.qualified_name() == *binding.qualified_name()
    }
}

```
verdict: 
reason: 

### crates/ruff_python_semantic/src/model.rs:969 — resolve_submodule
provenance: lsp
d_member: D2
```
        }

        // Grab, e.g., `pyarrow` from `import pyarrow as pa`.
        let call_path = import.qualified_name();
        let segment = call_path.segments().last()?;
        if *segment == symbol {
            return None;
```
verdict: 
reason: 

## Auto-accepted (both — LSP and prism agree; still source-verify before freezing)

(none)
