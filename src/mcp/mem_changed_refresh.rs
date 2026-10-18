use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Component, Path};

fn hash(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn git(root: &Path, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()?;
    ensure!(output.status.success(), "mirror Git identity unavailable");
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Edit {
    pub repo_root: String,
    pub commit: String,
    pub path: String,
    pub before: String,
    pub after: String,
    pub before_sha256: String,
    pub after_sha256: String,
}

impl Edit {
    pub(super) fn load(root: &Path, manifest: &Path) -> Result<Self> {
        ensure!(
            cfg!(feature = "memreport-changed-refresh"),
            "changed-refresh needs feature memreport-changed-refresh"
        );
        let edit: Self = serde_json::from_slice(&std::fs::read(manifest)?)?;
        edit.check(root)?;
        Ok(edit)
    }

    fn clean(&self, root: &Path) -> Result<()> {
        ensure!(
            root.canonicalize()?.to_str() == Some(&self.repo_root)
                && git(root, &["rev-parse", "--show-toplevel"])? == self.repo_root
                && git(root, &["rev-parse", "HEAD"])? == self.commit,
            "mirror identity mismatch"
        );
        ensure!(
            git(root, &["status", "--porcelain", "--untracked-files=all"])?.is_empty(),
            "dirty mirror"
        );
        Ok(())
    }

    fn check(&self, root: &Path) -> Result<String> {
        self.clean(root)?;
        ensure!(
            !self.before.is_empty() && !self.after.is_empty() && self.before != self.after,
            "empty or no-op edit"
        );
        ensure!(
            hash(&self.before) == self.before_sha256 && hash(&self.after) == self.after_sha256,
            "declared edit hash mismatch"
        );
        let relative = Path::new(&self.path);
        ensure!(
            !self.path.is_empty()
                && relative
                    .components()
                    .all(|c| matches!(c, Component::Normal(p) if p != ".git")),
            "edit path must be relative"
        );
        let path = root.canonicalize()?.join(relative);
        ensure!(
            path.canonicalize()? == path && path.is_file(),
            "edit target must be a regular mirror file, without symlinks"
        );
        let observed = hash(std::fs::read(path)?);
        ensure!(observed == self.before_sha256, "wrong starting hash");
        Ok(observed)
    }

    pub(super) fn run<T>(
        &self,
        root: &Path,
        refresh: impl FnOnce() -> Result<T>,
    ) -> Result<(T, Value)> {
        let before_sha256 = self.check(root)?;
        let path = root.join(&self.path);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            std::fs::write(&path, &self.after).context("applying changed edit")?;
            let after_sha256 = hash(std::fs::read(&path)?);
            ensure!(after_sha256 == self.after_sha256, "applied hash mismatch");
            let result = refresh()?;
            ensure!(
                hash(std::fs::read(&path)?) == self.after_sha256,
                "edit changed during refresh"
            );
            Ok((result, after_sha256))
        }));
        let restored = (|| -> Result<String> {
            ensure!(
                std::fs::symlink_metadata(&path)?.is_file(),
                "restore target replaced"
            );
            std::fs::write(&path, &self.before)?;
            let restored = hash(std::fs::read(&path)?);
            ensure!(restored == self.before_sha256, "restored hash mismatch");
            self.clean(root)?;
            Ok(restored)
        })();
        let restored_sha256 = restored.with_context(|| match &result {
            Ok(result) => format!("restore failed; refresh error: {:?}", result.as_ref().err()),
            Err(payload) => format!(
                "restore failed; refresh panicked: {}",
                payload
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                    .unwrap_or("non-string panic payload (see panic diagnostics)")
            ),
        })?;
        let (result, after_sha256) = match result {
            Ok(result) => result?,
            Err(payload) => std::panic::resume_unwind(payload),
        };
        Ok((
            result,
            json!({"edit": self, "before_sha256": before_sha256,
            "after_sha256": after_sha256, "restored_sha256": restored_sha256}),
        ))
    }
}

#[cfg(test)]
#[path = "mem_changed_refresh_tests.rs"]
mod tests;
