use super::types::{AliasExpr, CanonTypeError, NameIdentity, NamedType, OwnerHint};
use crate::ast::ParsedFile;
use crate::go_build_profile::GoBuildProfile;
use crate::resolution::dir_of;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn parse_expr(
    node: &tree_sitter::Node<'_>,
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    file: &str,
    profile: Option<&GoBuildProfile>,
    type_params: &BTreeSet<String>,
) -> Result<AliasExpr, CanonTypeError> {
    match node.kind() {
        "type_identifier" => {
            let name = parsed.node_text(node).trim();
            if name.is_empty() {
                return Err(CanonTypeError::UnknownCanonType);
            }
            if type_params.contains(name) {
                return Ok(AliasExpr::Param(name.to_string()));
            }
            if imports.contains_key(".") {
                return Err(CanonTypeError::QualifiedTypeIdentity);
            }
            let identity = imports.get("").filter(|path| !path.is_empty()).map_or(
                NameIdentity::Bare,
                |path| NameIdentity::Path {
                    path: path.clone(),
                    qualified: false,
                },
            );
            let owner_hint = profile.map_or(OwnerHint::None, |profile| OwnerHint::Local {
                package_dir: dir_of(file).to_string(),
                clause: profile.package_clause.clone(),
            });
            Ok(AliasExpr::Named(NamedType {
                identity,
                owner_hint,
                name: name.to_string(),
            }))
        }
        "qualified_type" => {
            let package = node
                .child_by_field_name("package")
                .ok_or(CanonTypeError::QualifiedTypeIdentity)?;
            let name = node
                .child_by_field_name("name")
                .ok_or(CanonTypeError::QualifiedTypeIdentity)?;
            let alias = parsed.node_text(&package).trim();
            let name = parsed.node_text(&name).trim();
            let path = imports
                .get(alias)
                .filter(|path| !path.is_empty())
                .ok_or(CanonTypeError::QualifiedTypeIdentity)?
                .clone();
            Ok(AliasExpr::Named(NamedType {
                identity: NameIdentity::Path {
                    path: path.clone(),
                    qualified: true,
                },
                owner_hint: OwnerHint::ImportPath(path),
                name: name.to_string(),
            }))
        }
        "pointer_type" => unary(
            node,
            parsed,
            imports,
            file,
            profile,
            type_params,
            AliasExpr::Pointer,
        ),
        "slice_type" => field_unary(
            node,
            "element",
            parsed,
            imports,
            file,
            profile,
            type_params,
            AliasExpr::Slice,
        ),
        "array_type" => {
            let inner = node
                .child_by_field_name("element")
                .ok_or(CanonTypeError::UnknownCanonType)?;
            let length = node
                .child_by_field_name("length")
                .map(|value| parsed.node_text(&value).trim().to_string())
                .unwrap_or_default();
            Ok(AliasExpr::Array(
                length,
                Box::new(parse_expr(
                    &inner,
                    parsed,
                    imports,
                    file,
                    profile,
                    type_params,
                )?),
            ))
        }
        "map_type" => {
            let key = node
                .child_by_field_name("key")
                .ok_or(CanonTypeError::UnknownCanonType)?;
            let value = node
                .child_by_field_name("value")
                .ok_or(CanonTypeError::UnknownCanonType)?;
            Ok(AliasExpr::Map(
                Box::new(parse_expr(
                    &key,
                    parsed,
                    imports,
                    file,
                    profile,
                    type_params,
                )?),
                Box::new(parse_expr(
                    &value,
                    parsed,
                    imports,
                    file,
                    profile,
                    type_params,
                )?),
            ))
        }
        "channel_type" => {
            let inner = node
                .child_by_field_name("value")
                .ok_or(CanonTypeError::UnknownCanonType)?;
            let prefix = parsed
                .source
                .get(node.start_byte()..inner.start_byte())
                .ok_or(CanonTypeError::UnknownCanonType)?
                .replace(char::is_whitespace, "");
            let direction = if prefix.starts_with("<-chan") {
                "<-chan"
            } else if prefix.starts_with("chan<-") {
                "chan<-"
            } else if prefix.starts_with("chan") {
                "chan"
            } else {
                return Err(CanonTypeError::UnknownCanonType);
            };
            Ok(AliasExpr::Channel(
                direction.to_string(),
                Box::new(parse_expr(
                    &inner,
                    parsed,
                    imports,
                    file,
                    profile,
                    type_params,
                )?),
            ))
        }
        "function_type" => Ok(AliasExpr::Function(
            parse_params(
                node.child_by_field_name("parameters").as_ref(),
                parsed,
                imports,
                file,
                profile,
                type_params,
            )?,
            parse_results(
                node.child_by_field_name("result").as_ref(),
                parsed,
                imports,
                file,
                profile,
                type_params,
            )?,
        )),
        "interface_type" => {
            let compact = parsed.node_text(node).replace(char::is_whitespace, "");
            (compact == "interface{}")
                .then(|| AliasExpr::Atom("any".to_string()))
                .ok_or(CanonTypeError::AnonymousInterface)
        }
        "generic_type" => {
            let base = node
                .child_by_field_name("type")
                .ok_or(CanonTypeError::UnknownCanonType)?;
            let args = node
                .child_by_field_name("type_arguments")
                .ok_or(CanonTypeError::UnknownCanonType)?;
            Ok(AliasExpr::Generic(
                Box::new(parse_expr(
                    &base,
                    parsed,
                    imports,
                    file,
                    profile,
                    type_params,
                )?),
                parse_named_children(&args, parsed, imports, file, profile, type_params)?,
            ))
        }
        "type_arguments" => Err(CanonTypeError::UnknownCanonType),
        "type_elem" => {
            let compact = parsed.node_text(node).replace(char::is_whitespace, "");
            let mut cursor = node.walk();
            let children: Vec<_> = node.named_children(&mut cursor).collect();
            if compact.contains('~') || compact.contains('|') || children.len() != 1 {
                return Err(CanonTypeError::Generic);
            }
            parse_expr(&children[0], parsed, imports, file, profile, type_params)
        }
        "parenthesized_type" => unary(node, parsed, imports, file, profile, type_params, |inner| {
            *inner
        }),
        _ => Err(CanonTypeError::UnknownCanonType),
    }
}

