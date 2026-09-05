//! Single source for the confidence/tier label attached to a finding by the
//! SARIF and targets serializers.
//! Phase 0 rule (roadmap 04 §3.4): AST-only algorithms are Exact by
//! construction; any CPG-derived finding is Unlabeled because DataFlow edges
//! carry no confidence yet. Item 2 (reaching-definitions labeling) adds
//! `classify_with_evidence(algorithm, quality, evidence: &EvidencePath)`
//! taking a min over the path; `classify` stays as the evidence-free entry.
//!
//! `Asserted` is a claim about the EVIDENCE PATH (no unlabeled or
//! name-only edge, clean parse of every evidence-bearing file), never about
//! the truth of the heuristic the algorithm encodes. A required CI check may
//! gate on `asserted`; whether an asserted finding is a real defect is the
//! reviewer's call.

use crate::ast::ParsedFile;
use crate::cpg::FlowConfidence;
use crate::cpg::{Relation, Trace};
use crate::data_flow::VarLocation;
use crate::resolution::{ResolutionConfidence, ResolvedCallEdge};
use crate::slice::{FileParseQuality, SliceFinding, SlicingAlgorithm};
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Categories `contract_slice::slice_delta` emits for findings computed against
/// an `--old-repo` tree; the only categories that depend on old-tree files.
const OLD_TREE_CONTRACT_CATEGORIES: [&str; 4] = [
    "contract_precondition_weakened",
    "contract_precondition_strengthened",
    "contract_postcondition_weakened",
    "contract_postcondition_strengthened",
];

/// How precisely a finding's evidence path is known. Mirrors
/// `ResolutionConfidence` plus `Unlabeled` for evidence Phase 0 cannot yet grade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum FindingConfidence {
    Exact,
    NameOnly,
    Unlabeled,
}

/// Whether a finding is safe to gate CI on (`Asserted`) or needs a reviewer's
/// judgment call (`Candidate`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum FindingTier {
    Asserted,
    /// The `Default`, so a `TargetsMeta::default()` filter admits both tiers —
    /// the same floor as the CLI's `--min-tier candidate`. Defaulting to
    /// `Asserted` would silently DROP candidate findings from a document built
    /// by an embedder that never set the field.
    #[default]
    Candidate,
}

/// Minimum confidence admitted by finding-bearing output formats.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
#[non_exhaustive]
pub enum MinConfidence {
    /// Only findings whose evidence path is entirely Exact.
    Exact,
    /// Exact, NameOnly AND Unlabeled. An unlabeled finding is not below
    /// nameonly; it is ungraded, and the default must retain legacy findings.
    #[default]
    #[value(name = "nameonly")]
    NameOnly,
}

impl MinConfidence {
    pub fn admits(self, confidence: FindingConfidence) -> bool {
        match self {
            Self::Exact => confidence == FindingConfidence::Exact,
            Self::NameOnly => true,
        }
    }
}

impl std::fmt::Display for MinConfidence {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Exact => "exact",
            Self::NameOnly => "nameonly",
        })
    }
}

/// Finding-confidence projection selected at runtime by `--resolution`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
#[non_exhaustive]
pub enum ResolutionMode {
    /// Report every CPG-derived finding as unlabeled/candidate. The
    /// conservative default and the byte-control mode.
    #[default]
    Nominal,
    /// Report the retained evidence-path labels.
    Scoped,
}

impl ResolutionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Nominal => "nominal",
            Self::Scoped => "scoped",
        }
    }
}

impl std::fmt::Display for ResolutionMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Per-file parse quality grade, ordered best → worst by declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ParseQuality {
    Clean,
    Degraded,
    Poor,
    Unparseable,
    Unknown,
}

impl ParseQuality {
    /// Maps a raw quality string (`"clean"|"degraded"|"poor"|"unparseable"`) to
    /// its variant; anything else (including an unrecognized grade) is `Unknown`.
    pub fn from_quality_str(q: &str) -> Self {
        match q {
            "clean" => Self::Clean,
            "degraded" => Self::Degraded,
            "poor" => Self::Poor,
            "unparseable" => Self::Unparseable,
            _ => Self::Unknown,
        }
    }

