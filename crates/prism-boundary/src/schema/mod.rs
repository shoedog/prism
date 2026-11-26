mod messages;
mod openapi;
mod references;
mod servers;
pub(crate) mod signature;
pub(crate) mod site;
mod status;
mod wire;

use crate::element::{CanonicalPath, OperationId, SuccessKey};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;

pub(super) use messages::project;
pub(crate) use messages::valid_media_proof;
pub(super) use openapi::read_operations;
use servers::Deployment;
pub use signature::signature;
use site::{Diagnostic, Site};
use status::Response;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Dialect {
    Swagger2,
    OpenApi30,
    OpenApi31,
}

#[derive(Debug)]
pub(super) struct Inventory {
    pub api: String,
    pub authority: String,
    pub digest: [u8; 32],
    pub info_version: Option<String>,
    pub operations: BTreeMap<OperationId, Operation>,
    pub unprojected: Vec<Declaration>,
    pub operation_gaps: Vec<crate::inventory::OperationGap>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug)]
pub(super) struct Operation {
    pub id: OperationId,
    pub operation_id: Option<String>,
    pub declarations: Vec<Declaration>,
    dialect: Dialect,
}

#[derive(Clone, Debug)]
pub(super) struct Declaration {
    pub source_path: String,
    pub source_method: String,
    pub path: CanonicalPath,
    pub referring_site: Site,
    pub site: Site,
    pub reference_chain: Vec<Site>,
    resource: Option<String>,
    parameters: Result<Option<(Option<String>, Site)>, ()>,
    pub deployments: Vec<Deployment>,
    pub responses: BTreeMap<String, Response>,
    pub success_keys: Vec<SuccessKey>,
    pub value: Value,
}

#[derive(Debug)]
pub enum SchemaError {
    Yaml,
    Json(serde_json::Error),
    Dialect,
    EntryTree,
}

impl SchemaError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Yaml => "yaml_format",
            Self::Json(_) => "invalid_json",
            Self::Dialect => "unsupported_schema_dialect",
            Self::EntryTree => "entry_tree_mismatch",
        }
    }

    pub fn scope(&self) -> &'static str {
        if matches!(self, Self::EntryTree) {
            "schema_source"
        } else {
            "schema_format"
        }
    }
}

pub(crate) fn parse_json(bytes: &[u8]) -> serde_json::Result<references::Parsed> {
    references::parse_json(bytes)
}

pub(crate) fn supported_retrieval_uri(uri: &str) -> bool {
    servers::Uri::absolute(uri).is_some()
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())?;
        if let Self::Json(error) = self {
            write!(f, " ({error})")?;
        }
        Ok(())
    }
}

impl std::error::Error for SchemaError {}
