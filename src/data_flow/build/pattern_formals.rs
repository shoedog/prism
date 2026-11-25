//! Runtime read and endpoint selection for object-pattern formals.

use crate::access_path::AccessPath;
use crate::ast::js_ts::props_keys::strip_transparent_wrappers;
use crate::ast::{ParsedFile, PathSpan};
use crate::data_flow::{VarAccessKind, VarLocation};
use crate::js_write_census::{
    tables::Tables,
    walk::{census_with, Census},
};
use std::borrow::Cow;
use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

pub(super) struct PatternFormal {
    pub(super) def: VarLocation,
    decl_line: usize,
    reads: BTreeMap<(AccessPath, usize), Vec<Read>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Region {
    Body,
    Parameter,
    NestedBody(usize),
    NestedParameter(usize),
}

struct Read {
    span: (usize, usize),
    region: Region,
    admitted: bool,
}

impl PatternFormal {
    pub(super) fn collect(
        parsed: &ParsedFile,
        owner: Node<'_>,
        file: &str,
        function: &str,
        start: usize,
        refused: &BTreeSet<String>,
        writes: &[PathSpan],
    ) -> Vec<Self> {
        let mut formals = BTreeMap::new();
        for element in parsed
            .js_ts_pattern_params(&owner)
            .into_iter()
            .flat_map(|p| p.elements)
        {
            if !element.admitted || refused.contains(&element.name) {
                continue;
            }
            let start_byte = element.bytes.start;
            let end_byte = element.bytes.end;
            let def = VarLocation {
                file: file.to_owned(),
                function: function.to_owned(),
                function_start_line: start,
                line: start,
                path: AccessPath::simple(&element.name),
                start_byte,
                end_byte,
                kind: VarAccessKind::Def,
            };
            formals.insert(
                element.name,
                Self {
                    def,
                    decl_line: parsed.line_for_byte(start_byte),
                    reads: BTreeMap::new(),
                },
            );
        }
        if formals.is_empty() {
            return Vec::new();
        }

        let named = !crate::ast::is_synthetic_owner(function);
        let copied_vars: BTreeSet<_> = if named {
            writes
                .iter()
                .filter_map(|span| {
                    let name = &span.path.base;
                    (span.path.is_simple()
                        && formals.contains_key(name)
                        && parsed.js_ts_seam_binding(&owner, name)
                        && uninitialized_body_var(parsed, owner, span))
                    .then_some(name)
                })
                .collect()
        } else {
            BTreeSet::new()
        };
        let boundaries = parsed.language.callable_boundary_node_types();
        let mut stack = Vec::new();
        stack.extend(
            owner
                .child_by_field_name("body")
                .map(|node| (node, Region::Body)),
        );
        stack.extend(
            owner
                .child_by_field_name("parameters")
                .map(|node| (node, Region::Parameter)),
        );
        while let Some((node, region)) = stack.pop() {
            if node.is_error() || node.is_missing() || parsed.language.is_erased_type_boundary(node)
            {
                continue;
            }
            if matches!(
                node.kind(),
                "identifier" | "undefined" | "shorthand_property_identifier"
            ) {
                let spelling = parsed.node_text(&node);
                let name = if spelling.contains('\\') {
                    ParsedFile::js_ts_decoded_identifier(spelling).map(Cow::Owned)
                } else {
                    Some(Cow::Borrowed(spelling))
                };
                if let Some(formal) = name.as_deref().and_then(|name| formals.get_mut(name)) {
                    let Some((operand, path)) = read_operand(parsed, node, &formal.def.path) else {
                        continue;
                    };
                    let root = node.start_byte();
                    let binds = parsed.js_ts_occurrence_binds_formal(&owner, &path.base, root)
                        // A named var without an initializer inherits the entry value at the
                        // parameter/body seam. Keep the identifier twin's recorded kill.
                        || (copied_vars.contains(&path.base)
                            && parsed.js_ts_def_scope_id(root, &path.base) == Some(owner.id()));
                    let immediate = region != Region::Parameter
                        || initialized_before(parsed, &formal.def, root);
                    formal
                        .reads
                        .entry((path, parsed.line_for_byte(operand.start_byte())))
                        .or_default()
                        .push(Read {
                            span: (operand.start_byte(), operand.end_byte()),
                            region,
                            admitted: binds && immediate,
                        });
                }
            }

            let mut cursor = node.walk();
            let nested = boundaries.contains(&node.kind());
            let eager_parameter_call =
                region == Region::Parameter && nested && directly_invoked(node);
            let mut children = node.named_children(&mut cursor).collect::<Vec<_>>();
            while let Some(child) = children.pop() {
                let deferred = nested
                    && ["parameters", "body", "parameter"]
                        .iter()
                        .any(|field| node.child_by_field_name(field) == Some(child));
                let child_region = if deferred && !eager_parameter_call {
                    match region {
                        Region::Parameter | Region::NestedParameter(_) => {
                            Region::NestedParameter(node.id())
                        }
                        _ => Region::NestedBody(node.id()),
                    }
                } else {
                    region
                };
                stack.push((child, child_region));
            }
        }
        formals
            .into_values()
            .filter(|formal| formal.reads.values().flatten().any(|read| read.admitted))
            .collect()
    }

