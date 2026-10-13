//! Caller-only negative binding evidence for E5. No module path resolution.
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn module_binding_evidence<'a>(
    files: impl IntoIterator<Item = (&'a String, &'a ParsedFile)>,
) -> BTreeMap<String, BTreeSet<String>> {
    files
        .into_iter()
        .filter_map(|(file, parsed)| {
            if !matches!(
                parsed.language,
                Language::JavaScript | Language::TypeScript | Language::Tsx
            ) {
                return None;
            }
            let root = parsed.tree.root_node();
            let mut cursor = root.walk();
            let module = root
                .named_children(&mut cursor)
                .any(|n| matches!(n.kind(), "import_statement" | "export_statement"));
            // The existing syntactic extractor sees scalar, destructured and
            // nested requires. Even conservative evidence keeps the fallback;
            // this mask never grants a positive binding or traverses a target.
            module.then(|| (file.clone(), parsed.extract_imports().into_keys().collect()))
        })
        .collect()
}
