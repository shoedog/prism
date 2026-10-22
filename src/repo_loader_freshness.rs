use super::{rel, Language, BUILTIN_SKIP_DIRS, MAX_FILE_BYTES};
use std::collections::BTreeSet;
use std::path::Path;

// Membership evidence with the same partial-walk tolerance as `walk`.
// Root refusal remains distinct from a tolerated unreadable subdirectory.
pub(crate) fn collect_freshness_source_paths(root: &Path) -> Option<BTreeSet<String>> {
    fn collect(root: &Path, dir: &Path, out: &mut BTreeSet<String>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)?.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if (BUILTIN_SKIP_DIRS.contains(&name.as_str()) && (kind.is_dir() || kind.is_symlink()))
                || kind.is_symlink()
            {
                continue;
            }
            if kind.is_dir() {
                if !name.starts_with('.') {
                    let _ = collect(root, &path, out);
                }
            } else if kind.is_file() {
                let relp = rel(root, &path);
                if Language::from_path(&relp).is_some()
                    && entry.metadata().is_ok_and(|m| m.len() <= MAX_FILE_BYTES)
                {
                    out.insert(relp);
                }
            }
        }
        Ok(())
    }
    let mut out = BTreeSet::new();
    collect(root, root, &mut out).ok()?;
    Some(out)
}
