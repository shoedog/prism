//! Value-version proof for body endpoints seeded by pattern formals.

use super::pattern_execution as execution;
use super::pattern_formals::{decoded_name, in_body_seam};
use super::pattern_projection::Stores;
use super::pattern_reads;
use super::pattern_signature_effects::Effects;
use crate::access_path::AccessPath;
use crate::ast::js_ts::{formal_binding, props_keys::strip_transparent_wrappers};
use crate::ast::ParsedFile;
use crate::data_flow::s1_evidence::{ProjectionCause, Refusal};
use crate::js_write_census::{
    identity::Comp,
    walk::{census_pattern_formals, Census, Site},
};
use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

type Span = [usize; 2];

pub(super) struct Values<'a> {
    parsed: &'a ParsedFile,
    owner: Node<'a>,
    census: OnceCell<Census>,
    signature: OnceCell<SignatureEffects<'a>>,
    stores: OnceCell<Stores>,
    projection_signature: OnceCell<Effects>,
}

#[derive(Default)]
struct SignatureEffects<'a> {
    fields: BTreeMap<Span, usize>,
    direct_eval: BTreeSet<Span>,
    aliases: Vec<(Node<'a>, Node<'a>)>,
    escaped_writers: BTreeSet<Span>,
}

impl<'a> Values<'a> {
    pub(super) fn new(parsed: &'a ParsedFile, owner: Node<'a>) -> Self {
        Self {
            parsed,
            owner,
            census: OnceCell::new(),
            signature: OnceCell::new(),
            stores: OnceCell::new(),
            projection_signature: OnceCell::new(),
        }
    }

    fn census(&self) -> &Census {
        self.census
            .get_or_init(|| census_pattern_formals(self.parsed, self.owner))
    }

    fn signature(&self) -> &SignatureEffects<'a> {
        self.signature
            .get_or_init(|| collect_signature_effects(self.parsed, self.owner))
    }

    pub(super) fn unresolved_sites(&self) -> BTreeSet<Span> {
        let signature = self.signature();
        let mut sites: BTreeSet<_> = signature.fields.keys().copied().collect();
        sites.extend(signature.aliases.iter().map(|(_, operand)| span(*operand)));
        sites.extend(self.projection_signature().sites());
        sites.extend(
            self.census()
                .sites
                .iter()
                .filter(|site| self.unresolved_signature_write(site))
                .map(|site| [site.target_start, site.target_end]),
        );
        sites
    }

    fn projection_signature(&self) -> &Effects {
        self.projection_signature
            .get_or_init(|| Effects::collect(self.parsed, self.owner))
    }

    pub(super) fn projection_effects(&self, path: &AccessPath, binding_end: usize) -> u8 {
        let stores = self
            .stores
            .get_or_init(|| Stores::collect(self.parsed, self.owner));
        let signature = self.projection_signature();
        let mut causes = stores
            .causes(self.owner, path)
            .fold(0, |bits, cause| bits | cause.bit());
        if signature.withholds(path, binding_end) {
            causes |= ProjectionCause::SignatureEffect.bit();
        }
        if signature.has_binding_effect() {
            causes |= ProjectionCause::S2.bit();
        }
        causes
    }

    pub(super) fn body_refusal(
        &self,
        path: &AccessPath,
        binding_end: usize,
        read: usize,
    ) -> Option<Refusal> {
        if self
            .signature()
            .direct_eval
            .iter()
            .any(|span| span[1] > binding_end)
            || self.census().sites.iter().any(|site| {
                self.unresolved_signature_write(site)
                    && site.moment > binding_end
                    && self.bound_write(&path.base, site, false)
                    && if path.has_fields() {
                        projection_overlaps(path, site)
                    } else {
                        site.path.comps.is_empty()
                    }
            })
        {
            return Some(Refusal::WriteEffect);
        }
        let census = self.census();
        if path.has_fields()
            && (census.refused() || self.parsed.js_ts_direct_eval_anywhere(&self.owner))
        {
            return Some(Refusal::ValueVersion);
        }
        let mut replaced = !path.has_fields()
            && census.sites.iter().any(|site| {
                site.moment > binding_end
                    && pattern_reads::unconditional_key_write(self.parsed, site)
                    && self.bound_write(&path.base, site, true)
            });
        let mut uncertain = false;
        for site in census
            .sites
            .iter()
            .filter(|site| !site.region.own_params && self.bound_write(&path.base, site, true))
        {
            if path.has_fields() && !projection_overlaps(path, site)
                || !path.has_fields() && !site.path.comps.is_empty()
            {
                continue;
            }
            if let Some(operation) = destructuring_operation(self.parsed, site) {
                if operation.byte_range().contains(&read) {
                    uncertain |= !read_precedes_target(operation, read, site);
                    continue;
                }
            }
            if path.has_fields() {
                if site.region.with_body || site.path.comps.contains(&Comp::Index) {
                    uncertain = true;
                } else {
                    replaced = true;
                }
            }
        }
        if replaced {
            Some(Refusal::Replaced)
        } else if uncertain {
            Some(Refusal::ValueVersion)
        } else {
            None
        }
    }

    pub(super) fn same_version(&self, path: &AccessPath, first: usize, last: usize) -> bool {
        if first == last {
            return true;
        }
        let census = self.census();
        if census.refused() || self.parsed.js_ts_direct_eval_anywhere(&self.owner) {
            return false;
        }
        !census.sites.iter().any(|site| {
            if site.region.own_params
                || !site.path.comps.is_empty()
                || !self.bound_write(&path.base, site, true)
            {
                return false;
            }
            if first <= site.moment && site.moment <= last {
                return true;
            }
            destructuring_operation(self.parsed, site).is_some_and(|operation| {
                first < operation.end_byte()
                    && operation.start_byte() <= last
                    && !read_precedes_target(operation, last, site)
            })
        })
    }

    fn unresolved_signature_write(&self, site: &Site) -> bool {
        if !site.region.own_params || !site.initialized || site.decl == "param" {
            return false;
        }
        if site.region.nested || site.region.instance_field {
            // A write in a stored literal is deferred. An opaque signature operation
            // covering the writer can expose or execute it before the owner body.
            let signature = self.signature();
            return signature
                .fields
                .iter()
                .any(|(span, &at)| span[0] <= site.target_start && site.target_end <= at)
                || signature
                    .escaped_writers
                    .iter()
                    .any(|span| span[0] <= site.target_start && site.target_end <= span[1]);
        }
        site.region.with_body
            || site.path.comps.contains(&Comp::Index)
            || destructuring_operation(self.parsed, site).is_some()
    }

    fn bound_write(&self, name: &str, site: &Site, body: bool) -> bool {
        site.initialized
            && site.decl != "param"
            && decoded_name(&site.path.base).as_deref() == Some(name)
            && (body || !in_body_seam(self.parsed, self.owner, name, site.target_start))
            && self
                .parsed
                .tree
                .root_node()
                .descendant_for_byte_range(site.target_start, site.target_end)
                .is_none_or(|target| {
                    execution::binding_scope(self.parsed, self.owner, target, name)
                        .is_none_or(|scope| scope == self.owner.id())
                })
    }
}

