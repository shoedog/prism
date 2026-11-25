//! Materialize the fixture's source anchors without reading the listing.
use super::{checked_path, listing::items, nonempty_str, site_range};
use prism::{ast::ParsedFile, framework_entries::routes::RootSelector, languages::Language};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::Path};

pub(super) fn is_site(key: &str) -> bool {
    matches!(key, "site" | "install_site" | "element_site")
}

pub(super) fn files_and_spans(
    value: &Value,
    dir: Option<&Path>,
    field: &str,
    errors: &mut Vec<String>,
) {
    match value {
        Value::Object(fields) => {
            if let (Some(dir), Some(file)) = (dir, fields.get("file")) {
                let source = checked_path(dir, file, false).and_then(|path| {
                    std::fs::read_to_string(path).map_err(|error| error.to_string())
                });
                match source {
                    Ok(source) => {
                        let offset =
                            |key| value[key].as_u64().and_then(|x| usize::try_from(x).ok());
                        if let (Some(start), Some(end)) = (offset("start_byte"), offset("end_byte"))
                        {
                            let span = source.get(start..end).filter(|text| !text.is_empty());
                            let line_at = |byte| {
                                source.as_bytes()[..byte]
                                    .iter()
                                    .filter(|&&c| c == b'\n')
                                    .count()
                                    + 1
                            };
                            if span.is_none()
                                || offset("line").is_some_and(|line| line != line_at(start))
                                || offset("start_line").is_some_and(|line| line != line_at(start))
                                || offset("end_line").is_some_and(|line| line != line_at(end - 1))
                            {
                                errors.push(format!("{field}: invalid source span"));
                            }
                        }
                        if let Some(byte) = offset("construction_byte") {
                            if source.get(byte..).is_none_or(str::is_empty) {
                                errors.push(format!("{field}: construction byte outside source"));
                            }
                        }
                        if let Some(byte) = offset("object_byte") {
                            if source.as_bytes().get(byte) != Some(&b'{') {
                                errors.push(format!(
                                    "{field}: plugin object does not start at byte {byte}"
                                ));
                            }
                        }
                    }
                    Err(error) => errors.push(format!("{field}.file: {error}")),
                }
            }
            if fields.contains_key("invocations") {
                let id = json!({
                    "file":value["file"], "binding":value["binding"],
                    "construction_byte":value["construction_byte"], "invocations":value["invocations"]
                });
                if RootSelector::parse(&id.to_string()).is_err() {
                    errors.push(format!("{field}: invalid router identity"));
                }
            }
            for (key, child) in fields {
                files_and_spans(child, dir, &format!("{field}.{key}"), errors);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                files_and_spans(child, dir, &format!("{field}[{index}]"), errors);
            }
        }
        _ => {}
    }
}

// Materialize fixture-only source anchors as the byte offsets the listing exposes.
pub(super) fn resolve_pins(
    want: &mut Value,
    sources: &BTreeMap<String, String>,
    field: &str,
    errors: &mut Vec<String>,
) {
    fn materialize(
        want: &mut Value,
        sources: &BTreeMap<String, String>,
        field: &str,
        plugin: Option<(&str, usize)>,
        errors: &mut Vec<String>,
    ) {
        let key = field.rsplit('.').next().unwrap_or(field);
        let source_anchor =
            is_site(key) || key.starts_with("invocations[") || key.starts_with("realm[");
        if source_anchor && want.is_object() {
            if want.get("text").is_some() {
                let range = want["file"]
                    .as_str()
                    .and_then(|file| sources.get(file))
                    .ok_or_else(|| format!("missing source for {}", want["file"]))
                    .and_then(|source| site_range(want, source));
                match range {
                    Ok((start, end)) => {
                        want["start_byte"] = json!(start);
                        want["end_byte"] = json!(end);
                    }
                    Err(error) => errors.push(format!("{field}.text: {error}")),
                }
            }
            let site = want.as_object_mut().unwrap();
            for key in ["text", "occurrence", "ordinal"] {
                site.remove(key);
            }
        }
        if want.get("construction_text").is_some() {
            let file = want["file"].as_str().unwrap_or_default();
            let after = plugin
                .filter(|(plugin_file, _)| *plugin_file == file)
                .map(|(_, byte)| byte);
            match sources
                .get(file)
                .ok_or_else(|| format!("missing source for {file}"))
                .and_then(|source| construction_byte(want, source, after))
            {
                Ok(byte) => want["construction_byte"] = json!(byte),
                Err(error) => errors.push(format!("{field}.construction_byte: {error}")),
            }
            let object = want.as_object_mut().unwrap();
            object.remove("construction_text");
            object.remove("occurrence");
        }
        match want {
            Value::Object(object) => {
                let origin = object.get("origin_plugin").and_then(|origin| {
                    Some((
                        origin["file"].as_str()?.to_owned(),
                        usize::try_from(origin["object_byte"].as_u64()?).ok()?,
                    ))
                });
                let plugin = origin
                    .as_ref()
                    .map(|(file, byte)| (file.as_str(), *byte))
                    .or(plugin);
                for (key, child) in object {
                    materialize(child, sources, &format!("{field}.{key}"), plugin, errors);
                }
            }
            Value::Array(items) => {
                for (index, child) in items.iter_mut().enumerate() {
                    materialize(child, sources, &format!("{field}[{index}]"), plugin, errors);
                }
            }
            _ => {}
        }
    }
    materialize(want, sources, field, None, errors);
}

