//! Terraform/HCL reference graph for cross-block data flow analysis.
//!
//! Terraform's "data flow" is a reference graph: `var.x` flows to `local.y`
//! flows to `resource.z.attribute`. This module builds and queries that graph
//! using `hcl-rs` for reference resolution.

use std::collections::{BTreeMap, BTreeSet};

/// The kind of a Terraform entity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum TfEntityKind {
    Variable,
    Local,
    Resource,
    Data,
    Output,
    Module,
    Provider,
}

/// A named entity in a Terraform module.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TfEntity {
    pub kind: TfEntityKind,
    /// Fully qualified name: "var.vpc_cidr", "aws_instance.web", "local.merged"
    pub name: String,
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
    /// Attribute names defined within this entity
    pub attributes: BTreeSet<String>,
}

/// A reference graph for Terraform/HCL files.
///
/// Nodes are named entities (resources, variables, locals, outputs, data sources).
/// Edges are reference dependencies (attribute X references var.Y).
#[derive(Debug, Clone, serde::Serialize)]
pub struct TerraformRefGraph {
    /// Named entities indexed by qualified name
    pub entities: BTreeMap<String, TfEntity>,
    /// Forward edges: entity → entities it references
    pub references: BTreeMap<String, BTreeSet<String>>,
    /// Reverse edges: entity → entities that reference it
    pub referenced_by: BTreeMap<String, BTreeSet<String>>,
}

impl TerraformRefGraph {
    /// Build a reference graph from HCL source files.
    ///
    /// Each entry in `sources` maps filename → HCL source text.
    pub fn build(sources: &BTreeMap<String, String>) -> Self {
        let mut graph = TerraformRefGraph {
            entities: BTreeMap::new(),
            references: BTreeMap::new(),
            referenced_by: BTreeMap::new(),
        };

        // Phase 1: Parse all files and index entities
        for (file, source) in sources {
            if let Ok(body) = hcl::parse(source) {
                graph.index_body(&body, file, source);
            }
        }

        // Phase 2: Resolve references within attribute expressions
        for (file, source) in sources {
            if let Ok(body) = hcl::parse(source) {
                graph.resolve_references(&body, file);
            }
        }

        graph
    }

    /// Index all named entities from a parsed HCL body.
    fn index_body(&mut self, body: &hcl::Body, file: &str, source: &str) {
        let line_offsets = compute_line_offsets(source);

        for structure in body.iter() {
            match structure {
                hcl::Structure::Block(block) => {
                    self.index_block(block, file, source, &line_offsets);
                }
                hcl::Structure::Attribute(attr) => {
                    // Top-level attributes (rare in Terraform, common in HCL)
                    let name = attr.key.to_string();
                    let (start, end) = attr_line_range(attr, source, &line_offsets);
                    self.entities.insert(
                        name.clone(),
                        TfEntity {
                            kind: TfEntityKind::Local,
                            name,
                            file: file.to_string(),
                            start_line: start,
                            end_line: end,
                            attributes: BTreeSet::new(),
                        },
                    );
                }
            }
        }
    }

    fn index_block(
        &mut self,
        block: &hcl::Block,
        file: &str,
        source: &str,
        line_offsets: &[usize],
    ) {
        let block_type = block.identifier.to_string();
        let labels: Vec<String> = block
            .labels
            .iter()
            .map(|l| l.as_str().to_string())
            .collect();

        let (kind, qualified_name) = match block_type.as_str() {
            "variable" => {
                let name = labels.first().cloned().unwrap_or_default();
                (TfEntityKind::Variable, format!("var.{}", name))
            }
            "locals" => {
                // locals block contains multiple local values — index each attribute
                let (start, end) = block_line_range(block, source, line_offsets);
                for attr in block.body.attributes() {
                    let local_name = format!("local.{}", attr.key);
                    let (attr_start, attr_end) = attr_line_range(attr, source, line_offsets);
                    self.entities.insert(
                        local_name.clone(),
                        TfEntity {
                            kind: TfEntityKind::Local,
                            name: local_name,
                            file: file.to_string(),
                            start_line: attr_start.max(start),
                            end_line: attr_end.min(end),
                            attributes: BTreeSet::new(),
                        },
                    );
                }
                return; // locals are handled per-attribute above
            }
            "resource" => {
                let rtype = labels.first().cloned().unwrap_or_default();
                let rname = labels.get(1).cloned().unwrap_or_default();
                (TfEntityKind::Resource, format!("{}.{}", rtype, rname))
            }
            "data" => {
                let dtype = labels.first().cloned().unwrap_or_default();
                let dname = labels.get(1).cloned().unwrap_or_default();
                (TfEntityKind::Data, format!("data.{}.{}", dtype, dname))
            }
            "output" => {
                let name = labels.first().cloned().unwrap_or_default();
                (TfEntityKind::Output, format!("output.{}", name))
            }
            "module" => {
                let name = labels.first().cloned().unwrap_or_default();
                (TfEntityKind::Module, format!("module.{}", name))
            }
            "provider" => {
                let name = labels.first().cloned().unwrap_or_default();
                (TfEntityKind::Provider, format!("provider.{}", name))
            }
            _ => return, // Skip unknown block types
        };

        let (start, end) = block_line_range(block, source, line_offsets);
        let mut attrs = BTreeSet::new();
        for attr in block.body.attributes() {
            attrs.insert(attr.key.to_string());
        }

        self.entities.insert(
            qualified_name.clone(),
            TfEntity {
                kind,
                name: qualified_name,
                file: file.to_string(),
                start_line: start,
                end_line: end,
                attributes: attrs,
            },
        );
    }

