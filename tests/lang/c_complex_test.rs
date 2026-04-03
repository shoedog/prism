#[path = "../common/mod.rs"]
mod common;
use common::*;

fn make_macro_heavy_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = include_str!("../fixtures/c/macro_heavy.c");

    let path = "tests/fixtures/c/macro_heavy.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff: line 34 (payload_len = CLAMP(...))
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([34]),
        }],
    };

    (files, diff)
}


fn make_onu_state_machine_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = r#"
#include <stdint.h>

typedef struct { int type; uint8_t data[64]; } ploam_msg_t;
#define RANGING_GRANT   1
#define RANGING_COMPLETE 2
#define ACTIVATE        3
#define DEREGISTRATION  4

enum onu_state { INIT, RANGING, REGISTERED, OPERATIONAL };
static enum onu_state current_state = INIT;

void handle_ploam_message(ploam_msg_t *msg) {
    switch(current_state) {
        case INIT:
            if (msg->type == RANGING_GRANT) {
                current_state = RANGING;
            }
            break;
        case RANGING:
            if (msg->type == RANGING_COMPLETE) {
                current_state = REGISTERED;
            }
            break;
        case REGISTERED:
            if (msg->type == ACTIVATE) {
                current_state = OPERATIONAL;
            }
            break;
    }
}
"#;

    let path = "tests/fixtures/c/onu_state_machine.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff: lines 20-22 (RANGING case handling)
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([20, 21, 22]),
        }],
    };

    (files, diff)
}


#[test]
fn test_onu_state_machine_original_diff() {
    let (files, diff) = make_onu_state_machine_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for onu_state_machine"
    );
    let total_lines: usize = result
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    assert!(
        total_lines > 0,
        "onu_state_machine OriginalDiff should include at least one line"
    );
}

#[test]
fn test_onu_state_machine_left_flow() {
    let (files, diff) = make_onu_state_machine_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for onu_state_machine"
    );
}

#[test]
fn test_large_function_original_diff() {
    let (files, diff) = make_large_function_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for large_function"
    );
    let total_files: usize = result.blocks.iter().map(|b| b.file_line_map.len()).sum();
    assert!(
        total_files >= 1,
        "large_function result should reference at least 1 file"
    );
}

#[test]
fn test_large_function_left_flow() {
    let (files, diff) = make_large_function_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for large_function without panic"
    );
}

#[test]
fn test_macro_heavy_original_diff() {
    let (files, diff) = make_macro_heavy_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for macro_heavy"
    );
    let total_files: usize = result.blocks.iter().map(|b| b.file_line_map.len()).sum();
    assert!(
        total_files >= 1,
        "macro_heavy result should reference at least 1 file"
    );
}

#[test]
fn test_macro_heavy_left_flow() {
    let (files, diff) = make_macro_heavy_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for macro_heavy without panic"
    );
}

#[test]
fn test_deep_switch_original_diff() {
    let (files, diff) = make_deep_switch_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for deep_switch"
    );
    let total_files: usize = result.blocks.iter().map(|b| b.file_line_map.len()).sum();
    assert!(
        total_files >= 1,
        "deep_switch result should reference at least 1 file"
    );
}

#[test]
fn test_deep_switch_left_flow() {
    let (files, diff) = make_deep_switch_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for deep_switch without panic"
    );
}

#[test]
fn test_broken_c_triggers_parse_warning() {
    // Code with unbalanced braces and invalid syntax that forces tree-sitter
    // into heavy error recovery, producing many ERROR nodes.
    let source = r#"
@@@ MACRO_CHAOS @@@
#define FOO( bar baz qux
int x = ))) + [[[;
typedef struct { int a; int b; } Foo
void broken( int a, { return a +;
@@@ MORE_GARBAGE @@@
int = = = ;
"#;
    let path = "src/broken.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();

    // tree-sitter error-recovers rather than failing, so we should have nodes
    assert!(
        parsed.parse_node_count > 0,
        "tree-sitter should still produce an AST (with errors)"
    );
    assert!(
        parsed.parse_error_count > 0,
        "Broken C code should produce ERROR nodes"
    );
    assert!(
        parsed.error_rate() > 0.1,
        "Error rate should exceed 10% for heavily broken code (got {})",
        parsed.error_rate()
    );

    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let warnings = algorithms::check_parse_warnings(&files);
    assert!(
        !warnings.is_empty(),
        "Broken C code should generate at least one parse warning"
    );
    // The warning should mention the file name
    assert!(
        warnings.iter().any(|w| w.contains("src/broken.c")),
        "Warning should reference the problematic file"
    );
}
