use crate::revision::Blob;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Site {
    pub file: String,
    pub tree: String,
    pub blob: String,
    pub pointer: Vec<String>,
}

impl Site {
    pub(super) fn from_blob(blob: Blob<'_>, pointer: Vec<String>) -> Self {
        Self {
            file: blob.path.into(),
            tree: blob.tree.into(),
            blob: blob.oid.into(),
            pointer,
        }
    }

    pub(super) fn child(&self, member: &str) -> Self {
        let mut site = self.clone();
        site.pointer.push(member.into());
        site
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Diagnostic {
    pub code: &'static str,
    pub scope: &'static str,
    pub site: Site,
    pub reference_chain: Vec<Site>,
    pub capture: Option<String>,
}

impl Diagnostic {
    pub(super) fn new(code: &'static str, scope: &'static str, site: Site) -> Self {
        Self {
            code,
            scope,
            site,
            reference_chain: Vec::new(),
            capture: None,
        }
    }
}
