use super::{use_lookup, BindingFacts, Line};
use crate::ast::ParsedFile;
use crate::data_flow::FlowEdge;
use crate::languages::Language;

/// A zero-width Lua edge with multiple same-line reads cannot select one byte.
/// If any candidate is owned by a different declaration or mask, flat facts
/// are unsafe; duplicate reads of the queried binding itself remain unchanged.
pub(in crate::cpg::reaching) fn masked_same_line_lookup(
    facts: &BindingFacts,
    parsed: &ParsedFile,
    edge: &FlowEdge,
) -> Option<Line> {
    if facts.language != Language::Lua || edge.to.start_byte < edge.to.end_byte {
        return None;
    }
    let def_index = facts.def_sites.get(&(
        edge.from.path.base.clone(),
        edge.from.line,
        edge.from.start_byte,
    ))?;
    let definition = &facts.def_bindings[*def_index].id;
    let candidates = use_lookup::candidate_bytes(parsed, edge);
    (candidates.len() > 1)
        .then(|| {
            candidates
                .into_iter()
                .filter_map(|byte| facts.visible_declaration(&edge.to.path.base, byte))
                .filter(|binding| &binding.id != definition)
                .filter_map(|binding| binding.declaration_line)
                .min()
        })
        .flatten()
}
