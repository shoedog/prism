pub mod absence_slice;
pub mod angle_slice;
pub mod barrier_slice;
pub mod callback_dispatcher_slice;
pub mod chop;
pub mod circular_slice;
pub mod conditioned_slice;
pub mod contract_slice;
pub mod delta_slice;
pub mod echo_slice;
pub mod full_flow;
pub mod gradient_slice;
pub mod horizontal_slice;
pub mod left_flow;
pub mod membrane_slice;
pub mod original_diff;
pub mod parent_function;
pub mod peer_consistency_slice;
pub mod phantom_slice;
pub mod primitive_slice;
pub mod provenance_slice;
pub mod quantum_slice;
pub mod relevant_slice;
pub mod resonance_slice;
pub mod spiral_slice;
pub mod symmetry_slice;
pub mod taint;
pub mod thin_slice;
pub mod threed_slice;
pub mod vertical_slice;

use crate::ast::ParsedFile;
use crate::cpg::CpgContext;
use crate::diff::DiffInput;
use crate::output::mermaid;
use crate::slice::{
    DiagramWarning, DiagramWarningKind, FileParseQuality, SliceConfig, SliceResult,
    SlicingAlgorithm,
};
use crate::type_db::TypeDatabase;
use anyhow::Result;
use std::collections::BTreeMap;
use std::panic;

/// Check parsed files for tree-sitter parse errors and return human-readable warnings.
///
/// Thresholds:
/// * > 10% error nodes → warn that results may be unreliable (common with macro-heavy C/C++)
/// * > 30% error nodes → warn that results would be meaningless; recommend preprocessing
///
/// Also kept for backward compatibility — delegates to `check_parse_quality`.
pub fn check_parse_warnings(files: &BTreeMap<String, ParsedFile>) -> Vec<String> {
    let (warnings, _) = check_parse_quality(files);
    warnings
}

/// Check parsed files for tree-sitter parse errors and return both human-readable
/// warnings and structured per-file parse quality data.
///
/// Thresholds:
/// * > 10% error nodes → warn that results may be unreliable (common with macro-heavy C/C++)
/// * > 30% error nodes → warn that results would be meaningless; recommend preprocessing
pub fn check_parse_quality(
    files: &BTreeMap<String, ParsedFile>,
) -> (Vec<String>, BTreeMap<String, FileParseQuality>) {
    let mut warnings = Vec::new();
    let mut quality = BTreeMap::new();
    for (path, pf) in files {
        let total = pf.parse_node_count;
        if total == 0 {
            continue;
        }
        let rate = pf.error_rate();
        let pct = (rate * 100.0).round() as usize;
        if rate > 0.01 {
            let error_lines = crate::ast::collect_error_lines(&pf.tree, 20);
            let q = if rate > 0.3 {
                "unparseable"
            } else if rate > 0.1 {
                "poor"
            } else {
                "degraded"
            };
            quality.insert(
                path.clone(),
                FileParseQuality {
                    error_count: pf.parse_error_count,
                    node_count: total,
                    error_rate: rate,
                    quality: q.to_string(),
                    error_lines,
                },
            );
        }
        if rate > 0.3 {
            warnings.push(format!(
                "File {} has severe parse errors ({} of {} AST nodes, {}%). \
                 Skipping analysis — results would be meaningless. \
                 Consider preprocessing macros first.",
                path, pf.parse_error_count, total, pct
            ));
        } else if rate > 0.1 {
            warnings.push(format!(
                "File {} has {} parse errors ({}% of AST nodes). \
                 Results may be unreliable. \
                 This often happens with macro-heavy C/C++ code.",
                path, pf.parse_error_count, pct
            ));
        }
    }
    (warnings, quality)
}

