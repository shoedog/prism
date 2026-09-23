//! Bounded native parameter API characterization; it reads one JSON request from stdin.
use anyhow::{bail, ensure, Context, Result};
use prism::{ast::ParsedFile, languages::Language};
use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, io::Read, process::ExitCode};
use tree_sitter::Node;

const REQUEST: &str = "prism.native-parameter-request/1";
const OUTPUT: &str = "prism.native-parameter-characterization/1";
const MAX_FILES: usize = 16;
const MAX_SITES: usize = 16;
const MAX_SOURCE: usize = 256 * 1024;
const MAX_INPUT: usize = 2 * 1024 * 1024;
const MAX_OUTPUT: usize = 4 * 1024 * 1024;

#[derive(Clone)]
struct Selector { path: String, start: usize, end: usize, kind: String, objects: Vec<usize>, later: Vec<usize> }
struct InputFile { path: String, sha256: String, bytes: usize, script_kind: String, source: String, sites: Vec<Selector> }
struct Request { manifest_sha256: String, binary_sha256: String, files: Vec<InputFile> }
#[derive(Serialize)]
struct Packet { schema: &'static str, measurement: &'static str, authorizes_runtime_edge: bool, native_entry_measured: bool, callee_resolution_measured: bool, input_manifest_sha256: String, native_binary_sha256: String, files: Vec<FileRow>, sites: Vec<SiteRow>, totals: Totals, next_action: &'static str, reason: &'static str }
#[derive(Serialize)]
struct FileRow { path: String, sha256: String, bytes: usize, language: &'static str, parse_error_count: usize }
#[derive(Clone, Serialize)]
struct Parameter { ordinal: usize, kind: String, start_byte: usize, end_byte: usize, pattern_kind: String, ordinary_required_identifier: bool }
#[derive(Clone, Serialize)]
struct Occurrence { name: String, start_byte: usize, end_byte: usize, source_ordinal: Option<usize> }
#[derive(Clone, Serialize)]
struct Candidate { kind: String, start_line: usize, end_line: usize, name: Option<String>, parameters: Vec<Parameter>, slots: Option<Vec<Occurrence>>, bindings: Vec<Occurrence> }
#[derive(Serialize)]
struct SiteRow { path: String, start_byte: usize, end_byte: usize, status: String, candidates: Vec<Candidate>, next_proof_eligibility: String }
#[derive(Serialize)]
struct Totals { files: usize, sites: usize, unique_named: usize, unique_unnamed: usize, missing: usize, ambiguous: usize, recovery_quarantined: usize }

fn main() -> ExitCode { match run() { Ok(()) => ExitCode::SUCCESS, Err(error) => { eprintln!("{error:#}"); ExitCode::FAILURE } } }
fn run() -> Result<()> {
    let mut input = Vec::new(); std::io::stdin().read_to_end(&mut input)?;
    ensure!(input.len() <= MAX_INPUT, "worker input limit exceeded");
    let request = decode(&serde_json::from_slice(&input).context("invalid request JSON")?)?;
    let packet = observe(request)?; let text = serde_json::to_string(&packet)?;
    ensure!(text.len() + 1 <= MAX_OUTPUT, "worker output limit exceeded");
    println!("{text}"); Ok(())
}

fn object<'a>(value: &'a Value, keys: &[&str]) -> Result<&'a Map<String, Value>> {
    let object = value.as_object().context("expected object")?;
    ensure!(object.len() == keys.len() && keys.iter().all(|key| object.contains_key(*key)), "unexpected object keys"); Ok(object)
}
fn text(object: &Map<String, Value>, key: &str) -> Result<String> { object[key].as_str().map(str::to_owned).context("expected string") }
fn number(object: &Map<String, Value>, key: &str) -> Result<usize> { usize::try_from(object[key].as_u64().context("expected safe integer")?).context("integer too large") }
fn hash(value: &str) -> bool { value.len() == 64 && value.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')) }
fn safe_path(path: &str) -> bool { !path.is_empty() && !path.starts_with('/') && !path.contains('\\') && !path.chars().any(|c| c.is_control()) && path.split('/').all(|part| !part.is_empty() && part != "." && part != "..") }
fn ordinals(value: &Value) -> Result<Vec<usize>> { let values = value.as_array().context("expected ordinal array")?; let result: Result<Vec<_>> = values.iter().map(|v| usize::try_from(v.as_u64().context("expected ordinal")?).context("ordinal too large")).collect(); let result = result?; ensure!(result.windows(2).all(|pair| pair[0] < pair[1]), "ordinals must be distinct increasing"); Ok(result) }
fn language(script: &str, path: &str) -> Result<(Language, &'static str)> { match (path.rsplit('.').next(), script) { (Some("js"), "JavaScript") | (Some("jsx"), "Jsx") => Ok((Language::JavaScript, "JavaScript")), (Some("ts"), "TypeScript") => Ok((Language::TypeScript, "TypeScript")), (Some("tsx"), "Tsx") => Ok((Language::Tsx, "Tsx")), _ => bail!("script kind/path mismatch") } }
fn sha(source: &str) -> String { format!("{:x}", Sha256::digest(source.as_bytes())) }

fn decode(value: &Value) -> Result<Request> {
    let top = object(value, &["schema", "input_manifest_sha256", "native_binary_sha256", "files"])?;
    ensure!(text(top, "schema")? == REQUEST, "unsupported request schema");
    let manifest_sha256 = text(top, "input_manifest_sha256")?; let binary_sha256 = text(top, "native_binary_sha256")?;
    ensure!(hash(&manifest_sha256) && hash(&binary_sha256), "invalid request hash");
    let values = top["files"].as_array().context("expected files array")?; ensure!(!values.is_empty() && values.len() <= MAX_FILES, "file limit exceeded");
    let mut files = Vec::new(); let mut paths = BTreeSet::new(); let mut selectors = BTreeSet::new(); let mut aggregate = 0;
    for value in values {
        let file = object(value, &["path", "sha256", "bytes", "script_kind", "source", "sites"])?;
        let path = text(file, "path")?; let sha256 = text(file, "sha256")?; let bytes = number(file, "bytes")?; let script_kind = text(file, "script_kind")?; let source = text(file, "source")?;
        ensure!(safe_path(&path) && hash(&sha256) && paths.insert(path.clone()), "invalid or duplicate file"); language(&script_kind, &path)?;
        ensure!(source.len() == bytes && sha(&source) == sha256, "source identity mismatch"); aggregate += bytes; ensure!(aggregate <= MAX_SOURCE, "source limit exceeded");
        let site_values = file["sites"].as_array().context("expected sites array")?; ensure!(!site_values.is_empty(), "file has no selectors");
        let mut sites = Vec::new();
        for value in site_values {
            let site = object(value, &["path", "start_byte", "end_byte", "compiler_kind", "object_ordinals", "later_required_ordinals"])?;
            let site_path = text(site, "path")?; let start = number(site, "start_byte")?; let end = number(site, "end_byte")?; let kind = text(site, "compiler_kind")?;
            ensure!(site_path == path && start < end && end <= bytes && source.is_char_boundary(start) && source.is_char_boundary(end), "invalid selector");
            ensure!(matches!(kind.as_str(), "FunctionDeclaration" | "FunctionExpression" | "ArrowFunction") && selectors.insert((path.clone(), start, end)), "invalid or duplicate selector");
            sites.push(Selector { path: site_path, start, end, kind, objects: ordinals(&site["object_ordinals"])?, later: ordinals(&site["later_required_ordinals"])? });
        }
        files.push(InputFile { path, sha256, bytes, script_kind, source, sites });
    }
    ensure!(selectors.len() <= MAX_SITES, "site limit exceeded"); Ok(Request { manifest_sha256, binary_sha256, files })
}

fn expected_kind(kind: &str) -> &str { match kind { "FunctionDeclaration" => "function_declaration", "FunctionExpression" => "function_expression", "ArrowFunction" => "arrow_function", _ => unreachable!() } }
fn status(parse_errors: usize, candidate_count: usize, named: bool) -> &'static str { if parse_errors > 0 { "recovery_quarantined" } else { match candidate_count { 0 => "missing", 1 if named => "unique_named", 1 => "unique_unnamed", _ => "ambiguous" } } }
fn parameters(node: Node<'_>) -> Vec<Node<'_>> {
    if let Some(list) = node.child_by_field_name("parameters") { let mut cursor = list.walk(); return list.named_children(&mut cursor).filter(|child| child.kind() != "comment").collect(); }
    node.child_by_field_name("parameter").into_iter().collect()
}
fn pattern(node: Node<'_>) -> Node<'_> { node.child_by_field_name("pattern").or_else(|| node.child_by_field_name("name")).unwrap_or(node) }
fn ordinary(node: Node<'_>) -> bool {
    if node.kind() == "identifier" { return true; }
    if node.kind() != "required_parameter" { return false; }
    let pattern = pattern(node); if pattern.kind() != "identifier" { return false; }
    let type_node = node.child_by_field_name("type"); let mut cursor = node.walk();
    let ordinary = node.children(&mut cursor).all(|child| child == pattern || type_node.is_some_and(|ty| child == ty) || child.kind() == "comment"); ordinary
}
fn observed_occurrences(rows: Vec<(String, usize, usize)>, parameters: &[Parameter]) -> Vec<Occurrence> { rows.into_iter().map(|(name, start_byte, end_byte)| { let matches: Vec<_> = parameters.iter().filter(|parameter| start_byte >= parameter.start_byte && end_byte <= parameter.end_byte).map(|parameter| parameter.ordinal).collect(); Occurrence { name, start_byte, end_byte, source_ordinal: (matches.len() == 1).then_some(matches[0]) } }).collect() }
fn candidate(parsed: &ParsedFile, node: Node<'_>) -> Candidate {
    let parameters: Vec<_> = parameters(node).into_iter().enumerate().map(|(ordinal, node)| { let pattern = pattern(node); Parameter { ordinal, kind: node.kind().to_owned(), start_byte: node.start_byte(), end_byte: node.end_byte(), pattern_kind: pattern.kind().to_owned(), ordinary_required_identifier: ordinary(node) } }).collect();
    let slots = parsed.function_parameter_slot_occurrences(&node).map(|rows| observed_occurrences(rows, &parameters));
    let bindings = observed_occurrences(parsed.function_parameter_occurrences(&node), &parameters);
    Candidate { kind: node.kind().to_owned(), start_line: node.start_position().row + 1, end_line: node.end_position().row + 1, name: parsed.language.function_name(&node).map(|name| parsed.node_text(&name).to_owned()), parameters, slots, bindings }
}
fn eligibility(status: &str, candidate: Option<&Candidate>, selector: &Selector) -> &'static str {
    if status != "unique_named" { return "not_clean_named"; } let candidate = candidate.unwrap();
    let Some(slots) = &candidate.slots else { return "native_slot_authority_unavailable"; };
    let lookup = |ordinal| candidate.parameters.iter().find(|parameter| parameter.ordinal == ordinal);
    let objects = selector.objects.iter().all(|ordinal| lookup(*ordinal).is_some_and(|parameter| parameter.pattern_kind == "object_pattern" && matches!(parameter.kind.as_str(), "object_pattern" | "required_parameter")));
    let later = selector.later.iter().all(|ordinal| lookup(*ordinal).is_some_and(|parameter| parameter.ordinary_required_identifier));
    let ordered = selector.objects.iter().all(|object| selector.later.iter().any(|later| later > object)) && selector.later.iter().all(|later| selector.objects.iter().any(|object| object < later));
    if selector.objects.is_empty() || selector.later.is_empty() || !objects || !later || !ordered { return "selection_native_shape_mismatch"; }
    if selector.later.iter().any(|ordinal| candidate.bindings.iter().any(|binding| binding.source_ordinal == Some(*ordinal)) && !slots.iter().any(|slot| slot.source_ordinal == Some(*ordinal))) { "eligible" } else { "no_selected_suffix_binding_gap" }
}
fn observe(mut request: Request) -> Result<Packet> {
    request.files.sort_by(|left, right| left.path.cmp(&right.path)); let mut files = Vec::new(); let mut sites = Vec::new(); let mut totals = Totals { files: request.files.len(), sites: 0, unique_named: 0, unique_unnamed: 0, missing: 0, ambiguous: 0, recovery_quarantined: 0 };
    for file in request.files {
        let (language, output_language) = language(&file.script_kind, &file.path)?; let parsed = ParsedFile::parse(&file.path, &file.source, language)?;
        files.push(FileRow { path: file.path.clone(), sha256: file.sha256, bytes: file.bytes, language: output_language, parse_error_count: parsed.parse_error_count });
        for selector in file.sites { let mut candidates: Vec<_> = parsed.all_functions().into_iter().filter(|node| node.start_byte() == selector.start && node.end_byte() == selector.end && node.kind() == expected_kind(&selector.kind)).map(|node| candidate(&parsed, node)).collect();
            candidates.sort_by(|left, right| (&left.kind, left.start_line, left.end_line, &left.name).cmp(&(&right.kind, right.start_line, right.end_line, &right.name)));
            let status = status(parsed.parse_error_count, candidates.len(), candidates.first().is_some_and(|candidate| candidate.name.is_some())).to_owned();
            match status.as_str() { "unique_named" => totals.unique_named += 1, "unique_unnamed" => totals.unique_unnamed += 1, "missing" => totals.missing += 1, "ambiguous" => totals.ambiguous += 1, _ => totals.recovery_quarantined += 1 }; totals.sites += 1;
            let next_proof_eligibility = eligibility(&status, candidates.first(), &selector).to_owned(); sites.push(SiteRow { path: selector.path, start_byte: selector.start, end_byte: selector.end, status, candidates, next_proof_eligibility });
        }
    }
    sites.sort_by(|left, right| (&left.path, left.start_byte, left.end_byte).cmp(&(&right.path, right.start_byte, right.end_byte)));
    let any_eligible = sites.iter().any(|site| site.next_proof_eligibility == "eligible");
    Ok(Packet { schema: OUTPUT, measurement: "native_parameter_api_characterization", authorizes_runtime_edge: false, native_entry_measured: false, callee_resolution_measured: false, input_manifest_sha256: request.manifest_sha256, native_binary_sha256: request.binary_sha256, files, sites, totals, next_action: if any_eligible { "bounded_entry_and_call_proof" } else { "defer" }, reason: if any_eligible { "named_native_binding_outside_legacy_prefix_requires_entry_and_call_proof" } else { "no_eligible_native_slot_gap" } })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terminal_statuses_prefer_recovery_and_preserve_duplicate_candidates() {
        assert_eq!(status(1, 1, true), "recovery_quarantined");
        assert_eq!(status(0, 0, false), "missing"); assert_eq!(status(0, 1, false), "unique_unnamed");
        assert_eq!(status(0, 1, true), "unique_named"); assert_eq!(status(0, 2, true), "ambiguous");
    }
    #[test]
    fn eligibility_distinguishes_a_prefix_gap_from_null_authority_and_no_gap() {
        let selector = Selector { path: "case.js".into(), start: 0, end: 1, kind: "FunctionDeclaration".into(), objects: vec![0], later: vec![1] };
        let parameter = |ordinal: usize, kind: &str, ordinary: bool| Parameter { ordinal, kind: kind.into(), start_byte: ordinal, end_byte: ordinal + 1, pattern_kind: kind.into(), ordinary_required_identifier: ordinary };
        let mut candidate = Candidate { kind: "function_declaration".into(), start_line: 1, end_line: 1, name: Some("take".into()), parameters: vec![parameter(0, "object_pattern", false), parameter(1, "identifier", true)], slots: Some(vec![Occurrence { name: "later".into(), start_byte: 1, end_byte: 2, source_ordinal: Some(1) }]), bindings: vec![Occurrence { name: "later".into(), start_byte: 1, end_byte: 2, source_ordinal: Some(1) }] };
        assert_eq!(eligibility("unique_named", Some(&candidate), &selector), "no_selected_suffix_binding_gap");
        candidate.slots = Some(vec![]); assert_eq!(eligibility("unique_named", Some(&candidate), &selector), "eligible");
        candidate.slots = None; assert_eq!(eligibility("unique_named", Some(&candidate), &selector), "native_slot_authority_unavailable");
    }
}