fn unary<F>(
    node: &tree_sitter::Node<'_>,
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    file: &str,
    profile: Option<&GoBuildProfile>,
    params: &BTreeSet<String>,
    wrap: F,
) -> Result<AliasExpr, CanonTypeError>
where
    F: FnOnce(Box<AliasExpr>) -> AliasExpr,
{
    let inner = node
        .named_child(0)
        .ok_or(CanonTypeError::UnknownCanonType)?;
    Ok(wrap(Box::new(parse_expr(
        &inner, parsed, imports, file, profile, params,
    )?)))
}

fn field_unary<F>(
    node: &tree_sitter::Node<'_>,
    field: &str,
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    file: &str,
    profile: Option<&GoBuildProfile>,
    params: &BTreeSet<String>,
    wrap: F,
) -> Result<AliasExpr, CanonTypeError>
where
    F: FnOnce(Box<AliasExpr>) -> AliasExpr,
{
    let inner = node
        .child_by_field_name(field)
        .ok_or(CanonTypeError::UnknownCanonType)?;
    Ok(wrap(Box::new(parse_expr(
        &inner, parsed, imports, file, profile, params,
    )?)))
}

fn parse_named_children(
    node: &tree_sitter::Node<'_>,
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    file: &str,
    profile: Option<&GoBuildProfile>,
    params: &BTreeSet<String>,
) -> Result<Vec<AliasExpr>, CanonTypeError> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .map(|child| parse_expr(&child, parsed, imports, file, profile, params))
        .collect()
}

fn parse_params(
    list: Option<&tree_sitter::Node<'_>>,
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    file: &str,
    profile: Option<&GoBuildProfile>,
    params: &BTreeSet<String>,
) -> Result<Vec<AliasExpr>, CanonTypeError> {
    let Some(list) = list else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    let mut cursor = list.walk();
    for declaration in list.named_children(&mut cursor) {
        let type_node = declaration
            .child_by_field_name("type")
            .ok_or(CanonTypeError::UnknownCanonType)?;
        let mut expr = parse_expr(&type_node, parsed, imports, file, profile, params)?;
        if declaration.kind() == "variadic_parameter_declaration" {
            expr = AliasExpr::Variadic(Box::new(expr));
        }
        let count = declaration
            .children(&mut declaration.walk())
            .filter(|child| child.kind() == "identifier")
            .count()
            .max(1);
        for _ in 0..count {
            out.push(expr.clone());
        }
    }
    Ok(out)
}

