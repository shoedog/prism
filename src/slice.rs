use crate::diff::DiffBlock;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Structured per-file parse quality information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileParseQuality {
    pub error_count: usize,
    pub node_count: usize,
    pub error_rate: f64,
    /// "clean" (<1%), "degraded" (1-10%), "poor" (10-30%), "unparseable" (>30%)
    pub quality: String,
    /// Lines containing ERROR nodes (first 20, to avoid bloat).
    pub error_lines: Vec<usize>,
}

/// A structured finding from a slicing algorithm.
/// Findings carry the *semantics* of what an algorithm detected.
/// DiffBlocks carry the *lines* to show. They're consumed at different
/// stages: blocks go into the reviewer's code context, findings go
/// into the reviewer's analysis hints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceFinding {
    pub algorithm: String,
    pub file: String,
    pub line: usize,
    pub severity: String, // "info", "warning", or "concern"
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_lines: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Parse quality grade of the source file: "clean", "degraded", "poor", or "unparseable".
    /// Set when the file has >1% ERROR nodes in its tree-sitter parse tree.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_quality: Option<String>,
}

/// All slicing strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlicingAlgorithm {
    // --- Paper algorithms (arXiv:2505.17928) ---
    /// Algorithm 6: Raw diff lines only.
    OriginalDiff,
    /// Algorithm 7: Entire parent function containing diff lines.
    ParentFunction,
    /// Algorithm 8: Backward data-flow tracing from L-values.
    LeftFlow,
    /// Algorithm 9: LeftFlow + forward tracing from R-values and callees.
    FullFlow,

    // --- Section 4: Established taxonomy ---
    /// Minimal backward slice — data deps only, no control deps.
    ThinSlice,
    /// Interprocedural with explicit depth/boundary controls.
    BarrierSlice,
    /// All statements on data flow paths between source and sink.
    Chop,
    /// Forward trace of untrusted values through the program.
    Taint,
    /// Backward slice + potential alternate branch paths.
    RelevantSlice,
    /// Backward slice pruned by a value assumption.
    ConditionedSlice,
    /// Minimal changes causing behavioral difference between versions.
    DeltaSlice,

    // --- Section 5: Theoretical extensions ---
    /// Adaptive-depth through concentric rings.
    SpiralSlice,
    /// Data flow cycle detection across function boundaries.
    CircularSlice,
    /// Concurrent state superposition enumeration.
    QuantumSlice,
    /// Peer pattern consistency analysis.
    HorizontalSlice,
    /// End-to-end feature path tracing.
    VerticalSlice,
    /// Cross-cutting concern tracing.
    AngleSlice,
    /// Temporal-structural risk integration.
    ThreeDSlice,

    // --- Section 5 extended: Novel theoretical extensions ---
    /// Missing counterpart detection (open without close, lock without unlock).
    AbsenceSlice,
    /// Change coupling from git history — files that usually co-change.
    ResonanceSlice,
    /// Broken symmetry detection (serialize without deserialize).
    SymmetrySlice,
    /// Continuous relevance scoring with distance decay.
    GradientSlice,
    /// Backward trace to classify data origin (user input, config, database, etc.).
    ProvenanceSlice,
    /// Recently deleted code that current changes may depend on.
    PhantomSlice,
    /// Module boundary impact — who calls this API and will they break.
    MembraneSlice,
    /// Ripple effect — downstream callers that may not handle changed semantics.
    EchoSlice,
    /// Implicit behavioral contract extraction and violation detection.
    ContractSlice,
}

