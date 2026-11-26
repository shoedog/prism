use super::servers::Uri;
use super::{Diagnostic, Site};
use crate::element::decode_fragment_pointer;
use crate::revision::{AdmittedSchemaInput, Blob, Format};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Document<'a> {
    pub value: Value,
    pub base: Option<Uri>,
    blob: Blob<'a>,
}

impl Document<'_> {
    pub fn site(&self, pointer: Vec<String>) -> Site {
        Site {
            file: self.blob.path.into(),
            tree: self.blob.tree.into(),
            blob: self.blob.oid.into(),
            pointer,
        }
    }
}

pub(super) struct Documents<'a> {
    pub entry: Document<'a>,
    admitted: BTreeMap<String, Vec<Document<'a>>>,
}

pub(super) struct Resolved<'r, 'a> {
    pub value: &'r Value,
    pub document: &'r Document<'a>,
    pub site: Site,
    pub chain: Vec<Site>,
}

impl<'a> Documents<'a> {
    pub fn new(input: &AdmittedSchemaInput<'a>, value: Value) -> Self {
        let entry = Document {
            value,
            blob: input.entry,
            base: input.retrieval_uri.and_then(Uri::absolute),
        };
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
            let value = if record.blob.format == Format::Json {
                serde_json::from_slice(record.blob.bytes).unwrap_or(Value::Null)
            } else {
                Value::Null
            };
            entries.push(Document {
                value,
                base: Some(base),
                blob: record.blob,
            });
        }
        Self { entry, admitted }
    }

    pub fn resolve<'r>(
        &'r self,
        mut document: &'r Document<'a>,
        mut value: &'r Value,
        mut site: Site,
        scope: &'static str,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<Resolved<'r, 'a>> {
        let mut chain = Vec::new();
        let mut visited = BTreeSet::new();
        let code = loop {
            if !visited.insert((document.base.as_ref().map(Uri::key), site.clone())) {
                break "recursive_reference";
            }
            if !value.is_object() {
                break "declaration_unreadable";
            }
            let Some(reference) = value.get("$ref") else {
                return Some(Resolved {
                    value,
                    document,
                    site,
                    chain,
                });
            };
            if scope == "path_item" && value.as_object().is_some_and(|members| members.len() > 1) {
                diagnostics.push(Diagnostic {
                    code: "path_item_reference_siblings",
                    scope,
                    site: site.clone(),
                });
            }
            let Some(reference) = reference.as_str() else {
                break "reference_pointer_unchecked";
            };
            let (address, fragment) = reference.split_once('#').unwrap_or((reference, ""));
            let pointer = match decode_fragment_pointer(fragment) {
                Ok(pointer) => pointer,
                Err(_) => break "reference_pointer_unchecked",
            };
            let target = match self.document_for(document, address) {
                Ok(doc) => doc,
                Err(code) => break code,
            };
            let Some(resolved) = lookup(&target.value, &pointer) else {
                break "reference_pointer_unchecked";
            };
            chain.push(site.child("$ref"));
            site = target.site(pointer);
            document = target;
            value = resolved;
        };
        diagnostics.push(Diagnostic { code, scope, site });
        None
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
            .ok_or("retrieval_base_unchecked")?;
        if uri.origin.scheme.is_none() {
            return Err("retrieval_base_unchecked");
        }
        let key = uri.key();
        let entry_matches = self.entry.base.as_ref().is_some_and(|uri| uri.key() == key);
        match (entry_matches, self.admitted.get(&key).map(Vec::as_slice)) {
            (true, None) => Ok(&self.entry),
            (true, Some([other])) if other.blob == self.entry.blob => Ok(&self.entry),
            (false, Some([doc])) if doc.value.is_object() => Ok(doc),
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