fn parse_results(
    result: Option<&tree_sitter::Node<'_>>,
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    file: &str,
    profile: Option<&GoBuildProfile>,
    params: &BTreeSet<String>,
) -> Result<Vec<AliasExpr>, CanonTypeError> {
    match result {
        None => Ok(Vec::new()),
        Some(node) if node.kind() == "parameter_list" => {
            parse_params(Some(node), parsed, imports, file, profile, params)
        }
        Some(node) => Ok(vec![parse_expr(
            node, parsed, imports, file, profile, params,
        )?]),
    }
}

pub(super) fn alias_parameters(
    parsed: &ParsedFile,
    node: &tree_sitter::Node<'_>,
) -> (Vec<String>, bool) {
    let Some(parameters) = node
        .child_by_field_name("type_parameters")
        .or_else(|| named_descendant(node, "type_parameter_list"))
    else {
        return (Vec::new(), true);
    };
    let text = parsed.node_text(&parameters).trim();
    let Some(list) = text
        .strip_prefix('[')
        .and_then(|text| text.strip_suffix(']'))
    else {
        return (Vec::new(), false);
    };
    let mut names = Vec::new();
    let mut pending = Vec::new();
    let mut supported = true;
    for part in list.split(',') {
        let words: Vec<_> = part.split_whitespace().collect();
        match words.as_slice() {
            [name] => pending.push((*name).to_string()),
            [name, constraint @ ..] => {
                names.append(&mut pending);
                names.push((*name).to_string());
                supported &= *constraint == ["any"];
            }
            _ => supported = false,
        }
    }
    if !pending.is_empty() {
        names.append(&mut pending);
        supported = false;
    }
    (names, supported)
}

fn named_descendant<'a>(node: &tree_sitter::Node<'a>, kind: &str) -> Option<tree_sitter::Node<'a>> {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == kind {
            return Some(child);
        }
        if let Some(found) = named_descendant(&child, kind) {
            return Some(found);
        }
    }
    None
}

pub(super) fn predeclared(name: &str) -> Option<&str> {
    match name {
        "byte" => Some("uint8"),
        "rune" => Some("int32"),
        "any" | "bool" | "comparable" | "complex64" | "complex128" | "error" | "float32"
        | "float64" | "int" | "int8" | "int16" | "int32" | "int64" | "string" | "uint"
        | "uint8" | "uint16" | "uint32" | "uint64" | "uintptr" => Some(name),
        _ => None,
    }
}

pub(crate) fn signature_imports(
    parsed: &ParsedFile,
    local_import_path: Option<&str>,
) -> BTreeMap<String, String> {
    let mut imports = parsed.extract_imports();
    if let Some(path) = local_import_path.filter(|path| !path.is_empty()) {
        imports.insert(String::new(), path.to_string());
    }
    if has_dot_import(parsed.tree.root_node()) {
        imports.insert(".".to_string(), String::new());
    }
    let defaults: Vec<_> = imports
        .iter()
        .filter(|(alias, path)| path.rsplit('/').next() == Some(alias.as_str()))
        .map(|(alias, path)| (alias.clone(), path.clone()))
        .collect();
    let explicit: BTreeSet<_> = imports
        .iter()
        .filter(|(alias, path)| path.rsplit('/').next() != Some(alias.as_str()))
        .map(|(alias, _)| alias.clone())
        .collect();
    let mut inferred = BTreeMap::new();
    let mut ambiguous = BTreeSet::new();
    for (_, path) in defaults {
        let Some(alias) = versionless_alias(&path) else {
            continue;
        };
        if explicit.contains(&alias) || ambiguous.contains(&alias) {
            continue;
        }
        match inferred.get(&alias) {
            None => {
                inferred.insert(alias, path);
            }
            Some(existing) if existing == &path => {}
            Some(_) => {
                inferred.remove(&alias);
                ambiguous.insert(alias);
            }
        }
    }
    imports.extend(inferred);
    imports
}

fn has_dot_import(node: tree_sitter::Node<'_>) -> bool {
    if node.kind() == "import_spec" {
        return node
            .children(&mut node.walk())
            .any(|child| child.kind() == "dot");
    }
    node.children(&mut node.walk()).any(has_dot_import)
}

fn versionless_alias(path: &str) -> Option<String> {
    let mut segments = path.rsplit('/');
    let last = segments.next()?;
    if last
        .strip_prefix('v')
        .is_some_and(|version| !version.is_empty() && version.chars().all(|c| c.is_ascii_digit()))
    {
        return segments
            .next()
            .filter(|name| !name.is_empty())
            .map(str::to_string);
    }
    let (name, version) = last.rsplit_once(".v")?;
    (!name.is_empty() && version.chars().all(|c| c.is_ascii_digit())).then(|| name.to_string())
}
