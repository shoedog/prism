use crate::diff::DiffBlock;
use crate::finding_confidence::EvidencePath;
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
    pub severity: String, // "info", "suggestion", "warning", or "concern"
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagrams: Vec<SliceGraph>,
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
    /// Peer-signature guard divergence — cluster of sibling functions sharing
    /// a first-parameter name where all/some lack a NULL guard.
    PeerConsistencySlice,
    /// Resolve function-pointer-in-struct registrations to their dispatcher
    /// invocation sites; flag NULL argument passing.
    CallbackDispatcherSlice,
    /// Deterministic security-primitive fingerprint sweep (hash truncation,
    /// weak-hash-for-identity, shell=True with interpolation, cert validation
    /// disabled, hardcoded secrets).
    PrimitiveSlice,
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
            "peer"
            | "peer_consistency"
            | "peerconsistency"
            | "peerconsistencyslice"
            | "peer_consistency_slice" => Some(Self::PeerConsistencySlice),
            "callback"
            | "callback_dispatcher"
            | "callbackdispatcher"
            | "callbackdispatcherslice"
            | "callback_dispatcher_slice"
            | "dispatcher" => Some(Self::CallbackDispatcherSlice),
            "primitive" | "primitiveslice" | "primitive_slice" => Some(Self::PrimitiveSlice),
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
            Self::PeerConsistencySlice => "PeerConsistencySlice",
            Self::CallbackDispatcherSlice => "CallbackDispatcherSlice",
            Self::PrimitiveSlice => "PrimitiveSlice",
        }
    }

    /// Whether this algorithm requires a CPG (DFG + CallGraph + CFG).
    ///
    /// AST-only algorithms (OriginalDiff, ParentFunction, ThinSlice, etc.) only
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
            Self::PeerConsistencySlice,
            Self::CallbackDispatcherSlice,
            Self::PrimitiveSlice,
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
            Self::PeerConsistencySlice,
            Self::CallbackDispatcherSlice,
            Self::PrimitiveSlice,
        ]
    }
}

/// Result of running a slicing algorithm on a diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SliceResult {
    pub algorithm: SlicingAlgorithm,
    pub blocks: Vec<DiffBlock>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<SliceFinding>,
    /// Selected evidence for each finding, aligned by index and kept out of
    /// every legacy output format.
    #[serde(skip)]
    pub evidence: Vec<Option<EvidencePath>>,
    /// Parse quality warnings for input files (e.g. high ERROR-node rate).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagrams: Vec<SliceGraph>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagram_warnings: Vec<DiagramWarning>,
}

