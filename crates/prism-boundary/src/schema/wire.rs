//! Swagger 2 and OpenAPI 3.0/3.1 JSON with admitted entry bytes and static pointer dependencies.
//! Dialect-eligible request bodies and explicit success keys with exact-over-range status selection.
//! Exact application/json, effective Swagger media lists, or admitted root-scoped Swagger omission proof.
//! Named property/required edges, homogeneous items, and schema-valued map remainders with typed pointers.
//! Feasible scalar, object, array, allOf, anyOf, and provably disjoint oneOf transitions within 16 edges.
//! Comparable fields use supported dialects and feasible required-value witnesses.
//! Union facts are shared across feasible arms within the per-union budget.
//! Exact JSON numbers and structural token kinds are preserved; repeated members are counted at their declaration boundary.
//! Empty enum declarations and effective type sets contribute to field signatures.
//! Stopped reference expansions retain edge requiredness and tag unreadable shape slots unknown.
//! A feasible edge beyond the inventory depth earns a counted cut after object feasibility is checked.
//! Swagger request omission proof establishes Content-Type selection. Status records use canonical JSON strings.
//! Scoped limits distinguish partial and withheld coverage.

mod composition;
mod constraints;
mod keywords;
mod object;

use self::composition::{target_node, Effect, Stop};
use self::constraints::{ARRAY, NULL, OBJECT};
use super::references::{Document, Documents, Located};
use super::{Dialect, Site};
use crate::element::{FieldId, MessageRoot, PointerToken};
use crate::inventory::{BlockEffect, FieldBlock, WireEvidence};
use crate::inventory::{DefinitionId, Field, FieldShape, Property, RegionLimit};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const MAX_DEPTH: usize = 16;

#[derive(Clone)]
struct Node<'r, 'a> {
    value: &'r Value,
    document: &'r Document<'a>,
    site: Site,
    active: Vec<(String, Vec<String>)>,
    definitions: Vec<DefinitionId>,
    origins: Vec<(usize, usize)>,
    policy: bool,
    ref_ignored: bool,
}

impl<'r, 'a> Node<'r, 'a> {
    fn new(location: Located<'r, 'a>) -> Self {
        Self {
            value: location.value,
            document: location.document,
            site: location.site,
            active: Vec::new(),
            definitions: Vec::new(),
            origins: Vec::new(),
            policy: false,
            ref_ignored: false,
        }
    }

    fn child(&self, value: &'r Value, pointer: &[&str]) -> Self {
        let mut node = self.clone();
        node.value = value;
        node.site
            .pointer
            .extend(pointer.iter().map(|part| (*part).into()));
        node.policy = false;
        node.ref_ignored = false;
        node
    }

    fn get(&self, key: &str) -> Option<&'r Value> {
        if self.ref_ignored && key == "$ref" {
            return None;
        }
        self.value.get(key)
    }

    fn resource_scope_unchecked(&self) -> bool {
        let mut value = &self.document.value;
        for key in &self.site.pointer {
            if value.get("$id").is_some_and(Value::is_string) {
                return true;
            }
            value = if value.is_array() {
                key.parse::<usize>().ok().and_then(|i| value.get(i))
            } else {
                value.get(key)
            }
            .unwrap_or(&Value::Null);
        }
        false
    }

    fn duplicate_members(&self) -> bool {
        self.document.has_duplicates(&self.site.pointer)
            || self.document.ambiguous_path(&self.site.pointer)
            || self.document.duplicate(&[], "jsonSchemaDialect")
            || ["enum", "const", "discriminator"].iter().any(|key| {
                self.get(key).is_some()
                    && self
                        .document
                        .duplicates_under(&self.site.child(key).pointer)
            })
    }
}

fn resource(document: &Document<'_>) -> String {
    document
        .base
        .as_ref()
        .map_or_else(|| document.blob.path.into(), super::servers::Uri::key)
}

