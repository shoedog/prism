#[path = "../common/mod.rs"]
mod common;
use common::*;

fn lang_matches(name: &str, lang_key: &str) -> bool {
    match lang_key {
        "python" | "javascript" | "typescript" | "rust" | "lua" | "terraform" | "tsx" | "bash" => {
            name.contains(lang_key)
        }
        "go" => name.contains("_go_") || name.ends_with("_go"),
        "java" => {
            !name.contains("javascript") && (name.contains("_java_") || name.ends_with("_java"))
        }
        "c" => !name.contains("_cpp") && (name.contains("_c_") || name.ends_with("_c")),
        "cpp" => name.contains("_cpp_") || name.ends_with("_cpp"),
        _ => name.contains(lang_key),
    }
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        match text[pos..].find(part) {
            Some(idx) => {
                if i == 0 && !pattern.starts_with('*') && idx != 0 {
                    return false;
                }
                pos += idx + part.len();
            }
            None => return false,
        }
    }
    if !pattern.ends_with('*') {
        pos == text.len()
    } else {
        true
    }
}

#[test]
fn test_algorithm_language_matrix() {
    // Map algorithm keywords → display name.
    // Each entry is (&[keywords], display_name). A test matches if it
    // contains ANY of the keywords. This accommodates tests that use
    // either the short form ("membrane") or the full form ("membrane_slice").
    let algorithms: &[(&[&str], &str)] = &[
        (&["original_diff"], "OriginalDiff"),
        (&["parent_function"], "ParentFunction"),
        (&["left_flow"], "LeftFlow"),
        (&["full_flow"], "FullFlow"),
        (&["thin_slice"], "ThinSlice"),
        (&["barrier_slice"], "BarrierSlice"),
        (&["taint"], "Taint"),
        (&["relevant_slice"], "RelevantSlice"),
        (&["conditioned_slice", "conditioned"], "ConditionedSlice"),
        (&["delta_slice"], "DeltaSlice"),
        (&["spiral_slice", "spiral"], "SpiralSlice"),
        (&["circular_slice", "circular"], "CircularSlice"),
        (&["quantum_slice", "quantum"], "QuantumSlice"),
        (&["horizontal_slice", "horizontal"], "HorizontalSlice"),
        (&["vertical_slice", "vertical"], "VerticalSlice"),
        (&["angle_slice", "angle"], "AngleSlice"),
        (&["threed_slice", "threed"], "ThreeDSlice"),
        (&["absence_slice", "absence"], "AbsenceSlice"),
        (&["resonance_slice", "resonance"], "ResonanceSlice"),
        (&["symmetry_slice", "symmetry"], "SymmetrySlice"),
        (&["gradient_slice", "gradient"], "GradientSlice"),
        (&["provenance_slice", "provenance"], "ProvenanceSlice"),
        (&["phantom_slice", "phantom"], "PhantomSlice"),
        (&["membrane_slice", "membrane"], "MembraneSlice"),
        (&["echo_slice", "echo"], "EchoSlice"),
        (&["contract_slice", "contract"], "ContractSlice"),
        (&["chop"], "Chop"),
    ];

    // All 12 supported languages
    let languages: &[(&str, &str)] = &[
        ("python", "Python"),
        ("javascript", "JS"),
        ("typescript", "TS"),
        ("go", "Go"),
        ("java", "Java"),
        ("c", "C"),
        ("cpp", "C++"),
        ("rust", "Rust"),
        ("lua", "Lua"),
        ("terraform", "TF"),
        ("tsx", "TSX"),
        ("bash", "Bash"),
    ];

    // Collect all test function names from this file (compile-time string)
    let all_test_files = &[
        "tests/algo/paper/paper_test.rs",
        "tests/algo/taxonomy/taint_cve_test.rs",
        "tests/algo/taxonomy/taint_lang_test.rs",
        "tests/algo/taxonomy/taint_sink_test.rs",
        "tests/algo/taxonomy/taint_sink_lang_test.rs",
        "tests/algo/taxonomy/taint_interprocedural_test.rs",
        "tests/algo/taxonomy/misc_test.rs",
        "tests/algo/taxonomy/misc_lang_test.rs",
        "tests/algo/theoretical/angle_horizontal_test.rs",
        "tests/algo/theoretical/quantum_test.rs",
        "tests/algo/theoretical/quantum_lang_test.rs",
        "tests/algo/theoretical/spiral_circular_test.rs",
        "tests/algo/theoretical/vertical_threed_test.rs",
        "tests/algo/novel/absence_test.rs",
        "tests/algo/novel/absence_lang_test.rs",
        "tests/algo/novel/absence_infra_test.rs",
        "tests/algo/novel/echo_misc_test.rs",
        "tests/algo/novel/echo_misc_lang_test.rs",
        "tests/algo/novel/membrane_test.rs",
        "tests/algo/novel/membrane_ext_test.rs",
        "tests/algo/novel/provenance_test.rs",
        "tests/algo/novel/provenance_lang_test.rs",
        "tests/algo/novel/contract_test.rs",
        "tests/algo/novel/contract_delta_test.rs",
        "tests/ast/access_path_test.rs",
        "tests/ast/binding_test.rs",
        "tests/ast/cpg_test.rs",
        "tests/ast/dfg_test.rs",
        "tests/ast/field_test.rs",
        "tests/ast/type_provider_test.rs",
        "tests/ast/cpp_type_provider_test.rs",
        "tests/ast/ts_type_provider_test.rs",
        "tests/ast/java_type_provider_test.rs",
        "tests/ast/rust_type_provider_test.rs",
        "tests/ast/comment_string_test.rs",
        "tests/cli/algo_test.rs",
        "tests/cli/validation_test.rs",
        "tests/cli/output_test.rs",
        "tests/integration/call_graph_test.rs",
        "tests/integration/core_test.rs",
        "tests/integration/coverage_test.rs",
        "tests/lang/c/algo_test.rs",
        "tests/lang/c/cve_test.rs",
        "tests/lang/c/cve_fixture_test.rs",
        "tests/lang/c/complex_test.rs",
        "tests/lang/c/firmware_test.rs",
        "tests/lang/c/algo_expand_test.rs",
        "tests/lang/cpp/cpp_test.rs",
        "tests/lang/cpp/algo_test.rs",
        "tests/lang/javascript/algo_test.rs",
        "tests/lang/javascript/destructuring_test.rs",
        "tests/lang/javascript/lang_test.rs",
        "tests/lang/go/algo_test.rs",
        "tests/lang/go/advanced_test.rs",
        "tests/lang/go/lang_test.rs",
        "tests/lang/java/algo_test.rs",
        "tests/lang/java/algo_expand_test.rs",
        "tests/lang/lua/lua_test.rs",
        "tests/lang/lua/algo_test.rs",
        "tests/lang/lua/algo_expand_test.rs",
        "tests/lang/rust/rust_test.rs",
        "tests/lang/rust/algo_test.rs",
        "tests/lang/terraform/terraform_test.rs",
        "tests/lang/terraform/algo_test.rs",
        "tests/lang/bash/bash_test.rs",
        "tests/lang/bash/algo_test.rs",
        "tests/lang/bash/algo_expand_test.rs",
        "tests/lang/typescript/typescript_test.rs",
        "tests/lang/typescript/lang_test.rs",
        "tests/lang/tsx/tsx_test.rs",
        "tests/lang/tsx/jsx_call_test.rs",
        "tests/lang/tsx/hooks_test.rs",
        "tests/lang/javascript/arrow_test.rs",
    ];
    let mut test_names_buf: Vec<String> = Vec::new();
    for tf in all_test_files {
        if let Ok(src) = std::fs::read_to_string(tf) {
            for line in src.lines() {
                let t = line.trim();
                if t.starts_with("fn test_") {
                    if let Some(n) = t.trim_start_matches("fn ").split('(').next() {
                        test_names_buf.push(n.to_string());
                    }
                }
            }
        }
    }
    let test_names: Vec<&str> = test_names_buf.iter().map(|s| s.as_str()).collect();

    // Build the matrix
    let col_w = 10usize;
    let row_w = 18usize;

    // Header
    let header: String = languages
        .iter()
        .map(|(_, name)| format!("{:>col_w$}", name))
        .collect::<Vec<_>>()
        .join("");
    println!("\nAlgorithm × Language Test Coverage Matrix");
    println!("{}", "=".repeat(row_w + col_w * languages.len()));
    println!("{:<row_w$}{}", "", header);
    println!("{}", "-".repeat(row_w + col_w * languages.len()));

    let mut covered = 0usize;
    let mut total = 0usize;

    for (algo_keys, algo_name) in algorithms {
        let row: String = languages
            .iter()
            .map(|(lang_key, _)| {
                total += 1;
                let has_test = test_names.iter().any(|name| {
                    algo_keys.iter().any(|k| name.contains(k)) && lang_matches(name, lang_key)
                });
                if has_test {
                    covered += 1;
                    format!("{:>col_w$}", "✓")
                } else {
                    format!("{:>col_w$}", "-")
                }
            })
            .collect::<Vec<_>>()
            .join("");
        println!("{:<row_w$}{}", algo_name, row);
    }

    println!("{}", "=".repeat(row_w + col_w * languages.len()));
    println!(
        "Coverage: {}/{} algorithm×language combinations ({:.0}%)",
        covered,
        total,
        covered as f64 / total as f64 * 100.0
    );
    println!();

    // Always passes — this is a reporting tool, not an enforcement test
}

