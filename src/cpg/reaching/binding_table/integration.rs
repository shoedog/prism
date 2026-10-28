use super::{rows, BindingRow};
use crate::ast::ParsedFile;
use crate::languages::Language;

pub(crate) fn c_slice3_wave_applies(parsed: &ParsedFile) -> bool {
    parsed.language == Language::C
}

pub(crate) fn rows_for_file(parsed: &ParsedFile) -> &'static [BindingRow] {
    rows(parsed.language)
}
