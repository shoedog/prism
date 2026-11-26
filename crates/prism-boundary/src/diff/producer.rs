use super::DiffBasis;
use crate::element::{CanonicalPath, OperationId};
use crate::revision::DependencyReceipt;
use crate::Coverage;
use std::collections::BTreeMap;
use std::ops::Range;

#[derive(Debug)]
pub struct ProducerInventory<'a> {
    pub basis: DiffBasis<'a>,
    pub coverage: Coverage,
    pub operations: Vec<ProducerOperation<'a>>,
}

#[derive(Debug)]
pub struct ProducerOperation<'a> {
    pub operation: OperationId,
    pub declaration: ProducerFact<'a, RouteDeclaration>,
    pub validation: ProducerFact<'a, Validator>,
    pub reference: ProducerFact<'a, HandlerReference>,
    pub code: ProducerFact<'a, HandlerBody>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProducerSlot {
    Declaration,
    Validation,
    Reference,
    Code,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProducerSiteKind {
    Declaration,
    Registration,
    Annotation,
    Validation,
    Reference,
    Endpoint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProducerSite {
    pub kind: ProducerSiteKind,
    pub tree: String,
    pub file: String,
    pub blob: String,
    pub span: Range<usize>,
}

#[derive(Debug)]
pub enum Fact<T> {
    Checked(Option<T>),
    Withheld { reason: String },
}

#[derive(Debug)]
pub struct ProducerFact<'a, T> {
    pub fact: Fact<T>,
    pub sites: Vec<ProducerSite>,
    pub dependencies: Vec<DependencyReceipt<'a>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RouteDeclaration {
    pub original_method: String,
    pub original_path: String,
    pub canonical_path: CanonicalPath,
    pub attributes: BTreeMap<String, AttributeValue>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AttributeValue {
    Boolean(bool),
    Integer(i64),
    String(String),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Validator {
    pub identity: Option<String>,
    pub digest: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HandlerEndpoint {
    Resolved(String),
    Inline,
}

#[derive(Debug, PartialEq, Eq)]
pub struct HandlerReference {
    pub expression: String,
    pub endpoint: HandlerEndpoint,
}

#[derive(Debug, PartialEq, Eq)]
pub struct HandlerBody {
    pub endpoint: HandlerEndpoint,
    pub digest: String,
}
