mod openapi;
mod references;
mod servers;
mod status;

use crate::element::{CanonicalPath, OperationId, SuccessKey};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;

pub(super) use openapi::read_operations;
use servers::Deployment;
use status::Response;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Dialect {
    Swagger2,
    OpenApi3,
}

#[derive(Debug)]
pub(super) struct Inventory {
    pub api: String,
    pub authority: String,
    pub digest: [u8; 32],
    pub info_version: Option<String>,
    pub operations: BTreeMap<OperationId, Operation>,
    pub unprojected: Vec<Declaration>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug)]
pub(super) struct Operation {
    pub id: OperationId,
    pub operation_id: Option<String>,
    pub declarations: Vec<Declaration>,
    pub responses: BTreeMap<String, Response>,
    pub success_keys: Vec<SuccessKey>,
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
    pub deployments: Vec<Deployment>,
    pub value: Value,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Site {
    pub file: String,
    pub tree: String,
    pub blob: String,
    pub pointer: Vec<String>,
}

impl Site {
    fn child(&self, member: &str) -> Self {
        let mut site = self.clone();
        site.pointer.push(member.into());
        site
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Diagnostic {
    pub code: &'static str,
    pub scope: &'static str,
    pub site: Site,
}

#[derive(Debug)]
pub(super) enum SchemaError {
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
