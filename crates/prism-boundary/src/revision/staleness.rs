mod inventory;
mod segments;
use self::inventory::{ElementFacts, ElementKey};
use super::{
    admission::{Ledger, Projection},
    AdmissionRequest, Availability, ReceiptKind, ReceiptOrigin, Refusal, Role, Scope,
};
use crate::model::StandingMismatch;
use crate::SchemaIndex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(crate) fn normalize_saved_shape(element_id: &str, signature: &Value) -> Option<Value> {
    ElementKey::parse(element_id)?.normalized_shape(signature)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SavedScan {
    pub scan_id: String,
    pub client_input_id: String,
    pub client_tree: String,
    pub api: String,
    pub authority: String,
    pub source_tree: String,
    pub source_hash: String,
    pub config_era: String,
    pub schema_inventory_id: String,
    pub expected_segments: Vec<String>,
    pub admitted_segments: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub segments: Vec<SavedSegment>,
    pub dependencies: Vec<String>,
    pub matches: Vec<SavedLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SavedSegment {
    pub path: String,
    pub scope: Scope,
    pub source_receipt_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SavedLocation {
    pub location_id: String,
    pub element_id: String,
    pub grade: String,
    pub signature: Value,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceSide {
    Before,
    After,
    Held,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SavedQuery {
    pub element_id: String,
    pub selected_input_id: String,
    pub side: ServiceSide,
    pub saved_location: Option<String>,
}

pub struct SavedScanEvidence<'a> {
    pub admission: &'a AdmissionRequest,
    pub bytes: &'a BTreeMap<String, Vec<u8>>,
    pub scan: Option<&'a SavedScan>,
    pub inventory: Option<&'a SchemaIndex>,
    pub selected: &'a SchemaIndex,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BindingStatus {
    pub scan_id: Option<String>,
    pub element_id: String,
    pub selected_input_id: String,
    pub side: ServiceSide,
    pub saved_location: Option<String>,
    pub status: String,
    pub reason: Option<String>,
    pub selected_presence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<Refusal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_shape: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_shape: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_grade: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct SavedStatusReport {
    pub binding_status: Vec<BindingStatus>,
    pub standing_mismatches: Vec<StandingMismatch>,
    pub admitted_raw_inputs: Vec<String>,
    pub admitted_saved_locations: Vec<String>,
}

pub fn saved_binding_status(
    query: &SavedQuery,
    evidence: &SavedScanEvidence<'_>,
) -> SavedStatusReport {
    let ledger = Ledger::new(evidence.admission, evidence.bytes);
    let (admission, projections) = ledger.resolve();
    let mut result = SavedStatusReport::default();
    for claim in &evidence.admission.claims {
        if admission
            .claims
            .get(&claim.id)
            .is_some_and(|decision| decision.status == "admitted")
            && ledger
                .input(&claim.input_id)
                .is_ok_and(|input| input.binding.role == Role::Consumer)
            && !result.admitted_raw_inputs.contains(&claim.input_id)
        {
            result.admitted_raw_inputs.push(claim.input_id.clone());
        }
    }
    let key = ElementKey::parse(&query.element_id);
    let verified = find_selected(query, evidence, &ledger, &projections);
    let selected_input = verified.as_ref().map(|(id, _)| *id);
    let selected = key
        .as_ref()
        .zip(verified.as_ref())
        .map(|(key, (_, index))| ElementFacts::from_index(key, index));
    let mut status = BindingStatus {
        scan_id: evidence.scan.map(|scan| scan.scan_id.clone()),
        element_id: query.element_id.clone(),
        selected_input_id: selected_input.unwrap_or(&query.selected_input_id).into(),
        side: query.side,
        saved_location: query.saved_location.clone(),
        status: "not_checked".into(),
        reason: None,
        selected_presence: selected
            .as_ref()
            .map_or("unknown", |facts| facts.presence)
            .into(),
        refusal: None,
        saved_shape: None,
        selected_shape: None,
        saved_grade: None,
    };
    let outcome = evaluate(
        query,
        evidence,
        &ledger,
        key.as_ref(),
        selected.as_ref(),
        &mut status,
        &mut result,
    );
    match outcome {
        Ok(state) => status.status = state.into(),
        Err(failure) => {
            status.reason = Some(failure.code.clone());
            status.refusal = Some(failure);
        }
    }
    result.binding_status.push(status);
    result
}

fn find_selected<'a>(
    query: &SavedQuery,
    evidence: &SavedScanEvidence<'a>,
    ledger: &Ledger<'a>,
    projections: &BTreeMap<&str, Projection<'a>>,
) -> Option<(&'a str, SchemaIndex)> {
    let source = evidence.selected.source.as_ref()?;
    let key = ElementKey::parse(&query.element_id)?;
    let mut matches = BTreeMap::new();
    for claim in &ledger.request.claims {
        let Some(projection) = projections.get(claim.id.as_str()) else {
            continue;
        };
        let scope = &projection.scope;
        let Ok(input) = ledger.input(&claim.input_id) else {
            continue;
        };
        let slot = ledger
            .request
            .slots
            .iter()
            .find(|slot| Some(slot.slot.as_str()) == claim.slot.as_deref());
        if slot.and_then(|slot| slot.service_side) != Some(query.side)
            || input.binding.role != Role::Producer
            || input.binding.api != source.api
            || input.binding.authority != source.authority
            || input.snapshot.identity() != Some((&source.tree, source.commit.as_deref()))
            || !scope.same_region(&evidence.selected.scope)
        {
            continue;
        }
        let Ok(index) = projection.index() else {
            continue;
        };
        if index.source.as_ref() == Some(source)
            && key.in_scope(Some(scope), &index)
            && index.byte_sha256 == evidence.selected.byte_sha256
            && index.input_closure == evidence.selected.input_closure
            && index.content_hash == evidence.selected.content_hash
            && index.info_version == evidence.selected.info_version
        {
            matches.insert(claim.input_id.as_str(), index);
        }
    }
    (matches.len() == 1)
        .then(|| matches.into_iter().next())
        .flatten()
}

fn evaluate(
    query: &SavedQuery,
    evidence: &SavedScanEvidence<'_>,
    ledger: &Ledger<'_>,
    element: Option<&ElementKey>,
    selected: Option<&ElementFacts>,
    status: &mut BindingStatus,
    result: &mut SavedStatusReport,
) -> Result<&'static str, Refusal> {
    let scan = evidence
        .scan
        .ok_or_else(|| Refusal::new("missing_saved_scan", &query.element_id))?;
    let fail = |code| Refusal::new(code, &scan.scan_id);
    let element = element.ok_or_else(|| fail("unsupported_kind"))?;
    let inventory = evidence
        .inventory
        .ok_or_else(|| fail("missing_inventory"))?;
    let source = inventory
        .source
        .as_ref()
        .ok_or_else(|| fail("missing_inventory"))?;
    let client = ledger.input(&scan.client_input_id)?;
    for (dimension, matches) in [
        (
            "client_tree",
            client.binding.role == Role::Consumer && client.binding.tree == scan.client_tree,
        ),
        (
            "api",
            client.binding.api == scan.api && source.api == scan.api,
        ),
        ("authority", source.authority == scan.authority),
        ("source_tree", source.tree == scan.source_tree),
        ("source_hash", inventory.content_hash == scan.source_hash),
        (
            "schema_inventory_id",
            scan.dependencies.contains(&scan.schema_inventory_id),
        ),
    ] {
        if !matches {
            return Err(fail("slot_mismatch").dimension(dimension));
        }
    }
    let (records, refusals) = ledger.closure(
        scan.dependencies
            .iter()
            .filter(|id| {
                !scan
                    .segments
                    .iter()
                    .any(|segment| segment.source_receipt_id.as_ref() == Some(id))
            })
            .map(|id| (id.as_str(), "missing_dependency")),
    );
    if let Some(refused) = refusals.into_iter().next() {
        return Err(refused);
    }
    let inventory_record = records
        .iter()
        .find(|record| {
            record.id == scan.schema_inventory_id && record.kind == ReceiptKind::Inventory
        })
        .ok_or_else(|| fail("missing_inventory"))?;
    let (inventory_owner, inventory_site_ok) = match &inventory_record.origin {
        ReceiptOrigin::NamedTree {
            input_id,
            tree,
            path,
            blob,
            availability: Availability::Available,
            ..
        } => (
            ledger.input(input_id)?,
            *tree == source.tree && *path == source.path && *blob == source.blob,
        ),
        _ => return Err(fail("missing_inventory")),
    };
    let bytes = &ledger.bytes[&scan.schema_inventory_id];
    let inventory_hash = format!("{:x}", Sha256::digest(bytes));
    let (inventory_dependencies, _) = ledger.closure(
        inventory_record
            .dependencies
            .iter()
            .map(|id| (id.as_str(), "missing_dependency")),
    );
    let inventory_blob_bound = inventory_record
        .digest
        .as_ref()
        .is_some_and(|digest| digest.value == inventory_hash)
        || inventory_dependencies.iter().any(|record| {
            record.kind == ReceiptKind::Blob
                && record.origin == inventory_record.origin
                && record
                    .digest
                    .as_ref()
                    .is_some_and(|digest| digest.value == inventory_hash)
        });
    if !inventory_site_ok
        || !inventory_blob_bound
        || inventory_hash != inventory.byte_sha256
        || inventory_owner.snapshot.identity() != Some((&source.tree, source.commit.as_deref()))
    {
        return Err(fail("slot_mismatch").dimension("schema_inventory_id"));
    }
    if scan.segments.is_empty()
        && !records.iter().any(|record| {
            record.kind == ReceiptKind::Source
                && matches!(&record.origin, ReceiptOrigin::NamedTree { input_id, tree, .. }
            if *input_id == scan.client_input_id && *tree == scan.client_tree)
        })
    {
        return Err(fail("slot_mismatch").dimension("client_tree"));
    }
    let config = client
        .binding
        .config
        .as_ref()
        .or(inventory_owner.binding.config.as_ref());
    if !config.is_some_and(|config| {
        config.era == scan.config_era
            && records.iter().any(|record| {
                record.kind == ReceiptKind::Config
                    && matches!(&record.origin, ReceiptOrigin::NamedTree { tree, path, blob, .. }
            if *tree == config.tree && *path == config.path && *blob == config.blob)
            })
    }) {
        return Err(fail("slot_mismatch").dimension("config_era"));
    }
    if !element.in_scope(scan.scope.as_ref(), inventory)
        || !element.in_scope(Some(&inventory_owner.binding.scope), inventory)
        || !element.in_scope(Some(&inventory.scope), inventory)
    {
        return Err(fail("scope_unchecked"));
    }

    let selected = selected
        .filter(|_| status.selected_input_id == query.selected_input_id)
        .ok_or_else(|| fail("slot_mismatch").dimension("selected_input_id"))?;
    if inventory.content_hash != evidence.selected.content_hash {
        result
            .standing_mismatches
            .push(StandingMismatch::SchemaMismatch {
                saved_content_hash: inventory.content_hash.clone(),
                selected_content_hash: evidence.selected.content_hash.clone(),
                saved_info_version: inventory.info_version.clone(),
                selected_info_version: evidence.selected.info_version.clone(),
            });
    }
    let saved = ElementFacts::from_index(element, inventory);
    let mut matches = scan.matches.iter().filter(|location| {
        query
            .saved_location
            .as_ref()
            .map_or(location.element_id == query.element_id, |id| {
                location.location_id == *id
            })
    });
    let location = matches.next();
    if matches.any(|other| Some(other) != location) {
        return Err(Refusal::new(
            "receipt_conflict",
            location.map_or(&scan.scan_id, |l| &l.location_id),
        ));
    }
    segments::check(scan, element, inventory, ledger, &records)?;
    let Some(location) = location else {
        if let Some(id) = &query.saved_location {
            return Err(Refusal::new("missing_dependency", id));
        }
        return match saved.presence {
            "present" => Ok("no_saved_match"),
            "absent" => Ok("element_absent_from_scan_schema"),
            _ => Err(fail("membership_unchecked")),
        };
    };
    status.saved_location = Some(location.location_id.clone());
    if location.element_id != query.element_id {
        return Err(Refusal::new("slot_mismatch", &location.location_id).dimension("element_id"));
    }
    status.saved_grade = Some(location.grade.clone());
    if location.grade != "bound" {
        return Err(Refusal::new(
            if location.grade == "candidate" {
                "saved_grade_unchecked"
            } else {
                "unsupported_kind"
            },
            &location.location_id,
        ));
    }
    let saved_shape = saved
        .shape
        .filter(|_| saved.comparable)
        .ok_or_else(|| fail("shape_incomparable"))?;
    let recorded = element.normalized_shape(&location.signature);
    if recorded.as_ref() != Some(&saved_shape) {
        return Err(Refusal::new(
            "saved_signature_mismatch",
            &location.location_id,
        ));
    }
    result
        .admitted_saved_locations
        .push(location.location_id.clone());
    let selected_shape = selected
        .shape
        .as_ref()
        .filter(|_| selected.comparable)
        .ok_or_else(|| fail("shape_incomparable"))?;
    status.saved_shape = Some(saved_shape.clone());
    status.selected_shape = Some(selected_shape.clone());
    Ok(if &saved_shape == selected_shape {
        "binding_current"
    } else {
        "binding_stale"
    })
}
