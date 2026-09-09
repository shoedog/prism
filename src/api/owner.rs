//! Input selection only. These public values are not executable-owner authority.
use std::path::PathBuf;

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct OwnerOptions {
    pub compiler: PathBuf,
    pub config: String,
}

impl OwnerOptions {
    /// Explicit experimental input selection; never discovers or installs a compiler.
    pub fn new(compiler: impl Into<PathBuf>, config: impl Into<String>) -> Self {
        Self {
            compiler: compiler.into(),
            config: config.into(),
        }
    }

    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.compiler.is_absolute(),
            "owner compiler must be an absolute path"
        );
        anyhow::ensure!(
            crate::executable_owner::relative(&self.config),
            "owner config must be a repository-relative path without traversal"
        );
        Ok(())
    }

    pub(crate) fn replace(
        &self,
        active: &mut Option<crate::navigation::NavigationSession>,
        root: &std::path::Path,
    ) -> anyhow::Result<()> {
        *active = None;
        self.validate()?;
        crate::build_pool::install(|| {
            crate::executable_owner::integration::replace_selected_session(
                active,
                root,
                &self.config,
                &self.compiler,
            )
        })
        .map_err(|reason| anyhow::anyhow!("owner acquisition failed: {reason}"))
    }
}

#[cfg(all(test, feature = "detached-owner-audit"))]
mod tests {
    use super::*;

    #[test]
    fn public_owner_session_resolves_contextual_member() {
        let root = tempfile::tempdir().unwrap();
        let corpus: serde_json::Value = serde_json::from_str(include_str!(
            "../../docs/eval/receiver-closure/executable-owner-fixtures.json"
        ))
        .unwrap();
        for (name, source) in corpus["files"].as_object().unwrap() {
            let file = root.path().join(name);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, source.as_str().unwrap()).unwrap();
        }
        std::fs::write(root.path().join("package.json"), r#"{"type":"module"}"#).unwrap();
        std::fs::write(
            root.path().join("tsconfig.json"),
            serde_json::json!({
                "compilerOptions":corpus["compiler_options"],"include":["src"]
            })
            .to_string(),
        )
        .unwrap();
        let mut options = crate::api::NavOptions::default();
        options.no_cache = true;
        options.owner = Some(OwnerOptions {
            compiler: std::env::var("PRISM_TYPESCRIPT")
                .expect("explicit compiler")
                .into(),
            config: "tsconfig.json".into(),
        });
        let session = crate::api::nav_session(root.path(), &options).unwrap();
        let graph = session.index.call_graph();
        let call = graph
            .calls
            .values()
            .flatten()
            .find(|c| c.caller.file == "src/app.ts" && c.callee_name == "m")
            .unwrap();
        let result = graph.resolve_call_site_full(call);
        assert_eq!(result.resolved.len(), 1);
        assert_eq!(
            result.resolved[0].confidence,
            crate::resolution::ResolutionConfidence::Exact
        );
        assert_eq!(result.resolved[0].target.file, "src/client.ts");
    }
}

#[cfg(test)]
mod selection_tests {
    use super::*;

    #[test]
    fn owner_selection_is_explicit_and_refuses_cache_before_io() {
        assert!(OwnerOptions::new("typescript.js", "tsconfig.json")
            .validate()
            .is_err());
        for config in [
            "",
            "/tsconfig.json",
            "../tsconfig.json",
            "a/../tsconfig.json",
            ".git/config",
            "a\\b",
        ] {
            assert!(OwnerOptions::new("/compiler/typescript.js", config)
                .validate()
                .is_err());
        }
        assert!(
            OwnerOptions::new("/compiler/typescript.js", "configs/tsconfig.json")
                .validate()
                .is_ok()
        );
        let root = tempfile::tempdir().unwrap();
        let mut options = crate::api::NavOptions::default();
        options.owner = Some(OwnerOptions::new("/absent/typescript.js", "tsconfig.json"));
        let error = crate::api::nav_session(root.path(), &options)
            .err()
            .unwrap();
        assert!(error.to_string().contains("--no-cache"));
        options.no_cache = true;
        options.cache_dir = Some(root.path().join("cache-sentinel"));
        assert!(crate::api::nav_session(root.path(), &options).is_err());
        assert!(!root.path().join("cache-sentinel").exists());
    }

    #[test]
    fn selected_owner_rejects_mixed_inputs_and_missing_compiler_without_fallback() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("a.py"), "def f(): pass\n").unwrap();
        let mut options = crate::api::NavOptions::default();
        options.no_cache = true;
        // Default remains usable without Node, compiler or config.
        assert!(crate::api::nav_session(root.path(), &options).is_ok());
        options.owner = Some(OwnerOptions::new(
            root.path().join("absent.js"),
            "tsconfig.json",
        ));
        let error = crate::api::nav_session(root.path(), &options)
            .err()
            .unwrap();
        assert!(error.to_string().contains("owner_requires_js_ts_only"));
        std::fs::remove_file(root.path().join("a.py")).unwrap();
        std::fs::write(root.path().join("a.ts"), "export const x = 1;\n").unwrap();
        let error = crate::api::nav_session(root.path(), &options)
            .err()
            .unwrap();
        assert!(error.to_string().contains("compiler_unavailable"));
        assert!(!root.path().join(".prism").exists());
    }
}
