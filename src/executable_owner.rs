//! Private epoch-bound proof construction; explicit cache-free activation only.
use crate::{
    ast::ParsedFile,
    call_graph::{CallGraph, FunctionId},
    languages::Language,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
};

mod acquisition;
pub(crate) mod integration;
type CallKey = (String, usize, usize);
type Result<T> = std::result::Result<T, String>;
const ANCHORS: [(&str, &str); 21] = [
    ("annotation", "TypeReference"),
    ("implementation", "ArrowFunction"),
    ("parameter", "Parameter"),
    ("binding", "BindingElement"),
    ("call", "CallExpression"),
    ("receiver", "Identifier"),
    ("callable_import", "ImportDeclaration"),
    ("callable_alias", "TypeAliasDeclaration"),
    ("binder", "TypeParameter"),
    ("binder_use", "TypeReference"),
    ("signature", "FunctionType"),
    ("signature_parameter", "Parameter"),
    ("props_reference", "TypeReference"),
    ("props_alias", "TypeAliasDeclaration"),
    ("property", "PropertySignature"),
    ("class_reference", "TypeReference"),
    ("class_import", "ImportDeclaration"),
    ("class_declaration", "ClassDeclaration"),
    ("class_keyword", "ClassKeyword"),
    ("member_declaration", "MethodDeclaration"),
    ("member_body", "Block"),
];

// These are private wire observations, NOT deserializable authority types.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Anchor {
    file: String,
    sha256: String,
    kind: String,
    start_utf16: usize,
    end_utf16: usize,
    start_byte: usize,
    end_byte: usize,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Candidate {
    anchors: BTreeMap<String, Anchor>,
    class_name: String,
    member_name: String,
    callable_module: String,
    class_module: String,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Source {
    file: String,
    sha256: String,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Observation {
    schema: String,
    authorizes_runtime_edge: bool,
    packet: serde_json::Value,
    sources: Vec<Source>,
    candidates: Vec<Candidate>,
    refusals: Vec<serde_json::Value>,
}

// No Serialize/Deserialize, public fields, setters, or caller-selected epoch IDs.
pub(crate) struct AuthenticatedProgramEpoch(Arc<Epoch>);
pub(crate) struct ExecutableOwnerProof {
    epoch: Arc<Epoch>,
    call: CallKey,
}
struct Epoch {
    _root: std::path::PathBuf,
    _config: String,
    _compiler: std::path::PathBuf,
    _evidence: Observation,
    _files: BTreeMap<String, ParsedFile>,
    members: BTreeMap<CallKey, FunctionId>,
}
fn ensure(condition: bool, reason: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(reason.to_owned())
    }
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(crate) fn relative(file: &str) -> bool {
    !file.is_empty()
        && !file.contains(['\\', ':', '\0'])
        && file
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != ".." && !p.eq_ignore_ascii_case(".git"))
}
fn js(language: Language) -> bool {
    matches!(
        language,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    )
}
fn owned_inputs(files: BTreeMap<String, ParsedFile>) -> Result<BTreeMap<String, ParsedFile>> {
    ensure(files.len() <= 512, "input_budget")?;
    let mut out = BTreeMap::new();
    let mut bytes = 0usize;
    for (file, parsed) in files {
        let detected = Language::from_path(&file);
        // A forged language tag must not hide a TS source from the census.
        ensure(
            relative(&file) && parsed.path == file && detected == Some(parsed.language),
            "input_identity",
        )?;
        if !js(parsed.language) {
            continue;
        }
        bytes = bytes
            .checked_add(parsed.source.len())
            .ok_or("input_budget")?;
        ensure(bytes <= 8 * 1024 * 1024, "input_budget")?;
        // ParsedFile has public mutable source/tree fields: reparse owned bytes,
        // rather than authenticating the text and trusting an unrelated old tree.
        let fresh =
            ParsedFile::parse(&file, &parsed.source, parsed.language).map_err(|_| "parse_error")?;
        ensure(fresh.parse_error_count == 0, "parse_error")?;
        out.insert(file, fresh);
    }
    ensure(!out.is_empty(), "empty_inputs")?;
    Ok(out)
}

impl AuthenticatedProgramEpoch {
    pub(crate) fn acquire(
        root: &Path,
        config: &str,
        compiler: &Path,
        files: BTreeMap<String, ParsedFile>,
    ) -> Result<Self> {
        ensure(relative(config), "config_identity")?;
        let files = owned_inputs(files)?;
        let root = root.canonicalize().map_err(|_| "root_unavailable")?;
        let compiler = compiler
            .canonicalize()
            .map_err(|_| "compiler_unavailable")?;
        // Only this independently configured, byte-pinned live acquisition path
        // may produce evidence for construction. There is no raw-packet API.
        let evidence = acquisition::reproduce(&root, config, &compiler, &files)?;
        validate_inputs(&evidence, &files)?;
        let graph = CallGraph::build(&files);
        let mut members = BTreeMap::new();
        for candidate in &evidence.candidates {
            let (call, target) = map_member(candidate, &files, &graph)?;
            ensure(members.insert(call, target).is_none(), "duplicate_call")?;
        }
        Ok(Self(Arc::new(Epoch {
            _root: root,
            _config: config.into(),
            _compiler: compiler,
            _evidence: evidence,
            _files: files,
            members,
        })))
    }
    pub(crate) fn prove(
        &self,
        file: &str,
        start: usize,
        end: usize,
    ) -> Option<ExecutableOwnerProof> {
        let call = (file.to_owned(), start, end);
        self.0
            .members
            .contains_key(&call)
            .then(|| ExecutableOwnerProof {
                epoch: Arc::clone(&self.0),
                call,
            })
    }
    pub(crate) fn target_for<'a>(
        &'a self,
        proof: &ExecutableOwnerProof,
        file: &str,
        start: usize,
        end: usize,
    ) -> Option<&'a FunctionId> {
        // The proof holds no copyable target/anchor fields to splice. Both owning
        // Arc and the consumer's exact original call key must agree.
        (Arc::ptr_eq(&self.0, &proof.epoch) && proof.call == (file.to_owned(), start, end))
            .then(|| self.0.members.get(&proof.call))
            .flatten()
    }
}

