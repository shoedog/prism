use crate::element::{schema_path, MessageRoot, PointerToken};
use crate::inventory::{success_key, BlockEffect};
use crate::revision::Scope;
use crate::{signature, Coverage, Field, Operation, RootKey, SchemaIndex};
use serde_json::{json, Value};

pub(super) struct ElementKey<'a> {
    id: &'a str,
    operation: String,
    method: &'a str,
    path: &'a str,
    field: Option<(RootKey, Vec<PointerToken>)>,
}

pub(super) struct ElementFacts {
    pub presence: &'static str,
    pub shape: Option<Value>,
    pub comparable: bool,
}

impl<'a> ElementKey<'a> {
    pub fn parse(id: &'a str) -> Option<Self> {
        let (method_path, field) = if let Some(op) = id.strip_prefix("op:") {
            (op, None)
        } else {
            let (root, pointer) = id.split_once("#/")?;
            let (method_path, root) = if let Some(request) = root.strip_prefix("field:req:") {
                (request, RootKey::Request)
            } else {
                let (response, key) = root.strip_prefix("field:resp:")?.rsplit_once(':')?;
                (response, RootKey::Response(success_key(key, true)?))
            };
            let pointer = pointer
                .split('/')
                .map(PointerToken::decode)
                .collect::<Result<_, _>>()
                .ok()?;
            (method_path, Some((root, pointer)))
        };
        let (method, path) = method_path.split_once(' ')?;
        if method.is_empty() || !path.starts_with('/') {
            return None;
        }
        Some(Self {
            id,
            operation: format!("op:{method_path}"),
            method,
            path,
            field,
        })
    }

    pub fn in_scope(&self, scope: Option<&Scope>, index: &SchemaIndex) -> bool {
        let Some(scope @ Scope::PathPrefix { .. }) = scope else {
            return true;
        };
        if let Some(operation) = index
            .operations
            .iter()
            .find(|op| op.id.to_string() == self.operation)
        {
            return !operation.declarations.is_empty()
                && operation
                    .declarations
                    .iter()
                    .all(|site| scope.contains_prefix(&site.canonical_path.segments));
        }
        !self.path.contains('{') && scope.contains_prefix(&schema_path(self.path, "", &[]).segments)
    }

    pub fn may_be_in_scope(&self, scope: &Scope, index: &SchemaIndex) -> bool {
        if let Some(operation) = index
            .operations
            .iter()
            .find(|op| op.id.to_string() == self.operation)
        {
            operation.declarations.is_empty()
                || operation
                    .declarations
                    .iter()
                    .any(|site| scope.contains_prefix(&site.canonical_path.segments))
        } else {
            self.path.contains('{') || self.in_scope(Some(scope), index)
        }
    }

    pub fn normalized_shape(&self, shape: &Value) -> Option<Value> {
        if self.field.is_some() {
            return normalize_signature(shape);
        }
        let mut records = std::collections::BTreeMap::new();
        for row in shape.as_array()? {
            let [id, signature] = row.as_array()?.as_slice() else {
                return None;
            };
            let id = id.as_str()?;
            let key = ElementKey::parse(id)?;
            if key.field.is_none()
                || key.operation != self.operation
                || records
                    .insert(id, normalize_signature(signature)?)
                    .is_some()
            {
                return None;
            }
        }
        Some(json!(records
            .into_iter()
            .map(|(id, signature)| json!([id, signature]))
            .collect::<Vec<_>>()))
    }
}

fn normalize_signature(value: &Value) -> Option<Value> {
    let mut value = value.clone();
    let slots = value.as_array_mut()?;
    if slots.len() != 5 {
        return None;
    }
    if let Some(enums) = slots[3].get_mut(1).and_then(Value::as_array_mut) {
        for item in enums.iter_mut() {
            if let Value::Number(number) = item {
                let spelling = crate::schema::signature::canonical_number(&number.to_string());
                *number = spelling.parse().ok()?;
            }
        }
        enums.sort_by_cached_key(Value::to_string);
    }
    serde_json::from_value::<crate::model::WireSignature>(value.clone()).ok()?;
    Some(value)
}

