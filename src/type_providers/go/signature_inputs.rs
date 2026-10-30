//! Reuse the phase's import scans for both aliases and signatures.
use crate::ast::ParsedFile;
use crate::go_type_alias::SignatureImportMemo;
use std::borrow::Cow;
use std::cell::OnceCell;
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub(super) enum ImportScan {
    Once,
    #[cfg(test)]
    EverySignature,
}

pub(super) struct FileSignatureImports<'a> {
    parsed: &'a ParsedFile,
    local_import_path: Option<&'a str>,
    mode: ImportScan,
    cached: OnceCell<BTreeMap<String, String>>,
    phase: Option<(&'a SignatureImportMemo<'a>, &'a str)>,
}

impl<'a> FileSignatureImports<'a> {
    pub(super) fn new(
        parsed: &'a ParsedFile,
        local_import_path: Option<&'a str>,
        mode: ImportScan,
    ) -> Self {
        Self {
            parsed,
            local_import_path,
            mode,
            cached: OnceCell::new(),
            phase: None,
        }
    }

    fn scan(&self) -> BTreeMap<String, String> {
        crate::go_type_alias::signature_imports(self.parsed, self.local_import_path)
    }

    pub(super) fn get(&self) -> Cow<'_, BTreeMap<String, String>> {
        match self.mode {
            ImportScan::Once => Cow::Borrowed(match self.phase {
                Some((memo, path)) => memo.get(path),
                None => self.cached.get_or_init(|| self.scan()),
            }),
            #[cfg(test)]
            ImportScan::EverySignature => Cow::Owned(match self.phase {
                Some((memo, path)) => memo.scan(path),
                None => self.scan(),
            }),
        }
    }
}

pub(super) fn extract_file(
    data: &mut super::GoTypeData,
    path: &str,
    parsed: &ParsedFile,
    memo: &SignatureImportMemo<'_>,
    aliases: &crate::go_type_alias::GoAliasResolver,
    mode: ImportScan,
) {
    let mut imports = FileSignatureImports::new(parsed, None, mode);
    imports.phase = Some((memo, path));
    super::GoTypeProvider::extract_from_file(data, path, parsed, &imports, aliases);
}

#[cfg(test)]
pub(super) use crate::go_type_alias::import_memo::measured;