#[test]
fn test_language_coverage_minimum() {
    const MIN_LANGS: usize = 2;

    let algorithms: &[(&[&str], &str)] = &[
        (&["original_diff"], "OriginalDiff"),
        (&["parent_function"], "ParentFunction"),
        (&["left_flow"], "LeftFlow"),
        (&["full_flow"], "FullFlow"),
        (&["thin_slice"], "ThinSlice"),
        (&["barrier_slice"], "BarrierSlice"),
        (&["taint"], "Taint"),
        (&["relevant_slice"], "RelevantSlice"),
        (&["conditioned_slice", "conditioned"], "ConditionedSlice"),
        (&["delta_slice"], "DeltaSlice"),
        (&["spiral_slice", "spiral"], "SpiralSlice"),
        (&["circular_slice", "circular"], "CircularSlice"),
        (&["quantum_slice", "quantum"], "QuantumSlice"),
        (&["horizontal_slice", "horizontal"], "HorizontalSlice"),
        (&["vertical_slice", "vertical"], "VerticalSlice"),
        (&["angle_slice", "angle"], "AngleSlice"),
        (&["threed_slice", "threed"], "ThreeDSlice"),
        (&["absence_slice", "absence"], "AbsenceSlice"),
        (&["resonance_slice", "resonance"], "ResonanceSlice"),
        (&["symmetry_slice", "symmetry"], "SymmetrySlice"),
        (&["gradient_slice", "gradient"], "GradientSlice"),
        (&["provenance_slice", "provenance"], "ProvenanceSlice"),
        (&["phantom_slice", "phantom"], "PhantomSlice"),
        (&["membrane_slice", "membrane"], "MembraneSlice"),
        (&["echo_slice", "echo"], "EchoSlice"),
        (&["contract_slice", "contract"], "ContractSlice"),
        (&["chop"], "Chop"),
    ];

    let lang_keys: &[&str] = &[
        "python",
        "javascript",
        "typescript",
        "go",
        "java",
        "c",
        "cpp",
        "rust",
        "lua",
        "terraform",
        "tsx",
        "bash",
    ];

    let all_test_files = &[
        "tests/algo/paper/paper_test.rs",
        "tests/algo/taxonomy/taint_cve_test.rs",
        "tests/algo/taxonomy/taint_lang_test.rs",
        "tests/algo/taxonomy/taint_sink_test.rs",
        "tests/algo/taxonomy/taint_sink_lang_test.rs",
        "tests/algo/taxonomy/taint_interprocedural_test.rs",
        "tests/algo/taxonomy/misc_test.rs",
        "tests/algo/taxonomy/misc_lang_test.rs",
        "tests/algo/theoretical/angle_horizontal_test.rs",
        "tests/algo/theoretical/quantum_test.rs",
        "tests/algo/theoretical/quantum_lang_test.rs",
        "tests/algo/theoretical/spiral_circular_test.rs",
        "tests/algo/theoretical/vertical_threed_test.rs",
        "tests/algo/novel/absence_test.rs",
        "tests/algo/novel/absence_lang_test.rs",
        "tests/algo/novel/absence_infra_test.rs",
        "tests/algo/novel/echo_misc_test.rs",
        "tests/algo/novel/echo_misc_lang_test.rs",
        "tests/algo/novel/membrane_test.rs",
        "tests/algo/novel/membrane_ext_test.rs",
        "tests/algo/novel/provenance_test.rs",
        "tests/algo/novel/provenance_lang_test.rs",
        "tests/algo/novel/contract_test.rs",
        "tests/algo/novel/contract_delta_test.rs",
        "tests/ast/access_path_test.rs",
        "tests/ast/binding_test.rs",
        "tests/ast/cpg_test.rs",
        "tests/ast/dfg_test.rs",
        "tests/ast/field_test.rs",
        "tests/ast/type_provider_test.rs",
        "tests/ast/cpp_type_provider_test.rs",
        "tests/ast/ts_type_provider_test.rs",
        "tests/ast/java_type_provider_test.rs",
        "tests/ast/rust_type_provider_test.rs",
        "tests/ast/comment_string_test.rs",
        "tests/cli/algo_test.rs",
        "tests/cli/validation_test.rs",
        "tests/cli/output_test.rs",
        "tests/integration/call_graph_test.rs",
        "tests/integration/core_test.rs",
        "tests/integration/coverage_test.rs",
        "tests/lang/c/algo_test.rs",
        "tests/lang/c/cve_test.rs",
        "tests/lang/c/cve_fixture_test.rs",
        "tests/lang/c/complex_test.rs",
        "tests/lang/c/firmware_test.rs",
        "tests/lang/c/algo_expand_test.rs",
        "tests/lang/cpp/cpp_test.rs",
        "tests/lang/cpp/algo_test.rs",
        "tests/lang/javascript/algo_test.rs",
        "tests/lang/javascript/destructuring_test.rs",
        "tests/lang/javascript/lang_test.rs",
        "tests/lang/go/algo_test.rs",
        "tests/lang/go/advanced_test.rs",
        "tests/lang/go/lang_test.rs",
        "tests/lang/java/algo_test.rs",
        "tests/lang/java/algo_expand_test.rs",
        "tests/lang/lua/lua_test.rs",
        "tests/lang/lua/algo_test.rs",
        "tests/lang/lua/algo_expand_test.rs",
        "tests/lang/rust/rust_test.rs",
        "tests/lang/rust/algo_test.rs",
        "tests/lang/terraform/terraform_test.rs",
        "tests/lang/terraform/algo_test.rs",
        "tests/lang/bash/bash_test.rs",
        "tests/lang/bash/algo_test.rs",
        "tests/lang/bash/algo_expand_test.rs",
        "tests/lang/typescript/typescript_test.rs",
        "tests/lang/typescript/lang_test.rs",
        "tests/lang/tsx/tsx_test.rs",
        "tests/lang/tsx/jsx_call_test.rs",
        "tests/lang/tsx/hooks_test.rs",
        "tests/lang/javascript/arrow_test.rs",
    ];
    let mut test_names_buf: Vec<String> = Vec::new();
    for tf in all_test_files {
        if let Ok(src) = std::fs::read_to_string(tf) {
            for line in src.lines() {
                let t = line.trim();
                if t.starts_with("fn test_") {
                    if let Some(n) = t.trim_start_matches("fn ").split('(').next() {
                        test_names_buf.push(n.to_string());
                    }
                }
            }
        }
    }
    let test_names: Vec<&str> = test_names_buf.iter().map(|s| s.as_str()).collect();

    let mut failures = Vec::new();
    for (algo_keys, algo_name) in algorithms {
        let lang_count = lang_keys
            .iter()
            .filter(|lang| {
                test_names.iter().any(|name| {
                    algo_keys.iter().any(|k| name.contains(k)) && lang_matches(name, lang)
                })
            })
            .count();
        if lang_count < MIN_LANGS {
            failures.push(format!(
                "  {} — tested in {} language(s), need ≥ {}",
                algo_name, lang_count, MIN_LANGS
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "Algorithms below minimum language coverage ({} languages):\n{}",
        MIN_LANGS,
        failures.join("\n")
    );
}

#[test]
fn test_coverage_matrix_validation() {
    use std::fs;

    let matrix_str =
        fs::read_to_string("coverage/matrix.json").expect("coverage/matrix.json should exist");
    let matrix: serde_json::Value =
        serde_json::from_str(&matrix_str).expect("matrix.json should be valid JSON");

    // Read all test function names from test files AND unit test modules
    let test_files = &[
        "tests/algo/paper/paper_test.rs",
        "tests/algo/taxonomy/taint_cve_test.rs",
        "tests/algo/taxonomy/taint_lang_test.rs",
        "tests/algo/taxonomy/taint_sink_test.rs",
        "tests/algo/taxonomy/taint_sink_lang_test.rs",
        "tests/algo/taxonomy/taint_interprocedural_test.rs",
        "tests/algo/taxonomy/misc_test.rs",
        "tests/algo/taxonomy/misc_lang_test.rs",
        "tests/algo/theoretical/angle_horizontal_test.rs",
        "tests/algo/theoretical/quantum_test.rs",
        "tests/algo/theoretical/quantum_lang_test.rs",
        "tests/algo/theoretical/spiral_circular_test.rs",
        "tests/algo/theoretical/vertical_threed_test.rs",
        "tests/algo/novel/absence_test.rs",
        "tests/algo/novel/absence_lang_test.rs",
        "tests/algo/novel/absence_infra_test.rs",
        "tests/algo/novel/echo_misc_test.rs",
        "tests/algo/novel/echo_misc_lang_test.rs",
        "tests/algo/novel/membrane_test.rs",
        "tests/algo/novel/membrane_ext_test.rs",
        "tests/algo/novel/provenance_test.rs",
        "tests/algo/novel/provenance_lang_test.rs",
        "tests/algo/novel/contract_test.rs",
        "tests/algo/novel/contract_delta_test.rs",
        "tests/ast/access_path_test.rs",
        "tests/ast/binding_test.rs",
        "tests/ast/cpg_test.rs",
        "tests/ast/dfg_test.rs",
        "tests/ast/field_test.rs",
        "tests/ast/type_provider_test.rs",
        "tests/ast/cpp_type_provider_test.rs",
        "tests/ast/ts_type_provider_test.rs",
        "tests/ast/java_type_provider_test.rs",
        "tests/ast/rust_type_provider_test.rs",
        "tests/ast/comment_string_test.rs",
        "tests/cli/algo_test.rs",
        "tests/cli/validation_test.rs",
        "tests/cli/output_test.rs",
        "tests/integration/call_graph_test.rs",
        "tests/integration/core_test.rs",
        "tests/integration/coverage_test.rs",
        "tests/lang/c/algo_test.rs",
        "tests/lang/c/cve_test.rs",
        "tests/lang/c/cve_fixture_test.rs",
        "tests/lang/c/complex_test.rs",
        "tests/lang/c/firmware_test.rs",
        "tests/lang/c/algo_expand_test.rs",
        "tests/lang/cpp/cpp_test.rs",
        "tests/lang/cpp/algo_test.rs",
        "tests/lang/javascript/algo_test.rs",
        "tests/lang/javascript/destructuring_test.rs",
        "tests/lang/javascript/lang_test.rs",
        "tests/lang/go/algo_test.rs",
        "tests/lang/go/advanced_test.rs",
        "tests/lang/go/lang_test.rs",
        "tests/lang/java/algo_test.rs",
        "tests/lang/java/algo_expand_test.rs",
        "tests/lang/lua/lua_test.rs",
        "tests/lang/lua/algo_test.rs",
        "tests/lang/lua/algo_expand_test.rs",
        "tests/lang/rust/rust_test.rs",
        "tests/lang/rust/algo_test.rs",
        "tests/lang/terraform/terraform_test.rs",
        "tests/lang/terraform/algo_test.rs",
        "tests/lang/bash/bash_test.rs",
        "tests/lang/bash/algo_test.rs",
        "tests/lang/bash/algo_expand_test.rs",
        "tests/lang/typescript/typescript_test.rs",
        "tests/lang/typescript/lang_test.rs",
        "tests/lang/tsx/tsx_test.rs",
        "tests/lang/tsx/jsx_call_test.rs",
        "tests/lang/tsx/hooks_test.rs",
        "tests/lang/javascript/arrow_test.rs",
        "src/cfg.rs",
        "src/cpg.rs",
        "src/type_db.rs",
        "src/data_flow.rs",
        "src/ast.rs",
        "src/call_graph.rs",
        "src/access_path.rs",
    ];
    let mut test_names: Vec<String> = Vec::new();
    for path in test_files {
        if let Ok(source) = fs::read_to_string(path) {
            for line in source.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("fn test_") {
                    if let Some(name) = trimmed.trim_start_matches("fn ").split('(').next() {
                        if !name.is_empty() {
                            test_names.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    assert!(
        test_names.len() > 300,
        "Should find >300 test names, found {}",
        test_names.len()
    );

    let mut handled_count = 0;
    let mut verified_count = 0;
    let mut warnings: Vec<String> = Vec::new();

    if let Some(features) = matrix["language_features"].as_object() {
        for (category, cat_features) in features {
            if let Some(cat_obj) = cat_features.as_object() {
                for (feature_name, spec) in cat_obj {
                    let status = spec["status"].as_str().unwrap_or("unknown");
                    if status != "handled" {
                        continue;
                    }
                    handled_count += 1;

                    if let Some(test_patterns) = spec["tests"].as_array() {
                        let has_match = test_patterns.iter().any(|pattern| {
                            let pat = pattern.as_str().unwrap_or("");
                            test_names.iter().any(|t| glob_match(pat, t))
                        });

                        if has_match {
                            verified_count += 1;
                        } else {
                            warnings.push(format!(
                                "{}/{}: claims handled with tests {:?} but no matching test found",
                                category,
                                feature_name,
                                test_patterns
                                    .iter()
                                    .map(|p| p.as_str().unwrap_or(""))
                                    .collect::<Vec<_>>()
                            ));
                        }
                    } else {
                        // No test patterns specified — covered by general algorithm tests
                        verified_count += 1;
                    }
                }
            }
        }
    }

    if !warnings.is_empty() {
        eprintln!("\nCoverage matrix warnings:");
        for w in &warnings {
            eprintln!("  WARN: {}", w);
        }
    }

    let mut gap_count = 0;
    if let Some(features) = matrix["language_features"].as_object() {
        for (_category, cat_features) in features {
            if let Some(cat_obj) = cat_features.as_object() {
                for (_feature_name, spec) in cat_obj {
                    if spec["status"].as_str() == Some("gap") {
                        gap_count += 1;
                    }
                }
            }
        }
    }

    eprintln!(
        "\nCoverage matrix: {}/{} handled features verified, {} gaps remaining",
        verified_count, handled_count, gap_count
    );

    // Coverage threshold: per-feature (not per-language-feature)
    let total = handled_count + gap_count;
    let coverage_pct = if total > 0 {
        100 * handled_count / total
    } else {
        0
    };
    assert!(
        coverage_pct >= 80,
        "Language feature coverage dropped below 80%: {}% ({}/{})",
        coverage_pct,
        handled_count,
        total
    );

    assert!(
        warnings.is_empty(),
        "Coverage matrix has {} unverified claims:\n  {}",
        warnings.len(),
        warnings.join("\n  ")
    );
}

#[test]
fn test_coverage_matrix_algorithm_completeness() {
    use std::fs;

    let matrix_str =
        fs::read_to_string("coverage/matrix.json").expect("coverage/matrix.json should exist");
    let matrix: serde_json::Value =
        serde_json::from_str(&matrix_str).expect("matrix.json should be valid JSON");

    let algo_cov = matrix["algorithm_coverage"]
        .as_object()
        .expect("algorithm_coverage should be an object");

    let expected_algos = vec![
        "original_diff",
        "parent_function",
        "left_flow",
        "full_flow",
        "thin_slice",
        "barrier_slice",
        "taint",
        "chop",
        "relevant_slice",
        "conditioned_slice",
        "delta_slice",
        "spiral_slice",
        "circular_slice",
        "quantum_slice",
        "horizontal_slice",
        "vertical_slice",
        "angle_slice",
        "threed_slice",
        "absence_slice",
        "resonance_slice",
        "symmetry_slice",
        "gradient_slice",
        "provenance_slice",
        "phantom_slice",
        "membrane_slice",
        "echo_slice",
        "contract_slice",
    ];

    let mut missing = Vec::new();
    for algo in &expected_algos {
        if !algo_cov.contains_key(*algo) {
            missing.push(*algo);
        }
    }
    assert!(
        missing.is_empty(),
        "Algorithms missing from coverage matrix: {:?}",
        missing
    );

    let languages = [
        "python",
        "javascript",
        "typescript",
        "go",
        "java",
        "c",
        "cpp",
        "rust",
        "lua",
        "terraform",
        "bash",
    ];
    for (algo, langs) in algo_cov {
        let covered = languages
            .iter()
            .filter(|l| langs[**l].as_str().unwrap_or("none") != "none")
            .count();
        assert!(
            covered >= 2,
            "Algorithm '{}' has coverage in only {} languages (need ≥2)",
            algo,
            covered
        );
    }
}
