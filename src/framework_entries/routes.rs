//! Source registrations and their query-local projection.
// Checkpoint 2 connects the source facts to the route table.
#[allow(dead_code)]
mod handler;
pub mod mounts;
#[allow(dead_code)]
mod python;
#[allow(dead_code)]
mod receivers;
mod table;
#[cfg(test)]
mod tests;

pub use receivers::{EntryKind, Invocation, RootEvidence, RootSelector, RouterId};
pub use table::RouteTable;

use crate::ast::ParsedFile;
use crate::call_graph::CallGraph;
use crate::manifest_snapshot::ManifestSnapshot;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    ReceiverUnproven,
    ImportedUnproven,
    TerminalReexport,
    MountChildUnproven,
    TableConsumerUnproven,
    NoKnownMount,
    VariablePrefix,
    NonLiteralPath,
    RegexPath,
    MountCycle,
    ExpansionTruncated,
    CallbackUnproven,
    MemberHandler,
    WrapperCall,
    InnerDecorator,
    ClassView,
    AnonymousUnindexed,
    SpreadCallback,
    HandlerObject,
    MethodValueReceiverUnproven,
    MethodValueReceiverShadowed,
    MethodValueMethodUnproven,
    MethodValueInterface,
    MethodValueExternal,
    FuncFieldValue,
    QualifiedFunctionValue,
    MethodExpression,
    HapiRegistrationUnproven,
    HapiPluginUnproven,
    HapiPluginUnregistered,
    HapiServerParamUnproven,
    HapiRouteNotLiteral,
    HapiHandlerObject,
    HapiHandlerConflict,
    RuntimeGeneratedRegistration,
    UnsupportedRegistration,
    BuildOnly,
    Superseded,
    DispatchConflict,
    EntryUnproven,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[error("entry_unproven: {entry:?}")]