fn projection_overlaps(path: &AccessPath, site: &Site) -> bool {
    path.fields
        .iter()
        .zip(&site.path.comps)
        .all(|(field, part)| match part {
            Comp::Index => true,
            Comp::Name(name) => decoded_name(name).is_none_or(|name| name == *field),
        })
}

fn destructuring_operation<'tree>(parsed: &'tree ParsedFile, site: &Site) -> Option<Node<'tree>> {
    let start = site.construct_start?;
    let operation = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(start, site.moment)?;
    let target = operation
        .child_by_field_name("left")
        .or_else(|| operation.child_by_field_name("name"))?;
    let target = strip_transparent_wrappers(target)?;
    matches!(target.kind(), "object_pattern" | "array_pattern").then_some(operation)
}

fn read_precedes_target(operation: Node<'_>, byte: usize, site: &Site) -> bool {
    if byte < operation.start_byte() {
        return true;
    }
    if ["right", "value"].iter().any(|field| {
        operation
            .child_by_field_name(field)
            .is_some_and(|node| node.byte_range().contains(&byte))
    }) {
        return true;
    }
    // In the first property, a computed key is evaluated before that
    // property's own target and every subsequent target in the operation.
    let Some(object) = operation
        .child_by_field_name("left")
        .and_then(strip_transparent_wrappers)
    else {
        return false;
    };
    if object.kind() != "object_pattern" {
        return false;
    }
    let mut cursor = object.walk();
    let Some(first) = object
        .named_children(&mut cursor)
        .find(|node| !node.is_extra())
    else {
        return false;
    };
    first.child_by_field_name("key").is_some_and(|key| {
        key.kind() == "computed_property_name"
            && key.byte_range().contains(&byte)
            && key.end_byte() <= site.target_start
    })
}

