//! Source authority comes only from included NEW sinks, in producer emission order.
use super::super::attribution::EmissionAttribution;
use super::super::report::keep_best_evidence;
use super::describe::allowed;
use prism::finding_confidence::EvidencePath;
use std::collections::{BTreeMap, BTreeSet};

type Source = (String, usize);
#[derive(Default)]
pub(super) struct Relations {
    pub(super) evidence: BTreeMap<Source, EvidencePath>,
    pub(super) backlinks: BTreeMap<Source, Vec<String>>,
}

pub(super) fn relations(
    attribution: &EmissionAttribution,
    included: &BTreeSet<usize>,
    original: &BTreeSet<String>,
    context: &BTreeSet<String>,
) -> Relations {
    let mut relations = Relations::default();
    for sink in &attribution.sinks {
        if !included.contains(&sink.finding_index) {
            continue;
        }
        for license in &sink.licensed {
            let key = (license.file.clone(), license.line);
            keep_best_evidence(&mut relations.evidence, key, license.evidence.clone());
        }
    }
    for sink in attribution
        .sinks
        .iter()
        .filter(|sink| included.contains(&sink.finding_index))
    {
        for license in &sink.licensed {
            if sink.file == license.file || !allowed(&sink.file, original, context) {
                continue;
            }
            let files = relations
                .backlinks
                .entry((license.file.clone(), license.line))
                .or_default();
            if !files.contains(&sink.file) {
                files.push(sink.file.clone());
            }
        }
    }
    relations
}
