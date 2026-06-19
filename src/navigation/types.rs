use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Location {
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum SymbolRef {
    Function {
        file: String,
        name: String,
        start_line: usize,
        end_line: usize,
        start_byte: usize,
        end_byte: usize,
        ordinal: usize,
    },
    Statement {
        file: String,
        line: usize,
        kind: String,
        start_byte: usize,
        end_byte: usize,
        ordinal: usize,
    },
    Variable {
        file: String,
        function: String,
        line: usize,
        path: String,
        access: String,
        start_byte: usize,
        end_byte: usize,
        ordinal: usize,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum Source {
    PrismCpg,
    HeuristicImport,
    ExternalIndex { name: String },
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum Reason {
    Calls {
        callee: String,
        call_site_line: usize,
        qualifier: Option<String>,
    },
    CalledBy {
        caller: String,
        call_site_line: usize,
    },
    Resolution {
        kind: String,
    },
    EnclosingFunction {
        function: SymbolRef,
    },
    Containment {
        parent: SymbolRef,
    },
    ResolvedImport {
        module: String,
        target_file: String,
    },
    UnresolvedImport {
        module: String,
    },
    Reasoning(ReasoningReason),
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EvidenceItem {
    pub symbol: Option<SymbolRef>,
    pub location: Location,
    pub score: f32,
    pub source: Source,
    pub fallback: bool,
    pub why: Vec<Reason>,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum WarningKind {
    ParseQuality,
    AmbiguousSymbol,
    IndirectCallApprox,
    UnresolvedModule,
    Collision,
    SkippedPath,
    ResultTruncated,
    Reasoning(ReasoningWarning),
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Warning {
    pub kind: WarningKind,
    pub message: String,
    pub location: Option<Location>,
}

/// A node in a graph-shaped result (`ego`, `repo-map`).
/// `symbol` is `None` for file-level nodes (repo-map); `Some` for symbol nodes (ego).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GraphNode {
    pub symbol: Option<SymbolRef>,
    pub location: Location,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GraphEdge {
    pub from: usize,
    pub to: usize,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GraphPayload {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum ReasoningReason {
    TaintedBy {
        source: SymbolRef,
        sanitizers_present_in_source_fn: Vec<String>,
        path_proven: bool,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum ReasoningWarning {
    SeedUnresolved {
        seed: String,
    },
    InterproceduralBoundary {
        sink: String,
    },
    Cleansed {
        source_function: String,
    },
    UnmodeledConstruct {
        language: String,
        construct: String,
        function: String,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum Reachability {
    Reached,
    NotReached,
    BoundaryExited,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SinkResult {
    pub sink: SymbolRef,
    pub reachability: Reachability,
    /// Index into the accompanying witness `GraphPayload.nodes` (NOT `Evidence.graph`, which the
    /// shared ego/repo-map paths truncate/reorder at `max_results`). Plan B owns wiring this to a
    /// non-truncating witness graph and repairing/clearing the index if that graph is ever clipped —
    /// a dangling index is a tracked Plan B follow-up (graph_node truncation repair). `None` until
    /// then.
    pub graph_node: Option<usize>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ReasoningSummary {
    /// Aggregate reachability across `per_sink`. AGGREGATION RULE (Plan B must honor this once it
    /// produces the field, before the wire shape freezes): the worst case in defect-finder terms —
    /// `Reached` if any sink is `Reached`, else `BoundaryExited` if any is `BoundaryExited`, else
    /// `NotReached`; `None` when there are no sinks. (Recall `NotReached` is "not reached within v1's
    /// traced scope," not proven absence.)
    pub reachability: Option<Reachability>,
    pub per_sink: Vec<SinkResult>,
    pub source_count: usize,
    pub frontier_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Evidence {
    pub query: String,
    pub items: Vec<EvidenceItem>,
    pub truncated: bool,
    pub warnings: Vec<Warning>,
    /// Present only for graph-shaped queries (`ego`, `repo-map`); omitted otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<GraphPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningSummary>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum QueryError {
    AmbiguousSymbol { candidates: Vec<SymbolRef> },
    SymbolNotFound { seed: String },
    LocationOutOfRange { file: String, line: usize },
    UnsupportedFile { file: String },
    UnknownEdge { edge: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_reasoning_omitted_when_none() {
        let e = Evidence {
            query: "q".into(),
            items: vec![],
            truncated: false,
            warnings: vec![],
            graph: None,
            reasoning: None,
        };
        assert!(!serde_json::to_string(&e).unwrap().contains("reasoning"));
    }
}