fn collect_signature_effects<'a>(parsed: &'a ParsedFile, owner: Node<'a>) -> SignatureEffects<'a> {
    let mut effects = SignatureEffects::default();
    let Some(list) = owner.child_by_field_name("parameters") else {
        return effects;
    };
    let boundaries = parsed.language.callable_boundary_node_types();
    let mut cursor = list.walk();
    let entries: Vec<_> = list.named_children(&mut cursor).collect();
    let mut stored = Vec::new();
    for entry in &entries {
        let Some((binding, Some(value))) = formal_binding(*entry) else {
            continue;
        };
        if binding.kind() != "identifier" {
            continue;
        }
        effects.aliases.extend(
            possible_aliases(value)
                .into_iter()
                .map(|operand| (binding, operand)),
        );
        if let Some(value) =
            strip_transparent_wrappers(value).filter(|node| boundaries.contains(&node.kind()))
        {
            stored.push((binding, value));
        }
    }
    let mut escaped = BTreeSet::new();
    let mut stack: Vec<_> = entries.into_iter().map(|node| (node, false)).collect();
    while let Some((node, mut exposed)) = stack.pop() {
        if !execution::runtime_node(parsed, node) || execution::decorated_callable(node).is_some() {
            continue;
        }
        if boundaries.contains(&node.kind()) {
            continue;
        }
        exposed |= matches!(
            node.kind(),
            "call_expression" | "new_expression" | "computed_property_name" | "tagged_template"
        );
        if exposed && node.kind() == "identifier" && !parsed.js_ts_identifier_is_not_a_read(node) {
            if let Some(name) = decoded_name(parsed.node_text(&node)) {
                if execution::binding_scope(parsed, owner, node, &name) == Some(owner.id()) {
                    escaped.insert(name.into_owned());
                }
            }
        }
        let property_read = matches!(node.kind(), "member_expression" | "subscript_expression")
            && !parsed.member_is_write_target(node);
        let get = matches!(
            node.kind(),
            "pair_pattern" | "object_assignment_pattern" | "shorthand_property_identifier_pattern"
        ) && node
            .parent()
            .is_some_and(|parent| parent.kind() == "object_pattern");
        if property_read
            || get
            || matches!(
                node.kind(),
                "computed_property_name"
                    | "call_expression"
                    | "new_expression"
                    | "array_pattern"
                    | "rest_pattern"
                    | "spread_element"
            )
        {
            // Get occurs before a property's leaf binding (or nested pattern).
            let at = match node.kind() {
                "pair_pattern" => node
                    .child_by_field_name("value")
                    .map_or(node.end_byte(), |value| value.start_byte()),
                "object_assignment_pattern" => node
                    .child_by_field_name("left")
                    .map_or(node.end_byte(), |left| left.start_byte()),
                "shorthand_property_identifier_pattern" => node.start_byte(),
                _ => node.end_byte(),
            };
            // The site retains its exact syntax range independently of timing.
            effects.fields.insert(span(node), at);
            if node.kind() == "call_expression"
                && !call_is_optional(node)
                && node
                    .child_by_field_name("function")
                    .and_then(strip_transparent_wrappers)
                    .is_some_and(|callee| {
                        callee.kind() == "identifier"
                            && decoded_name(parsed.node_text(&callee)).as_deref() == Some("eval")
                    })
            {
                effects.direct_eval.insert(span(node));
            }
        }
        let mut cursor = node.walk();
        stack.extend(
            node.named_children(&mut cursor)
                .map(|child| (child, exposed)),
        );
    }
    // A later opaque operation can receive a stored callable through an
    // earlier default alias. Only propagate from aliases the operation sees.
    for (binding, operand) in effects.aliases.iter().rev() {
        if decoded_name(parsed.node_text(binding))
            .is_some_and(|name| escaped.contains(name.as_ref()))
        {
            if let Some(name) = decoded_name(parsed.node_text(operand)) {
                if execution::binding_scope(parsed, owner, *operand, &name) == Some(owner.id()) {
                    escaped.insert(name.into_owned());
                }
            }
        }
    }
    effects
        .escaped_writers
        .extend(stored.into_iter().filter_map(|(binding, value)| {
            decoded_name(parsed.node_text(&binding))
                .filter(|name| escaped.contains(name.as_ref()))?;
            Some(span(value))
        }));
    effects
}

/// `eval?.(..)` and `(eval)?.(..)` are indirect calls, never direct eval.
fn call_is_optional(call: Node<'_>) -> bool {
    let mut cursor = call.walk();
    let optional = call
        .children(&mut cursor)
        .any(|child| matches!(child.kind(), "optional_chain" | "?."));
    optional
}

fn possible_aliases(value: Node<'_>) -> Vec<Node<'_>> {
    let mut result = Vec::new();
    let mut stack = vec![value];
    while let Some(node) = stack.pop().and_then(strip_transparent_wrappers) {
        match node.kind() {
            "identifier" => result.push(node),
            "sequence_expression" => {
                let mut cursor = node.walk();
                stack.extend(
                    node.named_children(&mut cursor)
                        .filter(|child| !child.is_extra())
                        .last(),
                );
            }
            "ternary_expression" => {
                stack.extend(node.child_by_field_name("consequence"));
                stack.extend(node.child_by_field_name("alternative"));
            }
            "assignment_expression" => stack.extend(node.child_by_field_name("right")),
            "binary_expression"
                if node
                    .child_by_field_name("operator")
                    .is_some_and(|operator| matches!(operator.kind(), "&&" | "||" | "??")) =>
            {
                stack.extend(node.child_by_field_name("left"));
                stack.extend(node.child_by_field_name("right"));
            }
            _ => {}
        }
    }
    result
}

fn span(node: Node<'_>) -> Span {
    [node.start_byte(), node.end_byte()]
}