fn definition(document: &Document<'_>, pointer: &[String]) -> Option<DefinitionId> {
    let segments = pointer.iter().map(String::as_str).collect::<Vec<_>>();
    let prefix = match segments.as_slice() {
        ["components", "schemas", _, ..] => &segments[..3],
        ["definitions" | "$defs", _, ..] => &segments[..2],
        _ => {
            let index = segments.iter().rposition(|part| *part == "$defs")?;
            segments.get(..index + 2)?
        }
    };
    let mut pointer = String::new();
    for segment in prefix {
        pointer.push('/');
        pointer.push_str(&segment.replace('~', "~0").replace('/', "~1"));
    }
    Some(DefinitionId {
        resource: resource(document),
        pointer,
    })
}

pub(super) fn declared_definitions(document: &Document<'_>) -> Vec<DefinitionId> {
    let mut declared = Vec::new();
    for (container, prefix) in [
        (&document.value["definitions"], vec!["definitions"]),
        (
            &document.value["components"]["schemas"],
            vec!["components", "schemas"],
        ),
        (&document.value["$defs"], vec!["$defs"]),
    ] {
        if let Some(schemas) = container.as_object() {
            for key in schemas.keys() {
                let mut pointer: Vec<_> = prefix.iter().map(|part| (*part).into()).collect();
                pointer.push(key.clone());
                declared.extend(definition(document, &pointer));
            }
        }
    }
    declared
}

pub(super) fn reference_reason(reason: &str) -> &'static str {
    match reason {
        "duplicate_member" => "duplicate_member",
        "recursive_reference" => "recursive",
        "retrieval_base_unchecked" => "retrieval_base_unchecked",
        "reference_blob_ambiguous" => "reference_blob_ambiguous",
        "reference_blob_unchecked" => "reference_blob_unavailable",
        _ => "reference_unresolved",
    }
}

pub(super) struct ResultTree {
    pub fields: Vec<(Field, bool)>,
    pub limits: Vec<RegionLimit>,
    pub evidence: WireEvidence,
    pub warnings: Vec<String>,
    pub definitions: BTreeSet<DefinitionId>,
    pub feasible: Option<bool>,
}

struct Reader<'r, 'a> {
    documents: &'r Documents<'a>,
    dialect: Dialect,
    request: bool,
}

pub(super) fn read<'r, 'a>(
    documents: &'r Documents<'a>,
    dialect: Dialect,
    root: &MessageRoot,
    schema: Located<'r, 'a>,
) -> ResultTree {
    let reader = Reader {
        documents,
        dialect,
        request: matches!(root, MessageRoot::Request(_)),
    };
    let tree = reader.visit(vec![target_node(schema)], Edge::root());
    let mut result = ResultTree {
        fields: Vec::new(),
        limits: Vec::new(),
        evidence: WireEvidence::default(),
        warnings: Vec::new(),
        definitions: BTreeSet::new(),
        feasible: (!tree.has_limits()).then_some(tree.feasible),
    };
    tree.flatten(root, &mut Vec::new(), true, &mut result);
    result
}

#[derive(Clone, Copy)]
struct Edge {
    depth: usize,
    property: bool,
    list_item: bool,
    required: bool,
    overlap: bool,
    union: bool,
}

impl Edge {
    fn root() -> Self {
        Self {
            depth: 0,
            property: false,
            list_item: false,
            required: false,
            overlap: false,
            union: false,
        }
    }

    fn child(self, property: bool, required: bool, overlap: bool) -> Self {
        Self {
            depth: self.depth + 1,
            property,
            list_item: false,
            required,
            overlap,
            union: self.union && property && required,
        }
    }
}

#[derive(Default)]
struct Tree {
    feasible: bool,
    types: u8,
    shape: Option<FieldShape>,
    type_stopped: bool,
    item_type_stopped: bool,
    field_definitions: BTreeSet<DefinitionId>,
    declaration_only: bool,
    uncertain_presence: bool,
    sites: BTreeSet<Site>,
    stops: BTreeSet<Stop>,
    warnings: BTreeSet<&'static str>,
    definitions: BTreeSet<DefinitionId>,
    children: BTreeMap<PointerToken, Tree>,
}

impl Tree {
    fn has_limits(&self) -> bool {
        !self.stops.is_empty() || self.children.values().any(Tree::has_limits)
    }