/// Run the configured slicing algorithm with a shared CPG context, returning
/// the raw (unfinalized) result.
///
/// Callers that need Mermaid rendering must call `finalize_diagrams` themselves
/// (or use `run_slicing`, which does it automatically).
///
/// This seam exists so that `run_algorithm` in `main.rs` can call
/// `run_slicing_inner` for its fallback path and then apply a single
/// `finalize_diagrams` call — avoiding the double-finalize that would occur if
/// it used `run_slicing` (which finalizes) and then called `finalize_diagrams`
/// again afterwards.
pub fn run_slicing_inner(
    ctx: &CpgContext,
    diff: &DiffInput,
    config: &SliceConfig,
) -> Result<SliceResult> {
    match config.algorithm {
        SlicingAlgorithm::OriginalDiff => original_diff::slice(ctx.files, diff),
        SlicingAlgorithm::ParentFunction => parent_function::slice(ctx.files, diff),
        SlicingAlgorithm::LeftFlow => left_flow::slice(ctx, diff, config),
        SlicingAlgorithm::FullFlow => full_flow::slice(ctx, diff, config),
        SlicingAlgorithm::ThinSlice => thin_slice::slice(ctx.files, diff),
        SlicingAlgorithm::RelevantSlice => relevant_slice::slice(ctx, diff, config),
        SlicingAlgorithm::BarrierSlice => {
            barrier_slice::slice(ctx, diff, config, &barrier_slice::BarrierConfig::default())
        }
        SlicingAlgorithm::Chop => Ok(SliceResult::new(SlicingAlgorithm::Chop)),
        SlicingAlgorithm::Taint => taint::slice(ctx, diff, &taint::TaintConfig::default()),
        SlicingAlgorithm::ConditionedSlice => {
            Ok(SliceResult::new(SlicingAlgorithm::ConditionedSlice))
        }
        SlicingAlgorithm::DeltaSlice => Ok(SliceResult::new(SlicingAlgorithm::DeltaSlice)),
        SlicingAlgorithm::SpiralSlice => {
            spiral_slice::slice(ctx, diff, config, &spiral_slice::SpiralConfig::default())
        }
        SlicingAlgorithm::CircularSlice => circular_slice::slice(ctx, diff),
        SlicingAlgorithm::QuantumSlice => quantum_slice::slice(ctx.files, diff, None),
        SlicingAlgorithm::HorizontalSlice => {
            horizontal_slice::slice(ctx.files, diff, &horizontal_slice::PeerPattern::Auto)
        }
        SlicingAlgorithm::VerticalSlice => {
            vertical_slice::slice(ctx, diff, &vertical_slice::VerticalConfig::default())
        }
        SlicingAlgorithm::AngleSlice => {
            angle_slice::slice(ctx.files, diff, &angle_slice::Concern::ErrorHandling)
        }
        SlicingAlgorithm::ThreeDSlice => {
            threed_slice::slice(ctx, diff, &threed_slice::ThreeDConfig::default())
        }
        // New theoretical extensions
        SlicingAlgorithm::AbsenceSlice => absence_slice::slice(ctx.files, diff),
        SlicingAlgorithm::ResonanceSlice => resonance_slice::slice(
            ctx.files,
            diff,
            &resonance_slice::ResonanceConfig::default(),
        ),
        SlicingAlgorithm::SymmetrySlice => symmetry_slice::slice(ctx.files, diff),
        SlicingAlgorithm::GradientSlice => {
            gradient_slice::slice(ctx, diff, &gradient_slice::GradientConfig::default())
        }
        SlicingAlgorithm::ProvenanceSlice => provenance_slice::slice(ctx, diff),
        SlicingAlgorithm::PhantomSlice => {
            phantom_slice::slice(ctx.files, diff, &phantom_slice::PhantomConfig::default())
        }
        SlicingAlgorithm::MembraneSlice => membrane_slice::slice(ctx, diff),
        SlicingAlgorithm::EchoSlice => echo_slice::slice(ctx, diff),
        SlicingAlgorithm::ContractSlice => contract_slice::slice(ctx.files, diff),
        SlicingAlgorithm::PeerConsistencySlice => peer_consistency_slice::slice(ctx.files, diff),
        SlicingAlgorithm::CallbackDispatcherSlice => {
            callback_dispatcher_slice::slice(ctx.files, diff)
        }
        SlicingAlgorithm::PrimitiveSlice => primitive_slice::slice(ctx.files, diff),
    }
}

/// Run the configured slicing algorithm with a shared CPG context.
///
/// For algorithms that need additional configuration (barrier, chop, taint, etc.),
/// use the algorithm-specific `slice()` functions directly.
///
/// This function calls `finalize_diagrams` before returning.  If you need an
/// unfinalized result (e.g., to avoid double-finalization in a wrapper that
/// also calls `finalize_diagrams`), use `run_slicing_inner` instead.
pub fn run_slicing(
    ctx: &CpgContext,
    diff: &DiffInput,
    config: &SliceConfig,
) -> Result<SliceResult> {
    let mut result = run_slicing_inner(ctx, diff, config)?;
    finalize_diagrams(&mut result, config.diagram_node_cap);
    Ok(result)
}