fn validate_inputs(evidence: &Observation, files: &BTreeMap<String, ParsedFile>) -> Result<()> {
    ensure(
        evidence.schema == "prism.detached-owner/1" && !evidence.authorizes_runtime_edge,
        "envelope",
    )?;
    let packet = &evidence.packet;
    ensure(
        packet["schema"] == "prism.callable-observation/20"
            && packet["authorizes_runtime_edge"] == false
            && packet["semantic_closure"]["policy"]
                == "prism.semantic-closure/merged-side-effect-v3"
            && packet["semantic_closure"]["complete"] == true
            && packet["semantic_closure"]["reasons"] == serde_json::json!([])
            && packet["compiler"]["verified"] == true
            && packet["closure"]["stable_snapshot"] == true,
        "semantic_closure",
    )?;
    let declared: BTreeMap<_, _> = evidence
        .sources
        .iter()
        .map(|s| (s.file.as_str(), s.sha256.as_str()))
        .collect();
    ensure(
        declared.len() == evidence.sources.len() && declared.len() == files.len(),
        "input_set",
    )?;
    let program = packet["snapshot"]["program_files"]
        .as_array()
        .ok_or("program_census")?;
    let project: Vec<_> = program
        .iter()
        .filter_map(|v| v.as_str()?.strip_prefix("project/"))
        .collect();
    ensure(
        project.len() == declared.len() && project.iter().all(|f| declared.contains_key(f)),
        "program_census",
    )?;
    let manifest = packet["snapshot"]["files"]
        .as_array()
        .ok_or("input_manifest")?;
    for (file, parsed) in files {
        let sha = hash(parsed.source.as_bytes());
        ensure(
            declared.get(file.as_str()) == Some(&sha.as_str()),
            "input_hash",
        )?;
        let id = format!("project/{file}");
        let rows: Vec<_> = manifest.iter().filter(|r| r["id"] == id).collect();
        ensure(
            rows.len() == 1 && rows[0]["sha256"] == sha && rows[0]["size"] == parsed.source.len(),
            "input_manifest",
        )?;
    }
    Ok(())
}

