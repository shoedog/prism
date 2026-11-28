use super::{Documents, Located};
use crate::schema::{Diagnostic, Site};
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

pub(in crate::schema) struct PathItem<'r, 'a> {
    pub site: Site,
    fields: BTreeMap<&'r str, Field<'r, 'a>>,
}

pub(in crate::schema) enum Field<'r, 'a> {
    Declared {
        location: Located<'r, 'a>,
        chain: Vec<Site>,
    },
    Overlap,
}

impl<'r, 'a> PathItem<'r, 'a> {
    pub fn resolve(
        documents: &'r Documents<'a>,
        root: Located<'r, 'a>,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<Self> {
        let site = root.site.clone();
        let locations = match documents.resolve(root) {
            Ok(locations) => locations,
            Err(failure) => {
                diagnostics.push(Diagnostic {
                    code: failure.code,
                    scope: "path_item",
                    site,
                    reference_chain: failure.chain,
                    capture: None,
                });
                return None;
            }
        };
        let mut fields = BTreeMap::new();
        let mut chain = Vec::new();
        for item in locations {
            for (name, value) in item.value.as_object().unwrap() {
                if name == "$ref" {
                    continue;
                }
                if item.duplicate(name) && !matches!(name.as_str(), "parameters" | "servers") {
                    diagnostics.push(Diagnostic::new(
                        "duplicate_member",
                        "path_item",
                        item.site.child(name),
                    ));
                    fields.insert(name.as_str(), Field::Overlap);
                    continue;
                }
                match fields.entry(name.as_str()) {
                    Entry::Vacant(entry) => {
                        entry.insert(Field::Declared {
                            location: Located {
                                value,
                                document: item.document,
                                site: item.site.child(name),
                            },
                            chain: chain.clone(),
                        });
                    }
                    Entry::Occupied(mut entry) => {
                        if !matches!(entry.get(), Field::Overlap) {
                            diagnostics.push(Diagnostic::new(
                                "path_item_field_unchecked",
                                "path_item",
                                site.child(name),
                            ));
                            entry.insert(Field::Overlap);
                        }
                    }
                }
            }
            if item.value.get("$ref").is_some() {
                chain.push(item.site.child("$ref"));
            }
        }
        Some(Self { site, fields })
    }

    pub fn field(&self, name: &str) -> Option<&Field<'r, 'a>> {
        self.fields.get(name)
    }
}