    /// Walk all expressions in the body and record references between entities.
    fn resolve_references(&mut self, body: &hcl::Body, file: &str) {
        for structure in body.iter() {
            match structure {
                hcl::Structure::Block(block) => {
                    let block_type = block.identifier.to_string();
                    let labels: Vec<String> = block
                        .labels
                        .iter()
                        .map(|l| l.as_str().to_string())
                        .collect();

                    let source_entity = match block_type.as_str() {
                        "variable" => {
                            format!("var.{}", labels.first().cloned().unwrap_or_default())
                        }
                        "locals" => {
                            // For locals, resolve each attribute independently
                            for attr in block.body.attributes() {
                                let local_name = format!("local.{}", attr.key);
                                let refs = extract_references_from_expr(&attr.expr);
                                for r in refs {
                                    self.add_reference(&local_name, &r);
                                }
                            }
                            continue;
                        }
                        "resource" => format!(
                            "{}.{}",
                            labels.first().cloned().unwrap_or_default(),
                            labels.get(1).cloned().unwrap_or_default()
                        ),
                        "data" => format!(
                            "data.{}.{}",
                            labels.first().cloned().unwrap_or_default(),
                            labels.get(1).cloned().unwrap_or_default()
                        ),
                        "output" => {
                            format!("output.{}", labels.first().cloned().unwrap_or_default())
                        }
                        "module" => {
                            format!("module.{}", labels.first().cloned().unwrap_or_default())
                        }
                        _ => continue,
                    };

                    // Walk all attributes in this block for references
                    let refs = extract_references_from_block_body(&block.body);
                    for r in refs {
                        self.add_reference(&source_entity, &r);
                    }

                    // Also recurse into nested blocks (e.g., ingress { ... })
                    for nested in block.body.blocks() {
                        let nested_refs = extract_references_from_block_body(&nested.body);
                        for r in nested_refs {
                            self.add_reference(&source_entity, &r);
                        }
                    }
                }
                hcl::Structure::Attribute(attr) => {
                    let name = attr.key.to_string();
                    let refs = extract_references_from_expr(&attr.expr);
                    for r in refs {
                        self.add_reference(&name, &r);
                    }
                }
            }
        }
        // Suppress unused variable warning — file is used for context in future extensions
        let _ = file;
    }

    fn add_reference(&mut self, from: &str, to: &str) {
        self.references
            .entry(from.to_string())
            .or_default()
            .insert(to.to_string());
        self.referenced_by
            .entry(to.to_string())
            .or_default()
            .insert(from.to_string());
    }

    /// Find all entities reachable forward from a given entity (BFS).
    pub fn forward_reachable(&self, from: &str) -> BTreeSet<String> {
        let mut visited = BTreeSet::new();
        let mut queue = vec![from.to_string()];
        while let Some(current) = queue.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if let Some(refs) = self.references.get(&current) {
                for r in refs {
                    if !visited.contains(r) {
                        queue.push(r.clone());
                    }
                }
            }
        }
        visited.remove(from);
        visited
    }

    /// Find all entities reachable backward from a given entity (BFS).
    pub fn backward_reachable(&self, from: &str) -> BTreeSet<String> {
        let mut visited = BTreeSet::new();
        let mut queue = vec![from.to_string()];
        while let Some(current) = queue.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if let Some(refs) = self.referenced_by.get(&current) {
                for r in refs {
                    if !visited.contains(r) {
                        queue.push(r.clone());
                    }
                }
            }
        }
        visited.remove(from);
        visited
    }

    /// Get all entities whose lines overlap the given set of diff lines in a file.
    pub fn entities_touching_lines(&self, file: &str, lines: &BTreeSet<usize>) -> Vec<String> {
        let mut result = Vec::new();
        for entity in self.entities.values() {
            if entity.file == file {
                for &line in lines {
                    if line >= entity.start_line && line <= entity.end_line {
                        result.push(entity.name.clone());
                        break;
                    }
                }
            }
        }
        result.sort();
        result.dedup();
        result
    }
}