    pub(super) fn endpoints(
        &self,
        parsed: &ParsedFile,
        owner: &Node<'_>,
        preferred: &BTreeMap<(AccessPath, usize), VarLocation>,
        census: &OnceCell<Option<Census>>,
        synthetic: bool,
        owner_is_unique: bool,
    ) -> Vec<VarLocation> {
        let members = self
            .reads
            .keys()
            .any(|(path, _)| path.has_fields())
            .then(|| parsed.formal_member_facts(owner, &self.def.path.base, self.def.start_byte));
        let mut endpoints = Vec::new();
        for (key @ (path, line), reads) in &self.reads {
            let mut admitted = reads.iter().filter(|read| read.admitted);
            // Default closures have their own runtime region and can run after body writes.
            // Each capture is materialized at the proved operand, without a signature lookup.
            endpoints.extend(admitted.clone().filter_map(|read| {
                matches!(read.region, Region::NestedParameter(_))
                    .then(|| self.use_at(path, *line, read.span))
            }));
            let Some(first) =
                admitted.find(|read| !matches!(read.region, Region::NestedParameter(_)))
            else {
                continue;
            };
            if path.has_fields() {
                if members.as_ref().is_some_and(|facts| !facts.is_killed(path)) {
                    endpoints.push(self.use_at(path, *line, first.span));
                }
                continue;
            }
            let candidate = preferred.get(key);
            if !synthetic && candidate.is_none() && *line <= self.decl_line {
                continue;
            }
            let uniform = reads
                .iter()
                .all(|read| read.admitted && read.region == first.region);
            let endpoint = if synthetic
                || !owner_is_unique
                || !uniform
                || !same_version(
                    parsed,
                    *owner,
                    census,
                    &path.base,
                    first,
                    reads.last().unwrap(),
                ) {
                self.use_at(path, *line, first.span)
            } else if let Some(existing) = candidate {
                let bytes = (existing.start_byte, existing.end_byte);
                if reads.iter().any(|read| read.span == bytes)
                    || representative_name(parsed, existing)
                {
                    existing.clone()
                } else {
                    self.use_at(path, *line, first.span)
                }
            } else {
                let anchor = parsed.line_start_byte(*line);
                self.use_at(path, *line, (anchor, anchor))
            };
            endpoints.push(endpoint);
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

fn same_version(
    parsed: &ParsedFile,
    owner: Node<'_>,
    census: &OnceCell<Option<Census>>,
    name: &str,
    first: &Read,
    last: &Read,
) -> bool {
    if first.span == last.span {
        return true;
    }
    census
        .get_or_init(|| {
            if parsed.js_ts_direct_eval_anywhere(&owner) {
                return None;
            }
            let facts = census_with(parsed, Tables::shared(), owner, false);
            (!facts.refused()).then_some(facts)
        })
        .as_ref()
        .is_some_and(|facts| {
            !facts.sites.iter().any(|site| {
                site.initialized
                    && site.path.base == name
                    && site.path.comps.is_empty()
                    && first.span.0 <= site.moment
                    && site.moment <= last.span.0
            })
        })
}

fn read_operand<'a>(
    parsed: &ParsedFile,
    root: Node<'a>,
    base: &AccessPath,
) -> Option<(Node<'a>, AccessPath)> {
    if parsed.js_ts_identifier_is_not_a_read(root)
        || parsed.js_ts_span_in_with_body(root.start_byte(), root.end_byte())
    {
        return None;
    }
    let mut operand = root;
    let mut path = base.clone();
    let mut receiver = root;
    while let Some(parent) = receiver.parent() {
        match parent.kind() {
            "jsx_closing_element" | "meta_property" => return None,
            "member_expression" if parent.child_by_field_name("object") == Some(receiver) => {
                let property = parent.child_by_field_name("property")?;
                path.fields.push(parsed.node_text(&property).to_owned());
                operand = parent;
            }
            "subscript_expression"
                if operand == root && parent.child_by_field_name("object") == Some(receiver) =>
            {
                if parsed.member_is_write_target(parent) {
                    return None;
                }
                break;
            }
            _ if strip_transparent_wrappers(parent) == Some(operand) => {}
            _ => break,
        }
        receiver = parent;
    }
    if operand != root && parsed.member_is_write_target(operand) {
        return None;
    }
    Some((operand, path))
}

fn directly_invoked(callable: Node<'_>) -> bool {
    let mut expression = callable;
    while let Some(parent) = expression.parent() {
        if strip_transparent_wrappers(parent) != Some(callable) {
            return matches!(parent.kind(), "call_expression" | "new_expression")
                && (parent.child_by_field_name("function") == Some(expression)
                    || parent.child_by_field_name("constructor") == Some(expression));
        }
        expression = parent;
    }
    false
}

fn initialized_before(parsed: &ParsedFile, def: &VarLocation, read: usize) -> bool {
    if read < def.end_byte {
        return false;
    }
    let mut node = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(def.start_byte, def.end_byte);
    while let Some(current) = node {
        let initializer = match current.kind() {
            "formal_parameters" => break,
            "assignment_pattern" | "object_assignment_pattern" => {
                current.child_by_field_name("right")
            }
            "required_parameter" | "optional_parameter" => current.child_by_field_name("value"),
            _ => None,
        };
        if initializer.is_some_and(|value| value.byte_range().contains(&read)) {
            return false;
        }
        node = current.parent();
    }
    true
}

fn representative_name(parsed: &ParsedFile, loc: &VarLocation) -> bool {
    loc.start_byte < loc.end_byte
        && parsed
            .tree
            .root_node()
            .descendant_for_byte_range(loc.start_byte, loc.end_byte)
            .is_some_and(|node| {
                node.kind() == "property_identifier"
                    && node
                        .parent()
                        .is_some_and(|parent| parent.kind() == "jsx_attribute")
            })
}

fn uninitialized_body_var(parsed: &ParsedFile, owner: Node<'_>, span: &PathSpan) -> bool {
    let Some(node) = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(span.start_byte, span.end_byte)
    else {
        return false;
    };
    parsed.js_ts_span_in_own_scope(&owner, span.start_byte, span.end_byte)
        && node.parent().is_some_and(|declarator| {
            declarator.kind() == "variable_declarator"
                && declarator.child_by_field_name("name") == Some(node)
                && declarator.child_by_field_name("value").is_none()
                && declarator
                    .parent()
                    .is_some_and(|decl| decl.kind() == "variable_declaration")
        })
}
