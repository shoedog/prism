# Terraform / HCL Language Support — Analysis & Plan

**Status:** Analysis complete, scaffolding ready
**Date:** 2026-04-02
**Priority:** Must-have (team's own repos)
**Estimated effort:** 2–3 weeks

---

## 1. Why Terraform Needs a Different Analysis Model

Terraform occupies a middle ground between procedural and declarative:

| Aspect | Procedural (C, Python, etc.) | Terraform |
|--------|------------------------------|-----------|
| Execution model | Sequential statements | Declarative DAG, provider-resolved |
| Variables | Mutable, reassignable | Immutable bindings (`var.`, `local.`, `module.`) |
| Functions | User-defined, call graph | Built-in functions only (`lookup`, `merge`, `format`) |
| Control flow | if/else, loops, goto | `count`, `for_each`, `dynamic`, ternary |
| Data flow | Def-use chains through assignments | Reference chains through `var.`/`local.`/`module.` interpolation |
| Side effects | I/O, mutation, syscalls | Resource CRUD via providers |

**Key insight:** Terraform's "data flow" is a **reference graph** — `var.x`
flows to `local.y` flows to `resource.z.attribute`. This maps naturally to
Prism's taint/provenance/membrane algorithms, but the reference resolution
mechanism is fundamentally different from tree-sitter-based def-use analysis.

---

## 2. Crate Evaluation

### tree-sitter-hcl (1.1.0)

**What it provides:** HCL2 syntax tree — blocks, attributes, expressions,
template interpolations. Integrates with Prism's existing tree-sitter pipeline.

**What it lacks:** No reference resolution. `var.vpc_cidr` is parsed as a
`variable_expr` node, but tree-sitter doesn't know which `variable "vpc_cidr"`
block defines it.

**Node types relevant to Prism:**
- `block` — resource, data, variable, output, module, locals, provider
- `attribute` — key = value within a block
- `expression` — values, including interpolations and function calls
- `variable_expr` — `var.x`, `local.y`, `module.z.output`
- `function_call` — built-in functions like `lookup()`, `merge()`
- `template_expr` — `"${var.x}-suffix"` string interpolations
- `conditional` — `condition ? true_val : false_val`
- `for_expr` — `[for x in var.list : x.id]`

### hcl-rs (0.18.5)

**What it provides:** Full HCL2 parser with serde support and expression
evaluation. Resolves the reference graph that tree-sitter cannot.

**Key types:**
- `Body` → list of `Block` and `Attribute`
- `Block` → has `identifier` (resource type), `labels` (resource name), `body`
- `Attribute` → has `key` and `Expression` value
- `Expression` → `Variable`, `FunctionCall`, `Conditional`, `ForExpr`, etc.
- `Traversal` → `var.vpc_cidr` is `Variable("var")` with `GetAttr("vpc_cidr")` traversal

**Limitation:** hcl-rs parses single files. It does not resolve cross-file
references (`module.foo.output_x`), terraform state, or provider schemas.
Cross-file resolution requires walking the module directory and correlating
`variable`/`output` blocks.

### Recommended approach: Dual parser

1. **tree-sitter-hcl** for Prism's standard AST pipeline (line extraction,
   function scoping, diff-line matching)
2. **hcl-rs** for reference resolution — build a `TerraformRefGraph` that maps
   each reference to its definition

---

## 3. Algorithm Mapping

### 3.1 TaintSlice — Variable flow to sensitive attributes

**Sources:** `var.*` (user-supplied tfvars), `data.*` (infrastructure queries)

**Sinks:** Security-sensitive resource attributes:
- `cidr_blocks`, `ingress`, `egress` (network ACLs)
- `policy` (IAM policy documents)
- `user_data` (EC2 launch scripts — shell injection vector)
- `command`, `inline` (provisioner commands)
- `environment` (Lambda/ECS env vars — secret leakage)
- `kms_key_id`, `sse_algorithm` (encryption config)

**Flow model:** `var.x` → `local.y = var.x` → `resource.z.attr = local.y`
This is a 2-hop reference chain. Prism traces the reference graph, not
def-use chains.

**What Prism catches that tfsec/checkov don't:** Multi-hop variable flow.
`var.allowed_cidrs` → `local.merged_cidrs = concat(var.allowed_cidrs, ...)` →
`aws_security_group.rule.cidr_blocks = local.merged_cidrs`. Static checkers
evaluate each block independently; Prism traces the full chain.

### 3.2 ProvenanceSlice — Origin classification

| Origin | Pattern | Classification |
|--------|---------|----------------|
| `var.*` | `variable` blocks | UserInput (comes from tfvars, CLI, CI pipeline) |
| `data.*` | `data` blocks | Database (infrastructure query) |
| `module.*` | `module` blocks | ModuleOutput (computed) |
| `local.*` | `locals` blocks | Computed (derived) |
| Literals | Hardcoded strings/numbers | Constant |