impl ElementFacts {
    pub fn from_index(key: &ElementKey<'_>, index: &SchemaIndex) -> Self {
        let operation = index
            .operations
            .iter()
            .find(|op| op.id.to_string() == key.operation);
        let gap = index.operation_gaps.iter().any(|gap| {
            gap.method
                .as_deref()
                .is_none_or(|method| method == key.method)
                && gap
                    .paths
                    .as_ref()
                    .is_none_or(|paths| paths.iter().any(|path| path.to_string() == key.path))
        }) || index.operation_coverage_incomplete && index.operation_gaps.is_empty();
        let Some(op) = operation else {
            return Self::without_shape(if gap { "unknown" } else { "absent" });
        };

        if let Some((root, pointer)) = &key.field {
            let known = index
                .fields
                .iter()
                .find(|field| field.id.to_string() == key.id);
            let observed = index
                .observed_fields
                .iter()
                .find(|field| field.id.to_string() == key.id);
            let wire = op.wire_evidence.get(root);
            let message = op.messages.iter().find(|message| &message.key == root);
            if let Some(field) = known.or(observed) {
                match crate::diff::field_detail_presence(op, root, field, known.is_some()) {
                    crate::Presence::Absent => return Self::without_shape("absent"),
                    crate::Presence::Unknown => return Self::without_shape("unknown"),
                    crate::Presence::Present => {}
                }
                let shape = field_signature(field);
                let comparable = !gap
                    && op.declarations.len() == 1
                    && known.is_some()
                    && shape_comparable(&shape)
                    && !wire.is_some_and(|wire| {
                        wire.slot_limits
                            .iter()
                            .any(|limit| &limit.pointer == pointer)
                    })
                    && !message.is_some_and(|message| {
                        message.limits.iter().any(|limit| &limit.pointer == pointer)
                    });
                return Self {
                    presence: "present",
                    shape: Some(shape),
                    comparable,
                };
            }
            let unreadable = gap
                || op.declarations.len() != 1
                || message.is_some_and(|message| {
                    matches!(
                        message.coverage,
                        Coverage::Withheld | Coverage::ObservedOnly
                    )
                })
                || wire.is_some_and(|wire| {
                    wire.empty_projection_with_unproved_media
                        || wire.blocks.iter().any(|block| {
                            pointer.starts_with(&block.pointer)
                                && (block.effect == BlockEffect::Field
                                    || pointer.len() > block.pointer.len())
                        })
                })
                || match root {
                    RootKey::Request => !op.request_eligible && op.request_authored_but_excluded,
                    RootKey::Response(_) => op.declarations.iter().any(|site| {
                        matches!(site.status_inventory, crate::StatusInventory::Withheld)
                    }),
                    RootKey::Fallback => true,
                };
            return Self::without_shape(if unreadable { "unknown" } else { "absent" });
        }

        let fields = index.fields.iter().filter(|field| match &field.id.root {
            MessageRoot::Request(owner) | MessageRoot::Response(owner, _) => owner == &op.id,
        });
        let mut rows: Vec<_> = fields
            .map(|field| json!([field.id.to_string(), field_signature(field)]))
            .collect();
        rows.sort_by(|a, b| a[0].as_str().cmp(&b[0].as_str()));
        let comparable =
            !gap && operation_shape_covered(op) && rows.iter().all(|row| shape_comparable(&row[1]));
        Self {
            presence: "present",
            shape: Some(json!(rows)),
            comparable,
        }
    }

    fn without_shape(presence: &'static str) -> Self {
        Self {
            presence,
            shape: None,
            comparable: false,
        }
    }
}

fn operation_shape_covered(op: &Operation) -> bool {
    op.declarations.len() == 1
        && !op.request_authored_but_excluded
        && op
            .messages
            .iter()
            .filter(|message| message.key != RootKey::Fallback)
            .all(|message| message.coverage == Coverage::Checked && message.limits.is_empty())
        && op
            .wire_evidence
            .values()
            .all(|wire| wire.slot_limits.is_empty())
}

fn field_signature(field: &Field) -> Value {
    serde_json::from_str(&signature(&field.shape)).expect("reader signature is JSON")
}

fn shape_comparable(shape: &Value) -> bool {
    shape.as_array().is_some_and(|slots| {
        slots
            .iter()
            .all(|slot| matches!(slot[0].as_str(), Some("known" | "missing")))
    })
}
