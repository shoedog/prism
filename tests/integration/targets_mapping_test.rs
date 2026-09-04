//! Pure projection/mapping acceptance tests for design §7.4.8.

use prism::algorithms::absence_slice::default_pairs;
use prism::api::{load_review_inputs, FindingTier, ReviewInputs, ReviewOptions};
use prism::languages::Language;
use prism::slice::{AlgorithmError, SliceFinding};
use prism::targets::mapping::{language_tag, map_finding, ABSENCE_PAIRS};
use prism::targets::{project, target_id, TargetsMeta};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn finding(algorithm: &str, category: Option<&str>, description: &str) -> SliceFinding {
    SliceFinding {
        algorithm: algorithm.to_string(),
        file: "a.py".to_string(),
        line: 2,
        severity: "warning".to_string(),
        description: description.to_string(),
        function_name: Some("read".to_string()),
        related_lines: Vec::new(),
        related_files: Vec::new(),
        category: category.map(str::to_string),
        parse_quality: None,
        diagrams: Vec::new(),
    }
}

fn inputs(path: &str) -> (TempDir, ReviewInputs) {
    let temp = TempDir::new().unwrap();
    let full = temp.path().join(path);
    fs::create_dir_all(full.parent().unwrap()).unwrap();
    fs::write(&full, "def read():\n    return 1\n").unwrap();
    let diff = serde_json::json!({
        "files": [{"file_path": path, "modify_type": "Modified", "diff_lines": [2]}]
    });
    let loaded = load_review_inputs(
        &ReviewOptions::new(temp.path()),
        &serde_json::to_string(&diff).unwrap(),
    )
    .unwrap();
    (temp, loaded)
}

fn meta(root: PathBuf) -> TargetsMeta {
    TargetsMeta {
        algorithms_run: vec!["Test".to_string()],
        repo_root: root,
        repo_sha: None,
        errors: Vec::new(),
        run_warnings: Vec::new(),
        min_severity_rank: prism::output::severity_rank("info"),
        min_tier: FindingTier::Candidate,
    }
}

#[test]
fn maps_echo_membrane_and_symmetry_verbatim_formats() {
    let echo = map_finding(&finding(
        "echo",
        Some("missing_error_handling"),
        "'serve' calls 'fetch' without handling: return value not checked",
    ));
    assert_eq!(
        (echo.kind, echo.property),
        ("external_call", "error_handled")
    );
    assert_eq!(echo.hint.unwrap().callee.as_deref(), Some("fetch"));

    let membrane = map_finding(&finding(
        "membrane",
        Some("unprotected_caller"),
        "unprotected call to 'fetch' from 'serve'",
    ));
    assert_eq!(
        (membrane.kind, membrane.property),
        ("boundary", "error_handled")
    );
    assert_eq!(membrane.hint.unwrap().callee.as_deref(), Some("fetch"));

    let symmetry = map_finding(&finding(
        "symmetry",
        Some("broken_symmetry"),
        "'serialize_x' changed but symmetric counterpart 'deserialize_x' was not",
    ));
    assert_eq!(
        (symmetry.kind, symmetry.property),
        ("contract", "counterpart_present")
    );
    assert_eq!(
        symmetry.hint.unwrap().counterpart.as_deref(),
        Some("deserialize_x")
    );

    let malformed = map_finding(&finding(
        "echo",
        Some("missing_error_handling"),
        "'serve' mentioned 'fetch' in unrelated future prose",
    ));
    assert!(
        malformed.hint.is_none(),
        "non-verbatim prose must not mint a hint"
    );
}