pub struct EntryError {
    pub reason: Reason,
    pub entry: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Site {
    pub file: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Framework {
    Chi,
    Django,
    Express,
    #[serde(rename = "fastapi")]
    FastApi,
    Flask,
    Gin,
    Hapi,
    #[serde(rename = "nestjs")]
    NestJs,
    #[serde(rename = "net/http")]
    NetHttp,
    ObjectTable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Http,
    Websocket,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Receiver {
    Router {
        #[serde(flatten)]
        identity: RouterId,
        #[serde(skip_serializing_if = "Option::is_none")]
        proof: Option<ReceiverProof>,
    },
    Unproven {
        reason: Reason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiverProof {
    PluginContract,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginOrigin {
    pub file: String,
    pub object_byte: usize,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum MethodSet {
    Known { known: BTreeSet<String> },
    Any(AnyMethod),
    Unknown { status: UnknownMethod, text: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnyMethod {
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnknownMethod {
    Unknown,
}

impl MethodSet {
    pub const ANY: Self = Self::Any(AnyMethod::Any);

    pub fn known(methods: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::Known {
            known: methods.into_iter().map(Into::into).collect(),
        }
    }

    pub fn unknown(text: impl Into<String>) -> Self {
        Self::Unknown {
            status: UnknownMethod::Unknown,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredPath {
    pub raw: String,
    pub literal: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Declaration {
    pub methods: Option<MethodSet>,
    pub path: DeclaredPath,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(from = "TokenWire", into = "TokenWire")]
pub enum PathToken {
    Literal(String),
    Parameter,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum TokenWire {
    Literal((LiteralTag, String)),
    Parameter((ParameterTag,)),
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LiteralTag {
    Literal,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ParameterTag {
    Parameter,
}

impl From<TokenWire> for PathToken {
    fn from(value: TokenWire) -> Self {
        match value {
            TokenWire::Literal((_, text)) => Self::Literal(text),
            TokenWire::Parameter(_) => Self::Parameter,
        }
    }
}

impl From<PathToken> for TokenWire {
    fn from(value: PathToken) -> Self {
        match value {
            PathToken::Literal(text) => Self::Literal((LiteralTag::Literal, text)),
            PathToken::Parameter => Self::Parameter((ParameterTag::Parameter,)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HostConstraint {
    pub host_kind: HostKind,
    pub hosts: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostKind {
    Unconstrained,
    Literal,
    Parameterized,
    Regex,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Version {
    Header { header: String, value: String },
    Media { value: Option<String> },
    Custom { value: Option<String> },
    Unknown { value: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ExposureStatus {
    Full { value: String },
    Suffix { value: String, reason: Reason },
    Dynamic { reason: Reason },
}

impl ExposureStatus {
    pub fn value(&self) -> Option<&str> {
        match self {
            Self::Full { value } | Self::Suffix { value, .. } => Some(value),
            Self::Dynamic { .. } => None,
        }
    }

    fn rank(&self) -> u8 {
        match self {
            Self::Full { .. } => 0,
            Self::Suffix { .. } => 1,
            Self::Dynamic { .. } => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exposure {
    pub root: Option<RouterId>,
    #[serde(flatten)]
    pub status: ExposureStatus,
    pub path_tokens: Option<Vec<PathToken>>,
    pub normalized: Option<String>,
    #[serde(flatten)]
    pub host: HostConstraint,
    pub method_guard: MethodSet,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_blocked: Option<MethodSet>,
    pub version: Option<Version>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub strict_slashes: Option<Option<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setup_guard: Option<String>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub shadowed: Option<Option<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mount_refusal: Option<Reason>,
}

fn present_nullable<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer).map(Some)
}

impl Exposure {
    fn sort_key(&self) -> impl Ord + '_ {
        (
            (self.status.rank(), self.status.value()),
            &self.root,
            &self.host,
            &self.method_guard,
            &self.version,
            self.strict_slashes,
            &self.setup_guard,
            self.shadowed,
            &self.binding_blocked,
            self.mount_refusal,
            &self.path_tokens,
            (&self.normalized, &self.status),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallableId {
    pub file: String,
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CallbackStatus {
    Resolved {
        #[serde(flatten)]
        identity: CallableId,
        /// Argument spelling retained until the table checks the receiver.
        #[serde(skip)]
        argument_text: String,
    },
    Anonymous {
        #[serde(flatten)]
        identity: CallableId,
        #[serde(skip)]
        argument_text: String,
    },
    Unresolved {
        text: String,
        reason: Reason,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum CallbackRole {
    Pre { group: usize },
    Handler,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Callback {
    pub position: usize,
    #[serde(flatten)]
    pub status: CallbackStatus,
    pub delegate: Option<CallableId>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub role: Option<CallbackRole>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchCut {
    pub root: Option<RouterId>,
    pub method_guard: MethodSet,
    pub path: Option<String>,
    pub site: Site,
    pub reason: Reason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Route {
    pub framework: Framework,
    pub protocol: Protocol,
    pub site: Site,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_site: Option<Site>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_site: Option<Site>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_ordinal: Option<usize>,
    pub receiver: Receiver,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_plugin: Option<PluginOrigin>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub declaration: Option<Declaration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_methods: Option<MethodSet>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "candidate_is_absent")]
    pub candidate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub superseded_keys: Vec<DispatchCut>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<DispatchCut>,
    pub paths: Vec<Exposure>,
    pub callbacks: Vec<Callback>,
}

fn candidate_is_absent(candidate: &Option<bool>) -> bool {
    *candidate != Some(true)
}

impl Route {
    fn sort_key(&self) -> impl Ord + '_ {
        let first = self.paths.first();
        let spelling = self.declaration.as_ref().map(|record| &record.path);
        let raw = spelling.map(|path| path.raw.as_str());
        let spelling = match first {
            Some(path) => (
                path.normalized
                    .as_deref()
                    .or_else(|| path.status.value())
                    .or(raw),
                None,
            ),
            None => (
                spelling.and_then(|path| path.literal.as_deref()).or(raw),
                raw,
            ),
        };
        let methods = self
            .declaration
            .as_ref()
            .and_then(|record| record.methods.as_ref());
        let plugin = self
            .origin_plugin
            .as_ref()
            .map(|plugin| (&plugin.file, plugin.object_byte));
        (
            spelling,
            methods,
            &self.site,
            &self.install_site,
            self.element_ordinal,
            plugin,
            self.framework,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum MountPrefix {
    Literal { value: String },
    Absent,
    Unevaluated { text: String },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RealmFrame {
    pub file: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub item_ordinal: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountChild {
    #[serde(flatten)]
    pub receiver: Receiver,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realm: Option<Vec<RealmFrame>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MountTransform {
    Identity,
    StripPrefix,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mount {
    pub parent: Receiver,
    pub child: MountChild,
    pub site: Site,
    pub prefix: MountPrefix,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard_pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_guard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method_guard: Option<MethodSet>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub host: Option<HostConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setup_guard: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform: Option<MountTransform>,
    pub alternative_ordinal: usize,
    pub child_ordinal: usize,
}

impl Mount {
    fn sort_key(&self) -> impl Ord + '_ {
        (
            &self.site,
            self.alternative_ordinal,
            self.child_ordinal,
            &self.parent,
            &self.child.receiver,
            &self.child.realm,
            // Every other serialized field, so records that agree on the
            // fields above order the same whatever the inventory order.
            (
                &self.setup_guard,
                &self.prefix,
                &self.guard_pattern,
                self.end_guard,
                &self.method_guard,
                &self.host,
                self.transform,
            ),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Diagnostic {
    pub site: Site,
    pub reason: Reason,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FailedRoot {
    pub root: RouterId,
    pub site: Site,
    pub reason: Reason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    pub enabled_adapters: BTreeSet<Framework>,
    pub registrations: usize,
    pub proven: usize,
    pub unproven: usize,
    pub candidates: usize,
    pub mounts: usize,
    pub diagnostics: Vec<Diagnostic>,
    pub failed_roots: Vec<FailedRoot>,
}

/// Compute a listing from the current parsed sources and immutable repository facts.
pub fn routes(
    _files: &BTreeMap<String, ParsedFile>,
    _call_graph: &CallGraph,
    _manifests: &ManifestSnapshot,
    entry: Option<RootSelector>,
) -> Result<RouteTable, EntryError> {
    // Adapter availability is empty until a source extractor is wired in.
    if let Some(entry) = entry {
        entry.resolve(&[])?;
    }
    Ok(RouteTable::new(vec![], vec![], vec![], vec![], []))
}
