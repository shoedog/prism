use super::{normalize_path, python::source::Source};
use prism::ast::ParsedFile;
use prism::languages::Language;
use prism::repo_loader::load_ast_sources;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

type PythonFiles = (BTreeMap<String, ParsedFile>, Vec<(String, String)>);

pub fn load_python_files(
    repo_root: &Path,
    only: Option<&BTreeSet<String>>,
) -> anyhow::Result<PythonFiles> {
    let only = only.map(|paths| {
        paths
            .iter()
            .map(|path| normalize_path(path))
            .collect::<BTreeSet<_>>()
    });
    let selected = |file: &str| {
        let file = normalize_path(file);
        Language::from_path(&file) == Some(Language::Python)
            && only.as_ref().is_none_or(|only| only.contains(&file))
    };
    let (files, skipped) =
        load_ast_sources(repo_root, &selected, &["venv", "site-packages"], |parsed| {
            Source::new(parsed).parsed().error_rate()
        })?;
    let files = files
        .into_iter()
        .map(|(path, parsed)| (normalize_path(&path), parsed))
        .collect();
    let mut skipped: Vec<_> = skipped
        .into_iter()
        .map(|skip| (normalize_path(&skip.path), format!("{:?}", skip.reason)))
        .collect();
    skipped.sort();
    Ok((files, skipped))
}