#[test]
fn maps_all_four_absence_categories_and_closed_pair_rows() {
    let descriptions: Vec<_> = ABSENCE_PAIRS.iter().map(|row| row.0).collect();
    let expected: Vec<_> = default_pairs()
        .into_iter()
        .map(|pair| pair.description)
        .collect();
    assert_eq!(
        ABSENCE_PAIRS.len(),
        65,
        "ABSENCE_PAIRS must cover all production PairedPattern descriptions"
    );
    assert_eq!(
        descriptions, expected,
        "ABSENCE_PAIRS must preserve every production PairedPattern description in order"
    );

    for category in [
        "missing_counterpart",
        "missing_close_on_error_path",
        "close_only_on_error_path",
    ] {
        let mapped = map_finding(&finding(
            "absence",
            Some(category),
            if category == "missing_counterpart" {
                "file open without close in function 'read' (line 2)"
            } else if category == "missing_close_on_error_path" {
                "lock without unlock in 'read': resource opened at line 2 not freed on error path 'goto out' at line 8"
            } else {
                "transaction begin without commit/rollback in 'read': close only reachable via error path (goto), not on normal return at line 8"
            },
        ));
        assert_eq!(
            (mapped.kind, mapped.property),
            ("resource_acquire", "resource_released")
        );
        if category == "missing_counterpart" {
            let hint = mapped.hint.unwrap();
            assert_eq!(hint.counterpart.as_deref(), Some("close"));
            assert_eq!(hint.kind.as_deref(), Some("filesystem"));
        } else if category == "missing_close_on_error_path" {
            // spec §2.4.2: "lock without unlock"'s close_patterns include
            // "release(", which does not contain "unlock", so no counterpart
            // (or kind) is minted and the hint is entirely absent.
            assert!(
                mapped.hint.is_none(),
                "lock without unlock must not invent a counterpart that fails the every-close-pattern rule"
            );
        } else {
            assert!(
                mapped.hint.is_none(),
                "ambiguous commit/rollback must not invent a counterpart or kind"
            );
        }
    }

    let double_close = map_finding(&finding(
        "absence",
        Some("double_close"),
        "potential double-close in 'read': close() at line 4 and label 'out' at line 8",
    ));
    assert_eq!(
        (double_close.kind, double_close.property),
        ("resource_release", "resource_not_double_released")
    );
    assert_eq!(
        double_close.hint.unwrap().counterpart.as_deref(),
        Some("close")
    );
}