    /// From the AUTHORITATIVE per-file map produced by `algorithms::check_parse_quality`
    /// (`ReviewInputs.parse_quality`). That map is SPARSE: `check_parse_quality` inserts a
    /// file only when its error rate exceeds 1% (`src/algorithms/mod.rs:75-84`), so absence
    /// means "clean" for a file prism parsed and "unknown" for a file it did not. Hence: for
    /// each file — in `map` → its grade; else in `parsed` → Clean; else Unknown. Result = the
    /// worst over `files`; Unknown when `files` is empty. The sparse map itself is never
    /// modified (legacy `json`/`review` output serialises it byte-for-byte).
    pub fn min_over(
        files: &[&str],
        map: &BTreeMap<String, FileParseQuality>,
        parsed: &BTreeMap<String, ParsedFile>,
    ) -> Self {
        if files.is_empty() {
            return Self::Unknown;
        }
        let mut worst = Self::Clean;
        for file in files {
            let quality = match map.get(*file) {
                Some(fpq) => Self::from_quality_str(&fpq.quality),
                None if parsed.contains_key(*file) => Self::Clean,
                None => Self::Unknown,
            };
            worst = worst.max(quality);
        }
        worst
    }

    /// Lowercase name, matching the serde spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Degraded => "degraded",
            Self::Poor => "poor",
            Self::Unparseable => "unparseable",
            Self::Unknown => "unknown",
        }
    }
}

/// Files whose parse quality bears on a finding's evidence: the anchor file plus
/// every `related_files` entry (symmetry's counterpart, callback registrations,
/// primitive's callee file, provenance's origin file), deduplicated with order
/// preserved.
pub fn evidence_files(finding: &SliceFinding) -> Vec<&str> {
    let mut files: Vec<&str> = vec![finding.file.as_str()];
    for related in &finding.related_files {
        let related = related.as_str();
        if !files.contains(&related) {
            files.push(related);
        }
    }
    files
}

/// Files whose parse quality bears on the selected witness. Legacy finding
/// locations stay unchanged; DataFlow endpoint files and call-edge caller files
/// are grading inputs only.
pub fn selected_evidence_files<'a>(
    finding: &'a SliceFinding,
    evidence: &'a EvidencePath,
) -> Vec<&'a str> {
    let mut files = evidence_files(finding);
    for hop in &evidence.hops {
        match hop {
            EvidenceHop::DataFlow { from, to, .. } => {
                for file in [from.file.as_str(), to.file.as_str()] {
                    if !files.contains(&file) {
                        files.push(file);
                    }
                }
            }
            EvidenceHop::Call { edge, .. } => {
                let file = edge.caller.file.as_str();
                if !files.contains(&file) {
                    files.push(file);
                }
            }
        }
    }
    files
}

/// The one entry point both serializers use. Encodes the evidence rules: contract findings
/// computed against an `--old-repo` tree (categories `contract_precondition_weakened`,
/// `contract_precondition_strengthened`, `contract_postcondition_weakened`,
/// `contract_postcondition_strengthened` — the only categories `slice_delta` emits) depend on
/// old-tree files prism parsed separately and never graded → `Unknown`; everything else →
/// `min_over(&evidence_files(f), map, parsed)`.
pub fn parse_quality_for(
    finding: &SliceFinding,
    map: &BTreeMap<String, FileParseQuality>,
    parsed: &BTreeMap<String, ParsedFile>,
) -> ParseQuality {
    match finding.category.as_deref() {
        Some(category) if OLD_TREE_CONTRACT_CATEGORIES.contains(&category) => ParseQuality::Unknown,
        _ => ParseQuality::min_over(&evidence_files(finding), map, parsed),
    }
}

