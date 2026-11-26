//! Uncached, occurrence-level evidence from the S1 producer.

use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::cpg::FlowConfidence;
use crate::languages::Language;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Binding {
    SameBinding,
    OtherBinding,
    Unresolved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    #[serde(rename = "refused_param_list_decorator")]
    Decorator,
    #[serde(rename = "refused_nested_param_list")]
    NestedList,
    #[serde(rename = "refused_param_list_shape")]
    Shape,
    #[serde(rename = "refused_s1_binding")]
    Binding,
    #[serde(rename = "refused_param_list_order")]
    Order,
    #[serde(rename = "refused_param_list_write_effect")]
    WriteEffect,
    #[serde(rename = "refused_s1_replaced")]
    Replaced,
    #[serde(rename = "refused_s1_value_version")]
    ValueVersion,
    #[serde(rename = "refused_s1_endpoint")]
    Endpoint,
}

const REFUSALS: [Refusal; 9] = [
    Refusal::Decorator,
    Refusal::NestedList,
    Refusal::Shape,
    Refusal::Binding,
    Refusal::Order,
    Refusal::WriteEffect,
    Refusal::Replaced,
    Refusal::ValueVersion,
    Refusal::Endpoint,
];

impl Refusal {
    fn counter(self) -> &'static str {
        match self {
            Self::Decorator => "refused_param_list_decorator",
            Self::NestedList => "refused_nested_param_list",
            Self::Shape => "refused_param_list_shape",
            Self::Binding => "refused_s1_binding",
            Self::Order => "refused_param_list_order",
            Self::WriteEffect => "refused_param_list_write_effect",
            Self::Replaced => "refused_s1_replaced",
            Self::ValueVersion => "refused_s1_value_version",
            Self::Endpoint => "refused_s1_endpoint",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Admission {
    Body,
    Bare,
    Reader,
    OtherBinding,
    Refused(Refusal),
}

impl Admission {
    pub(super) fn admits(self) -> bool {
        matches!(self, Self::Body | Self::Bare | Self::Reader)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Disposition {
    Emitted,
    OtherBinding,
    Refused(Refusal),
    CapWithheld,
}

impl Disposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Emitted => "emitted",
            Self::OtherBinding => "other-binding",
            Self::Refused(reason) => reason.counter(),
            Self::CapWithheld => "cap-withheld",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct AttemptKey {
    pub owner: [usize; 2],
    pub binding: [usize; 2],
    pub operand: [usize; 2],
    pub path: AccessPath,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Endpoint {
    pub bytes: [usize; 2],
    pub path: AccessPath,
    pub label: Option<FlowConfidence>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionCause {
    BareOccurrence,
    Arguments,
    With,
    OtherRootStore,
    SameRootNonComparableStore,
    SignatureEffect,
    S2,
}

impl ProjectionCause {
    const ALL: [Self; 7] = [
        Self::BareOccurrence,
        Self::Arguments,
        Self::With,
        Self::OtherRootStore,
        Self::SameRootNonComparableStore,
        Self::SignatureEffect,
        Self::S2,
    ];

    pub(super) fn bit(self) -> u8 {
        1 << (self as u8)
    }

    pub(super) fn from_bits(bits: u8) -> Vec<Self> {
        Self::ALL
            .into_iter()
            .filter(|cause| bits & cause.bit() != 0)
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Attempt {
    pub key: AttemptKey,
    pub binding: Binding,
    pub admission: Admission,
    pub justifies_def: bool,
    pub endpoint: Option<Endpoint>,
    pub disposition: Disposition,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub projection_causes: Vec<ProjectionCause>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapKind {
    Definitions,
    Pairs,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct CapDefinition {
    pub bytes: [usize; 2],
    pub path: AccessPath,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct CapPair {
    pub source: CapDefinition,
    pub target: CapDefinition,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct CapWithholding {
    pub owner: [usize; 2],
    pub kind: CapKind,
    pub definitions: Vec<CapDefinition>,
    pub pairs: Vec<CapPair>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct FileEvidence {
    pub grammar: Language,
    pub source_digest: String,
    pub attempts: Vec<Attempt>,
    pub cap_withholdings: Vec<CapWithholding>,
    pub signature_write_sites: Vec<([usize; 2], [usize; 2])>,
}

impl FileEvidence {
    pub(super) fn new(parsed: &ParsedFile) -> Self {
        Self {
            grammar: parsed.language,
            source_digest: format!("{:x}", Sha256::digest(parsed.source.as_bytes())),
            attempts: Vec::new(),
            cap_withholdings: Vec::new(),
            signature_write_sites: Vec::new(),
        }
    }

    pub(super) fn finish(&mut self) {
        self.cap_withholdings.sort_by_key(|event| event.owner);
        self.signature_write_sites.sort_unstable();
        self.signature_write_sites.dedup();
        self.attempts.sort_by(|a, b| a.key.cmp(&b.key));
        assert!(self
            .attempts
            .windows(2)
            .all(|pair| pair[0].key != pair[1].key));
    }
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Evidence {
    pub source_revision: String,
    pub files: BTreeMap<String, FileEvidence>,
}

pub const COUNTER_KEYS: &[&str] = &[
    "candidates",
    "emitted",
    "other-binding",
    "cap-withheld",
    "refused_param_list_decorator",
    "refused_nested_param_list",
    "refused_param_list_shape",
    "refused_s1_binding",
    "refused_param_list_order",
    "refused_param_list_write_effect",
    "refused_s1_replaced",
    "refused_s1_value_version",
    "refused_s1_endpoint",
    "unresolved_param_list_write_sites",
    "refused_s1_rd_cap",
    "refused_s1_pair_cap",
];

impl Evidence {
    pub fn counters(&self) -> BTreeMap<&'static str, u64> {
        let mut counts: BTreeMap<_, _> = COUNTER_KEYS.iter().map(|&key| (key, 0)).collect();
        for file in self.files.values() {
            *counts.get_mut("unresolved_param_list_write_sites").unwrap() +=
                file.signature_write_sites.len() as u64;
            counts
                .entry("candidates")
                .and_modify(|count| *count += file.attempts.len() as u64);
            for event in &file.cap_withholdings {
                let key = match event.kind {
                    CapKind::Definitions => "refused_s1_rd_cap",
                    CapKind::Pairs => "refused_s1_pair_cap",
                };
                *counts.get_mut(key).expect("registered cap") += 1;
            }
            for attempt in &file.attempts {
                *counts
                    .get_mut(attempt.disposition.as_str())
                    .expect("registered disposition") += 1;
            }
        }
        let refusals = REFUSALS
            .iter()
            .map(|reason| counts[reason.counter()])
            .sum::<u64>();
        assert_eq!(
            counts["candidates"],
            counts["emitted"] + counts["other-binding"] + refusals + counts["cap-withheld"]
        );
        counts
    }
}