#[test]
fn counterpart_is_a_substring_of_every_close_pattern_or_none() {
    // spec §2.4.2: a counterpart may be emitted only when the candidate name
    // is a case-insensitive substring of EVERY close_patterns entry of that
    // PairedPattern. Otherwise it must be None.
    let pairs = default_pairs();
    let mut violations = Vec::new();
    for (description, counterpart, _kind) in ABSENCE_PAIRS {
        let Some(counterpart) = counterpart else {
            continue;
        };
        let pair = pairs
            .iter()
            .find(|pair| pair.description == *description)
            .unwrap_or_else(|| panic!("no PairedPattern for description {description:?}"));
        let needle = counterpart.to_lowercase();
        for close_pattern in &pair.close_patterns {
            if !close_pattern.to_lowercase().contains(&needle) {
                violations.push(format!(
                    "{description:?}: counterpart {counterpart:?} is not a substring of close pattern {close_pattern:?}"
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "ABSENCE_PAIRS rows violate spec §2.4.2 (counterpart must be a case-insensitive \
         substring of every close pattern): {violations:#?}"
    );
}

#[test]
fn counterpart_omission_matches_the_spec_2_4_2_examples() {
    let row = |description: &str| {
        ABSENCE_PAIRS
            .iter()
            .find(|row| row.0 == description)
            .unwrap_or_else(|| panic!("no ABSENCE_PAIRS row for {description:?}"))
    };
    // close_patterns are [".status()", ".output()", ".spawn("] — "spawn" is
    // not a substring of ".status()" or ".output()", so no counterpart.
    assert_eq!(row("Rust Command created but never executed").1, None);
    // close_patterns include "release(" — "unlock" is not a substring of it,
    // so no counterpart.
    assert_eq!(row("lock without unlock").1, None);
    // close_patterns are ["close(", "fclose(", "Close(", ".close()"] — every
    // one contains "close", so the counterpart survives.
    assert_eq!(row("file open without close").1, Some("close"));
}

#[test]
fn maps_contract_rows_and_unknown_contract_violation_shape() {
    let guard = map_finding(&finding(
        "contract",
        Some("contract_violation"),
        "Guard clause modified in 'f': null constraint on 'x' (`if not x`). Verify callers still receive valid values.",
    ));
    assert_eq!(
        (guard.kind, guard.property),
        ("contract", "precondition_holds")
    );

    let returns = map_finding(&finding(
        "contract",
        Some("contract_violation"),
        "Return behavior modified in 'f': nullable postcondition. Verify callers handle the new return pattern.",
    ));
    assert_eq!(returns.property, "postcondition_holds");

    let unknown = map_finding(&finding(
        "contract",
        Some("contract_violation"),
        "future contract violation shape",
    ));
    assert_eq!(unknown.property, "unknown");

    for category in [
        "contract_precondition_weakened",
        "contract_precondition_strengthened",
        "contract",
    ] {
        assert_eq!(
            map_finding(&finding("contract", Some(category), "verbatim row")).property,
            "precondition_holds"
        );
    }
    for category in [
        "contract_postcondition",
        "contract_postcondition_new_null",
        "contract_postcondition_weakened",
        "contract_postcondition_strengthened",
    ] {
        assert_eq!(
            map_finding(&finding("contract", Some(category), "verbatim row")).property,
            "postcondition_holds"
        );
    }
}

#[test]
fn maps_provenance_taint_peer_and_fallback_rows() {
    let database = map_finding(&finding(
        "provenance",
        Some("untrusted_origin"),
        "variable 'row' has database origin: MEDIUM — may contain user-supplied data",
    ));
    assert_eq!(
        (database.kind, database.property),
        ("other", "origin_trusted")
    );
    assert_eq!(
        database.detail.as_deref(),
        Some("database origin at use site")
    );
    assert_eq!(database.hint.unwrap().kind.as_deref(), Some("db"));

    let user = map_finding(&finding(
        "provenance",
        Some("untrusted_origin"),
        "variable 'q' has user_input origin: HIGH — requires validation/sanitization",
    ));
    assert_eq!(
        user.detail.as_deref(),
        Some("user_input origin at use site")
    );
    assert!(user.hint.is_none());

    assert_eq!(
        map_finding(&finding(
            "taint",
            Some("taint_source"),
            "taint source: origin of tainted data at line 2"
        ))
        .kind,
        "data_origin"
    );
    for category in ["taint_sink", "unquoted_expansion"] {
        let mapped = map_finding(&finding("taint", Some(category), "verbatim taint row"));
        assert_eq!(
            (mapped.kind, mapped.property),
            ("other", "not_reached_by_taint")
        );
    }
    assert_eq!(
        map_finding(&finding(
            "peer_consistency",
            Some("peer_guard_divergence"),
            "peer prose"
        ))
        .property,
        "peer_consistent"
    );
    let fallback = map_finding(&finding("future", Some("brand_new"), "future prose"));
    assert_eq!((fallback.kind, fallback.property), ("other", "unknown"));
    for algorithm in ["callback_dispatcher", "primitive"] {
        let mapped = map_finding(&finding(algorithm, Some("any_category"), "verbatim row"));
        assert_eq!((mapped.kind, mapped.property), ("other", "unknown"));
        assert!(mapped.hint.is_none());
    }
}

#[test]
fn projection_is_total_normalizes_and_deduplicates_with_warnings() {
    let (temp, inputs) = inputs("a/b.py");
    let mut good = finding(
        "absence",
        None,
        "file open without close in function 'read' (line 2)",
    );
    good.file = "a\\b.py".to_string();
    good.severity = "critical".to_string();
    good.related_lines = vec![5, 0, 3, 5];
    good.related_files = vec!["z.py".to_string(), "z.py".to_string(), "a.py".to_string()];

    let mut line_zero = good.clone();
    line_zero.line = 0;
    let mut escaping = good.clone();
    escaping.file = "../x.py".to_string();
    let mut windows_absolute = good.clone();
    windows_absolute.file = "C:\\outside.py".to_string();
    let mut unc_absolute = good.clone();
    unc_absolute.file = "\\\\server\\share\\x.py".to_string();

    let doc = project(
        &[
            good.clone(),
            good,
            line_zero,
            escaping,
            windows_absolute,
            unc_absolute,
        ],
        &inputs,
        &meta(temp.path().to_path_buf()),
    );
    assert_eq!(doc.targets.len(), 1);
    let target = &doc.targets[0];
    assert_eq!(target.site.file, "a/b.py");
    assert_eq!(target.category, "uncategorized");
    assert_eq!(target.severity, "concern");
    let related = target.related.as_ref().unwrap();
    assert_eq!(related.lines, vec![3, 5]);
    assert_eq!(related.files, vec!["a.py", "z.py"]);
    assert!(doc
        .warnings
        .iter()
        .any(|warning| warning.contains("normalised path separators")));
    assert!(doc
        .warnings
        .iter()
        .any(|warning| warning.contains("unknown severity 'critical' mapped to concern")));
    assert!(doc
        .warnings
        .iter()
        .any(|warning| warning.contains("duplicate id")));
    assert!(doc
        .warnings
        .iter()
        .any(|warning| warning.contains("line 0")));
    assert!(doc
        .warnings
        .iter()
        .any(|warning| warning.contains("path escaping repo root")));
    assert_eq!(
        doc.warnings
            .iter()
            .filter(|warning| warning.contains("path escaping repo root"))
            .count(),
        3,
        "Unix, Windows-drive, and UNC absolute paths must all be rejected"
    );
}

#[test]
fn projection_emits_bounds_and_prefers_enclosing_symbol_on_disagreement() {
    let (temp, inputs) = inputs("a.py");
    let mut symmetry = finding(
        "symmetry",
        Some("broken_symmetry"),
        "'serialize_x' changed but symmetric counterpart 'deserialize_x' was not",
    );
    symmetry.function_name = Some("serialize_x".to_string());
    let doc = project(&[symmetry], &inputs, &meta(temp.path().to_path_buf()));
    let target = &doc.targets[0];
    assert_eq!(target.site.symbol.as_deref(), Some("read"));
    assert_eq!(target.site.function_start_line, Some(1));
    assert_eq!(target.site.function_end_line, Some(2));
    assert!(doc.warnings.iter().any(|warning| {
        warning.contains("symbol read differs from finding's function serialize_x at a.py:2")
    }));
}

#[test]
fn projection_warns_when_bounds_are_omitted_for_an_unparsed_file() {
    let (temp, inputs) = inputs("a.py");
    let mut missing = finding("absence", Some("missing_counterpart"), "future pair");
    missing.file = "missing.py".to_string();
    let doc = project(&[missing], &inputs, &meta(temp.path().to_path_buf()));
    let target = &doc.targets[0];
    assert_eq!(target.site.function_start_line, None);
    assert_eq!(target.site.function_end_line, None);
    assert!(doc.warnings.iter().any(|warning| {
        warning == "targets: function bounds omitted for missing.py:2: file not parsed"
    }));
}

#[test]
fn projection_warnings_describe_the_unfiltered_finding_population() {
    let (temp, inputs) = inputs("a/b.py");
    let mut noisy = finding("future", None, "future finding");
    noisy.file = "a\\b.py".to_string();
    noisy.severity = "critical".to_string();
    noisy.function_name = Some("named_elsewhere".to_string());
    let mut metadata = meta(temp.path().to_path_buf());
    metadata.min_tier = FindingTier::Asserted;

    let doc = project(&[noisy.clone(), noisy], &inputs, &metadata);
    assert!(
        doc.targets.is_empty(),
        "candidate findings must be filtered"
    );
    for expected in [
        "normalised path separators",
        "unknown severity 'critical' mapped to concern",
        "symbol read differs from finding's function named_elsewhere",
        "duplicate id",
    ] {
        assert!(
            doc.warnings
                .iter()
                .any(|warning| warning.contains(expected)),
            "filtered finding warning missing: {expected}; warnings={:?}",
            doc.warnings
        );
    }
}

#[test]
fn language_lowering_is_a_subset_of_the_schema_enum() {
    let schema: Value =
        serde_json::from_str(&fs::read_to_string("docs/contracts/targets.schema.json").unwrap())
            .unwrap();
    let allowed: BTreeSet<&str> = schema["$defs"]["site"]["properties"]["language"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    let lowered: BTreeSet<&str> = Language::all().into_iter().map(language_tag).collect();
    assert!(
        lowered.is_subset(&allowed),
        "{lowered:?} not within {allowed:?}"
    );
    assert_eq!(
        lowered.len(),
        Language::all().len(),
        "lowering must be one-to-one"
    );
}

#[test]
fn target_id_uses_the_canonical_nine_element_json_array() {
    use sha2::{Digest, Sha256};
    let lines = vec![3, 5];
    let files = vec!["a.py".to_string(), "z.py".to_string()];
    let expected = serde_json::to_vec(&serde_json::json!([
        "a.py",
        2,
        "read",
        "absence",
        "missing_counterpart",
        "file open without close in function 'read' (line 2)",
        "warning",
        lines,
        files,
    ]))
    .unwrap();
    assert_eq!(
        target_id(
            "a.py",
            2,
            Some("read"),
            "absence",
            "missing_counterpart",
            "file open without close in function 'read' (line 2)",
            "warning",
            &[3, 5],
            &["a.py".to_string(), "z.py".to_string()],
        ),
        format!("{:x}", Sha256::digest(expected))
    );
}

#[test]
fn projection_preserves_algorithm_errors_as_partial_coverage() {
    let (temp, inputs) = inputs("a.py");
    let mut metadata = meta(temp.path().to_path_buf());
    metadata.errors.push(AlgorithmError {
        algorithm: "DeltaSlice".to_string(),
        error: "fixture error".to_string(),
    });
    metadata.run_warnings.push("fixture warning".to_string());
    let doc = project(&[], &inputs, &metadata);
    assert_eq!(doc.errors.len(), 1);
    assert_eq!(doc.errors[0].algorithm, "DeltaSlice");
    assert_eq!(doc.errors[0].error, "fixture error");
    let serialized = serde_json::to_string(&doc).unwrap();
    let keys = [
        "schema_version",
        "producer",
        "repo",
        "diff",
        "targets",
        "errors",
        "warnings",
    ];
    let positions: Vec<_> = keys
        .iter()
        .map(|key| serialized.find(&format!("\"{key}\"")).unwrap())
        .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "top-level keys must follow schema property order: {serialized}"
    );
}
