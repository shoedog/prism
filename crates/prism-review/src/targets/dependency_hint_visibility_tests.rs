use crate::targets::dependency_hint::{
    root_identity, sole_receiver_constructor, RootIdentity, SiteContext,
};
use prism::{ast::ParsedFile, languages::Language};
use std::{collections::BTreeMap, path::Path};

#[test]
fn crate_sibling_can_resolve_root_identity() {
    let parsed = ParsedFile::parse(
        "app.py",
        "import custom.client; custom.client.get(u)",
        Language::Python,
    )
    .unwrap();
    let call = parsed
        .tree
        .root_node()
        .named_child(1)
        .unwrap()
        .named_child(0)
        .unwrap();
    let context = SiteContext {
        file: "app.py",
        repo_root: Path::new("/nonexistent-prism-test-repo-root"),
        known_files: &BTreeMap::new(),
        resolved_name: None,
    };
    let identity: RootIdentity = root_identity(&parsed, call, &context).unwrap();
    assert_eq!(identity.resolved_path.as_deref(), Some("custom.client.get"));
    assert!(identity.root_is_library_spelling);
    assert!(sole_receiver_constructor(&parsed, call, "custom.client").is_none());
    assert!(root_identity(&parsed, parsed.tree.root_node(), &context).is_none());
}
