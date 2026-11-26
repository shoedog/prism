//! Select proved Use endpoints for pattern-formal entry Defs.

use super::*;
use crate::data_flow::build::pattern_values::Values;

impl PatternFormal {
    pub(in crate::data_flow::build) fn endpoints(
        &self,
        parsed: &ParsedFile,
        owner: &Node<'_>,
        preferred: &BTreeMap<(AccessPath, usize), VarLocation>,
        values: &Values<'_>,
        synthetic: bool,
        owner_is_unique: bool,
    ) -> Vec<Selected<'_>> {
        let mut endpoints = Vec::new();
        let bare = self.reads.iter().any(|((path, _), reads)| {
            !path.has_fields()
                && reads.iter().any(|read| {
                    read.region != Region::Other && read.binding == Binding::SameBinding
                })
        });
        let mixed = (u8::from(bare) * ProjectionCause::BareOccurrence.bit())
            | (u8::from(self.reads_arguments) * ProjectionCause::Arguments.bit());
        for (key @ (path, line), reads) in &self.reads {
            let candidate = preferred.get(key);
            for read in reads {
                if read.admission == Admission::Reader
                    || read.admission == Admission::Bare
                        && (synthetic || candidate.is_some() || *line > self.decl_line)
                {
                    endpoints.push(Selected {
                        loc: self.use_at(path, *line, read.span),
                        signature: true,
                        original_path: path,
                        covered: vec![read],
                    });
                }
            }
            let projection_effects = if path.has_fields()
                && reads.iter().any(|read| read.admission == Admission::Body)
            {
                values.projection_effects(path, self.def.end_byte)
            } else {
                0
            };
            let projection = path.has_fields() && mixed == 0 && projection_effects == 0;
            let selected_path = if projection { path } else { &self.def.path };
            let body_reads: Vec<_> = reads
                .iter()
                .filter(|read| {
                    if read.admission != Admission::Body {
                        return false;
                    }
                    if path.has_fields() {
                        read.projection_causes.set(mixed | projection_effects);
                        read.projection_write_effect
                            .set(mixed == 0 && projection_effects != 0);
                    }
                    let refusal =
                        values.body_refusal(selected_path, self.def.end_byte, read.root.0);
                    read.body_refusal.set(refusal);
                    refusal.is_none()
                })
                .collect();
            let Some(&first) = body_reads.first() else {
                continue;
            };
            if projection {
                endpoints.push(Selected {
                    loc: self.use_at(path, *line, first.span),
                    signature: false,
                    original_path: path,
                    covered: body_reads
                        .into_iter()
                        .filter(|read| read.region == first.region)
                        .collect(),
                });
                continue;
            }
            let candidate = preferred.get(&(selected_path.clone(), *line));
            if !path.has_fields() && !synthetic && candidate.is_none() && *line <= self.decl_line {
                continue;
            }
            let uniform = self
                .reads
                .iter()
                .filter(|((_, at_line), _)| at_line == line)
                .all(|((other, _), occurrences)| {
                    other == path
                        && occurrences.iter().all(|read| {
                            read.admission == Admission::Body
                                && read.body_refusal.get().is_none()
                                && read.region == first.region
                        })
                });
            let representative = candidate.cloned().unwrap_or_else(|| {
                let anchor = parsed.line_start_byte(*line);
                self.use_at(selected_path, *line, (anchor, anchor))
            });
            let start = representative.start_byte;
            let end = representative.end_byte;
            let representative_span = |read: &Read| {
                if path.has_fields() {
                    read.root
                } else {
                    read.span
                }
            };
            let (loc, covered) = if !synthetic
                && owner_is_unique
                && uniform
                && (start == end && !path.has_fields()
                    || reads
                        .iter()
                        .any(|read| representative_span(read) == (start, end))
                    || representative_name(parsed, &representative, &body_reads))
                && representative_in_body(parsed, *owner, &representative, first.region)
                && values.same_version(
                    selected_path,
                    start.min(first.root.0),
                    start.max(body_reads.last().unwrap().root.0),
                ) {
                (representative, body_reads)
            } else {
                (
                    self.use_at(selected_path, *line, representative_span(first)),
                    vec![first],
                )
            };
            endpoints.push(Selected {
                loc,
                signature: false,
                original_path: path,
                covered,
            });
        }
        endpoints
    }

    fn use_at(
        &self,
        path: &AccessPath,
        line: usize,
        (start_byte, end_byte): (usize, usize),
    ) -> VarLocation {
        VarLocation {
            line,
            path: path.clone(),
            start_byte,
            end_byte,
            kind: VarAccessKind::Use,
            ..self.def.clone()
        }
    }
}

fn representative_in_body(
    parsed: &ParsedFile,
    owner: Node<'_>,
    loc: &VarLocation,
    region: Region,
) -> bool {
    let Some(body) = owner.child_by_field_name("body") else {
        return false;
    };
    if !body.byte_range().contains(&loc.start_byte) || loc.end_byte > body.end_byte() {
        return false;
    }
    let Some(node) =
        body.descendant_for_byte_range(loc.start_byte, loc.end_byte.max(loc.start_byte + 1))
    else {
        return false;
    };
    if parsed.js_ts_def_scope_id(loc.start_byte, &loc.path.base) != Some(owner.id()) {
        return false;
    }
    let mut ancestor = Some(node);
    let kinds = parsed.language.callable_boundary_node_types();
    while let Some(node) = ancestor {
        if node == owner
            || (kinds.contains(&node.kind())
                && node
                    .child_by_field_name("body")
                    .is_some_and(|body| body.byte_range().contains(&loc.start_byte)))
        {
            return region == Region::Body(node.id());
        }
        ancestor = node.parent();
    }
    false
}

/// A JSX attribute name stands for a read only when that attribute's own
/// value holds a certified read (`x={x}`), never a sibling attribute's read.
fn representative_name(parsed: &ParsedFile, loc: &VarLocation, reads: &[&Read]) -> bool {
    loc.start_byte < loc.end_byte
        && parsed
            .tree
            .root_node()
            .descendant_for_byte_range(loc.start_byte, loc.end_byte)
            .is_some_and(|node| {
                node.kind() == "property_identifier"
                    && node
                        .parent()
                        .filter(|parent| parent.kind() == "jsx_attribute")
                        .is_some_and(|attribute| {
                            reads.iter().any(|read| {
                                attribute.start_byte() <= read.root.0
                                    && read.root.1 <= attribute.end_byte()
                            })
                        })
            })
}
