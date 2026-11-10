//! The endpoint of one `dfg_edge_dump` row, split out of `queries.rs`.

use crate::access_path::AccessPath;
use crate::cpg::{CpgNode, VarAccess};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct DfgEdgeEndpoint {
    pub file: String,
    pub line: usize,
    pub path: AccessPath,
    pub access: &'static str,
    /// Occurrence bytes, emitted only with `PRISM_EDGES_BYTES=1` (a diagnostic for the
    /// alias-twin gate's byte-level edge identity); absent from the default output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_byte: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_byte: Option<usize>,
}

/// `PRISM_EDGES_BYTES=1` adds `start_byte`/`end_byte` to both endpoints of `dfg_edge_dump`
/// rows. Read once per process; off by default.
fn edges_with_bytes() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("PRISM_EDGES_BYTES").is_ok_and(|v| v == "1"))
}

pub(super) fn dfg_endpoint(node: &CpgNode) -> Option<DfgEdgeEndpoint> {
    let CpgNode::Variable {
        file,
        line,
        path,
        access,
        start_byte,
        end_byte,
        ..
    } = node
    else {
        return None;
    };
    let with_bytes = edges_with_bytes();
    Some(DfgEdgeEndpoint {
        start_byte: with_bytes.then_some(*start_byte),
        end_byte: with_bytes.then_some(*end_byte),
        file: file.clone(),
        line: *line,
        path: path.clone(),
        access: match access {
            VarAccess::Def => "def",
            VarAccess::Use => "use",
        },
    })
}
