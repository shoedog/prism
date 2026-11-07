use super::{
    normalize_repo_dir, path_to_repo_string, GoImportPathReason, GoImportPathResolution,
    GoModuleGraph, ModuleBoundary,
};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeMap;
use std::path::Path;

impl GoModuleGraph {
    pub(crate) fn import_path_for_dir(&mut self, dir: &str) -> Result<String, GoImportPathReason> {
        let Some(dir) = normalize_repo_dir(Path::new("/"), "", dir) else {
            return Err(GoImportPathReason::NoGoMod);
        };
        if let Some(cached) = self.memo.get(&dir) {
            return cached.clone();
        }
        let identity = self.compute_import_path(&dir);
        self.memo.insert(dir, identity.clone());
        identity
    }

    pub(crate) fn resolve_files(
        &mut self,
        files: &BTreeMap<String, ParsedFile>,
    ) -> GoImportPathResolution {
        let mut result = GoImportPathResolution {
            graph: self.telemetry.clone(),
            ..GoImportPathResolution::default()
        };
        for (path, parsed) in files {
            if parsed.language != Language::Go {
                continue;
            }
            let dir = Path::new(path)
                .parent()
                .and_then(path_to_repo_string)
                .unwrap_or_default();
            match self.import_path_for_dir(&dir) {
                Ok(import_path) => {
                    result.paths.insert(path.clone(), import_path);
                    result.proven_files += 1;
                }
                Err(reason) => {
                    result.unproven_files += 1;
                    *result
                        .reasons
                        .entry(reason.as_str().to_string())
                        .or_default() += 1;
                }
            }
        }
        result
    }

    fn compute_import_path(&self, dir: &str) -> Result<String, GoImportPathReason> {
        if self.telemetry.workspace_invalid {
            return Err(GoImportPathReason::WorkspaceInvalid);
        }
        let mut boundary_dir = dir;
        loop {
            if let Some(boundary) = self.boundaries.get(boundary_dir) {
                return match boundary {
                    ModuleBoundary::Malformed => Err(GoImportPathReason::Malformed),
                    ModuleBoundary::Symlink => Err(GoImportPathReason::Symlink),
                    ModuleBoundary::Valid(_) => {
                        let Some(base) = self.providers.get(boundary_dir) else {
                            return Err(if self.replace_unproven_dirs.contains(boundary_dir) {
                                GoImportPathReason::ReplaceUnproven
                            } else {
                                GoImportPathReason::InactiveModule
                            });
                        };
                        let suffix = dir.strip_prefix(boundary_dir).unwrap_or_default();
                        let suffix = suffix.strip_prefix('/').unwrap_or(suffix);
                        Ok(if suffix.is_empty() {
                            base.clone()
                        } else {
                            format!("{}/{suffix}", base.trim_end_matches('/'))
                        })
                    }
                };
            }
            let Some((parent, _)) = boundary_dir.rsplit_once('/') else {
                if boundary_dir.is_empty() {
                    break;
                }
                boundary_dir = "";
                continue;
            };
            boundary_dir = parent;
        }
        Err(GoImportPathReason::NoGoMod)
    }
}