/// Extract variable/local/data/module references from an HCL expression.
fn extract_references_from_expr(expr: &hcl::Expression) -> BTreeSet<String> {
    let mut refs = BTreeSet::new();
    collect_refs_from_expr(expr, &mut refs);
    refs
}

fn collect_refs_from_expr(expr: &hcl::Expression, refs: &mut BTreeSet<String>) {
    match expr {
        hcl::Expression::Variable(var) => {
            let name = var.to_string();
            // Standalone variable references like "var", "local", "data", "module"
            // are part of traversals; skip bare names
            if !matches!(
                name.as_str(),
                "var" | "local" | "data" | "module" | "each" | "self" | "null" | "true" | "false"
            ) {
                refs.insert(name);
            }
        }
        hcl::Expression::Traversal(traversal) => {
            let root = traversal.expr.to_string();
            match root.as_str() {
                "var" | "local" | "data" | "module" => {
                    // Build qualified reference: var.name, local.name, etc.
                    let mut parts = vec![root.clone()];
                    for op in &traversal.operators {
                        match op {
                            hcl::TraversalOperator::GetAttr(ident) => {
                                parts.push(ident.to_string());
                            }
                            hcl::TraversalOperator::Index(idx) => {
                                collect_refs_from_expr(idx, refs);
                            }
                            _ => {}
                        }
                    }
                    if parts.len() >= 2 {
                        // For "var.x" → "var.x"; for "data.type.name" → "data.type.name"
                        refs.insert(parts.join("."));
                    }
                }
                _ => {
                    // Could be resource references like aws_instance.web.id
                    // Build the reference chain
                    let mut parts = vec![root];
                    for op in &traversal.operators {
                        if let hcl::TraversalOperator::GetAttr(ident) = op {
                            parts.push(ident.to_string());
                        }
                    }
                    if parts.len() >= 2 {
                        // Try "type.name" (first two parts) as a resource reference
                        let resource_ref = format!("{}.{}", parts[0], parts[1]);
                        refs.insert(resource_ref);
                    }
                }
            }
        }
        hcl::Expression::FuncCall(call) => {
            for arg in &call.args {
                collect_refs_from_expr(arg, refs);
            }
        }
        hcl::Expression::Conditional(cond) => {
            collect_refs_from_expr(&cond.cond_expr, refs);
            collect_refs_from_expr(&cond.true_expr, refs);
            collect_refs_from_expr(&cond.false_expr, refs);
        }
        hcl::Expression::Operation(op) => match op.as_ref() {
            hcl::Operation::Unary(unary) => {
                collect_refs_from_expr(&unary.expr, refs);
            }
            hcl::Operation::Binary(binary) => {
                collect_refs_from_expr(&binary.lhs_expr, refs);
                collect_refs_from_expr(&binary.rhs_expr, refs);
            }
        },
        hcl::Expression::ForExpr(for_expr) => {
            collect_refs_from_expr(&for_expr.collection_expr, refs);
            if let Some(key) = &for_expr.key_expr {
                collect_refs_from_expr(key, refs);
            }
            collect_refs_from_expr(&for_expr.value_expr, refs);
            if let Some(cond) = &for_expr.cond_expr {
                collect_refs_from_expr(cond, refs);
            }
        }
        hcl::Expression::Array(items) => {
            for item in items {
                collect_refs_from_expr(item, refs);
            }
        }
        hcl::Expression::Object(obj) => {
            for (k, v) in obj {
                match k {
                    hcl::ObjectKey::Expression(expr) => collect_refs_from_expr(expr, refs),
                    hcl::ObjectKey::Identifier(_) => {} // identifiers aren't references
                    _ => {}
                }
                collect_refs_from_expr(v, refs);
            }
        }
        hcl::Expression::TemplateExpr(tmpl) => {
            // Template expressions contain interpolations as embedded HCL
            // The string representation may contain ${var.x} references
            // hcl-rs resolves these during parsing into the expression tree
            let text = tmpl.to_string();
            extract_refs_from_template_text(&text, refs);
        }
        hcl::Expression::Parenthesis(inner) => {
            collect_refs_from_expr(inner, refs);
        }
        // Literals, null, bool — no references
        _ => {}
    }
}