/// Grade the authoritative selected witness without changing the finding's
/// legacy anchor or related-file projection. Missing evidence retains the
/// evidence-free parse-quality behavior; its confidence is handled separately.
pub fn parse_quality_for_selected_evidence(
    finding: &SliceFinding,
    evidence: Option<&EvidencePath>,
    map: &BTreeMap<String, FileParseQuality>,
    parsed: &BTreeMap<String, ParsedFile>,
) -> ParseQuality {
    match finding.category.as_deref() {
        Some(category) if OLD_TREE_CONTRACT_CATEGORIES.contains(&category) => ParseQuality::Unknown,
        _ => match evidence {
            Some(evidence) => {
                ParseQuality::min_over(&selected_evidence_files(finding, evidence), map, parsed)
            }
            None => ParseQuality::min_over(&evidence_files(finding), map, parsed),
        },
    }
}

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct EvidencePath {
    pub hops: Vec<EvidenceHop>,
    pub crossed_unlabeled: bool,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum EvidenceHop {
    DataFlow {
        from: VarLocation,
        to: VarLocation,
        confidence: FlowConfidence,
    },
    Call {
        edge: ResolvedCallEdge,
        confidence: ResolutionConfidence,
    },
}

impl EvidencePath {
    /// The evidence-free artifact represented by [`classify`]. AST-only
    /// algorithms have a valid empty path; CPG and unknown algorithms do not.
    pub fn unlabeled_for(algorithm: &str) -> Self {
        let crossed_unlabeled =
            SlicingAlgorithm::from_str(algorithm).is_none_or(|algorithm| algorithm.needs_cpg());
        Self {
            hops: Vec::new(),
            crossed_unlabeled,
        }
    }

    /// Recover the selected root-to-sink witness from a trace's parent tree.
    pub fn from_trace(trace: &Trace, root: NodeIndex, sink: NodeIndex) -> Self {
        let mut evidence = Self::default();
        let mut current = sink;
        let mut seen = std::collections::BTreeSet::new();
        while current != root {
            if !seen.insert(current) {
                evidence.crossed_unlabeled = true;
                break;
            }
            let Some(&(parent, relation)) = trace.parents_by_root.get(&(root, current)) else {
                evidence.crossed_unlabeled = true;
                break;
            };
            match relation {
                Relation::DataFlow => match trace.data_flow_hops.get(&(parent, current)) {
                    Some(hop) => evidence.hops.push(hop.clone()),
                    None => evidence.crossed_unlabeled = true,
                },
                Relation::AssignmentPropagation
                | Relation::RecoveredDefUse
                | Relation::CallDescent
                | Relation::ReturnInput
                | Relation::ReturnFlow => evidence.crossed_unlabeled = true,
            }
            current = parent;
        }
        evidence.hops.reverse();
        evidence
    }
}

pub fn classify_with_evidence(
    algorithm: &str,
    parse_quality: ParseQuality,
    evidence: &EvidencePath,
) -> (FindingConfidence, FindingTier) {
    let Some(algorithm) = SlicingAlgorithm::from_str(algorithm) else {
        return (FindingConfidence::Unlabeled, FindingTier::Candidate);
    };
    let confidence =
        if evidence.crossed_unlabeled || (algorithm.needs_cpg() && evidence.hops.is_empty()) {
            FindingConfidence::Unlabeled
        } else if evidence.hops.iter().any(|hop| match hop {
            EvidenceHop::DataFlow { confidence, .. } => !confidence.is_exact(),
            EvidenceHop::Call { confidence, .. } => *confidence != ResolutionConfidence::Exact,
        }) {
            FindingConfidence::NameOnly
        } else {
            FindingConfidence::Exact
        };
    let tier = if confidence == FindingConfidence::Exact && parse_quality == ParseQuality::Clean {
        FindingTier::Asserted
    } else {
        FindingTier::Candidate
    };
    (confidence, tier)
}

/// Apply the runtime emitter projection without changing the retained evidence.
/// Nominal mode under-claims every CPG-derived finding; AST-only findings keep
/// their evidence-derived Phase 0 grade in both modes.
pub fn classify_for_resolution(
    algorithm: &str,
    parse_quality: ParseQuality,
    evidence: Option<&EvidencePath>,
    resolution: ResolutionMode,
) -> (FindingConfidence, FindingTier) {
    if resolution == ResolutionMode::Nominal
        && SlicingAlgorithm::from_str(algorithm).is_some_and(|algorithm| algorithm.needs_cpg())
    {
        return (FindingConfidence::Unlabeled, FindingTier::Candidate);
    }
    evidence.map_or(
        (FindingConfidence::Unlabeled, FindingTier::Candidate),
        |evidence| classify_with_evidence(algorithm, parse_quality, evidence),
    )
}

/// Classifies a finding's confidence and tier from its producing algorithm and
/// the worst parse quality over its evidence files. Pure and total; never reads
/// the CPG.
pub fn classify(algorithm: &str, parse_quality: ParseQuality) -> (FindingConfidence, FindingTier) {
    classify_with_evidence(
        algorithm,
        parse_quality,
        &EvidencePath::unlabeled_for(algorithm),
    )
}

#[cfg(test)]
mod tests;