### 3.3 MembraneSlice — Module boundary analysis

Terraform modules have explicit interfaces:
- **Inputs:** `variable` blocks define what a module accepts
- **Outputs:** `output` blocks define what a module exposes
- **Membrane:** The module boundary is the set of `variable` + `output` blocks

When a diff touches a resource inside a module, MembraneSlice identifies:
1. Which `variable` inputs feed into the changed resource
2. Which `output` values are derived from the changed resource
3. Which callers (`module` blocks in parent) pass values through those variables

### 3.4 AbsenceSlice — Missing companion resources

| Resource | Expected companion | Pattern |
|----------|-------------------|---------|
| `aws_s3_bucket` | `aws_s3_bucket_server_side_encryption_configuration` | Encryption |
| `aws_s3_bucket` | `aws_s3_bucket_public_access_block` | Public access block |
| `aws_security_group` | `aws_security_group_rule` (with restrict) | Explicit rules |
| `aws_db_instance` | `aws_db_instance` with `storage_encrypted = true` | Encryption |
| `aws_lambda_function` | `aws_cloudwatch_log_group` | Logging |

### 3.5 SymmetrySlice — Resource pair consistency

If a resource is added in one environment/region, the corresponding resource
should exist in others. This applies to:
- Multi-region deployments (DNS, CDN, failover)
- Environment parity (dev/staging/prod)
- Module instances with `count` or `for_each`

---

## 4. Architecture: TerraformRefGraph

```rust
/// A reference graph for Terraform/HCL files.
///
/// Nodes are named entities (resources, variables, locals, outputs, data sources).
/// Edges are reference dependencies (attribute X references var.Y).
pub struct TerraformRefGraph {
    /// Named entities: "var.vpc_cidr", "local.merged", "aws_instance.web"
    pub entities: BTreeMap<String, TfEntity>,
    /// Forward edges: entity → entities it references
    pub references: BTreeMap<String, BTreeSet<String>>,
    /// Reverse edges: entity → entities that reference it
    pub referenced_by: BTreeMap<String, BTreeSet<String>>,
}

pub struct TfEntity {
    pub kind: TfEntityKind,  // Variable, Local, Resource, Data, Output, Module
    pub name: String,        // Fully qualified: "aws_instance.web"
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
    pub attributes: BTreeMap<String, TfAttribute>,
}

pub enum TfEntityKind {
    Variable,   // variable "name" {}
    Local,      // locals { name = ... }
    Resource,   // resource "type" "name" {}
    Data,       // data "type" "name" {}
    Output,     // output "name" {}
    Module,     // module "name" {}
    Provider,   // provider "name" {}
}
```

### Reference resolution algorithm

1. Parse all `.tf` files in the module directory with hcl-rs
2. Index all named entities by their qualified name
3. Walk each attribute expression, extract `var.*`, `local.*`, `data.*`,
   `module.*` references
4. Build forward and reverse edge maps
5. For taint analysis: BFS forward from source entities through reference edges

---

## 5. Implementation Plan

### Step 1: Language scaffolding (2–3 days)
- Add `Language::Terraform` to enum with `.tf` extension
- Add `tree-sitter-hcl` to Cargo.toml
- Implement `function_node_types()` → `["block"]` (resources are the unit)
- Implement other language methods (control flow, assignments, calls)
- Basic parsing test

### Step 2: TerraformRefGraph (4–5 days)
- New file: `src/terraform.rs` (~300 lines)
- Parse with hcl-rs, build entity index, resolve references
- Forward/reverse reference traversal
- Unit tests for reference chains

### Step 3: Algorithm wiring (3–4 days)
- TaintSlice: reference chain traversal from `var.*` to sensitive attributes
- ProvenanceSlice: origin classification per §3.2
- MembraneSlice: module boundary interface extraction
- Integration tests with Terraform fixtures

### Step 4: CLI integration (1 day)
- `--repo` with `.tf` files auto-detected
- Output includes Terraform-specific finding types

---

## 6. Scaffolding Files

Minimal scaffolding created alongside this plan:
- `src/languages/mod.rs` — `Language::Terraform` variant (deferred to implementation PR)
- `tests/fixtures/terraform/` — sample `.tf` files for test development

---

## 7. What Prism Does NOT Replace

| Tool | Purpose | Complementary? |
|------|---------|----------------|
| **tfsec** | Policy rule checking (open SGs, missing encryption) | Yes — Prism traces data flow, tfsec checks rules |
| **checkov** | CIS benchmark compliance | Yes — orthogonal |
| **terraform plan** | Actual resource diff | Yes — Prism analyzes code, not plan output |
| **sentinel** | Policy-as-code for Terraform Cloud | Yes — different enforcement point |

Prism's value-add is **cross-block data flow**: tracing how a `var.` input
propagates through `local.` definitions to reach a sensitive resource attribute.
This is the gap in existing Terraform static analysis tools.
