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
use crate::slice::{FileParseQuality, SliceConfig, SliceResult, SlicingAlgorithm};
use crate::type_db::TypeDatabase;
use anyhow::Result;
use std::collections::BTreeMap;

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

/// Run the configured slicing algorithm with a shared CPG context.
///
/// For algorithms that need additional configuration (barrier, chop, taint, etc.),
/// use the algorithm-specific `slice()` functions directly.
pub fn run_slicing(
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
