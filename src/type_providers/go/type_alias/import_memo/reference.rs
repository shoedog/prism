// Frozen c84f3769 import computation, compiled only for equivalence tests.
use crate::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn signature_imports_reference(
    parsed: &ParsedFile,
    local_import_path: Option<&str>,
) -> BTreeMap<String, String> {
    let mut imports = parsed.extract_imports();
    if let Some(path) = local_import_path.filter(|path| !path.is_empty()) {
        imports.insert(String::new(), path.to_string());
    }
    if has_dot_import(parsed.tree.root_node()) {
        imports.insert(".".to_string(), String::new());
    }
    let defaults: Vec<_> = imports
        .iter()
        .filter(|(alias, path)| path.rsplit('/').next() == Some(alias.as_str()))
        .map(|(alias, path)| (alias.clone(), path.clone()))
        .collect();
    let explicit: BTreeSet<_> = imports
        .iter()
        .filter(|(alias, path)| path.rsplit('/').next() != Some(alias.as_str()))
        .map(|(alias, _)| alias.clone())
        .collect();
    let mut inferred = BTreeMap::new();
    let mut ambiguous = BTreeSet::new();
    for (_, path) in defaults {
        let Some(alias) = versionless_alias(&path) else {
            continue;
        };
        if explicit.contains(&alias) || ambiguous.contains(&alias) {
            continue;
        }
        match inferred.get(&alias) {
            None => {
                inferred.insert(alias, path);
            }
            Some(existing) if existing == &path => {}
            Some(_) => {
                inferred.remove(&alias);
                ambiguous.insert(alias);
            }
        }
    }
    imports.extend(inferred);
    imports
}

fn has_dot_import(node: tree_sitter::Node<'_>) -> bool {
    if node.kind() == "import_spec" {
        return node
            .children(&mut node.walk())
            .any(|child| child.kind() == "dot");
    }
    node.children(&mut node.walk()).any(has_dot_import)
}

fn versionless_alias(path: &str) -> Option<String> {
    let mut segments = path.rsplit('/');
    let last = segments.next()?;
    if last
        .strip_prefix('v')
        .is_some_and(|version| !version.is_empty() && version.chars().all(|c| c.is_ascii_digit()))
    {
        return segments
            .next()
            .filter(|name| !name.is_empty())
            .map(str::to_string);
    }
    let (name, version) = last.rsplit_once(".v")?;
    (!name.is_empty() && version.chars().all(|c| c.is_ascii_digit())).then(|| name.to_string())
}
