use super::*;

pub(super) fn identifier_visible_from(
    parsed: &ParsedFile,
    node: Node<'_>,
    selected_row: Option<&super::super::binding_table::BindingRow>,
    def: &DefSite,
) -> Option<usize> {
    (parsed.language == Language::Java)
        .then(|| {
            selected_row.and_then(|row| {
                transfer::row_introduced_identifiers(parsed, node, row.fields)
                    .into_iter()
                    .find(|identifier| {
                        identifier.start_byte() == def.start_byte
                            && parsed.node_text(identifier) == def.path.base
                    })
                    .map(|identifier| identifier.end_byte())
            })
        })
        .flatten()
}

pub(super) fn row_excludes_identifier(
    parsed: &ParsedFile,
    node: Node<'_>,
    row: &super::super::binding_table::BindingRow,
    def: &DefSite,
) -> bool {
    transfer::row_excludes_identifier(parsed, node, row.fields, def.start_byte)
        || (parsed.language == Language::Java
            && !row.fields.is_empty()
            && transfer::row_introduced_identifiers(parsed, node, row.fields)
                .into_iter()
                .all(|identifier| {
                    identifier.start_byte() != def.start_byte
                        || parsed.node_text(&identifier) != def.path.base
                }))
}
