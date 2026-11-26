mod document;
mod path_item;
mod raw;

pub(super) use document::{contains_member, Document};
pub(super) use path_item::{Field, PathItem};
pub(super) use raw::{parse as parse_json, Parsed};

use super::servers::Uri;
use super::Site;
use crate::element::decode_fragment_pointer;
use crate::revision::{AdmittedSchemaInput, Format};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Documents<'a> {
    pub entry: Document<'a>,
    admitted: BTreeMap<String, Vec<Document<'a>>>,
}

pub(super) struct Located<'r, 'a> {
    pub value: &'r Value,
    pub document: &'r Document<'a>,
    pub site: Site,
}

impl<'r, 'a> Located<'r, 'a> {
    pub fn child(&self, key: &str) -> Option<Self> {
        Some(Self {
            value: self.value.get(key)?,
            document: self.document,
            site: self.site.child(key),
        })
    }

    pub fn duplicate(&self, member: &str) -> bool {
        self.document.duplicate(&self.site.pointer, member)
    }

    pub fn has_duplicates(&self) -> bool {
        self.document.has_duplicates(&self.site.pointer)
    }
}

pub(super) struct ReferenceFailure {
    pub code: &'static str,
    pub site: Site,
    pub chain: Vec<Site>,
}

impl<'a> Documents<'a> {
    pub fn new(input: &AdmittedSchemaInput<'a>, parsed: Parsed) -> Self {
        let entry = Document::new(
            input.entry,
            parsed,
            input.retrieval_uri.and_then(Uri::absolute),
        );
        let mut admitted: BTreeMap<String, Vec<Document<'a>>> = BTreeMap::new();
        for record in input.references {
            let Some(base) = Uri::absolute(record.retrieval_uri) else {
                continue;
            };
            let entries = admitted.entry(base.key()).or_default();
            if entries.iter().any(|doc| doc.blob == record.blob) {
                continue;
            }
            // Keep unread blobs in the URI inventory so a conflicting receipt cannot be ignored.
            let parsed = if record.blob.format == Format::Json {
                parse_json(record.blob.bytes).ok()
            } else {
                None
            };
            let readable = parsed.is_some();
            let mut document = Document::new(record.blob, parsed.unwrap_or_default(), Some(base));
            document.readable = readable;
            entries.push(document);
        }
        Self { entry, admitted }
    }

    pub fn locate(&self, resource: Option<&str>, site: &Site) -> Option<Located<'_, 'a>> {
        let document = match resource {
            Some(key) if self.entry.base.as_ref().is_none_or(|uri| uri.key() != key) => {
                self.admitted.get(key)?.first()?
            }
            _ => &self.entry,
        };
        Some(Located {
            value: lookup(&document.value, &site.pointer)?,
            document,
            site: site.clone(),
        })
    }

    pub fn documents(&self) -> impl Iterator<Item = &Document<'a>> {
        std::iter::once(&self.entry).chain(self.admitted.values().filter_map(|docs| {
            match docs.as_slice() {
                [document] if document.readable => Some(document),
                _ => None,
            }
        }))
    }

    pub fn resolve<'r>(
        &'r self,
        mut current: Located<'r, 'a>,
    ) -> Result<Vec<Located<'r, 'a>>, Box<ReferenceFailure>> {
        let mut locations = Vec::new();
        let mut visited = BTreeSet::new();
        let (code, site) = loop {
            let base = current.document.base.as_ref().map(Uri::key);
            if !visited.insert((base, current.site.clone())) {
                break ("recursive_reference", current.site);
            }
            if !current.value.is_object() {
                break ("declaration_unreadable", current.site);
            }
            if current.duplicate("$ref") {
                break ("duplicate_member", current.site);
            }
            let target = current
                .value
                .get("$ref")
                .map(|reference| self.reference_target(current.document, reference));
            locations.push(current);
            match target {
                None => return Ok(locations),
                Some(Ok(target)) => current = target,
                Some(Err(code)) => break (code, locations.last().unwrap().site.clone()),
            }
        };
        let chain = locations
            .iter()
            .map(|node| node.site.child("$ref"))
            .collect();
        Err(Box::new(ReferenceFailure { code, site, chain }))
    }

    pub fn reference_target<'r>(
        &'r self,
        document: &'r Document<'a>,
        reference: &Value,
    ) -> Result<Located<'r, 'a>, &'static str> {
        let reference = reference.as_str().ok_or("reference_pointer_unchecked")?;
        let (address, fragment) = reference.split_once('#').unwrap_or((reference, ""));
        let pointer =
            decode_fragment_pointer(fragment).map_err(|_| "reference_pointer_unchecked")?;
        let document = self.document_for(document, address)?;
        if document.ambiguous_path(&pointer) {
            return Err("duplicate_member");
        }
        let value = lookup(&document.value, &pointer).ok_or("reference_pointer_unchecked")?;
        let site = document.site(pointer);
        Ok(Located {
            value,
            document,
            site,
        })
    }

    fn document_for<'r>(
        &'r self,
        current: &'r Document<'a>,
        address: &str,
    ) -> Result<&'r Document<'a>, &'static str> {
        if address.is_empty() {
            return Ok(current);
        }
        let uri = Uri::parse(address)
            .ok_or("reference_uri_unchecked")?
            .resolve(current.base.as_ref())
            .filter(|uri| uri.origin.scheme.is_some())
            .ok_or("retrieval_base_unchecked")?;
        let key = uri.key();
        let entry_matches = self.entry.base.as_ref().is_some_and(|uri| uri.key() == key);
        match (entry_matches, self.admitted.get(&key).map(Vec::as_slice)) {
            (true, None) => Ok(&self.entry),
            (true, Some([other])) if other.blob == self.entry.blob => Ok(&self.entry),
            (false, Some([doc])) if doc.readable => Ok(doc),
            (true, Some(_)) | (false, Some([_, _, ..])) => Err("reference_blob_ambiguous"),
            _ => Err("reference_blob_unchecked"),
        }
    }
}

fn lookup<'a>(mut value: &'a Value, pointer: &[String]) -> Option<&'a Value> {
    for segment in pointer {
        value = if value.is_array() {
            if segment.starts_with('0') && segment.len() > 1
                || !segment.bytes().all(|b| b.is_ascii_digit())
            {
                return None;
            }
            value.get(segment.parse::<usize>().ok()?)?
        } else {
            value.get(segment)?
        };
    }
    Some(value)
}
