use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Type of modification to a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModifyType {
    /// File was added
    Added,
    /// File was modified
    Modified,
    /// File was deleted
    Deleted,
    /// File was renamed
    Renamed,
}

impl ModifyType {
    pub fn code(&self) -> &str {
        match self {
            Self::Added => "A",
            Self::Modified => "M",
            Self::Deleted => "D",
            Self::Renamed => "R",
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code {
            "A" => Self::Added,
            "D" => Self::Deleted,
            "R" => Self::Renamed,
            _ => Self::Modified,
        }
    }
}

/// Diff information for a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffInfo {
    pub file_path: String,
    pub modify_type: ModifyType,
    /// Set of 1-indexed line numbers that were changed.
    pub diff_lines: BTreeSet<usize>,
}

/// Parsed unified diff input containing all changed files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffInput {
    pub files: Vec<DiffInfo>,
}

/// A hunk from a unified diff.
#[derive(Debug)]
struct Hunk {
    new_start: usize,
    new_count: usize,
}

impl DiffInput {
    /// Retain only files whose paths are in `filter`. No-op if `filter` is `None`.
    pub fn filter_files(&mut self, filter: Option<&std::collections::HashSet<String>>) {
        if let Some(f) = filter {
            self.files.retain(|info| f.contains(&info.file_path));
        }
    }

    /// Parse a unified diff string into a `DiffInput`.
    pub fn parse_unified_diff(diff_text: &str) -> Self {
        let mut files = Vec::new();
        let mut current_file: Option<String> = None;
        let mut current_lines: BTreeSet<usize> = BTreeSet::new();
        let mut current_hunk: Option<Hunk> = None;
        let mut line_offset: usize = 0;

        for line in diff_text.lines() {
            if line.starts_with("+++ b/") || line.starts_with("+++ ") {
                // Flush previous file
                if let Some(file) = current_file.take() {
                    if !current_lines.is_empty() {
                        files.push(DiffInfo {
                            file_path: file,
                            modify_type: ModifyType::Modified,
                            diff_lines: std::mem::take(&mut current_lines),
                        });
                    }
                }
                let path = line
                    .strip_prefix("+++ b/")
                    .or_else(|| line.strip_prefix("+++ "))
                    .unwrap_or(line)
                    .to_string();
                current_file = Some(path);
                current_hunk = None;
            } else if line.starts_with("@@ ") {
                // Parse hunk header: @@ -old_start,old_count +new_start,new_count @@
                if let Some(hunk) = parse_hunk_header(line) {
                    line_offset = 0;
                    current_hunk = Some(hunk);
                }
            } else if current_hunk.is_some() {
                let hunk = current_hunk.as_ref().unwrap();
                if line.starts_with('+') {
                    let line_num = hunk.new_start + line_offset;
                    if line_num <= hunk.new_start + hunk.new_count {
                        current_lines.insert(line_num);
                    }
                    line_offset += 1;
                } else if line.starts_with('-') {
                    // Deleted line: don't increment new-file offset
                } else {
                    // Context line
                    line_offset += 1;
                }
            }
        }

        // Flush last file
        if let Some(file) = current_file {
            if !current_lines.is_empty() {
                files.push(DiffInfo {
                    file_path: file,
                    modify_type: ModifyType::Modified,
                    diff_lines: current_lines,
                });
            }
        }

        DiffInput { files }
    }