fn node_at<'a>(
    parsed: &'a ParsedFile,
    start: usize,
    end: usize,
    kinds: &[&str],
) -> Option<tree_sitter::Node<'a>> {
    let mut nodes = vec![parsed.tree.root_node()];
    while let Some(node) = nodes.pop() {
        if node.start_byte() > start || node.end_byte() < end {
            continue;
        }
        if node.start_byte() == start
            && node.end_byte() == end
            && (kinds.is_empty() || kinds.contains(&node.kind()))
        {
            return Some(node);
        }
        let mut cursor = node.walk();
        nodes.extend(node.children(&mut cursor));
    }
    None
}
fn validate_anchor(
    anchor: &Anchor,
    expected: &str,
    files: &BTreeMap<String, ParsedFile>,
) -> Result<()> {
    let file = anchor
        .file
        .strip_prefix("project/")
        .ok_or("anchor_domain")?;
    let parsed = files.get(file).ok_or("anchor_file")?;
    let text = &parsed.source;
    ensure(
        anchor.kind == expected && anchor.sha256 == hash(text.as_bytes()),
        "anchor_identity",
    )?;
    ensure(
        anchor.start_byte < anchor.end_byte
            && anchor.end_byte <= text.len()
            && text.is_char_boundary(anchor.start_byte)
            && text.is_char_boundary(anchor.end_byte),
        "anchor_range",
    )?;
    ensure(
        text[..anchor.start_byte].encode_utf16().count() == anchor.start_utf16
            && text[..anchor.end_byte].encode_utf16().count() == anchor.end_utf16,
        "anchor_encoding",
    )?;
    ensure(
        node_at(parsed, anchor.start_byte, anchor.end_byte, &[]).is_some(),
        "anchor_parser",
    )
}
fn map_member(
    candidate: &Candidate,
    files: &BTreeMap<String, ParsedFile>,
    graph: &CallGraph,
) -> Result<(CallKey, FunctionId)> {
    ensure(candidate.anchors.len() == ANCHORS.len(), "anchor_census")?;
    for (name, kind) in ANCHORS {
        validate_anchor(
            candidate.anchors.get(name).ok_or("anchor_missing")?,
            kind,
            files,
        )?;
    }
    let a = |name: &str| &candidate.anchors[name];
    for (outer, inner) in [
        ("implementation", "parameter"),
        ("parameter", "binding"),
        ("implementation", "call"),
        ("call", "receiver"),
        ("annotation", "props_reference"),
        ("props_alias", "property"),
        ("property", "class_reference"),
        ("callable_alias", "binder"),
        ("callable_alias", "signature"),
        ("signature", "signature_parameter"),
        ("signature_parameter", "binder_use"),
        ("class_declaration", "class_keyword"),
        ("class_declaration", "member_declaration"),
        ("member_declaration", "member_body"),
    ] {
        let (x, y) = (a(outer), a(inner));
        ensure(
            x.file == y.file && x.start_byte <= y.start_byte && x.end_byte >= y.end_byte,
            "anchor_containment",
        )?;
    }
    let call = a("call");
    let caller = call.file.strip_prefix("project/").ok_or("caller")?;
    let class = a("class_declaration");
    let defining = class.file.strip_prefix("project/").ok_or("owner")?;
    let parsed = &files[defining];
    let span = (a("class_keyword").start_byte, class.end_byte);
    ensure(
        graph
            .clean_class_spans
            .get(&(defining.into(), candidate.class_name.clone()))
            == Some(&span),
        "clean_class",
    )?;
    let indexed: BTreeSet<_> = files.keys().cloned().collect();
    for (module, target) in [
        (&candidate.class_module, class),
        (&candidate.callable_module, a("callable_alias")),
    ] {
        let targets = crate::call_graph::js_ts_relative_module_candidates(module, caller, &indexed)
            .ok_or("module_identity")?;
        ensure(
            targets
                == [target
                    .file
                    .strip_prefix("project/")
                    .ok_or("module_domain")?],
            "module_identity",
        )?;
    }
    let member = a("member_declaration");
    let node = node_at(
        parsed,
        member.start_byte,
        member.end_byte,
        &["method_definition"],
    )
    .ok_or("method_node")?;
    let owner = node
        .parent()
        .and_then(|body| body.parent())
        .ok_or("method_owner")?;
    ensure(
        owner.start_byte() == span.0
            && owner.end_byte() == span.1
            && !parsed.js_ts_method_is_static(&node)
            && !parsed.js_ts_method_slot_unproven(&node),
        "method_slot",
    )?;
    let (start_line, end_line) = parsed.node_line_range(&node);
    let target = FunctionId {
        file: defining.into(),
        name: candidate.member_name.clone(),
        start_line,
        end_line,
    };
    ensure(
        graph
            .functions
            .get(&target.name)
            .map(|v| v.iter().filter(|f| **f == target).count())
            == Some(1)
            && graph
                .methods
                .get(&(candidate.class_name.clone(), target.name.clone()))
                .is_some_and(|v| v.contains(&target))
            && graph.method_class_span.get(&target) == Some(&span)
            && !graph.method_class_span_ambiguous.contains(&target)
            && !graph.js_ts_static_methods.contains(&target)
            && !graph.js_ts_unproven_instance_methods.contains(&target),
        "method_identity",
    )?;
    ensure(
        node_at(
            &files[caller],
            call.start_byte,
            call.end_byte,
            &["call_expression"],
        )
        .is_some(),
        "call_node",
    )?;
    Ok(((caller.into(), call.start_byte, call.end_byte), target))
}

#[cfg(all(test, feature = "detached-owner-audit"))]
mod audit_tests;
#[cfg(test)]
mod tests;
