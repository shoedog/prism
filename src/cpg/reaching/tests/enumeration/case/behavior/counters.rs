use super::{DfgLabelStats, RdFileStats};

#[derive(Clone, Copy)]
pub(super) enum CounterField {
    LabelExact,
    LabelLoopCarried,
    LabelKilled,
    LabelOwnershipUncertain,
    LabelSameLine,
    LabelCfgIncomplete,
    LabelAliasUnstable,
    LabelCall,
    LabelCaptureImmediate,
    RdFunctionsOverCap,
    RdFunctionsWithoutCfg,
}

pub(super) fn select_counter(case_id: &str, field: &str) -> CounterField {
    match field {
        "dfg_label_exact" => CounterField::LabelExact,
        "dfg_label_loop_carried" => CounterField::LabelLoopCarried,
        "dfg_label_nameonly_killed" => CounterField::LabelKilled,
        "dfg_label_nameonly_ownership_uncertain" => CounterField::LabelOwnershipUncertain,
        "dfg_label_nameonly_sameline" => CounterField::LabelSameLine,
        "dfg_label_nameonly_cfg_incomplete" => CounterField::LabelCfgIncomplete,
        "dfg_label_nameonly_alias_unstable" => CounterField::LabelAliasUnstable,
        "dfg_label_nameonly_call" => CounterField::LabelCall,
        "dfg_label_capture_immediate" => CounterField::LabelCaptureImmediate,
        "dfg_rd_functions_over_cap" => CounterField::RdFunctionsOverCap,
        "dfg_rd_functions_without_cfg" => CounterField::RdFunctionsWithoutCfg,
        _ => panic!("{case_id}: unsupported measured counter {field}"),
    }
}

pub(super) fn counter_value(
    counter: CounterField,
    label_stats: &DfgLabelStats,
    rd_stats: &RdFileStats,
) -> i64 {
    (match counter {
        CounterField::LabelExact => label_stats.dfg_label_exact,
        CounterField::LabelLoopCarried => label_stats.dfg_label_loop_carried,
        CounterField::LabelKilled => label_stats.dfg_label_nameonly_killed,
        CounterField::LabelOwnershipUncertain => label_stats.dfg_label_nameonly_ownership_uncertain,
        CounterField::LabelSameLine => label_stats.dfg_label_nameonly_sameline,
        CounterField::LabelCfgIncomplete => label_stats.dfg_label_nameonly_cfg_incomplete,
        CounterField::LabelAliasUnstable => label_stats.dfg_label_nameonly_alias_unstable,
        CounterField::LabelCall => label_stats.dfg_label_nameonly_call,
        CounterField::LabelCaptureImmediate => rd_stats.capture_immediate_edges,
        CounterField::RdFunctionsOverCap => rd_stats.functions_over_cap,
        CounterField::RdFunctionsWithoutCfg => rd_stats.functions_without_cfg,
    }) as i64
}
