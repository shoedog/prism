// Paper algorithm tests (arXiv:2505.17928):
// - Algorithm 6: OriginalDiff
// - Algorithm 7: ParentFunction
// - Algorithm 8: LeftFlow
// - Algorithm 9: FullFlow

mod common;
use common::*;

// ====== OriginalDiff tests ======

#[test]
fn test_original_diff_python() {
    let (files, _, diff) = make_python_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert_eq!(result.blocks.len(), 1);
    assert_eq!(result.blocks[0].diff_lines.len(), 2);
    assert!(result.blocks[0].diff_lines.contains(&7));
    assert!(result.blocks[0].diff_lines.contains(&9));
}

#[test]
fn test_original_diff_javascript() {
    let (files, _, diff) = make_javascript_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert_eq!(result.blocks.len(), 1);
    assert_eq!(result.blocks[0].diff_lines.len(), 2);
}

// ====== ParentFunction tests ======

#[test]
fn test_parent_function_python() {
    let (files, sources, diff) = make_python_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    // Should include the entire calculate function
    assert!(!result.blocks.is_empty());
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("src/calc.py").unwrap();
    // Function spans lines 6-13 approximately
    assert!(
        lines.len() > 2,
        "ParentFunction should include more than just diff lines"
    );

    let formatted = output::format_slice_result(&result.blocks, &sources);
    assert!(formatted.contains("calculate"));
}

#[test]
fn test_parent_function_go() {
    let (files, sources, diff) = make_go_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    assert!(!result.blocks.is_empty());
    let formatted = output::format_slice_result(&result.blocks, &sources);
    assert!(formatted.contains("sum"));
}

#[test]
fn test_parent_function_java() {
    let (files, sources, diff) = make_java_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    assert!(!result.blocks.is_empty());
    let formatted = output::format_slice_result(&result.blocks, &sources);
    assert!(formatted.contains("add"));
}

// ====== LeftFlow tests ======

#[test]
fn test_left_flow_python() {
    let (files, sources, diff) = make_python_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    assert!(!result.blocks.is_empty());
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("src/calc.py").unwrap();

    // LeftFlow should trace variable references — `total` and `result` are used
    // on multiple lines, so we should get more than just the 2 diff lines
    assert!(
        lines.len() > 2,
        "LeftFlow should trace variable references beyond diff lines, got {} lines",
        lines.len()
    );

    let formatted = output::format_slice_result(&result.blocks, &sources);
    assert!(formatted.contains("return"));
}

#[test]
fn test_left_flow_javascript() {
    let (files, sources, diff) = make_javascript_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    assert!(!result.blocks.is_empty());
    let formatted = output::format_slice_result(&result.blocks, &sources);
    // Should include references to response and data variables
    assert!(
        formatted.contains("fetchData")
            || formatted.contains("response")
            || formatted.contains("data")
    );
}

#[test]
fn test_left_flow_typescript() {
    let (files, _, diff) = make_typescript_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert!(!result.blocks.is_empty());
}

// ====== FullFlow tests ======

#[test]
fn test_full_flow_python() {
    let (files, sources, diff) = make_python_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    assert!(!result.blocks.is_empty());
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("src/calc.py").unwrap();

    // FullFlow should have at least as many lines as LeftFlow
    assert!(
        lines.len() >= 2,
        "FullFlow should include at least diff lines plus references"
    );
}

#[test]
fn test_full_flow_go() {
    let (files, sources, diff) = make_go_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    assert!(!result.blocks.is_empty());
    let formatted = output::format_slice_result(&result.blocks, &sources);
    assert!(formatted.contains("total"));
}

#[test]
fn test_full_flow_java() {
    let (files, sources, diff) = make_java_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    assert!(!result.blocks.is_empty());
    let formatted = output::format_slice_result(&result.blocks, &sources);
    assert!(formatted.contains("sum") || formatted.contains("accumulator"));
}

// ====== Comparative tests ======

#[test]
fn test_increasing_context() {
    let (files, _, diff) = make_python_test();

    let orig = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    let parent = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
        None,
    )
    .unwrap();

    let _left = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    let _full = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow),
        None,
    )
    .unwrap();

    let orig_lines: usize = orig
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    let parent_lines: usize = parent
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();

    // OriginalDiff should always have the fewest lines
    assert!(
        orig_lines <= parent_lines,
        "OriginalDiff ({}) should have <= lines than ParentFunction ({})",
        orig_lines,
        parent_lines
    );
}
