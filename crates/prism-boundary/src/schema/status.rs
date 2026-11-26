use super::{Diagnostic, Dialect, Operation, Site};
use crate::element::SuccessKey;
use serde_json::Value;
use std::collections::BTreeMap;

impl Operation {
    pub fn default_response(&self) -> Option<&Response> {
        self.responses.get("default")
    }

    pub fn response_key(&self, code: u16) -> Option<&str> {
        if !(100..=599).contains(&code) {
            return None;
        }
        let key = code.to_string();
        self.responses
            .get_key_value(&key)
            .or_else(|| {
                (self.dialect == Dialect::OpenApi3)
                    .then(|| self.responses.get_key_value(&format!("{}XX", code / 100)))
                    .flatten()
            })
            .map(|(key, _)| key.as_str())
            .or_else(|| self.default_response().map(|_| "default"))
    }

    pub fn can_have_body(&self, code: u16) -> bool {
        self.id.method() != "HEAD" && !matches!(code, 100..=199 | 204 | 205 | 304)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Response {
    pub site: Site,
    pub value: Value,
}

pub(super) fn responses(
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
            diagnostics.push(Diagnostic {
                code: "responses_unreadable",
                scope: "responses",
                site,
            });
            BTreeMap::new()
        }
        None => BTreeMap::new(),
    }
}

pub(super) fn success_key(key: &str, dialect: Dialect) -> Option<SuccessKey> {
    match key.as_bytes() {
        b"2XX" if dialect == Dialect::OpenApi3 => Some(SuccessKey::Range),
        [b'2', b'0'..=b'9', b'0'..=b'9'] => Some(SuccessKey::Exact(key.parse().unwrap())),
        _ => None,
    }
}