impl SlicingAlgorithm {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "originaldiff" | "original_diff" | "onlydiff" => Some(Self::OriginalDiff),
            "parentfunction" | "parent_function" | "function" => Some(Self::ParentFunction),
            "leftflow" | "left_flow" | "relevantcode" => Some(Self::LeftFlow),
            "fullflow" | "full_flow" | "relevantcoderhs" => Some(Self::FullFlow),
            "thin" | "thinslice" | "thin_slice" => Some(Self::ThinSlice),
            "barrier" | "barrierslice" | "barrier_slice" => Some(Self::BarrierSlice),
            "chop" | "chopping" => Some(Self::Chop),
            "taint" | "taint_analysis" => Some(Self::Taint),
            "relevant" | "relevantslice" | "relevant_slice" => Some(Self::RelevantSlice),
            "conditioned" | "conditionedslice" | "conditioned_slice" => {
                Some(Self::ConditionedSlice)
            }
            "delta" | "deltaslice" | "delta_slice" => Some(Self::DeltaSlice),
            "spiral" | "spiralslice" | "spiral_slice" => Some(Self::SpiralSlice),
            "circular" | "circularslice" | "circular_slice" => Some(Self::CircularSlice),
            "quantum" | "quantumslice" | "quantum_slice" => Some(Self::QuantumSlice),
            "horizontal" | "horizontalslice" | "horizontal_slice" => Some(Self::HorizontalSlice),
            "vertical" | "verticalslice" | "vertical_slice" => Some(Self::VerticalSlice),
            "angle" | "angleslice" | "angle_slice" => Some(Self::AngleSlice),
            "3d" | "threed" | "threedslice" | "threed_slice" => Some(Self::ThreeDSlice),
            "absence" | "absenceslice" | "absence_slice" => Some(Self::AbsenceSlice),
            "resonance" | "resonanceslice" | "resonance_slice" => Some(Self::ResonanceSlice),
            "symmetry" | "symmetryslice" | "symmetry_slice" => Some(Self::SymmetrySlice),
            "gradient" | "gradientslice" | "gradient_slice" => Some(Self::GradientSlice),
            "provenance" | "provenanceslice" | "provenance_slice" => Some(Self::ProvenanceSlice),
            "phantom" | "phantomslice" | "phantom_slice" | "ghost" => Some(Self::PhantomSlice),
            "membrane" | "membraneslice" | "membrane_slice" | "boundary" => {
                Some(Self::MembraneSlice)
            }
            "echo" | "echoslice" | "echo_slice" | "ripple" => Some(Self::EchoSlice),
            "contract" | "contractslice" | "contract_slice" => Some(Self::ContractSlice),
            _ => None,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::OriginalDiff => "OriginalDiff",
            Self::ParentFunction => "ParentFunction",
            Self::LeftFlow => "LeftFlow",
            Self::FullFlow => "FullFlow",
            Self::ThinSlice => "ThinSlice",
            Self::BarrierSlice => "BarrierSlice",
            Self::Chop => "Chop",
            Self::Taint => "Taint",
            Self::RelevantSlice => "RelevantSlice",
            Self::ConditionedSlice => "ConditionedSlice",
            Self::DeltaSlice => "DeltaSlice",
            Self::SpiralSlice => "SpiralSlice",
            Self::CircularSlice => "CircularSlice",
            Self::QuantumSlice => "QuantumSlice",
            Self::HorizontalSlice => "HorizontalSlice",
            Self::VerticalSlice => "VerticalSlice",
            Self::AngleSlice => "AngleSlice",
            Self::ThreeDSlice => "ThreeDSlice",
            Self::AbsenceSlice => "AbsenceSlice",
            Self::ResonanceSlice => "ResonanceSlice",
            Self::SymmetrySlice => "SymmetrySlice",
            Self::GradientSlice => "GradientSlice",
            Self::ProvenanceSlice => "ProvenanceSlice",
            Self::PhantomSlice => "PhantomSlice",
            Self::MembraneSlice => "MembraneSlice",
            Self::EchoSlice => "EchoSlice",
            Self::ContractSlice => "ContractSlice",
        }
    }

    /// Whether this algorithm requires a CPG (DFG + CallGraph + CFG).
    ///
    /// AST-only algorithms (OriginalDiff, ParentFunction, LeftFlow, etc.) only
    /// need parsed files. Skipping CPG construction for these saves significant
    /// time, especially in test suites where many algorithms are exercised.
    pub fn needs_cpg(&self) -> bool {
        matches!(
            self,
            Self::LeftFlow
                | Self::FullFlow
                | Self::RelevantSlice
                | Self::ConditionedSlice
                | Self::BarrierSlice
                | Self::Chop
                | Self::Taint
                | Self::DeltaSlice
                | Self::SpiralSlice
                | Self::CircularSlice
                | Self::VerticalSlice
                | Self::ThreeDSlice
                | Self::GradientSlice
                | Self::ProvenanceSlice
                | Self::MembraneSlice
                | Self::EchoSlice
        )
    }

    /// The default review suite: all algorithms that don't require git history.
    pub fn review_suite() -> Vec<Self> {
        vec![
            Self::LeftFlow,
            Self::FullFlow,
            Self::ThinSlice,
            Self::RelevantSlice,
            Self::BarrierSlice,
            Self::Taint,
            Self::AbsenceSlice,
            Self::SymmetrySlice,
            Self::MembraneSlice,
            Self::EchoSlice,
            Self::GradientSlice,
            Self::ProvenanceSlice,
            Self::HorizontalSlice,
            Self::VerticalSlice,
            Self::AngleSlice,
            Self::CircularSlice,
            Self::SpiralSlice,
            Self::ContractSlice,
        ]
    }

    /// List all available algorithms.
    pub fn all() -> Vec<Self> {
        vec![
            Self::OriginalDiff,
            Self::ParentFunction,
            Self::LeftFlow,
            Self::FullFlow,
            Self::ThinSlice,
            Self::BarrierSlice,
            Self::Chop,
            Self::Taint,
            Self::RelevantSlice,
            Self::ConditionedSlice,
            Self::DeltaSlice,
            Self::SpiralSlice,
            Self::CircularSlice,
            Self::QuantumSlice,
            Self::HorizontalSlice,
            Self::VerticalSlice,
            Self::AngleSlice,
            Self::ThreeDSlice,
            Self::AbsenceSlice,
            Self::ResonanceSlice,
            Self::SymmetrySlice,
            Self::GradientSlice,
            Self::ProvenanceSlice,
            Self::PhantomSlice,
            Self::MembraneSlice,
            Self::EchoSlice,
            Self::ContractSlice,
        ]
    }
}

