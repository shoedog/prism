//! Thin DFG call-site wrapper; the underlying searches retain their existing behavior.
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

#[allow(clippy::too_many_arguments)]
pub(crate) fn search_refs<'a>(
    parsed: &'a ParsedFile,
    member_refs: &crate::ast::member_reference_index::MemberReferenceSearch<'a>,
    owner: &Node<'a>,
    path: &AccessPath,
    def_line: usize,
    def_byte: usize,
    within: Option<(usize, usize)>,
    reads_only: bool,
) -> BTreeMap<usize, (usize, usize)> {
    let run = || {
        member_refs.find_path_reference_spans_fenced_in(
            owner, path, def_line, def_byte, within, reads_only,
        )
    };
    if !super::enabled() {
        return run();
    }
    let fenced = matches!(
        parsed.language,
        crate::languages::Language::JavaScript
            | crate::languages::Language::TypeScript
            | crate::languages::Language::Tsx
    ) && (reads_only
        || parsed.language.function_name(owner).is_none()
        || !parsed.js_ts_seam_binding(owner, &path.base));
    let bytes = within
        .filter(|_| fenced)
        .and_then(|(s, e)| parsed.tree.root_node().descendant_for_byte_range(s, e))
        .filter(|n| within == Some((n.start_byte(), n.end_byte())))
        .unwrap_or(*owner)
        .byte_range()
        .len();
    super::owner::search(path.is_simple(), bytes, run)
}
