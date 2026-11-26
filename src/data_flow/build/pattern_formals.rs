//! Runtime read and endpoint selection for object-pattern formals.

use crate::access_path::AccessPath;
use crate::ast::{ParsedFile, PathSpan};
use crate::cpg::FlowConfidence;
use crate::data_flow::s1_evidence::{
    Admission, Attempt, AttemptKey, Binding, Disposition, Endpoint, FileEvidence, ProjectionCause,
    Refusal,
};
use crate::data_flow::{VarAccessKind, VarLocation};
use crate::languages::Language;
use std::borrow::Cow;
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

use super::pattern_execution as execution;
#[path = "pattern_signature.rs"]
mod signature;

use super::pattern_projection::{read_operand, reads_owner_arguments};
#[path = "pattern_endpoints.rs"]
mod endpoints;

pub(super) struct PatternFormal {
    pub(super) def: VarLocation,
    /// A producer guard (eval, mapped `arguments`, seam) refuses the formal: its reads
    /// stay in the ledger as refusals, and it supplies no Def or endpoint.
    guarded: bool,
    reads_arguments: bool,
    decl_line: usize,
    reads: BTreeMap<(AccessPath, usize), Vec<Read>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Region {
    Body(usize),
    Formals,
    Other,
}

struct Read {
    span: (usize, usize),
    root: (usize, usize),
    region: Region,
    binding: Binding,
    admission: Admission,
    body_refusal: Cell<Option<Refusal>>,
    projection_causes: Cell<u8>,
    projection_write_effect: Cell<bool>,
}

pub(super) struct Selected<'a> {
    pub(super) loc: VarLocation,
    pub(super) signature: bool,
    original_path: &'a AccessPath,
    covered: Vec<&'a Read>,
}

impl Selected<'_> {
    pub(super) fn credit(
        &self,
        formal: &PatternFormal,
        owner: Node<'_>,
        evidence: &mut FileEvidence,
        label: FlowConfidence,
    ) {
        for read in &self.covered {
            formal.record(
                owner,
                evidence,
                self.original_path,
                read,
                Some((&self.loc, Some(label))),
            );
        }
    }

    pub(super) fn withhold(
        &self,
        formal: &PatternFormal,
        owner: Node<'_>,
        evidence: &mut FileEvidence,
    ) {
        for read in &self.covered {
            formal.record(
                owner,
                evidence,
                self.original_path,
                read,
                Some((&self.loc, None)),
            );
        }
    }

    pub(super) fn refuse(&self, reason: Refusal) {
        for read in &self.covered {
            read.body_refusal.set(Some(reason));
        }
    }
}

#[derive(Clone, Copy)]
struct Ancestry {
    region: Region,
    decorator: bool,
    nested_list: bool,
}

