//! Algorithm 6: OriginalDiff
//!
//! The simplest slicing strategy — includes only the raw diff lines.
//! This is the baseline that provides minimal context.

use crate::ast::ParsedFile;
use crate::diff::{DiffBlock, DiffInput};
use crate::slice::{SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::BTreeMap;

pub fn slice(
    _files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::OriginalDiff);

    for (block_id, diff_info) in diff.files.iter().enumerate() {
        let mut block = DiffBlock::new(
            block_id,
            diff_info.file_path.clone(),
            diff_info.modify_type.clone(),
        );

        for &line in &diff_info.diff_lines {
            block.add_line(&diff_info.file_path, line, true);
        }

        result.blocks.push(block);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{DiffInfo, ModifyType};
    use std::collections::BTreeSet;

    #[test]
    fn test_original_diff_basic() {
        let diff = DiffInput {
            files: vec![DiffInfo {
                file_path: "test.py".into(),
                modify_type: ModifyType::Modified,
                diff_lines: BTreeSet::from([10, 11, 15]),
            }],
        };

        let result = slice(&BTreeMap::new(), &diff).unwrap();
        assert_eq!(result.blocks.len(), 1);
        assert_eq!(result.blocks[0].diff_lines.len(), 3);
        assert!(result.blocks[0].diff_lines.contains(&10));
        assert!(result.blocks[0].diff_lines.contains(&11));
        assert!(result.blocks[0].diff_lines.contains(&15));
    }
}
