//! Compact-only RD decoder: check set sequences before collecting private keys.
use super::{NoBodyEntryKey, RdFileStats, RdFunctionKey};
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
struct WireRdFileStats {
    functions_over_cap: usize,
    functions_without_cfg: usize,
    no_body_entry: usize,
    over_cap_function_keys: Vec<RdFunctionKey>,
    without_cfg_function_keys: Vec<RdFunctionKey>,
    no_body_entry_keys: Vec<NoBodyEntryKey>,
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
                if s.no_body_entry_keys.windows(2).any(|w| w[0] >= w[1]) {
                    return Err(serde::de::Error::custom("DFG RD set order/duplicate"));
                }
                Ok((
                    file,
                    Self {
                        functions_over_cap: s.functions_over_cap,
                        functions_without_cfg: s.functions_without_cfg,
                        no_body_entry: s.no_body_entry,
                        over_cap_function_keys: s.over_cap_function_keys.into_iter().collect(),
                        without_cfg_function_keys: s
                            .without_cfg_function_keys
                            .into_iter()
                            .collect(),
                        no_body_entry_keys: s.no_body_entry_keys.into_iter().collect(),
                    },
                ))
            })
            .collect()
    }
}