impl Ancestry {
    fn new(region: Region) -> Self {
        Self {
            region,
            decorator: false,
            nested_list: false,
        }
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
            if !element.admitted {
                continue;
            }
            let guarded = refused.contains(&element.name);
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
                    guarded,
                    reads_arguments: false,
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
        let eligible = formals
            .values()
            .filter(|formal| !formal.guarded)
            .map(|formal| formal.def.start_byte)
            .collect();
        let signature = signature::Signature::certify(parsed, owner, &eligible);
        let parameters = owner.child_by_field_name("parameters");
        let body = owner.child_by_field_name("body");
        let mut cursor = owner.walk();
        let mut stack: Vec<_> = owner
            .named_children(&mut cursor)
            .map(|child| {
                let region = if Some(child) == parameters {
                    Region::Formals
                } else if Some(child) == body {
                    Region::Body(owner.id())
                } else {
                    Region::Other
                };
                (child, Ancestry::new(region))
            })
            .collect();
        if owner.kind() == "method_definition"
            && matches!(parsed.language, Language::TypeScript | Language::Tsx)
            && owner
                .parent()
                .is_some_and(|parent| parent.kind() == "class_body")
        {
            let mut previous = owner.prev_named_sibling();
            while let Some(node) = previous {
                if node.kind() == "decorator" {
                    stack.push((node, Ancestry::new(Region::Other)));
                } else if !node.is_extra() {
                    break;
                }
                previous = node.prev_named_sibling();
            }
        }
        stack.reverse();
        let callable_kinds = parsed.language.callable_boundary_node_types();
        let mut reads_arguments = false;
        while let Some((node, mut ancestry)) = stack.pop() {
            if !execution::runtime_node(parsed, node) {
                continue;
            }
            ancestry.decorator |= execution::decorated_callable(node).is_some();
            if matches!(
                node.kind(),
                "identifier" | "undefined" | "shorthand_property_identifier"
            ) {
                let name = decoded_name(parsed.node_text(&node));
                if name.as_deref() == Some("arguments") && ancestry.region != Region::Other {
                    reads_arguments |= reads_owner_arguments(parsed, owner, node);
                }
                if let Some(formal) = name.as_deref().and_then(|name| formals.get_mut(name)) {
                    let Some((operand, path)) = read_operand(parsed, node, &formal.def.path) else {
                        continue;
                    };
                    let root = node.start_byte();
                    let in_body_var = in_body_seam(parsed, owner, &path.base, root);
                    let scope = execution::binding_scope(parsed, owner, node, &path.base);
                    let binding = match scope {
                        Some(id) if id == owner.id() && !in_body_var => Binding::SameBinding,
                        Some(_) => Binding::OtherBinding,
                        None => Binding::Unresolved,
                    };
                    let copied_body_var = in_body_var
                        && scope == Some(owner.id())
                        && copied_vars.contains(&path.base);
                    let mut admission =
                        admit(&signature, operand, ancestry, binding, copied_body_var);
                    if formal.guarded && admission.admits() {
                        admission = Admission::Refused(Refusal::WriteEffect);
                    }
                    formal
                        .reads
                        .entry((path, parsed.line_for_byte(operand.start_byte())))
                        .or_default()
                        .push(Read {
                            span: (operand.start_byte(), operand.end_byte()),
                            root: (root, node.end_byte()),
                            region: ancestry.region,
                            binding,
                            admission,
                            body_refusal: Cell::new(None),
                            projection_causes: Cell::new(0),
                            projection_write_effect: Cell::new(false),
                        });
                }
            }
            let mut cursor = node.walk();
            let first = stack.len();
            let callable = callable_kinds.contains(&node.kind());
            for child in node.named_children(&mut cursor) {
                let mut child_ancestry = ancestry;
                if callable {
                    child_ancestry.nested_list |= ["parameters", "parameter"]
                        .iter()
                        .any(|field| node.child_by_field_name(field) == Some(child));
                    if matches!(ancestry.region, Region::Body(_))
                        && node.child_by_field_name("body") == Some(child)
                    {
                        child_ancestry.region = Region::Body(node.id());
                    }
                }
                stack.push((child, child_ancestry));
            }
            stack[first..].reverse();
        }
        formals
            .into_values()
            .map(|mut formal| {
                formal.reads_arguments = reads_arguments;
                formal
            })
            .collect()
    }

    pub(super) fn guarded(&self) -> bool {
        self.guarded
    }

    pub(super) fn produces_def(&self) -> bool {
        self.reads
            .values()
            .flatten()
            .any(|read| read.admission.admits())
    }

    fn finish_evidence(
        &self,
        owner: Node<'_>,
        evidence: &mut FileEvidence,
        credited: &BTreeSet<AttemptKey>,
    ) {
        for ((path, _), reads) in &self.reads {
            for read in reads {
                if !credited.contains(&self.attempt_key(owner, path, read)) {
                    self.record(owner, evidence, path, read, None);
                }
            }
        }
    }

    fn attempt_key(&self, owner: Node<'_>, path: &AccessPath, read: &Read) -> AttemptKey {
        AttemptKey {
            owner: [owner.start_byte(), owner.end_byte()],
            binding: [self.def.start_byte, self.def.end_byte],
            operand: [read.span.0, read.span.1],
            path: path.clone(),
        }
    }

    fn record(
        &self,
        owner: Node<'_>,
        evidence: &mut FileEvidence,
        path: &AccessPath,
        read: &Read,
        selected: Option<(&VarLocation, Option<FlowConfidence>)>,
    ) {
        let disposition = match (read.admission, selected) {
            (_, Some((_, None))) => Disposition::CapWithheld,
            (Admission::Body, _) if read.projection_write_effect.get() => {
                Disposition::Refused(Refusal::WriteEffect)
            }
            (_, Some((_, Some(_)))) => Disposition::Emitted,
            (Admission::OtherBinding, _) => Disposition::OtherBinding,
            (Admission::Refused(reason), _) => Disposition::Refused(reason),
            _ => Disposition::Refused(read.body_refusal.get().unwrap_or(Refusal::Endpoint)),
        };
        evidence.attempts.push(Attempt {
            key: self.attempt_key(owner, path, read),
            binding: read.binding,
            admission: read.admission,
            justifies_def: read.admission.admits(),
            endpoint: selected.map(|(endpoint, label)| Endpoint {
                bytes: [endpoint.start_byte, endpoint.end_byte],
                path: endpoint.path.clone(),
                label,
            }),
            disposition,
            projection_causes: ProjectionCause::from_bits(read.projection_causes.get()),
        });
    }
}