    fn merge(&mut self, other: Tree) {
        self.merge_tree(other, false);
    }

    fn merge_tree(&mut self, other: Tree, alternative: bool) {
        self.feasible |= other.feasible;
        self.declaration_only |= other.declaration_only;
        self.uncertain_presence |= other.uncertain_presence;
        self.sites.extend(other.sites);
        self.field_definitions.extend(other.field_definitions);
        self.types |= other.types;
        self.type_stopped |= other.type_stopped;
        self.item_type_stopped |= other.item_type_stopped;
        match (&mut self.shape, other.shape) {
            (Some(a), Some(b)) => {
                merge_property(&mut a.field_type, b.field_type);
                merge_property(&mut a.format, b.format);
                merge_property(&mut a.required, b.required);
                merge_property(&mut a.enum_values, b.enum_values);
                merge_property(&mut a.item_type, b.item_type);
            }
            (None, shape) => self.shape = shape,
            (_, None) => {}
        }
        self.stops.extend(other.stops);
        self.warnings.extend(other.warnings);
        self.definitions.extend(other.definitions);
        if alternative {
            for (token, child) in &mut self.children {
                if !other.children.contains_key(token) {
                    child.mark_branch_dependent();
                }
            }
        }
        for (token, mut child) in other.children {
            match self.children.entry(token) {
                std::collections::btree_map::Entry::Occupied(mut entry) => {
                    entry.get_mut().merge_tree(child, alternative);
                }
                std::collections::btree_map::Entry::Vacant(entry) => {
                    if alternative {
                        child.mark_branch_dependent();
                    }
                    entry.insert(child);
                }
            }
        }
    }

    fn mark_branch_dependent(&mut self) {
        if let Some(shape) = &mut self.shape {
            merge_property(&mut shape.field_type, Property::BranchDependent);
            merge_property(&mut shape.format, Property::BranchDependent);
            merge_property(&mut shape.required, Property::BranchDependent);
            merge_property(&mut shape.enum_values, Property::BranchDependent);
            merge_property(&mut shape.item_type, Property::BranchDependent);
        }
        for child in self.children.values_mut() {
            child.mark_branch_dependent();
        }
    }

    fn child(&mut self, token: PointerToken, child: Tree, depth: usize) {
        if depth < MAX_DEPTH {
            self.children.entry(token).or_default().merge(child);
        } else if child.has_limits() {
            self.stops.extend(
                child
                    .stops
                    .into_iter()
                    .map(|Stop(reason, _)| Stop(reason, Effect::Notice)),
            );
        } else if depth == MAX_DEPTH && (child.feasible || child.shape.is_some()) {
            self.stops.insert(Stop("depth_cut", Effect::Notice));
        }
    }

    fn flatten(
        self,
        root: &MessageRoot,
        pointer: &mut Vec<PointerToken>,
        comparable: bool,
        result: &mut ResultTree,
    ) {
        result.definitions.extend(self.definitions.iter().cloned());
        result
            .warnings
            .extend(self.warnings.iter().map(|warning| (*warning).into()));
        let mut reasons = BTreeSet::new();
        for Stop(reason, effect) in &self.stops {
            if reasons.insert(*reason) {
                result.limits.push(RegionLimit {
                    pointer: pointer.clone(),
                    reason: (*reason).into(),
                });
            }
            let effect = match effect {
                Effect::Field => BlockEffect::Field,
                Effect::Children => BlockEffect::Children,
                Effect::Notice => continue,
            };
            result.evidence.blocks.push(FieldBlock {
                pointer: pointer.clone(),
                reason: (*reason).into(),
                effect,
            });
        }
        if let Some(shape) = self.shape {
            if matches!(pointer.last(), Some(PointerToken::Property(_))) {
                if self.item_type_stopped {
                    result.evidence.stopped_item_types.insert(pointer.clone());
                }
                let field = Field {
                    id: FieldId::new(root.clone(), pointer.clone()),
                    shape,
                    definitions: self.field_definitions.into_iter().collect(),
                    sites: self.sites.into_iter().collect(),
                };
                let readable = comparable
                    && !self.uncertain_presence
                    && !self.stops.iter().any(|stop| stop.1 == Effect::Field);
                if readable && self.declaration_only {
                    result.evidence.declaration_only.insert(pointer.clone());
                }
                result
                    .fields
                    .push((field, readable && !self.declaration_only));
            }
        }
        let descend = comparable
            && !self.uncertain_presence
            && !self.stops.iter().any(|stop| stop.1 != Effect::Notice);
        for (token, child) in self.children {
            pointer.push(token);
            child.flatten(root, pointer, descend, result);
            pointer.pop();
        }
    }
}

