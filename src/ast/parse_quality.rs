//! Per-file tree-sitter parse quality (moved from `algorithms`, which re-exports it, #701 S5).

use crate::ast::ParsedFile;
use crate::slice::FileParseQuality;
use std::collections::BTreeMap;

/// Check parsed files for tree-sitter parse errors and return human-readable warnings.
///
/// Thresholds:
/// * > 10% error nodes → warn that results may be unreliable (common with macro-heavy C/C++)
/// * > 30% error nodes → warn that results would be meaningless; recommend preprocessing
///
/// Also kept for backward compatibility — delegates to `check_parse_quality`.
pub fn check_parse_warnings(files: &BTreeMap<String, ParsedFile>) -> Vec<String> {
    let (warnings, _) = check_parse_quality(files);
    warnings
}

/// Check parsed files for tree-sitter parse errors and return both human-readable
/// warnings and structured per-file parse quality data.
///
/// Thresholds:
/// * > 10% error nodes → warn that results may be unreliable (common with macro-heavy C/C++)
/// * > 30% error nodes → warn that results would be meaningless; recommend preprocessing
pub fn check_parse_quality(
    files: &BTreeMap<String, ParsedFile>,
) -> (Vec<String>, BTreeMap<String, FileParseQuality>) {
    let mut warnings = Vec::new();
    let mut quality = BTreeMap::new();
    for (path, pf) in files {
        let total = pf.parse_node_count;
        if total == 0 {
            continue;
        }
        let rate = pf.error_rate();
        let pct = (rate * 100.0).round() as usize;
        if rate > 0.01 {
            let error_lines = crate::ast::collect_error_lines(&pf.tree, 20);
            let q = if rate > 0.3 {
                "unparseable"
            } else if rate > 0.1 {
                "poor"
            } else {
                "degraded"
            };
            quality.insert(
                path.clone(),
                FileParseQuality {
                    error_count: pf.parse_error_count,
                    node_count: total,
                    error_rate: rate,
                    quality: q.to_string(),
                    error_lines,
                },
            );
        }
        if rate > 0.3 {
            warnings.push(format!(
                "File {} has severe parse errors ({} of {} AST nodes, {}%). \
                 Skipping analysis — results would be meaningless. \
                 Consider preprocessing macros first.",
                path, pf.parse_error_count, total, pct
            ));
        } else if rate > 0.1 {
            warnings.push(format!(
                "File {} has {} parse errors ({}% of AST nodes). \
                 Results may be unreliable. \
                 This often happens with macro-heavy C/C++ code.",
                path, pf.parse_error_count, pct
            ));
        }
    }
    (warnings, quality)
}
