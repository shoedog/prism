use super::{c, c_wave, rows, BindingRow};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::sync::OnceLock;

pub(crate) fn c_slice3_wave_applies(parsed: &ParsedFile) -> bool {
    parsed.language == Language::C && !parsed.path.ends_with(".h")
}

pub(crate) fn rows_for_file(parsed: &ParsedFile) -> &'static [BindingRow] {
    if parsed.language == Language::C && !c_slice3_wave_applies(parsed) {
        static C_HEADER_ROWS: OnceLock<Vec<BindingRow>> = OnceLock::new();
        C_HEADER_ROWS.get_or_init(|| {
            c::pre_e4b_rows()
                .iter()
                .chain(c_wave::PRE_E4B_ROWS)
                .chain(c_wave::ROWS)
                .copied()
                .collect()
        })
    } else {
        rows(parsed.language)
    }
}
