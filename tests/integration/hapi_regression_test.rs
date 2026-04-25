#[path = "../common/mod.rs"]
mod common;
use common::*;

use prism::slice::SliceResult;
use std::fs;
use std::path::PathBuf;

fn fixture_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures");
    p
}

#[test]
fn test_hapi_4552_review_suite_smoke() {
    // 1. Load the diff.
    let diff_path = fixture_root().join("hapi-4552.diff");
    let diff_text = fs::read_to_string(&diff_path).expect("read hapi-4552.diff");
    let diff_input = DiffInput::parse_unified_diff(&diff_text);

    // 2. Load the trimmed source for each diff-touched file.
    let source_root = fixture_root().join("hapi-4552-source");
    let mut files = BTreeMap::new();
    for diff_info in &diff_input.files {
        let abs = source_root.join(&diff_info.file_path);
        let source = match fs::read_to_string(&abs) {
            Ok(s) => s,
            Err(_) => continue, // skip files we don't have a fixture for (e.g., test/transmit.js)
        };
        let parsed =
            ParsedFile::parse(&diff_info.file_path, &source, Language::JavaScript).unwrap();
        files.insert(diff_info.file_path.clone(), parsed);
    }
    assert!(
        !files.is_empty(),
        "should load at least one fixture source file"
    );

    // 3. Run the review suite — loop over algorithms, collect successful results.
    let mut results: Vec<SliceResult> = Vec::new();
    let mut error_count = 0usize;
    let total_algos = SlicingAlgorithm::review_suite().len();
    for algo in SlicingAlgorithm::review_suite() {
        let config = SliceConfig::default().with_algorithm(algo);
        match algorithms::run_slicing_compat(&files, &diff_input, &config, None) {
            Ok(r) => results.push(r),
            Err(e) => {
                eprintln!("NOTE: {:?} returned Err: {}", algo, e);
                error_count += 1;
            }
        }
    }
    assert!(
        error_count < total_algos,
        "all {} review-suite algorithms returned Err — possible regression",
        total_algos
    );

    // 4. Structural assertions.
    assert!(
        !results.is_empty(),
        "review preset should produce at least one algorithm's result"
    );

    // (a) LeftFlow (the default algorithm) fires.
    let has_left_flow = results
        .iter()
        .any(|r| r.algorithm == SlicingAlgorithm::LeftFlow);
    assert!(
        has_left_flow,
        "review preset should include LeftFlow output"
    );

    // (b) The diff lines (after-apply line numbers in lib/transmit.js: 269 stream.on('close', aborted),
    //     368 from.on('close', ...), 378 internals.destroyPipe def) appear in at least one block.
    let target_lines: BTreeSet<usize> = BTreeSet::from([269, 368, 378]);
    let mut found_lines: BTreeSet<usize> = BTreeSet::new();
    for result in &results {
        for block in &result.blocks {
            if let Some(per_file) = block.file_line_map.get("lib/transmit.js") {
                for &ln in per_file.keys() {
                    if target_lines.contains(&ln) {
                        found_lines.insert(ln);
                    }
                }
            }
        }
    }
    assert!(
        !found_lines.is_empty(),
        "expected at least one of lines {:?} to appear in some block; got file_line_maps that didn't include any. \
         (This may indicate that line-number alignment between the trimmed source and the diff has drifted.)",
        target_lines
    );
}
