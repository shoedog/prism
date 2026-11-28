//! P2 producer bindings: Go source inputs and swaggo annotation bindings.
//!
//! The extraction core takes receipt-bound inputs and the admitted schema index. It does not
//! read the filesystem or Git. PROVISIONAL: it inherits the admission scope in the crate README.

use crate::model::{BindingsDocument, DocumentInput};
use crate::SchemaIndex;

/// The adapters that can bind source sites to operations and fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProducerAdapter {
    SwaggoAnnotation,
    GoStructField,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProducerOptions {
    pub adapters: Vec<ProducerAdapter>,
}

impl Default for ProducerOptions {
    /// P2-1 implements the annotation adapter only; the struct-field adapter lands in P2-2.
    fn default() -> Self {
        Self {
            adapters: vec![ProducerAdapter::SwaggoAnnotation],
        }
    }
}

/// One file the caller read from the named tree.
#[derive(Clone, Copy, Debug)]
pub struct SourceBlob<'a> {
    pub tree: &'a str,
    pub path: &'a str,
    pub blob: &'a str,
    pub bytes: &'a [u8],
}

/// One file the caller expects to be examined, with the blob id the tree lists for it.
#[derive(Clone, Copy, Debug)]
pub struct ExpectedFile<'a> {
    pub path: &'a str,
    pub blob: &'a str,
}

/// Receipt-bound source inputs. `input` carries the snapshot identity, the repository, the
/// role (producer), the authority (`annotation`) and the scope (API).
#[derive(Clone, Copy, Debug)]
pub struct ProducerInputs<'a> {
    pub input: &'a DocumentInput,
    pub expected: &'a [ExpectedFile<'a>],
    pub files: &'a [SourceBlob<'a>],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProducerError {
    NoAdapters,
    NotImplemented(ProducerAdapter),
    WrongRole,
    WrongAuthority,
    UnsupportedSnapshot,
    UnsupportedScope,
}

impl std::fmt::Display for ProducerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoAdapters => f.write_str("no producer adapter selected"),
            Self::NotImplemented(adapter) => write!(f, "{adapter:?} is not implemented"),
            Self::WrongRole => f.write_str("producer bindings need a producer input"),
            Self::WrongAuthority => {
                f.write_str("annotation bindings need the annotation authority")
            }
            Self::UnsupportedSnapshot => f.write_str("snapshot kind is not supported"),
            Self::UnsupportedScope => f.write_str("only the API scope is supported"),
        }
    }
}

impl std::error::Error for ProducerError {}

pub fn bind_producer(
    inputs: &ProducerInputs<'_>,
    schema: &SchemaIndex,
    opts: &ProducerOptions,
) -> Result<BindingsDocument, ProducerError> {
    let _ = (schema, opts);
    Ok(BindingsDocument {
        schema_version: "0.1".into(),
        input: Some(inputs.input.clone()),
        ..BindingsDocument::default()
    })
}