/// Extract var/local/data/module references from template strings.
fn extract_refs_from_template_text(text: &str, refs: &mut BTreeSet<String>) {
    // Look for patterns like var.name, local.name in template text
    for prefix in &["var.", "local.", "data.", "module."] {
        let mut search_from = 0;
        while let Some(pos) = text[search_from..].find(prefix) {
            let abs_pos = search_from + pos;
            let rest = &text[abs_pos..];
            // Extract the full dotted reference
            let ref_end = rest
                .find(|c: char| !c.is_alphanumeric() && c != '.' && c != '_')
                .unwrap_or(rest.len());
            let reference = &rest[..ref_end];
            if reference.split('.').count() >= 2 {
                refs.insert(reference.to_string());
            }
            search_from = abs_pos + ref_end;
        }
    }
}

/// Extract all references from all attributes in a block body.
fn extract_references_from_block_body(body: &hcl::Body) -> BTreeSet<String> {
    let mut refs = BTreeSet::new();
    for attr in body.attributes() {
        let attr_refs = extract_references_from_expr(&attr.expr);
        refs.extend(attr_refs);
    }
    refs
}

/// Compute byte offsets for each line start in the source.
fn compute_line_offsets(source: &str) -> Vec<usize> {
    let mut offsets = vec![0];
    for (i, c) in source.char_indices() {
        if c == '\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}

/// Estimate line range for a block by scanning source text.
///
/// hcl-rs doesn't provide span/position information, so we use a heuristic:
/// search for the block's identifier and labels in the source to find start line,
/// then count braces to find the end.
fn block_line_range(block: &hcl::Block, source: &str, line_offsets: &[usize]) -> (usize, usize) {
    let block_type = block.identifier.to_string();
    let labels: Vec<String> = block
        .labels
        .iter()
        .map(|l| format!("\"{}\"", l.as_str()))
        .collect();

    // Build a search pattern: block_type "label1" "label2"
    let pattern = if labels.is_empty() {
        format!("{} {{", block_type)
    } else {
        format!("{} {}", block_type, labels.join(" "))
    };

    for (line_idx, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with(&pattern) || trimmed.contains(&pattern) {
            let start_line = line_idx + 1; // 1-indexed

            // Find closing brace by counting brace depth
            let mut depth = 0i32;
            let mut found_open = false;
            for (end_idx, end_line) in source.lines().enumerate().skip(line_idx) {
                for ch in end_line.chars() {
                    if ch == '{' {
                        depth += 1;
                        found_open = true;
                    } else if ch == '}' {
                        depth -= 1;
                    }
                }
                if found_open && depth == 0 {
                    return (start_line, end_idx + 1);
                }
            }
            return (start_line, source.lines().count());
        }
    }

    let _ = line_offsets; // Used in future extensions
    (1, source.lines().count())
}

/// Estimate line range for an attribute.
fn attr_line_range(attr: &hcl::Attribute, source: &str, _line_offsets: &[usize]) -> (usize, usize) {
    let key = attr.key.to_string();
    for (line_idx, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with(&key) && (trimmed.contains('=') || trimmed.contains(" =")) {
            // Check this is our attribute (not a substring match)
            let after_key = &trimmed[key.len()..].trim_start();
            if after_key.starts_with('=') || after_key.is_empty() {
                return (line_idx + 1, line_idx + 1);
            }
        }
    }
    (1, 1)
}

/// Terraform-specific taint sinks: security-sensitive resource attributes.
pub const TF_TAINT_SINKS: &[&str] = &[
    "cidr_blocks",
    "ipv6_cidr_blocks",
    "ingress",
    "egress",
    "policy",
    "assume_role_policy",
    "user_data",
    "user_data_base64",
    "command",
    "inline",
    "environment",
    "kms_key_id",
    "sse_algorithm",
    "connection",
    "provisioner",
    "security_groups",
    "vpc_security_group_ids",
    "iam_instance_profile",
    "role",
    "role_arn",
    "principal",
    "actions",
    "resources",
    "effect",
];

/// Terraform-specific absence pairs: resources that should have companion resources.
pub const TF_ABSENCE_PAIRS: &[(&str, &str, &str)] = &[
    (
        "aws_s3_bucket",
        "aws_s3_bucket_server_side_encryption_configuration",
        "S3 bucket missing encryption configuration",
    ),
    (
        "aws_s3_bucket",
        "aws_s3_bucket_public_access_block",
        "S3 bucket missing public access block",
    ),
    (
        "aws_s3_bucket",
        "aws_s3_bucket_versioning",
        "S3 bucket missing versioning configuration",
    ),
    (
        "aws_db_instance",
        "aws_db_instance", // Check for storage_encrypted attribute
        "DB instance should have storage_encrypted = true",
    ),
    (
        "aws_lambda_function",
        "aws_cloudwatch_log_group",
        "Lambda function missing CloudWatch log group",
    ),
    (
        "aws_ecs_service",
        "aws_cloudwatch_log_group",
        "ECS service missing CloudWatch log group",
    ),
    (
        "aws_instance",
        "aws_ebs_encryption_by_default",
        "EC2 instance — consider enabling EBS encryption by default",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ref_graph_basic() {
        let source = r#"
variable "allowed_cidrs" {
  type = list(string)
}

locals {
  merged = concat(var.allowed_cidrs, ["10.0.0.0/8"])
}

resource "aws_security_group" "web" {
  ingress {
    cidr_blocks = local.merged
  }
}
"#;
        let mut sources = BTreeMap::new();
        sources.insert("main.tf".to_string(), source.to_string());
        let graph = TerraformRefGraph::build(&sources);

        // Should have entities
        assert!(
            graph.entities.contains_key("var.allowed_cidrs"),
            "Should have var.allowed_cidrs entity"
        );
        assert!(
            graph.entities.contains_key("local.merged"),
            "Should have local.merged entity"
        );
        assert!(
            graph.entities.contains_key("aws_security_group.web"),
            "Should have aws_security_group.web entity"
        );

        // local.merged references var.allowed_cidrs
        let local_refs = graph.references.get("local.merged").unwrap();
        assert!(
            local_refs.contains("var.allowed_cidrs"),
            "local.merged should reference var.allowed_cidrs, got: {:?}",
            local_refs
        );

        // aws_security_group.web references local.merged
        let sg_refs = graph.references.get("aws_security_group.web").unwrap();
        assert!(
            sg_refs.contains("local.merged"),
            "SG should reference local.merged, got: {:?}",
            sg_refs
        );
    }

    #[test]
    fn test_reachability() {
        let source = r#"
variable "cidrs" {
  type = list(string)
}

locals {
  merged = var.cidrs
}

resource "aws_security_group" "web" {
  ingress {
    cidr_blocks = local.merged
  }
}
"#;
        let mut sources = BTreeMap::new();
        sources.insert("main.tf".to_string(), source.to_string());
        let graph = TerraformRefGraph::build(&sources);

        // backward_reachable from var.cidrs: who references var.cidrs?
        // local.merged references var.cidrs, SG references local.merged
        let back_from_var = graph.backward_reachable("var.cidrs");
        assert!(
            back_from_var.contains("local.merged"),
            "var.cidrs backward should include local.merged, got: {:?}",
            back_from_var
        );
        assert!(
            back_from_var.contains("aws_security_group.web"),
            "var.cidrs backward should include SG (transitive), got: {:?}",
            back_from_var
        );

        // forward_reachable from SG: what does SG reference?
        let forward_from_sg = graph.forward_reachable("aws_security_group.web");
        assert!(
            forward_from_sg.contains("local.merged"),
            "SG forward should reach local.merged, got: {:?}",
            forward_from_sg
        );
    }

    #[test]
    fn test_entities_touching_lines() {
        let source = r#"
variable "cidrs" {
  type = list(string)
}

resource "aws_instance" "web" {
  ami = "ami-123"
  instance_type = "t3.micro"
}
"#;
        let mut sources = BTreeMap::new();
        sources.insert("main.tf".to_string(), source.to_string());
        let graph = TerraformRefGraph::build(&sources);

        // Line 7 is inside aws_instance.web
        let mut lines = BTreeSet::new();
        lines.insert(7);
        let touching = graph.entities_touching_lines("main.tf", &lines);
        assert!(
            touching.contains(&"aws_instance.web".to_string()),
            "Line 7 should touch aws_instance.web, got: {:?}",
            touching
        );
    }
}