/// Result of running a slicing algorithm on a diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceResult {
    pub algorithm: SlicingAlgorithm,
    pub blocks: Vec<DiffBlock>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<SliceFinding>,
    /// Parse quality warnings for input files (e.g. high ERROR-node rate).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

impl SliceResult {
    pub fn new(algorithm: SlicingAlgorithm) -> Self {
        Self {
            algorithm,
            blocks: Vec::new(),
            findings: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

/// Configuration for the slicing engine.
#[derive(Debug, Clone)]
pub struct SliceConfig {
    pub algorithm: SlicingAlgorithm,
    /// Maximum lines to include in a branch before summarizing (default: 5).
    pub max_branch_lines: usize,
    /// Whether to include return statements in LeftFlow/FullFlow.
    pub include_returns: bool,
    /// Whether to trace into called functions (FullFlow).
    pub trace_callees: bool,
    /// Build the CPG from only diff-changed files + direct callers/callees.
    /// Reduces CPG construction time proportionally to the scope reduction.
    pub scoped_cpg: bool,
}

impl Default for SliceConfig {
    fn default() -> Self {
        Self {
            algorithm: SlicingAlgorithm::LeftFlow,
            max_branch_lines: 5,
            include_returns: true,
            trace_callees: true,
            scoped_cpg: false,
        }
    }
}

impl SliceConfig {
    pub fn with_algorithm(mut self, algo: SlicingAlgorithm) -> Self {
        self.algorithm = algo;
        self
    }
}

/// Result of running multiple slicing algorithms on the same diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSliceResult {
    pub version: String,
    pub algorithms_run: Vec<String>,
    pub results: Vec<SliceResult>,
    pub findings: Vec<SliceFinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<AlgorithmError>,
    /// Parse quality warnings for input files (e.g. high ERROR-node rate).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    /// Structured per-file parse quality data.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parse_quality: BTreeMap<String, FileParseQuality>,
}

/// A per-algorithm error captured during multi-algorithm runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmError {
    pub algorithm: String,
    pub error: String,
}
