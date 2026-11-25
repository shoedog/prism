//! Runtime read and endpoint selection for object-pattern formals.

use crate::access_path::AccessPath;
use crate::ast::js_ts::props_keys::strip_transparent_wrappers;
use crate::ast::{ParsedFile, PathSpan};
use crate::data_flow::{VarAccessKind, VarLocation};
use crate::js_write_census::{
    identity::Comp,
    walk::{census_pattern_formals, Census, Site},
};
use std::borrow::Cow;
use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

#[path = "pattern_execution.rs"]
mod execution;

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

impl Read {
    fn in_body(&self) -> bool {
        matches!(self.region, Region::Body | Region::NestedBody(_))
    }
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
        let mut stack = Vec::new();
        stack.extend(
            owner
                .child_by_field_name("body")
                .map(|node| (node, Region::Body, None)),
        );
        stack.extend(
            owner
                .child_by_field_name("parameters")
                .map(|node| (node, Region::Parameter, None)),
        );
        while let Some((node, mut region, decorator_region)) = stack.pop() {
            if !execution::runtime_node(parsed, node) {
                continue;
            }
            if execution::decorated_callable(node).is_some() {
                let Some(outer) = decorator_region else {
                    continue;
                };
                region = outer;
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
                    let binds = execution::binding_scope(parsed, owner, node, &path.base)
                        == Some(owner.id())
                        && (copied_vars.contains(&path.base)
                            || !in_body_seam(parsed, owner, &path.base, root));
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
            let first_child = stack.len();
            for child in node.named_children(&mut cursor) {
                let decorator_region = if child.kind() == "formal_parameters" {
                    Some(region)
                } else {
                    decorator_region
                };
                stack.push((
                    child,
                    execution::child_region(parsed, node, child, region),
                    decorator_region,
                ));
                if region == Region::Parameter
                    && execution::suspends_following_siblings(node, child)
                {
                    region = Region::NestedParameter(node.id());
                }
            }
            stack[first_child..].reverse();
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
                if owner_writes(parsed, *owner, census).is_some_and(|facts| {
                    !facts.sites.iter().any(|site| {
                        // The member lane's body kill inventory excludes optional signature
                        // writes (p24). A read in an immediate default checks signature writes.
                        site.region.own_params != first.in_body()
                            && (first.in_body() || site.moment <= first.span.0)
                            && bound_write(parsed, *owner, &path.base, site, first.in_body())
                            && projection_overlaps(path, site)
                    })
                }) {
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
            let representative = candidate.cloned().unwrap_or_else(|| {
                let anchor = parsed.line_start_byte(*line);
                self.use_at(path, *line, (anchor, anchor))
            });
            let start = representative.start_byte;
            let end = representative.end_byte;
            let endpoint = if !synthetic
                && owner_is_unique
                && uniform
                && (start == end
                    || reads.iter().any(|read| read.span == (start, end))
                    || representative_name(parsed, &representative))
                && same_version(
                    parsed,
                    *owner,
                    census,
                    &path.base,
                    start.min(first.span.0),
                    start.max(reads.last().unwrap().span.0),
                    first.in_body(),
                ) {
                representative
            } else {
                self.use_at(path, *line, first.span)
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
    first: usize,
    last: usize,
    in_body: bool,
) -> bool {
    if first == last {
        return true;
    }
    owner_writes(parsed, owner, census).is_some_and(|facts| {
        !facts.sites.iter().any(|site| {
            site.path.comps.is_empty()
                && first <= site.moment
                && site.moment <= last
                && bound_write(parsed, owner, name, site, in_body)
        })
    })
}

fn owner_writes<'a>(
    parsed: &ParsedFile,
    owner: Node<'_>,
    census: &'a OnceCell<Option<Census>>,
) -> Option<&'a Census> {
    census
        .get_or_init(|| {
            if parsed.js_ts_direct_eval_anywhere(&owner) {
                return None;
            }
            let facts = census_pattern_formals(parsed, owner);
            (!facts.refused()).then_some(facts)
        })
        .as_ref()
}

fn bound_write(
    parsed: &ParsedFile,
    owner: Node<'_>,
    name: &str,
    site: &Site,
    in_body: bool,
) -> bool {
    site.initialized
        && site.decl != "param"
        && decoded_name(&site.path.base).as_deref() == Some(name)
        && (in_body || !in_body_seam(parsed, owner, name, site.target_start))
        && parsed
            .tree
            .root_node()
            .descendant_for_byte_range(site.target_start, site.target_end)
            .is_some_and(|target| {
                execution::binding_scope(parsed, owner, target, name) == Some(owner.id())
            })
}

fn in_body_seam(parsed: &ParsedFile, owner: Node<'_>, name: &str, at: usize) -> bool {
    parsed.js_ts_seam_binding(&owner, name)
        && owner
            .child_by_field_name("body")
            .is_some_and(|body| body.byte_range().contains(&at))
}

fn decoded_name(text: &str) -> Option<Cow<'_, str>> {
    if text.contains('\\') {
        ParsedFile::js_ts_decode_identifier_text(text).map(Cow::Owned)
    } else {
        Some(Cow::Borrowed(text))
    }
}

fn projection_overlaps(path: &AccessPath, site: &Site) -> bool {
    // Static projections retain the identifier lane's computed-write boundary.
    let mut write = AccessPath::simple(&path.base);
    for component in &site.path.comps {
        let Comp::Name(field) = component else {
            return false;
        };
        let Some(field) = decoded_name(field) else {
            return true;
        };
        write.fields.push(field.into_owned());
    }
    path.fields.starts_with(&write.fields) || write.fields.starts_with(&path.fields)
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
                path.fields
                    .push(decoded_name(parsed.node_text(&property))?.into_owned());
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
