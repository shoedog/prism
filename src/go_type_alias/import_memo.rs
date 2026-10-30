//! A phase owns both immutable inputs, so a file key also fixes its local path.
use crate::ast::ParsedFile;
use std::cell::OnceCell;
use std::collections::BTreeMap;

type Imports = BTreeMap<String, String>;

pub(crate) struct SignatureImportMemo<'a> {
    files: &'a BTreeMap<String, ParsedFile>,
    local_paths: &'a BTreeMap<String, String>,
    entries: BTreeMap<&'a str, OnceCell<Imports>>,
    missing: Imports,
}

impl<'a> SignatureImportMemo<'a> {
    pub(crate) fn new(
        files: &'a BTreeMap<String, ParsedFile>,
        local_paths: &'a BTreeMap<String, String>,
    ) -> Self {
        Self {
            files,
            local_paths,
            entries: files
                .keys()
                .map(|path| (path.as_str(), OnceCell::new()))
                .collect(),
            missing: BTreeMap::new(),
        }
    }

    pub(crate) fn get(&self, file: &str) -> &Imports {
        let Some(entry) = self.entries.get(file) else {
            return &self.missing;
        };
        entry.get_or_init(|| self.scan(file))
    }

    pub(crate) fn scan(&self, file: &str) -> Imports {
        super::signature_imports(
            &self.files[file],
            self.local_paths.get(file).map(String::as_str),
        )
    }
}

pub(super) enum SignatureImportSource<'a> {
    Maps(&'a BTreeMap<String, Imports>),
    Memo(&'a SignatureImportMemo<'a>),
}

impl SignatureImportSource<'_> {
    pub(super) fn get(&self, path: &str) -> Option<&Imports> {
        match self {
            Self::Maps(maps) => maps.get(path),
            Self::Memo(memo) => Some(memo.get(path)),
        }
    }
}

#[cfg(test)]
pub(crate) mod reference;
#[cfg(test)]
mod tests;

#[cfg(test)]
thread_local! { static SCANS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

#[cfg(test)]
pub(super) fn record_scan() {
    SCANS.with(|n| n.set(n.get() + 1));
}

#[cfg(test)]
pub(crate) fn measured<R>(f: impl FnOnce() -> R) -> (R, usize) {
    SCANS.with(|n| n.set(0));
    let result = f();
    (result, SCANS.with(std::cell::Cell::get))
}