    /// Create a `DiffInput` from a JSON structure.
    ///
    /// Expected format:
    /// ```json
    /// {
    ///   "files": [
    ///     {
    ///       "file_path": "src/main.py",
    ///       "modify_type": "Modified",
    ///       "diff_lines": [10, 11, 12, 25]
    ///     }
    ///   ]
    /// }
    /// ```
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

fn parse_hunk_header(line: &str) -> Option<Hunk> {
    // @@ -old_start,old_count +new_start,new_count @@
    let after_at = line.strip_prefix("@@ ")?;
    let plus_idx = after_at.find('+')?;
    let new_part = &after_at[plus_idx + 1..];
    let end = new_part.find(" @@").unwrap_or(new_part.len());
    let new_part = &new_part[..end];

    let (start, count) = if let Some((s, c)) = new_part.split_once(',') {
        (s.parse::<usize>().ok()?, c.parse::<usize>().ok()?)
    } else {
        (new_part.parse::<usize>().ok()?, 1)
    };

    Some(Hunk {
        new_start: start,
        new_count: count,
    })
}

/// A single block of sliced output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffBlock {
    pub block_id: usize,
    pub file: String,
    pub modify_type: ModifyType,
    pub diff_lines: BTreeSet<usize>,
    /// Maps filename -> (line_number -> is_diff_line).
    /// `true` means the line is a changed diff line, `false` means context.
    pub file_line_map: BTreeMap<String, BTreeMap<usize, bool>>,
}

impl DiffBlock {
    pub fn new(block_id: usize, file: String, modify_type: ModifyType) -> Self {
        Self {
            block_id,
            file,
            modify_type,
            diff_lines: BTreeSet::new(),
            file_line_map: BTreeMap::new(),
        }
    }

    /// Add a line to the slice. `is_diff` indicates whether it's a changed line.
    pub fn add_line(&mut self, file: &str, line: usize, is_diff: bool) {
        self.file_line_map
            .entry(file.to_string())
            .or_default()
            .insert(line, is_diff);
        if is_diff {
            self.diff_lines.insert(line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_unified_diff() {
        let diff = r#"diff --git a/src/main.py b/src/main.py
--- a/src/main.py
+++ b/src/main.py
@@ -10,6 +10,8 @@ def foo():
     x = 1
     y = 2
+    z = x + y
+    print(z)
     return x
"#;
        let input = DiffInput::parse_unified_diff(diff);
        assert_eq!(input.files.len(), 1);
        assert_eq!(input.files[0].file_path, "src/main.py");
        assert!(input.files[0].diff_lines.contains(&12));
        assert!(input.files[0].diff_lines.contains(&13));
    }

    #[test]
    fn test_parse_hunk_header() {
        let hunk = parse_hunk_header("@@ -10,6 +10,8 @@ def foo():").unwrap();
        assert_eq!(hunk.new_start, 10);
        assert_eq!(hunk.new_count, 8);
    }

    #[test]
    fn test_filter_files() {
        let mut input = DiffInput {
            files: vec![
                DiffInfo {
                    file_path: "src/auth.py".to_string(),
                    modify_type: ModifyType::Modified,
                    diff_lines: BTreeSet::from([10, 11]),
                },
                DiffInfo {
                    file_path: "src/api/client.py".to_string(),
                    modify_type: ModifyType::Modified,
                    diff_lines: BTreeSet::from([5]),
                },
                DiffInfo {
                    file_path: "src/utils.py".to_string(),
                    modify_type: ModifyType::Modified,
                    diff_lines: BTreeSet::from([20]),
                },
            ],
        };

        let filter: std::collections::HashSet<String> = ["src/auth.py", "src/api/client.py"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        input.filter_files(Some(&filter));

        assert_eq!(input.files.len(), 2);
        assert!(input.files.iter().any(|f| f.file_path == "src/auth.py"));
        assert!(input
            .files
            .iter()
            .any(|f| f.file_path == "src/api/client.py"));
        assert!(!input.files.iter().any(|f| f.file_path == "src/utils.py"));
    }

    #[test]
    fn test_filter_files_none_keeps_all() {
        let mut input = DiffInput {
            files: vec![
                DiffInfo {
                    file_path: "src/auth.py".to_string(),
                    modify_type: ModifyType::Modified,
                    diff_lines: BTreeSet::from([1]),
                },
                DiffInfo {
                    file_path: "src/utils.py".to_string(),
                    modify_type: ModifyType::Modified,
                    diff_lines: BTreeSet::from([2]),
                },
            ],
        };
        input.filter_files(None);
        assert_eq!(input.files.len(), 2);
    }
}