fn merge_property<T: PartialEq>(left: &mut Property<T>, right: Property<T>) {
    if *left != right {
        *left = if matches!(&*left, Property::Unknown) || matches!(right, Property::Unknown) {
            Property::Unknown
        } else {
            Property::BranchDependent
        };
    }
}

impl<'r, 'a> Reader<'r, 'a> {
    fn visit(&self, nodes: Vec<Node<'r, 'a>>, mut edge: Edge) -> Tree {
        let field_definitions = nodes
            .iter()
            .filter_map(|node| node.definitions.last())
            .cloned()
            .collect();
        let sites = nodes.iter().map(|node| node.site.clone()).collect();
        let expansion = self.expand(nodes);
        let stopped_reference = expansion.stops.iter().any(|Stop(reason, _)| {
            matches!(
                *reason,
                "reference_unresolved"
                    | "reference_blob_unavailable"
                    | "reference_blob_ambiguous"
                    | "retrieval_base_unchecked"
                    | "recursive"
                    | "reference_scope"
            )
        });
        edge.union |= expansion
            .warnings
            .iter()
            .any(|warning| matches!(*warning, "anyOf_union" | "oneOf_union"));
        let mut result = Tree {
            field_definitions,
            sites,
            stops: expansion.stops,
            warnings: expansion.warnings,
            definitions: expansion.definitions,
            ..Tree::default()
        };
        if edge.overlap {
            result.warnings.insert("allof_overlap");
        }
        let observed = result.stops.iter().any(|stop| stop.1 != Effect::Notice);
        let mut alternatives = Tree::default();
        let mut impossible = Tree::default();
        let mut has_alternative = false;
        for branch in expansion.branches {
            let branch = self.branch(&branch, edge, observed);
            if branch.feasible || branch.has_limits() {
                alternatives.merge_tree(branch, has_alternative);
                has_alternative = true;
            } else {
                impossible.merge(branch);
            }
        }
        result.merge(alternatives);
        // Empty-enum declarations can survive an impossible arm as observations.
        impossible
            .children
            .retain(|token, _| !result.children.contains_key(token));
        if result.shape.is_some() {
            impossible.shape = None;
            impossible.type_stopped = false;
            impossible.item_type_stopped = false;
            impossible.declaration_only = false;
        }
        result.merge(impossible);
        if stopped_reference && result.shape.is_some() {
            result.shape = Some(FieldShape {
                required: Property::Known(edge.required),
                ..unknown_shape()
            });
        }
        if edge.required
            && result
                .stops
                .iter()
                .any(|Stop(reason, _)| *reason == "recursive")
            && !result.feasible
        {
            result.stops.insert(Stop("recursive", Effect::Field));
        }
        result
    }

