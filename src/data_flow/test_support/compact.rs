//! Read-only compact storage observation for ordinary unit regressions.
use super::*;
use crate::data_flow::compact::CompactDfg;

pub(in crate::data_flow) fn storage(g: &CompactDfg) -> Vec<u8> {
    bytes(&(
        g.edges().iter().map(|e| e.to_owned()).collect::<Vec<_>>(),
        g.defs().map(|(k, v)| (k, v.to_vec())).collect::<Vec<_>>(),
        g.uses().map(|(k, v)| (k, v.to_vec())).collect::<Vec<_>>(),
        g.forward()
            .map(|(k, v)| (k, v.to_vec()))
            .collect::<Vec<_>>(),
        g.backward()
            .map(|(k, v)| (k, v.to_vec()))
            .collect::<Vec<_>>(),
        g.labels()
            .map(|(e, c)| ((e.from, e.to), c))
            .collect::<Vec<_>>(),
        g.exact_labels().collect::<Vec<_>>(),
        g.rd_function_stats(),
    ))
}
