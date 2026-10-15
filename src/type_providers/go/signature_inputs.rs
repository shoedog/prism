//! One signature-import scan per file extraction; no cache survives the file.
use crate::ast::ParsedFile;
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
        }
    }

    fn scan(&self) -> BTreeMap<String, String> {
        #[cfg(test)]
        SCANS.with(|n| n.set(n.get() + 1));
        crate::go_type_alias::signature_imports(self.parsed, self.local_import_path)
    }

    pub(super) fn get(&self) -> Cow<'_, BTreeMap<String, String>> {
        Cow::Owned(self.scan())
    }
}

#[cfg(test)]
thread_local! { static SCANS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

#[cfg(test)]
pub(super) fn measured<R>(f: impl FnOnce() -> R) -> (R, usize) {
    SCANS.with(|n| n.set(0));
    let result = f();
    (result, SCANS.with(std::cell::Cell::get))
}
