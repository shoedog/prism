use super::{Diagnostic, Dialect, Operation, Site};
use crate::element::SuccessKey;
use crate::inventory::{can_have_body, success_key};
use serde_json::Value;
use std::collections::BTreeMap;

impl Operation {
    pub fn responses(&self) -> &BTreeMap<String, Response> {
        &self.declarations[0].responses
    }

    pub fn success_keys(&self) -> &[SuccessKey] {
        &self.declarations[0].success_keys
    }

    pub fn default_response(&self) -> Option<&Response> {
        self.responses().get("default")
    }

    pub fn response_key(&self, code: u16) -> Option<&str> {
        if !(100..=599).contains(&code) {
            return None;
        }
        let key = code.to_string();
        self.responses()
            .get_key_value(&key)
            .or_else(|| {
                (self.dialect != Dialect::Swagger2)
                    .then(|| self.responses().get_key_value(&format!("{}XX", code / 100)))
                    .flatten()
            })
            .map(|(key, _)| key.as_str())
            .or_else(|| self.default_response().map(|_| "default"))
    }

    pub fn can_have_body(&self, code: u16) -> bool {
        can_have_body(&self.id, code)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Response {
    pub site: Site,
    pub value: Value,
}

pub(super) fn read_statuses(
    operation: &Value,
    site: &Site,
    dialect: Dialect,
    diagnostics: &mut Vec<Diagnostic>,
) -> (BTreeMap<String, Response>, Vec<SuccessKey>) {
    let responses = responses(operation, site, diagnostics);
    let success_keys = responses
        .keys()
        .filter_map(|key| success_key(key, dialect != Dialect::Swagger2))
        .collect::<Vec<_>>();
    if let Some(fallback) = responses.get("default") {
        // The default can describe a success only if neither 2XX nor all 100 exact codes (200–299) cover it.
        if !success_keys.contains(&SuccessKey::Range) && success_keys.len() < 100 {
            diagnostics.push(Diagnostic::new(
                "fallback_success",
                "response_body",
                fallback.site.clone(),
            ));
        }
    }
    (responses, success_keys)
}

fn responses(
    operation: &Value,
    site: &Site,
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<String, Response> {
    let site = site.child("responses");
    match operation.get("responses") {
        Some(Value::Object(responses)) => responses
            .iter()
            .map(|(key, value)| {
                (
                    key.clone(),
                    Response {
                        site: site.child(key),
                        value: value.clone(),
                    },
                )
            })
            .collect(),
        Some(_) => {
            diagnostics.push(Diagnostic::new("responses_unreadable", "responses", site));
            BTreeMap::new()
        }
        None => BTreeMap::new(),
    }
}