pub(super) fn finish_owner_evidence(
    formals: &[PatternFormal],
    owner: Node<'_>,
    evidence: &mut FileEvidence,
) {
    let span = [owner.start_byte(), owner.end_byte()];
    let credited = evidence
        .attempts
        .iter()
        .rev()
        .take_while(|attempt| attempt.key.owner == span)
        .map(|attempt| attempt.key.clone())
        .collect();
    for formal in formals {
        formal.finish_evidence(owner, evidence, &credited);
    }
}

/// Recognize a forced signature label already recorded under the exact pair.
pub(crate) fn is_signature_pair(parsed: &ParsedFile, from: &VarLocation, to: &VarLocation) -> bool {
    if from.kind != VarAccessKind::Def
        || to.kind != VarAccessKind::Use
        || !from.path.is_simple()
        || from.path != to.path
        || from.start_byte >= from.end_byte
        || to.start_byte >= to.end_byte
        || !parsed.language.is_js_family()
    {
        return false;
    }
    let root = parsed.tree.root_node();
    let Some(binding) = root.descendant_for_byte_range(from.start_byte, from.end_byte) else {
        return false;
    };
    let Some(operand) = root.descendant_for_byte_range(to.start_byte, to.end_byte) else {
        return false;
    };
    if binding.byte_range() != (from.start_byte..from.end_byte)
        || operand.byte_range() != (to.start_byte..to.end_byte)
    {
        return false;
    }
    let mut ancestor = binding.parent();
    while let Some(node) = ancestor {
        if node.kind() == "formal_parameters" {
            let Some(owner) = node.parent() else {
                return false;
            };
            if !node.byte_range().contains(&operand.start_byte()) {
                return false;
            }
            let eligible: BTreeSet<_> = parsed
                .js_ts_pattern_params(&owner)
                .into_iter()
                .flat_map(|pattern| pattern.elements)
                .filter(|element| element.admitted)
                .map(|element| element.bytes.start)
                .collect();
            return eligible.contains(&binding.start_byte())
                && matches!(
                    signature::Signature::certify(parsed, owner, &eligible).admission(operand),
                    Admission::Bare | Admission::Reader
                );
        }
        ancestor = node.parent();
    }
    false
}

fn admit(
    signature: &signature::Signature,
    operand: Node<'_>,
    ancestry: Ancestry,
    binding: Binding,
    copied_body_var: bool,
) -> Admission {
    if ancestry.region == Region::Other {
        return Admission::OtherBinding;
    }
    if ancestry.decorator {
        return Admission::Refused(Refusal::Decorator);
    }
    if ancestry.nested_list {
        return Admission::Refused(Refusal::NestedList);
    }
    let admission = match ancestry.region {
        Region::Formals => signature.admission(operand),
        _ => Admission::Body,
    };
    if matches!(
        admission,
        Admission::Refused(Refusal::Shape | Refusal::Binding)
    ) {
        return admission;
    }
    // An uninitialized named body var has a distinct binding, but its
    // value is copied from the parameter at body entry.
    if copied_body_var && matches!(ancestry.region, Region::Body(_)) {
        return Admission::Body;
    }
    match binding {
        Binding::OtherBinding => Admission::OtherBinding,
        Binding::Unresolved => Admission::Refused(Refusal::Binding),
        Binding::SameBinding => admission,
    }
}

pub(super) fn in_body_seam(parsed: &ParsedFile, owner: Node<'_>, name: &str, at: usize) -> bool {
    parsed.js_ts_seam_binding(&owner, name)
        && owner
            .child_by_field_name("body")
            .is_some_and(|body| body.byte_range().contains(&at))
}

pub(super) fn decoded_name(text: &str) -> Option<Cow<'_, str>> {
    if text.contains('\\') {
        ParsedFile::js_ts_decode_identifier_text(text).map(Cow::Owned)
    } else {
        Some(Cow::Borrowed(text))
    }
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
