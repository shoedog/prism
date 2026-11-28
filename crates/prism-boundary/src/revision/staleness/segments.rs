use super::{ElementKey, Ledger, ReceiptKind, ReceiptOrigin, Refusal, SavedScan, SchemaIndex};
use crate::revision::AdmissionReceipt;
use std::collections::BTreeSet;

pub(super) fn check(
    scan: &SavedScan,
    element: &ElementKey<'_>,
    inventory: &SchemaIndex,
    ledger: &Ledger<'_>,
    common: &[&AdmissionReceipt],
) -> Result<(), Refusal> {
    let matches_source = |record: &AdmissionReceipt, path: &str| {
        record.kind == ReceiptKind::Source
            && matches!(&record.origin, ReceiptOrigin::NamedTree { input_id, tree, path: found, .. }
                if input_id == &scan.client_input_id && tree == &scan.client_tree && found == path)
    };
    if scan.segments.is_empty() {
        if scan.expected_segments.is_empty() {
            return Err(Refusal::new("missing_dependency", &scan.scan_id).dimension("segments"));
        }
        for path in &scan.expected_segments {
            if !scan.admitted_segments.contains(path)
                || !common.iter().any(|record| matches_source(record, path))
            {
                return Err(Refusal::new("missing_dependency", path).dimension("segments"));
            }
        }
        return Ok(());
    }

    let expected: BTreeSet<_> = scan.expected_segments.iter().map(String::as_str).collect();
    let recorded: BTreeSet<_> = scan
        .segments
        .iter()
        .map(|segment| segment.path.as_str())
        .collect();
    if expected != recorded || scan.segments.len() != recorded.len() {
        return Err(Refusal::new("slot_mismatch", &scan.scan_id).dimension("segments"));
    }
    let mut relevant = false;
    for segment in &scan.segments {
        if !element.may_be_in_scope(&segment.scope, inventory) {
            continue;
        }
        relevant = true;
        let missing = || Refusal::new("missing_dependency", &segment.path).dimension("segments");
        let id = segment
            .source_receipt_id
            .as_deref()
            .filter(|id| scan.dependencies.iter().any(|dependency| dependency == id))
            .ok_or_else(missing)?;
        let (records, reasons) = ledger.closure([(id, "missing_dependency")]);
        if let Some(reason) = reasons.into_iter().next() {
            return Err(reason);
        }
        if !records.iter().any(|record| {
            record.id == id
                && matches_source(record, &segment.path)
                && record
                    .scope
                    .as_ref()
                    .is_some_and(|scope| scope.same_region(&segment.scope))
        }) {
            return Err(missing());
        }
    }
    if !relevant {
        return Err(Refusal::new("missing_dependency", &scan.scan_id).dimension("segments"));
    }
    Ok(())
}
