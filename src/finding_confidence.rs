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

use crate::slice::{FileParseQuality, SliceFinding, SlicingAlgorithm};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum FindingTier {
    Asserted,
    Candidate,
}

/// Per-file parse quality grade, ordered best → worst by declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ParseQuality {
    Clean,
    Degraded,
    Poor,
    Unparseable,
    Unknown,
}

/// The BUILD's dataflow-labeling capability (roadmap 04 §3.6 `--resolution`): a
/// DIFFERENT AXIS from a finding's confidence. Phase 0: "nominal" = DataFlow
/// edges unlabeled.
pub const RESOLUTION_MODE: &str = "nominal";

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
    /// (`ReviewInputs.parse_quality`): the worst quality over `files`; `Unknown` if
    /// `files` is empty or any file is absent from the map. Never derived from
    /// `SliceFinding.parse_quality`, which is `None` both before annotation and
    /// for clean files.
    pub fn min_over(files: &[&str], map: &BTreeMap<String, FileParseQuality>) -> Self {
        if files.is_empty() {
            return Self::Unknown;
        }
        let mut worst = Self::Clean;
        for file in files {
            match map.get(*file) {
                Some(quality) => worst = worst.max(Self::from_quality_str(&quality.quality)),
                None => return Self::Unknown,
            }
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

/// Classifies a finding's confidence and tier from its producing algorithm and
/// the worst parse quality over its evidence files. Pure and total; never reads
/// the CPG.
pub fn classify(algorithm: &str, parse_quality: ParseQuality) -> (FindingConfidence, FindingTier) {
    match SlicingAlgorithm::from_str(algorithm) {
        None => (FindingConfidence::Unlabeled, FindingTier::Candidate),
        Some(algorithm) => {
            let confidence = if algorithm.needs_cpg() {
                FindingConfidence::Unlabeled
            } else {
                FindingConfidence::Exact
            };
            let tier =
                if confidence == FindingConfidence::Exact && parse_quality == ParseQuality::Clean {
                    FindingTier::Asserted
                } else {
                    FindingTier::Candidate
                };
            (confidence, tier)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slice::{FileParseQuality, SliceFinding};
    use std::collections::BTreeMap;

    fn fpq(q: &str) -> FileParseQuality {
        FileParseQuality {
            error_count: 0,
            node_count: 1,
            error_rate: 0.0,
            quality: q.to_string(),
            error_lines: vec![],
        }
    }
    fn finding(algorithm: &str, file: &str, related: &[&str]) -> SliceFinding {
        SliceFinding {
            algorithm: algorithm.into(),
            file: file.into(),
            line: 1,
            severity: "warning".into(),
            description: String::new(),
            function_name: None,
            related_lines: vec![],
            related_files: related.iter().map(|s| s.to_string()).collect(),
            category: None,
            parse_quality: None,
            diagrams: vec![],
        }
    }

    #[test]
    fn ast_only_clean_is_exact_asserted() {
        assert_eq!(
            classify("absence", ParseQuality::Clean),
            (FindingConfidence::Exact, FindingTier::Asserted)
        );
    }
    #[test]
    fn cpg_algorithm_is_unlabeled_candidate() {
        assert_eq!(
            classify("echo", ParseQuality::Clean),
            (FindingConfidence::Unlabeled, FindingTier::Candidate)
        );
    }
    #[test]
    fn degraded_or_unknown_parse_is_candidate() {
        assert_eq!(
            classify("absence", ParseQuality::Degraded),
            (FindingConfidence::Exact, FindingTier::Candidate)
        );
        assert_eq!(
            classify("absence", ParseQuality::Unknown),
            (FindingConfidence::Exact, FindingTier::Candidate)
        );
    }
    #[test]
    fn every_production_algorithm_string_parses() {
        for s in [
            "absence",
            "callback_dispatcher",
            "contract",
            "echo",
            "membrane",
            "peer_consistency",
            "primitive",
            "provenance",
            "symmetry",
            "taint",
        ] {
            assert!(
                crate::slice::SlicingAlgorithm::from_str(s).is_some(),
                "{s} must round-trip through from_str"
            );
        }
    }
    #[test]
    fn unknown_algorithm_is_unlabeled_candidate() {
        assert_eq!(
            classify("not_an_algorithm", ParseQuality::Clean),
            (FindingConfidence::Unlabeled, FindingTier::Candidate)
        );
    }
    #[test]
    fn serde_spellings() {
        assert_eq!(
            serde_json::to_string(&FindingConfidence::Exact).unwrap(),
            "\"exact\""
        );
        assert_eq!(
            serde_json::to_string(&FindingConfidence::Unlabeled).unwrap(),
            "\"unlabeled\""
        );
        assert_eq!(
            serde_json::to_string(&FindingTier::Candidate).unwrap(),
            "\"candidate\""
        );
        assert_eq!(ParseQuality::Poor.as_str(), "poor");
    }
    #[test]
    fn min_over_takes_the_worst_and_fails_unknown() {
        let mut map = BTreeMap::new();
        map.insert("a.py".to_string(), fpq("clean"));
        map.insert("b.py".to_string(), fpq("degraded"));
        assert_eq!(ParseQuality::min_over(&["a.py"], &map), ParseQuality::Clean);
        assert_eq!(
            ParseQuality::min_over(&["a.py", "b.py"], &map),
            ParseQuality::Degraded
        );
        assert_eq!(
            ParseQuality::min_over(&["a.py", "missing.py"], &map),
            ParseQuality::Unknown
        );
        assert_eq!(ParseQuality::min_over(&[], &map), ParseQuality::Unknown);
        map.insert("c.py".to_string(), fpq("weird"));
        assert_eq!(
            ParseQuality::min_over(&["c.py"], &map),
            ParseQuality::Unknown
        );
    }
    #[test]
    fn evidence_files_is_anchor_then_related() {
        assert_eq!(
            evidence_files(&finding("symmetry", "a.py", &["b.py"])),
            vec!["a.py", "b.py"]
        );
        assert_eq!(
            evidence_files(&finding("absence", "a.py", &[])),
            vec!["a.py"]
        );
        assert_eq!(
            evidence_files(&finding("echo", "a.py", &["a.py", "b.py", "b.py"])),
            vec!["a.py", "b.py"]
        );
    }
}