impl SliceResult {
    pub fn new(algorithm: SlicingAlgorithm) -> Self {
        Self {
            algorithm,
            blocks: Vec::new(),
            findings: Vec::new(),
            evidence: Vec::new(),
            warnings: Vec::new(),
            diagrams: Vec::new(),
            diagram_warnings: Vec::new(),
        }
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Restore the finding/evidence invariant at algorithm boundaries. AST-only
    /// findings have a valid empty witness; missing CPG evidence stays `None`.
    pub(crate) fn align_evidence(&mut self) {
        if self.findings.len() == self.evidence.len() {
            return;
        }
        if !self.algorithm.needs_cpg() && self.evidence.is_empty() {
            self.evidence
                .resize(self.findings.len(), Some(EvidencePath::default()));
            return;
        }
        self.warnings.push(format!(
            "evidence alignment mismatch for {}: {} findings, {} artifacts",
            self.algorithm.name(),
            self.findings.len(),
            self.evidence.len()
        ));
        self.evidence.truncate(self.findings.len());
        let fill = (!self.algorithm.needs_cpg()).then(EvidencePath::default);
        self.evidence.resize(self.findings.len(), fill);
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
    /// Maximum number of nodes a diagram may contain before truncation.
    pub diagram_node_cap: usize,
    /// Exit non-zero if any bug-class diagram warning is emitted.
    pub strict_diagrams: bool,
}

impl Default for SliceConfig {
    fn default() -> Self {
        Self {
            algorithm: SlicingAlgorithm::LeftFlow,
            max_branch_lines: 5,
            include_returns: true,
            trace_callees: true,
            scoped_cpg: false,
            diagram_node_cap: 40,
            strict_diagrams: false,
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
    /// Diagram warnings collected at the multi-run level (e.g. from finalize_diagrams).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagram_warnings: Vec<DiagramWarning>,
}

impl MultiSliceResult {
    /// Walk every per-algorithm result and the top-level list, returning all diagram
    /// warnings as a single flat `Vec`. CLI consumers use this to drive
    /// `--strict-diagrams` exit-code behaviour and stderr emission.
    pub fn aggregate_diagram_warnings(&self) -> Vec<DiagramWarning> {
        let mut out: Vec<DiagramWarning> = self.diagram_warnings.clone();
        for r in &self.results {
            out.extend(r.diagram_warnings.iter().cloned());
        }
        out
    }
}

/// A per-algorithm error captured during multi-algorithm runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmError {
    pub algorithm: String,
    pub error: String,
}

/// Shape category for a diagram. Drives which renderer template is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphShape {
    Chain,
    Layered,
    Cycle,
    Fanout,
}

/// Semantic role of a node, drives classDef styling in Mermaid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    Origin,
    Source,
    Sink,
    Step,
    Caller,
    Callee,
}

/// Edge style maps directly to Mermaid arrow syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeStyle {
    Solid,  // -->
    Bold,   // ==> (cycle back-edge)
    Dotted, // -.->
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub kind: NodeKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub style: EdgeStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeCluster {
    pub label: String,
    pub node_ids: Vec<String>,
}

/// A flow / call / cycle / fanout diagram for one algorithm output.
/// Algorithms populate the structured fields. The `mermaid` string is filled
/// by the finalize pass via `output::mermaid::render`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceGraph {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub shape: GraphShape,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clusters: Vec<NodeCluster>,
    /// Pre-rendered Mermaid string. Populated by finalize pass; algorithms must leave empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mermaid: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagramWarningKind {
    /// Renderer panicked. Caught by catch_unwind. Indicates a bug in the renderer.
    RenderPanic,
    /// Graph edge references a node id not present in `nodes`. Indicates a bug in the algorithm.
    DanglingEdge,
    /// Two nodes share an id. Second is dropped. Indicates a bug in the algorithm.
    DuplicateNodeId,
    /// Algorithm pushed a SliceGraph with zero nodes. Indicates a bug in the algorithm.
    EmptyGraph,
    /// Label exceeded 80 characters and was truncated. Informational.
    LabelTruncated,
    /// Node count exceeded the cap; nodes were elided in the rendered string. Informational.
    NodeCapExceeded,
}