    fn branch(&self, nodes: &[Node<'r, 'a>], edge: Edge, observed: bool) -> Tree {
        let mut tree = Tree::default();
        for node in nodes {
            tree.definitions.extend(node.definitions.iter().cloned());
            tree.stops.extend(keywords::stops(
                node,
                self.dialect,
                edge.property,
                edge.required,
                self.request,
                edge.union,
            ));
        }
        let (mut types, mut field_type) = constraints::types(nodes, self.dialect);
        let multi_type = matches!(&field_type, Property::Known(names) if names.len() > 1);
        let blocked = observed || tree.stops.iter().any(|stop| stop.1 != Effect::Notice);
        tree.type_stopped = edge.list_item && blocked;
        let feasibility = constraints::feasible(nodes, types, self.dialect);
        if !blocked && feasibility.unknown {
            tree.stops
                .insert(Stop("scalar_feasibility_unchecked", Effect::Field));
        }
        let mut item_type = Property::Missing;
        if blocked {
            for node in nodes {
                Self::observe(node.value, edge.depth, &mut tree);
            }
        } else {
            types = feasibility.types;
            if edge.depth <= MAX_DEPTH + 1
                && types & OBJECT != 0
                && !self.object(nodes, edge, &mut tree)
            {
                types &= !OBJECT;
            }
            if edge.depth <= MAX_DEPTH && types & ARRAY != 0 {
                let (feasible, item) = self.array(nodes, edge, &mut tree);
                if !feasible {
                    types &= !ARRAY;
                }
                item_type = item;
            }
        }
        let type_set_unchecked = blocked || feasibility.unknown;
        // Items sign surviving types, or declared types when a stop blocks pruning.
        if multi_type {
            field_type = if types == NULL || !type_set_unchecked || edge.list_item {
                constraints::type_set(types)
            } else {
                Property::BranchDependent
            };
        }
        tree.feasible = !blocked && types & feasibility.witnesses != 0;
        tree.types = types;
        let empty_enum = nodes.iter().any(|node| {
            node.get("enum")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty)
        });
        tree.declaration_only = empty_enum && !tree.feasible;
        if tree.feasible || blocked || empty_enum || tree.has_limits() && !nodes.is_empty() {
            let mut format = None;
            for value in nodes
                .iter()
                .filter_map(|node| node.get("format").and_then(Value::as_str))
            {
                let value = Property::Known(value.to_owned());
                if let Some(existing) = &mut format {
                    merge_property(existing, value);
                } else {
                    format = Some(value);
                }
            }
            tree.shape = Some(FieldShape {
                field_type,
                format: format.unwrap_or(Property::Missing),
                required: Property::Known(edge.required),
                enum_values: feasibility.enum_values,
                item_type,
            });
        }
        tree
    }

    fn observe(schema: &Value, depth: usize, tree: &mut Tree) {
        if depth >= MAX_DEPTH {
            return;
        }
        if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
            for (name, schema) in properties {
                let field = tree
                    .children
                    .entry(PointerToken::property(name))
                    .or_default();
                field.shape.get_or_insert_with(unknown_shape);
                Self::observe(schema, depth + 1, field);
            }
        }
        for (key, token) in [
            ("items", PointerToken::ListItem),
            ("contains", PointerToken::ListItem),
            ("additionalProperties", PointerToken::MapValue),
        ] {
            if let Some(schema) = schema.get(key).filter(|value| {
                value.is_object() && (key != "items" || schema.get("additionalItems").is_none())
            }) {
                Self::observe(schema, depth + 1, tree.children.entry(token).or_default());
            }
        }
        if let Some(values) = schema.get("enum").and_then(Value::as_array) {
            for value in values.iter().filter(|value| value.is_object()) {
                Self::observe_instance(value, depth, tree);
            }
        }
        if schema.get("if").is_some() {
            for key in ["then", "else"] {
                if let Some(schema) = schema.get(key) {
                    Self::observe(schema, depth, tree);
                }
            }
        }
        if let Some(dependencies) = schema.get("dependentSchemas").and_then(Value::as_object) {
            for schema in dependencies.values() {
                Self::observe(schema, depth, tree);
            }
        }
    }

    fn observe_instance(value: &Value, depth: usize, tree: &mut Tree) {
        if depth >= MAX_DEPTH {
            return;
        }
        if let Some(fields) = value.as_object() {
            for (name, value) in fields {
                let field = tree
                    .children
                    .entry(PointerToken::property(name))
                    .or_default();
                field.shape.get_or_insert_with(unknown_shape);
                Self::observe_instance(value, depth + 1, field);
            }
        }
    }
}

fn unknown_shape() -> FieldShape {
    FieldShape {
        field_type: Property::Unknown,
        format: Property::Unknown,
        required: Property::Unknown,
        enum_values: Property::Unknown,
        item_type: Property::Unknown,
    }
}