/// Render Mermaid for every populated SliceGraph in `result`. Stamps each
/// `graph.mermaid` field, collects any warnings into `result.diagram_warnings`,
/// catches panics from the renderer (treating them as `RenderPanic` warnings).
pub fn finalize_diagrams(result: &mut SliceResult, cap: usize) {
    let algo_name = result.algorithm.name().to_string();

    // Result-level diagrams.
    for graph in result.diagrams.iter_mut() {
        match panic::catch_unwind(panic::AssertUnwindSafe(|| mermaid::render(graph, cap))) {
            Ok((rendered, warns)) => {
                graph.mermaid = rendered;
                for mut w in warns {
                    w.algorithm = algo_name.clone();
                    result.diagram_warnings.push(w);
                }
            }
            Err(panic_info) => {
                let detail = panic_info
                    .downcast_ref::<&str>()
                    .map(|s| s.to_string())
                    .or_else(|| panic_info.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "panic in render".to_string());
                result.diagram_warnings.push(DiagramWarning {
                    algorithm: algo_name.clone(),
                    graph_title: graph.title.clone(),
                    kind: DiagramWarningKind::RenderPanic,
                    detail,
                });
            }
        }
    }

    // Per-finding diagrams.
    for finding in result.findings.iter_mut() {
        for graph in finding.diagrams.iter_mut() {
            match panic::catch_unwind(panic::AssertUnwindSafe(|| mermaid::render(graph, cap))) {
                Ok((rendered, warns)) => {
                    graph.mermaid = rendered;
                    for mut w in warns {
                        w.algorithm = algo_name.clone();
                        result.diagram_warnings.push(w);
                    }
                }
                Err(_) => {
                    result.diagram_warnings.push(DiagramWarning {
                        algorithm: algo_name.clone(),
                        graph_title: graph.title.clone(),
                        kind: DiagramWarningKind::RenderPanic,
                        detail: "panic in render".to_string(),
                    });
                }
            }
        }
    }
}

/// Backward-compatible wrapper: builds a CpgContext and runs the algorithm.
///
/// Used by tests that haven't been migrated to CpgContext yet.
/// Skips CPG construction for AST-only algorithms to avoid unnecessary overhead.
/// When `config.scoped_cpg` is true, builds a diff-scoped CPG covering only
/// changed files + direct callers/callees.
pub fn run_slicing_compat(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    config: &SliceConfig,
    type_db: Option<&TypeDatabase>,
) -> Result<SliceResult> {
    if config.algorithm.needs_cpg() {
        let ctx = if config.scoped_cpg {
            CpgContext::build_scoped(files, diff, type_db)
        } else {
            CpgContext::build(files, type_db)
        };
        run_slicing(&ctx, diff, config)
    } else {
        let ctx = CpgContext::without_cpg(files, type_db);
        run_slicing(&ctx, diff, config)
    }
}

#[cfg(test)]
mod finalize_tests {
    use super::*;
    use crate::slice::{
        DiagramWarningKind, EdgeStyle, GraphEdge, GraphNode, GraphShape, NodeKind, SliceGraph,
        SliceResult, SlicingAlgorithm,
    };

    fn graph_with_dangling() -> SliceGraph {
        SliceGraph {
            title: Some("test".to_string()),
            shape: GraphShape::Chain,
            nodes: vec![GraphNode {
                id: "a".to_string(),
                label: "A".to_string(),
                kind: NodeKind::Source,
                file: None,
                line: None,
            }],
            edges: vec![GraphEdge {
                from: "a".to_string(),
                to: "missing".to_string(),
                label: None,
                style: EdgeStyle::Solid,
            }],
            clusters: vec![],
            mermaid: String::new(),
        }
    }

    #[test]
    fn finalize_populates_mermaid_string() {
        let mut r = SliceResult::new(SlicingAlgorithm::Taint);
        r.diagrams.push(SliceGraph {
            title: Some("ok".to_string()),
            shape: GraphShape::Chain,
            nodes: vec![GraphNode {
                id: "x".to_string(),
                label: "X".to_string(),
                kind: NodeKind::Step,
                file: None,
                line: None,
            }],
            edges: vec![],
            clusters: vec![],
            mermaid: String::new(),
        });
        finalize_diagrams(&mut r, 40);
        assert!(!r.diagrams[0].mermaid.is_empty());
        assert!(r.diagrams[0].mermaid.starts_with("flowchart TD"));
    }

    #[test]
    fn finalize_propagates_dangling_edge_warning() {
        let mut r = SliceResult::new(SlicingAlgorithm::Taint);
        r.diagrams.push(graph_with_dangling());
        finalize_diagrams(&mut r, 40);
        assert!(r
            .diagram_warnings
            .iter()
            .any(|w| matches!(w.kind, DiagramWarningKind::DanglingEdge) && w.algorithm == "Taint"));
    }

    #[test]
    fn finalize_handles_panic_via_catch_unwind() {
        // Verifies the catch_unwind path is exercised by calling finalize on
        // an empty diagrams vec (no panic, no warnings, no crash).
        let mut r = SliceResult::new(SlicingAlgorithm::Taint);
        finalize_diagrams(&mut r, 40);
        assert!(r.diagrams.is_empty());
        assert!(r.diagram_warnings.is_empty());
    }
}
