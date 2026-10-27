use crate::ast::ParsedFile;
use crate::languages::Language;
use tree_sitter::Node;

pub(super) fn direct_block_uncertainty(
    parsed: &ParsedFile,
    binder: Node<'_>,
    use_byte: Option<usize>,
) -> bool {
    parsed.language == Language::Cpp
        && binder.kind() == "declaration"
        && binder.parent().is_some_and(|scope| {
            scope.kind() == "compound_statement"
                && use_byte
                    .is_some_and(|byte| binder.start_byte() <= byte && byte < scope.end_byte())
        })
}
