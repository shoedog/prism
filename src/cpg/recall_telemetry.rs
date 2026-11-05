//! Recall telemetry shares the dfg-stats/call-stats projection and cache path.
use super::DfgLabelStats;
use crate::data_flow::DataFlowGraph;

impl DfgLabelStats {
    pub(super) fn record_recall_stats(&mut self, dfg: &DataFlowGraph) {
        self.dfg_recall_candidates = 0;
        self.dfg_recall_max_candidates = 0;
        self.dfg_recall_functions_capped = 0;
        self.dfg_recall_candidates_capped = 0;
        self.dfg_recall_functions_over_rd_cap = 0;
        self.dfg_recall_functions_without_cfg = 0;
        for stats in dfg.rd_function_stats().values() {
            let r = &stats.recall;
            self.dfg_recall_candidates += r.candidates();
            self.dfg_recall_max_candidates = self.dfg_recall_max_candidates.max(r.max_candidates());
            self.dfg_recall_functions_capped += r.functions_capped();
            self.dfg_recall_candidates_capped += r.candidates_capped();
            self.dfg_recall_functions_over_rd_cap += r.functions_over_cap();
            self.dfg_recall_functions_without_cfg += r.functions_without_cfg();
        }
    }
}
