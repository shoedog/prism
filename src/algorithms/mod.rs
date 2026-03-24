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
use anyhow::Result;
use std::collections::BTreeMap;

/// Run the configured slicing algorithm on parsed files with the given diff.
///
/// For algorithms that need additional configuration (barrier, chop, taint, etc.),
/// use the algorithm-specific `slice()` functions directly.
pub fn run_slicing(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    config: &SliceConfig,
) -> Result<SliceResult> {
    match config.algorithm {
        SlicingAlgorithm::OriginalDiff => original_diff::slice(files, diff),
        SlicingAlgorithm::ParentFunction => parent_function::slice(files, diff),
        SlicingAlgorithm::LeftFlow => left_flow::slice(files, diff, config),
        SlicingAlgorithm::FullFlow => full_flow::slice(files, diff, config),
        SlicingAlgorithm::ThinSlice => thin_slice::slice(files, diff),
        SlicingAlgorithm::RelevantSlice => relevant_slice::slice(files, diff, config),
        SlicingAlgorithm::BarrierSlice => {
            barrier_slice::slice(files, diff, config, &barrier_slice::BarrierConfig::default())
        }
        SlicingAlgorithm::Chop => Ok(SliceResult::new(SlicingAlgorithm::Chop)),
        SlicingAlgorithm::Taint => taint::slice(files, diff, &taint::TaintConfig::default()),
        SlicingAlgorithm::ConditionedSlice => Ok(SliceResult::new(SlicingAlgorithm::ConditionedSlice)),
        SlicingAlgorithm::DeltaSlice => Ok(SliceResult::new(SlicingAlgorithm::DeltaSlice)),
        SlicingAlgorithm::SpiralSlice => {
            spiral_slice::slice(files, diff, config, &spiral_slice::SpiralConfig::default())
        }
        SlicingAlgorithm::CircularSlice => circular_slice::slice(files, diff),
        SlicingAlgorithm::QuantumSlice => quantum_slice::slice(files, diff, None),
        SlicingAlgorithm::HorizontalSlice => {
            horizontal_slice::slice(files, diff, &horizontal_slice::PeerPattern::Auto)
        }
        SlicingAlgorithm::VerticalSlice => {
            vertical_slice::slice(files, diff, &vertical_slice::VerticalConfig::default())
        }
        SlicingAlgorithm::AngleSlice => {
            angle_slice::slice(files, diff, &angle_slice::Concern::ErrorHandling)
        }
        SlicingAlgorithm::ThreeDSlice => {
            threed_slice::slice(files, diff, &threed_slice::ThreeDConfig::default())
        }
        // New theoretical extensions
        SlicingAlgorithm::AbsenceSlice => absence_slice::slice(files, diff),
        SlicingAlgorithm::ResonanceSlice => {
            resonance_slice::slice(files, diff, &resonance_slice::ResonanceConfig::default())
        }
        SlicingAlgorithm::SymmetrySlice => symmetry_slice::slice(files, diff),
        SlicingAlgorithm::GradientSlice => {
            gradient_slice::slice(files, diff, &gradient_slice::GradientConfig::default())
        }
        SlicingAlgorithm::ProvenanceSlice => provenance_slice::slice(files, diff),
        SlicingAlgorithm::PhantomSlice => {
            phantom_slice::slice(files, diff, &phantom_slice::PhantomConfig::default())
        }
        SlicingAlgorithm::MembraneSlice => membrane_slice::slice(files, diff),
        SlicingAlgorithm::EchoSlice => echo_slice::slice(files, diff),
    }
}
