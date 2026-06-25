use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeRole {
    Production,
    Test,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CodeContext {
    pub role: CodeRole,
    pub is_test: bool,
    pub reasons: Vec<&'static str>,
}

pub fn classify_file(path: &str) -> CodeContext {
    let components: Vec<&str> = path
        .split(['/', '\\'])
        .filter(|component| !component.is_empty())
        .collect();
    let normalized_components: Vec<String> = components
        .iter()
        .map(|component| component.to_ascii_lowercase())
        .collect();
    let basename = components.last().copied().unwrap_or(path);
    let basename_lower = basename.to_ascii_lowercase();

    let mut decisive_reasons = Vec::new();
    push_authoritative_path_reasons(&normalized_components, &mut decisive_reasons);
    push_filename_reasons(basename, &basename_lower, &mut decisive_reasons);

    let is_test = !decisive_reasons.is_empty();
    let mut reasons = Vec::new();
    if is_test {
        push_authoritative_path_reasons(&normalized_components, &mut reasons);
        push_ancillary_path_reasons(&normalized_components, &mut reasons);
        push_filename_reasons(basename, &basename_lower, &mut reasons);
    }

    let role = if is_test {
        CodeRole::Test
    } else {
        CodeRole::Production
    };
    CodeContext {
        role,
        is_test,
        reasons,
    }
}

fn push_authoritative_path_reasons(components: &[String], reasons: &mut Vec<&'static str>) {
    let rules = [
        ("test", "path_component:test"),
        ("tests", "path_component:tests"),
        ("__tests__", "path_component:__tests__"),
        ("testdata", "path_component:testdata"),
    ];
    push_component_reasons(components, &rules, reasons);
}

fn push_ancillary_path_reasons(components: &[String], reasons: &mut Vec<&'static str>) {
    let rules = [
        ("spec", "path_component:spec"),
        ("specs", "path_component:specs"),
        ("fixture", "path_component:fixture"),
        ("fixtures", "path_component:fixtures"),
    ];
    push_component_reasons(components, &rules, reasons);
}

fn push_component_reasons(
    components: &[String],
    rules: &[(&'static str, &'static str)],
    reasons: &mut Vec<&'static str>,
) {
    for (component, reason) in rules {
        if components.iter().any(|candidate| candidate == component) {
            push_reason(reasons, reason);
        }
    }
}

fn push_filename_reasons(basename: &str, basename_lower: &str, reasons: &mut Vec<&'static str>) {
    if basename_lower.starts_with("test_") && basename_lower.ends_with(".py") {
        push_reason(reasons, "filename_prefix:test_");
    }
    if basename_lower.ends_with("_test.py") {
        push_reason(reasons, "filename_suffix:_test.py");
    }
    if basename_lower.ends_with("_test.go") {
        push_reason(reasons, "filename_suffix:_test.go");
    }
    if basename_lower.ends_with("_test.rs") {
        push_reason(reasons, "filename_suffix:_test.rs");
    }
    if basename_lower.ends_with("_spec.rb") {
        push_reason(reasons, "filename_suffix:_spec.rb");
    }
    if basename.ends_with("Test.java") {
        push_reason(reasons, "filename_suffix:Test.java");
    }
    if basename.ends_with("Tests.java") {
        push_reason(reasons, "filename_suffix:Tests.java");
    }
    if basename.ends_with("IT.java") {
        push_reason(reasons, "filename_suffix:IT.java");
    }
    if basename_lower.contains(".test.") {
        push_reason(reasons, "filename_infix:.test.");
    }
    if basename_lower.contains(".spec.") {
        push_reason(reasons, "filename_infix:.spec.");
    }
}

fn push_reason(reasons: &mut Vec<&'static str>, reason: &'static str) {
    if !reasons.contains(&reason) {
        reasons.push(reason);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reasons(path: &str) -> Vec<&'static str> {
        classify_file(path).reasons
    }

    #[test]
    fn classifies_common_test_paths_and_filenames() {
        let cases = [
            ("tests/foo.py", vec!["path_component:tests"]),
            ("src/foo_test.go", vec!["filename_suffix:_test.go"]),
            ("test_bar.py", vec!["filename_prefix:test_"]),
            ("foo_test.py", vec!["filename_suffix:_test.py"]),
            ("foo.test.ts", vec!["filename_infix:.test."]),
            ("foo.spec.tsx", vec!["filename_infix:.spec."]),
            ("src/FooTest.java", vec!["filename_suffix:Test.java"]),
            ("src/FooIT.java", vec!["filename_suffix:IT.java"]),
            ("src/foo_test.rs", vec!["filename_suffix:_test.rs"]),
            ("pkg/testdata/input.go", vec!["path_component:testdata"]),
            (
                "spec/user_spec.rb",
                vec!["path_component:spec", "filename_suffix:_spec.rb"],
            ),
        ];

        for (path, expected_reasons) in cases {
            let context = classify_file(path);
            assert_eq!(context.role, CodeRole::Test, "{path}");
            assert!(context.is_test, "{path}");
            assert_eq!(context.reasons, expected_reasons, "{path}");
        }
    }

    #[test]
    fn classifies_production_when_no_decisive_test_signal_exists() {
        for path in [
            "src/lib.rs",
            "docs/superpowers/specs/foo.rs",
            "src/fixtures/example.rs",
        ] {
            let context = classify_file(path);
            assert_eq!(context.role, CodeRole::Production, "{path}");
            assert!(!context.is_test, "{path}");
            assert!(context.reasons.is_empty(), "{path}");
        }
    }

    #[test]
    fn keeps_multiple_reasons_in_deterministic_order() {
        assert_eq!(
            reasons("tests/fixtures/foo_test.go"),
            vec![
                "path_component:tests",
                "path_component:fixtures",
                "filename_suffix:_test.go"
            ]
        );
        assert_eq!(
            reasons("pkg/spec/foo.spec.ts"),
            vec!["path_component:spec", "filename_infix:.spec."]
        );
    }

    #[test]
    fn handles_backslash_separators() {
        let context = classify_file(r"pkg\testdata\input.go");
        assert_eq!(context.role, CodeRole::Test);
        assert_eq!(context.reasons, vec!["path_component:testdata"]);
    }
}