pub(super) fn collect_sources(
    dir: &Path,
    value: &Value,
    sources: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    match value {
        Value::Object(object) => {
            if object.contains_key("construction_text")
                || (object.contains_key("file") && object.contains_key("text"))
            {
                let file = nonempty_str(&value["file"]).ok_or("source anchor needs a filename")?;
                if !sources.contains_key(file) {
                    let path = checked_path(dir, &value["file"], false)?;
                    let source = fs::read_to_string(&path)
                        .map_err(|err| format!("{}: {err}", path.display()))?;
                    sources.insert(file.to_owned(), source);
                }
            }
            for child in object.values() {
                collect_sources(dir, child, sources)?;
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_sources(dir, child, sources)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// These fixtures select a local export, a direct factory return, or a pinned
/// identity. Derive the expected allocation byte from the fixture source.
pub(super) fn selected_root(dir: &Path, entry: &str) -> Result<Value, String> {
    use prism::framework_entries::routes::EntryKind;
    let selector =
        EntryKind::parse(entry).ok_or_else(|| format!("{entry}: invalid fixture entry"))?;
    let (files, binding, factory, export) = match selector {
        EntryKind::Python {
            module,
            binding,
            factory,
        } => {
            let path = module.replace('.', "/");
            (
                vec![format!("{path}.py"), format!("{path}/__init__.py")],
                binding,
                factory,
                None,
            )
        }
        EntryKind::JavaScript { file, export } => {
            let binding = export.as_deref().unwrap_or("app").to_owned();
            (vec![file], binding, false, Some(export))
        }
        EntryKind::Identity(identity) => return Ok(json!(identity)),
    };
    let mut files = files.into_iter().filter_map(|file| {
        let path = checked_path(dir, &json!(file), false).ok()?;
        Some((file, std::fs::read_to_string(path).ok()?))
    });
    let (file, source) = files
        .next()
        .filter(|_| files.next().is_none())
        .ok_or_else(|| format!("{entry}: expected one entry source"))?;
    let named_export = export.as_ref().is_some_and(Option::is_some);
    if let Some(export) = export {
        let proven = source
            .lines()
            .map(|line| line.trim().trim_end_matches(';'))
            .any(|line| match export.as_deref() {
                Some(name) => {
                    line == format!("module.exports.{name} = {binding}")
                        || line == format!("exports.{name} = {binding}")
                        || line.starts_with(&format!("export const {name} ="))
                }
                None => {
                    line == format!("module.exports = {binding}")
                        || line == format!("export default {binding}")
                }
            });
        if !proven {
            return Err(format!("{entry}: expected a direct fixture export"));
        }
    }
    let (binding, region, region_start) = if factory {
        let parsed = ParsedFile::parse(&file, &source, Language::Python)
            .map_err(|error| error.to_string())?;
        let top = parsed.tree.root_node();
        let mut cursor = top.walk();
        let function = top
            .named_children(&mut cursor)
            .find(|node| {
                node.kind() == "function_definition"
                    && node
                        .child_by_field_name("name")
                        .is_some_and(|name| &source[name.byte_range()] == binding)
            })
            .and_then(|node| node.child_by_field_name("body"))
            .ok_or_else(|| format!("{entry}: missing factory body"))?;
        let mut cursor = function.walk();
        let returns: Vec<_> = function
            .named_children(&mut cursor)
            .filter(|node| node.kind() == "return_statement")
            .collect();
        let [returned] = returns.as_slice() else {
            return Err(format!("{entry}: expected one direct return"));
        };
        let returned = returned
            .named_child(0)
            .filter(|node| node.kind() == "identifier")
            .ok_or_else(|| format!("{entry}: expected a returned local binding"))?;
        (
            source[returned.byte_range()].to_owned(),
            &source[function.byte_range()],
            function.start_byte(),
        )
    } else {
        (binding, source.as_str(), 0)
    };

    let mut offset = region_start;
    let mut hits = Vec::new();
    let mut indirect_named_constructor = false;
    for line in region.split_inclusive('\n') {
        let statement = line.trim_start();
        let prefix = statement.strip_prefix("export ").unwrap_or(statement);
        let prefix = ["const ", "let ", "var "]
            .into_iter()
            .find_map(|keyword| prefix.strip_prefix(keyword))
            .unwrap_or(prefix);
        let (name, rhs) = prefix.split_once('=').unwrap_or_default();
        let constructor = rhs.trim_start();
        if name.trim() == binding && (factory || statement.len() == line.len()) {
            if [
                "express(",
                "Flask(",
                "flask.Flask(",
                "FastAPI(",
                "APIRouter(",
                "express.Router(",
            ]
            .iter()
            .any(|pattern| constructor.starts_with(pattern))
            {
                hits.push(offset + line.len() - constructor.len());
            } else if named_export
                && constructor.split_once('(').is_some_and(|(callee, _)| {
                    EntryKind::parse(&format!("{file}#{}", callee.trim_end())).is_some()
                })
            {
                indirect_named_constructor = true;
            }
        }
        offset += line.len();
    }
    let [byte] = hits.as_slice() else {
        if hits.is_empty() && indirect_named_constructor {
            return Err(format!("{entry}: unsupported selector shape: owned by S3a"));
        }
        return Err(format!("{entry}: expected a unique fixture construction"));
    };
    Ok(
        json!({"file":file,"binding":binding,"construction_byte":byte,
        "invocations": if factory { vec![json!({"entry":entry})] } else { vec![] }}),
    )
}

pub(super) fn validate(row: &Value, dir: &Path, errors: &mut Vec<String>) {
    let mut sources = BTreeMap::new();
    for key in [
        "expect",
        "expect_mounts",
        "expect_diagnostics",
        "expect_failed_roots",
        "entry_queries",
        "expect_bindings",
    ] {
        if let Err(error) = collect_sources(dir, &row[key], &mut sources) {
            errors.push(format!("{key}: {error}"));
            continue;
        }
        let mut pins = row[key].clone();
        resolve_pins(&mut pins, &sources, key, errors);
        files_and_spans(&pins, Some(dir), key, errors);
        if key == "expect_mounts" {
            super::listing::validate_mount_pins(items(&pins), errors);
        }
    }
    if let Some(entry) = row.get("query_entry").unwrap_or(&row["entry"]).as_str() {
        if let Err(error) = selected_root(dir, entry) {
            errors.push(format!("entry: {error}"));
        }
    }
    for query in items(&row["entry_queries"]) {
        let entry = query["entry"].as_str().unwrap_or_default();
        if query.get("expect_error").is_some() && query["expect_error"] != "entry_unproven"
            || query.get("expect_error").is_none() && RootSelector::parse(entry).is_err()
        {
            errors.push(format!("entry_queries: invalid query pin {query}"));
        }
    }
    for key in ["expect_diagnostics", "expect_failed_roots"] {
        for item in items(&row[key]) {
            if !item["site"].is_object() || !item["reason"].is_string() {
                errors.push(format!("{key}: expected a site and reason"));
            }
        }
    }
}

fn construction_byte(anchor: &Value, source: &str, after: Option<usize>) -> Result<usize, String> {
    let text = anchor["construction_text"]
        .as_str()
        .filter(|text| !text.is_empty())
        .ok_or("expected nonempty construction_text")?;
    let binding = anchor["binding"].as_str().unwrap_or_default();
    let hits: Vec<_> = source.match_indices(text).map(|(byte, _)| byte).collect();
    if let [byte] = hits.as_slice() {
        return Ok(*byte);
    }
    let matches: Vec<_> = hits
        .into_iter()
        .filter(|&byte| after.is_none_or(|start| byte > start))
        .filter(|&byte| {
            let line = source[..byte]
                .rsplit('\n')
                .next()
                .unwrap_or_default()
                .trim_end();
            let lhs = line.trim_end_matches(['=', ':']).trim_end();
            let assignment = lhs != line && lhs.split_whitespace().last() == Some(binding);
            let formal = (line.ends_with("register(") || line.trim_start() == "register: async (")
                && text.starts_with(binding);
            let controller = source[byte + text.len()..]
                .trim_start()
                .strip_prefix("class ")
                .and_then(|tail| tail.split_whitespace().next())
                == Some(binding);
            assignment || formal || controller
        })
        .collect();
    if let [byte] = matches.as_slice() {
        return Ok(*byte);
    }
    if let (Some(_), Some(byte)) = (after, matches.first()) {
        return Ok(*byte);
    }
    if anchor.get("occurrence").is_some() {
        let site = json!({"text":text,"occurrence":anchor["occurrence"]});
        return site_range(&site, source).map(|(start, _)| start);
    }
    Err(format!(
        "ambiguous construction anchor {binding:?} at {text:?}"
    ))
}

#[test]
fn named_factory_instance_selector_explains_s3a_ownership() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("app.js"),
        concat!(
            "const express = require('express');\n",
            "function makeApp() { return express(); }\n",
            "const left = makeApp();\n",
            "module.exports.left = left;\n",
        ),
    )
    .unwrap();
    let error = selected_root(dir.path(), "app.js#left").unwrap_err();
    assert!(
        error.contains("unsupported selector shape: owned by S3a"),
        "{error}"
    );
    let missing = selected_root(dir.path(), "app.js#missing").unwrap_err();
    assert!(
        missing.contains("expected a direct fixture export"),
        "{missing}"
    );
}
