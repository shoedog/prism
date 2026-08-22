//! Declaration-provenance snapshots for Go owner-identity lanes.
//!
//! Build profiles are intentionally not part of [`GoOwnerIdentity`]. Each
//! snapshot retains its defining file so consumers can apply the caller's
//! package/build visibility and certainty floor at consult time.

use crate::resolution::GoOwnerIdentity;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GoStructDeclaration {
    pub defining_file: String,
    pub fields: BTreeMap<String, String>,
    pub embedded_types: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GoInterfaceDeclaration {
    pub defining_file: String,
    pub methods: BTreeSet<String>,
    pub embedded_types: BTreeSet<String>,
    pub generic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GoMethodDeclaration {
    pub defining_file: String,
    pub method_name: String,
}

pub type GoStructDeclarations = BTreeMap<GoOwnerIdentity, BTreeSet<GoStructDeclaration>>;
pub type GoInterfaceDeclarations = BTreeMap<GoOwnerIdentity, BTreeSet<GoInterfaceDeclaration>>;
pub type GoMethodDeclarations = BTreeMap<GoOwnerIdentity, BTreeSet<GoMethodDeclaration>>;
