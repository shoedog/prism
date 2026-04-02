pub mod absence_slice;
pub mod angle_slice;
pub mod barrier_slice;
pub mod chop;
pub mod circular_slice;
pub mod conditioned_slice;
pub mod delta_slice;
pub mod echo_slice;
pub mod full_flow;
pub mod gradient_slice;
pub mod horizontal_slice;
pub mod left_flow;
pub mod membrane_slice;
pub mod original_diff;
pub mod parent_function;
pub mod phantom_slice;
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
use crate::diff::DiffInput;
use crate::slice::{SliceConfig, SliceResult, SlicingAlgorithm};
use crate::type_db::TypeDatabase;
use anyhow::Result;
use std::collections::BTreeMap;

/// Check parsed files for tree-sitter parse errors and return human-readable warnings.
///
/// Thresholds:
/// * > 10% error nodes → warn that results may be unreliable (common with macro-heavy C/C++)
/// * > 30% error nodes → warn that results would be meaningless; recommend preprocessing
pub fn check_parse_warnings(files: &BTreeMap<String, ParsedFile>) -> Vec<String> {
    let mut warnings = Vec::new();
    for (path, pf) in files {
        let total = pf.parse_node_count;
        if total == 0 {
            continue;
        }
        let rate = pf.error_rate();
        let pct = (rate * 100.0).round() as usize;
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
    warnings
}

/// Run the configured slicing algorithm on parsed files with the given diff.
///
/// For algorithms that need additional configuration (barrier, chop, taint, etc.),
/// use the algorithm-specific `slice()` functions directly.
pub fn run_slicing(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    config: &SliceConfig,
    type_db: Option<&TypeDatabase>,
) -> Result<SliceResult> {
    match config.algorithm {
        SlicingAlgorithm::OriginalDiff => original_diff::slice(files, diff),
        SlicingAlgorithm::ParentFunction => parent_function::slice(files, diff),
        SlicingAlgorithm::LeftFlow => left_flow::slice(files, diff, config),
        SlicingAlgorithm::FullFlow => full_flow::slice(files, diff, config),
        SlicingAlgorithm::ThinSlice => thin_slice::slice(files, diff),
        SlicingAlgorithm::RelevantSlice => relevant_slice::slice(files, diff, config),
        SlicingAlgorithm::BarrierSlice => barrier_slice::slice(
            files,
            diff,
            config,
            &barrier_slice::BarrierConfig::default(),
            type_db,
        ),
        SlicingAlgorithm::Chop => Ok(SliceResult::new(SlicingAlgorithm::Chop)),
        SlicingAlgorithm::Taint => {
            taint::slice(files, diff, &taint::TaintConfig::default(), type_db)
        }
        SlicingAlgorithm::ConditionedSlice => {
            Ok(SliceResult::new(SlicingAlgorithm::ConditionedSlice))
        }
        SlicingAlgorithm::DeltaSlice => Ok(SliceResult::new(SlicingAlgorithm::DeltaSlice)),
        SlicingAlgorithm::SpiralSlice => spiral_slice::slice(
            files,
            diff,
            config,
            &spiral_slice::SpiralConfig::default(),
            type_db,
        ),
        SlicingAlgorithm::CircularSlice => circular_slice::slice(files, diff, type_db),
        SlicingAlgorithm::QuantumSlice => quantum_slice::slice(files, diff, None),
        SlicingAlgorithm::HorizontalSlice => {
            horizontal_slice::slice(files, diff, &horizontal_slice::PeerPattern::Auto)
        }
        SlicingAlgorithm::VerticalSlice => vertical_slice::slice(
            files,
            diff,
            &vertical_slice::VerticalConfig::default(),
            type_db,
        ),
        SlicingAlgorithm::AngleSlice => {
            angle_slice::slice(files, diff, &angle_slice::Concern::ErrorHandling)
        }
        SlicingAlgorithm::ThreeDSlice => {
            threed_slice::slice(files, diff, &threed_slice::ThreeDConfig::default(), type_db)
        }
        // New theoretical extensions
        SlicingAlgorithm::AbsenceSlice => absence_slice::slice(files, diff),
        SlicingAlgorithm::ResonanceSlice => {
            resonance_slice::slice(files, diff, &resonance_slice::ResonanceConfig::default())
        }
        SlicingAlgorithm::SymmetrySlice => symmetry_slice::slice(files, diff),
        SlicingAlgorithm::GradientSlice => gradient_slice::slice(
            files,
            diff,
            &gradient_slice::GradientConfig::default(),
            type_db,
        ),
        SlicingAlgorithm::ProvenanceSlice => provenance_slice::slice(files, diff, type_db),
        SlicingAlgorithm::PhantomSlice => {
            phantom_slice::slice(files, diff, &phantom_slice::PhantomConfig::default())
        }
        SlicingAlgorithm::MembraneSlice => membrane_slice::slice(files, diff, type_db),
        SlicingAlgorithm::EchoSlice => echo_slice::slice(files, diff, type_db),
    }
}
