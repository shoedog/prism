use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Location {
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum SymbolRef {
    Function {
        file: String,
        name: String,
        start_line: usize,
        end_line: usize,
        ordinal: usize,
    },
    Statement {
        file: String,
        line: usize,
        kind: String,
        ordinal: usize,
    },
    Variable {
        file: String,
        function: String,
        line: usize,
        path: String,
        access: String,
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
pub struct Evidence {
    pub query: String,
    pub items: Vec<EvidenceItem>,
    pub truncated: bool,
    pub warnings: Vec<Warning>,
    /// Present only for graph-shaped queries (`ego`, `repo-map`); omitted otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<GraphPayload>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum QueryError {
    AmbiguousSymbol { candidates: Vec<SymbolRef> },
    SymbolNotFound { seed: String },
    LocationOutOfRange { file: String, line: usize },
    UnsupportedFile { file: String },
    UnknownEdge { edge: String },
}
