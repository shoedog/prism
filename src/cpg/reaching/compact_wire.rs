//! Compact-only RD decoder: check set sequences before collecting private keys.
use super::{FallbackFunctionKey, RdFileStats, RdFunctionKey};
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
struct WireRdFileStats {
    functions_over_cap: usize,
    functions_without_cfg: usize,
    fallback_functions_capped: usize,
    fallback_edges_capped: usize,
    dfg_pruned_write_only: usize,
    dfg_pruned_scope: usize,
    dfg_pruned_dominated: usize,
    dfg_pruned_order: usize,

    over_cap_function_keys: Vec<RdFunctionKey>,
    without_cfg_function_keys: Vec<RdFunctionKey>,
    fallback_capped_function_counts: Vec<(FallbackFunctionKey, usize)>,
    recall: super::recall_stats::RecallStats,
    loop_plan: std::collections::BTreeMap<String, usize>,
}
impl RdFileStats {
    pub(crate) fn deserialize_compact_rows<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Vec<(String, Self)>, D::Error> {
        let rows = Vec::<(String, WireRdFileStats)>::deserialize(d)?;
        rows.into_iter()
            .map(|(file, s)| {
                for keys in [&s.over_cap_function_keys, &s.without_cfg_function_keys] {
                    if keys.windows(2).any(|w| w[0] >= w[1]) {
                        return Err(serde::de::Error::custom("DFG RD set order/duplicate"));
                    }
                }
                if s.fallback_capped_function_counts
                    .windows(2)
                    .any(|w| w[0].0 >= w[1].0)
                {
                    return Err(serde::de::Error::custom("DFG RD cap map order/duplicate"));
                }
                Ok((
                    file,
                    Self {
                        functions_over_cap: s.functions_over_cap,
                        functions_without_cfg: s.functions_without_cfg,
                        fallback_functions_capped: s.fallback_functions_capped,
                        fallback_edges_capped: s.fallback_edges_capped,
                        fallback_capped_function_counts: s
                            .fallback_capped_function_counts
                            .into_iter()
                            .collect(),
                        dfg_pruned_write_only: s.dfg_pruned_write_only,
                        dfg_pruned_scope: s.dfg_pruned_scope,
                        dfg_pruned_dominated: s.dfg_pruned_dominated,
                        dfg_pruned_order: s.dfg_pruned_order,
                        recall: s.recall,
                        loop_plan: s.loop_plan,

                        over_cap_function_keys: s.over_cap_function_keys.into_iter().collect(),
                        without_cfg_function_keys: s
                            .without_cfg_function_keys
                            .into_iter()
                            .collect(),
                    },
                ))
            })
            .collect()
    }
}

// JSON cannot encode tuple map keys. Ordered rows also retain the native codec's
// map-length/key/value byte layout without allocating during serialization.
pub(super) mod cap_counts {
    use super::FallbackFunctionKey;
    use serde::{Deserialize, Deserializer, Serializer};
    use std::collections::BTreeMap;

    pub fn serialize<S: Serializer>(
        counts: &BTreeMap<FallbackFunctionKey, usize>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        s.collect_seq(counts)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<BTreeMap<FallbackFunctionKey, usize>, D::Error> {
        let rows = Vec::<(FallbackFunctionKey, usize)>::deserialize(d)?;
        if rows.windows(2).any(|w| w[0].0 >= w[1].0) {
            return Err(serde::de::Error::custom("DFG RD cap map order/duplicate"));
        }
        Ok(rows.into_iter().collect())
    }
}