impl DiagramWarningKind {
    /// True for kinds that indicate a real bug (algorithm or renderer).
    /// `--strict-diagrams` exits non-zero when any bug-class warning is present.
    pub fn is_bug(self) -> bool {
        matches!(
            self,
            Self::RenderPanic | Self::DanglingEdge | Self::DuplicateNodeId | Self::EmptyGraph
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagramWarning {
    pub algorithm: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_title: Option<String>,
    pub kind: DiagramWarningKind,
    pub detail: String,
}

#[cfg(test)]
mod diagram_tests {
    use super::*;

    #[test]
    fn slice_result_default_omits_diagram_fields_in_json() {
        // A new empty SliceResult must serialize without `diagrams` or `diagram_warnings`
        // so existing JSON consumers see byte-identical output.
        let r = SliceResult::new(SlicingAlgorithm::OriginalDiff);
        let json = serde_json::to_string(&r).unwrap();
        assert!(!json.contains("diagrams"));
        assert!(!json.contains("diagram_warnings"));
    }

    #[test]
    fn slice_config_diagram_node_cap_default() {
        let c = SliceConfig::default();
        assert_eq!(c.diagram_node_cap, 40);
        assert!(!c.strict_diagrams);
    }

    #[test]
    fn slice_graph_serde_round_trip() {
        let g = SliceGraph {
            title: Some("data flow".to_string()),
            shape: GraphShape::Chain,
            nodes: vec![
                GraphNode {
                    id: "n_foo_42".to_string(),
                    label: "foo.c:42 read_input".to_string(),
                    kind: NodeKind::Source,
                    file: Some("foo.c".to_string()),
                    line: Some(42),
                },
                GraphNode {
                    id: "n_bar_10".to_string(),
                    label: "bar.c:10 render".to_string(),
                    kind: NodeKind::Step,
                    file: Some("bar.c".to_string()),
                    line: Some(10),
                },
            ],
            edges: vec![GraphEdge {
                from: "n_foo_42".to_string(),
                to: "n_bar_10".to_string(),
                label: Some("tainted".to_string()),
                style: EdgeStyle::Solid,
            }],
            clusters: vec![NodeCluster {
                label: "UI".to_string(),
                node_ids: vec!["n_bar_10".to_string()],
            }],
            mermaid: "graph LR\n  n_foo_42 --> n_bar_10".to_string(),
        };
        let json = serde_json::to_string(&g).unwrap();
        let back: SliceGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(g.title, back.title);
        assert_eq!(g.shape, back.shape);
        assert_eq!(g.nodes, back.nodes);
        assert_eq!(g.edges, back.edges);
        assert_eq!(g.clusters, back.clusters);
        assert_eq!(g.mermaid, back.mermaid);
    }

    #[test]
    fn multi_slice_result_aggregates_diagram_warnings() {
        use crate::slice::DiagramWarningKind;
        let mut r1 = SliceResult::new(SlicingAlgorithm::Taint);
        r1.diagram_warnings.push(DiagramWarning {
            algorithm: "Taint".to_string(),
            graph_title: None,
            kind: DiagramWarningKind::EmptyGraph,
            detail: "no nodes".to_string(),
        });
        let r2 = SliceResult::new(SlicingAlgorithm::EchoSlice);

        let multi = MultiSliceResult {
            version: "test".to_string(),
            algorithms_run: vec!["Taint".to_string(), "EchoSlice".to_string()],
            results: vec![r1, r2],
            findings: vec![],
            errors: vec![],
            warnings: vec![],
            parse_quality: BTreeMap::new(),
            diagram_warnings: vec![],
        };
        let aggregated = multi.aggregate_diagram_warnings();
        assert_eq!(aggregated.len(), 1);
        assert_eq!(aggregated[0].kind, DiagramWarningKind::EmptyGraph);
    }

    #[test]
    fn diagram_warning_serde_round_trip_and_kind_classification() {
        let w = DiagramWarning {
            algorithm: "Taint".to_string(),
            graph_title: Some("Data flow".to_string()),
            kind: DiagramWarningKind::DanglingEdge,
            detail: "edge from n_a to n_b — n_b not in nodes".to_string(),
        };
        let json = serde_json::to_string(&w).unwrap();
        let back: DiagramWarning = serde_json::from_str(&json).unwrap();
        assert_eq!(w.kind, back.kind);
        assert_eq!(w.detail, back.detail);
        assert_eq!(w, back);

        // Bug-class predicate must be deterministic:
        assert!(DiagramWarningKind::RenderPanic.is_bug());
        assert!(DiagramWarningKind::DanglingEdge.is_bug());
        assert!(DiagramWarningKind::DuplicateNodeId.is_bug());
        assert!(DiagramWarningKind::EmptyGraph.is_bug());
        assert!(!DiagramWarningKind::LabelTruncated.is_bug());
        assert!(!DiagramWarningKind::NodeCapExceeded.is_bug());
    }
}
