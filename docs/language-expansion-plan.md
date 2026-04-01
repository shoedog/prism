# Prism Language Expansion: Analysis & Plan

### March 31, 2026

-----

## 1. Context & Motivation

Prism currently supports 7 languages (Python, JavaScript, TypeScript, Go, Java, C, C++) with 26 slicing algorithms, 151 tests, and tree-sitter-based AST parsing. It serves two distinct audiences:

1. **Platypus team repos** — Python, Go, Rust, JavaScript/TypeScript, and Terraform (HCL). These are the codebases the team writes daily.
1. **Firmware and infrastructure review** — C/C++ firmware for CPE devices (DOCSIS modems, GPON ONUs, CMTS platforms, OLTs, managed switches), shell scripts, Lua (LuCI), Dockerfiles, docker-compose, and various declarative configuration formats (YANG, Protocol Buffers, device trees, linker scripts, Makefiles).

The code review agent pipeline uses Prism to extract targeted context for LLM reviewers — reducing the amount of code/text they need to reason about for each MR. Expanding language coverage directly increases the reviewer’s effectiveness across more of access network’s codebase.

-----

## 2. Two Analysis Models

The candidate languages divide cleanly into two categories based on how “slicing” applies:

### 2.1 Procedural Languages — Slicing Applies Directly

Languages with functions, variables, control flow, and data flow. Prism’s existing algorithms (taint, provenance, absence, membrane, quantum, circular, data flow, call graph, etc.) apply with language-specific patterns.

**Languages:** Rust, Shell/Bash, Lua, Terraform/HCL (partial — has a variable/reference graph)

### 2.2 Declarative Formats — Context Extraction, Not Slicing

Languages/formats that describe configuration, schemas, or structure rather than procedural logic. These have no def-use chains or call graphs, but they do have reference/containment graphs that enable targeted context extraction.

**Formats:** YANG/NETCONF, Protocol Buffers/FlatBuffers, Dockerfiles, docker-compose, Device Tree Source, Linker Scripts, Makefiles/CMake, Assembly

The analysis operation for declarative formats is: given a diff, parse the format into a structured representation, identify which logical units (resources, messages, targets, services) are touched, trace their references/dependencies, and emit the relevant subtree as context for the LLM reviewer.

This is “slicing for structured data” — simpler than program slicing but serving the same purpose: reducing what the reviewer needs to read.

-----

## 3. Procedural Languages — Prism Extension Plan

### 3.1 Rust

**Priority:** 1 (must-have — team’s own code)

**Rationale:** The Platypus team writes Rust. Prism itself is written in Rust. The Linux kernel has official Rust support since 6.1. Firmware teams are adopting Rust for new kernel drivers and safety-critical daemons within a 12–24 month horizon.

**Tree-sitter crate:** `tree-sitter-rust` (0.24.2) — mature, actively maintained, handles full Rust grammar including lifetimes, traits, macros, and async/await.

**Algorithm coverage:**

|Algorithm      |Rust-specific behavior                                                                                      |Patterns                                                                                                           |
|---------------|------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------|
|TaintSlice     |`unsafe` blocks as explicit taint boundaries; `*mut T` dereference in unsafe is the primary safety concern  |Sinks: `ptr::write`, `ptr::copy`, `transmute`, `from_raw_parts`, `CString::from_raw`                               |
|AbsenceSlice   |`Mutex::lock()` returning `MutexGuard` without explicit drop; `File::open` without read/close               |Open: `Mutex::lock(`, `RwLock::write(`, `File::open(`, `TcpListener::bind(` / Close: `drop(`, `.close(`, scope exit|
|MembraneSlice  |`pub` vs `pub(crate)` vs `pub(super)` boundary analysis is directly meaningful for API surface review       |Visibility modifiers as membrane boundaries                                                                        |
|CircularSlice  |Ownership prevents most cycles, but `Rc<RefCell<T>>` and `Arc<Mutex<T>>` cycles are a real memory leak class|`Rc::new(`, `Arc::new(` in cycle detection                                                                         |
|ProvenanceSlice|Standard input/env origins                                                                                  |Sources: `std::io::stdin()`, `std::env::args()`, `std::env::var(`, `std::fs::read_to_string(`                      |
|QuantumSlice   |`async`/`await`, `tokio::spawn`, `std::thread::spawn`, channels                                             |Async: `spawn(`, `select!`, `channel(`, `.send(`, `.recv(`                                                         |

**Implementation effort:** 2–3 weeks

**Additional crate consideration:** `syn` (2.0) provides full-fidelity Rust AST parsing with type information that tree-sitter lacks. For Rust specifically, `syn` can resolve `unsafe` block boundaries, lifetime annotations, and trait implementations more precisely than tree-sitter’s syntactic-only model. However, `syn` only parses Rust, whereas tree-sitter integrates with Prism’s multi-language architecture. Recommendation: use tree-sitter for consistency, add `syn`-based analysis as a secondary pass if the tree-sitter-only approach produces too many false positives on unsafe/lifetime patterns.

-----

### 3.2 Terraform / HCL

**Priority:** 2 (must-have — team’s own code)

**Rationale:** The Platypus team writes Terraform in a separate repo. Terraform has a unique position: it’s partially procedural (variable references, expressions, module calls form a dependency DAG) and partially declarative (resource configuration is constraint-based). Prism’s slicing algorithms apply to the procedural subset.

**Tree-sitter crate:** `tree-sitter-hcl` (1.1.0) — HCL and Terraform grammar for tree-sitter.

**Additional crate:** `hcl-rs` (0.19.6) — full HCL2 parser with serde support and expression evaluation. This is more capable than tree-sitter for Terraform because it can resolve `var.`, `local.`, `module.` references and build a dependency graph. Recommendation: use both — `tree-sitter-hcl` for Prism’s existing AST infrastructure, `hcl-rs` for reference resolution that tree-sitter can’t do.

**Algorithm coverage:**

|Algorithm      |Terraform-specific behavior                                                                                                              |Patterns                                                                                                      |
|---------------|-----------------------------------------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------|
|TaintSlice     |`var.` inputs flowing to security-sensitive resource attributes (e.g., user-supplied CIDR reaching `ingress` rules, IAM policy documents)|Sinks: `policy`, `ingress`, `cidr_blocks`, `user_data`, `command`, `inline` / Sources: `var.`, `data.` lookups|
|ProvenanceSlice|Classify origins as `tfvars` (user input), `data` sources (infrastructure query), `module` outputs, hardcoded values                     |`var.` → tfvars, `data.` → infrastructure, `module.` → module output                                          |
|MembraneSlice  |Module boundary analysis — what variables/outputs cross the module interface                                                             |`variable {}` and `output {}` blocks define the membrane                                                      |
|SymmetrySlice  |If a resource is added, corresponding security group rules, IAM policies, or DNS records should also be added                            |Resource-type-specific symmetry pairs                                                                         |
|AbsenceSlice   |Limited — “missing encryption config” is rule-based (tfsec/checkov territory), not absence-pair                                          |Possible: `aws_s3_bucket` without `aws_s3_bucket_server_side_encryption_configuration`                        |

**What Prism catches that tfsec/checkov don’t:** Data-flow issues — tainted variable flowing through three levels of `local.` definitions to reach a sensitive attribute. Static policy checkers evaluate each resource block in isolation; Prism traces the variable graph.

**What tfsec/checkov catch that Prism doesn’t:** Policy rules — “is this IAM policy too broad,” “is encryption missing,” “is this security group open to 0.0.0.0/0.” These are constraint checks, not data flow.

**Recommendation:** Add HCL to Prism for taint/provenance/membrane analysis. Continue running tfsec/checkov as external tools alongside Prism in the reviewer pipeline.

**Implementation effort:** 2–3 weeks (including `hcl-rs` reference resolution)

-----

### 3.3 Shell / Bash

**Priority:** 3 (should-have — firmware review target)

**Rationale:** Init scripts (`/etc/init.d/`), firmware update scripts (`sysupgrade`, `fw_update`), factory provisioning scripts, and build automation are shell/bash. Command injection through unquoted variables in shell scripts is a common and high-severity class. Shell scripts also set security-sensitive configurations (firewall rules, permissions). Bugs in update scripts can brick devices.

**Tree-sitter crate:** `tree-sitter-bash` (0.25.1) — mature, handles most bash constructs.

**Algorithm coverage:**

|Algorithm      |Shell-specific behavior                                                                                 |Patterns                                                                                                                                |
|---------------|--------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------|
|TaintSlice     |This is the killer use case — shell command injection is trivial to introduce and hard to audit manually|Sources: `$1`, `$@`, `$*`, `$INPUT`, `read`, `curl` / Sinks: `eval`, `exec`, ``` backtick, `$(...)`, unquoted `$var` in command position|
|AbsenceSlice   |Signal handler cleanup                                                                                  |Open: `trap` setup / Close: corresponding cleanup handler                                                                               |
|ProvenanceSlice|Script argument origin classification                                                                   |`$1`–`$9` → script_args, `$INPUT`/`read` → user_input, `source`/`.` → config                                                            |
|QuantumSlice   |Background processes                                                                                    |`&` suffix, `wait`, subshells                                                                                                           |

**Implementation effort:** 1–2 weeks

-----

### 3.4 Lua

**Priority:** 4 (should-have — firmware review target)

**Rationale:** OpenWrt’s LuCI web interface (used on home gateways and some ONUs) is entirely Lua. Lua security bugs in LuCI are a direct attack surface — command injection through `os.execute()`, path traversal in file access, and authentication bypasses have appeared in the CVE database.

**Tree-sitter crate:** `tree-sitter-lua` (0.5.0) — mature and actively maintained.

**Algorithm coverage:**

|Algorithm      |Lua-specific behavior                                         |Patterns                                                                                                                                   |
|---------------|--------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------|
|TaintSlice     |Command injection and template injection in LuCI web interface|Sources: `ngx.req.*`, form POST body, `luci.http.formvalue` / Sinks: `os.execute(`, `io.popen(`, `luci.sys.exec(`, `loadstring(`, `dofile(`|
|ProvenanceSlice|User input classification for web context                     |`ngx.var.*`, `luci.http.*` → user_input, `uci.get(` → config                                                                               |
|MembraneSlice  |Module `require` boundaries                                   |`require(` calls define module edges                                                                                                       |
|AbsenceSlice   |File handle cleanup                                           |Open: `io.open(` / Close: `file:close(`                                                                                                    |
|SymmetrySlice  |Encode/decode, serialize/deserialize pairs                    |`encode`/`decode`, `serialize`/`deserialize`                                                                                               |

**Implementation effort:** 1–2 weeks

-----

## 4. Declarative Formats — Context Extraction Plan

For declarative formats, the goal is not full slicing but targeted context extraction: given a diff, emit the minimum relevant structure so the LLM reviewer reasons about less text. The analysis operation is:

1. Parse the format into a structured representation
1. Identify which logical units (resources, messages, targets, services, nodes) are touched by the diff
1. Trace references/dependencies N hops outward
1. Emit as structured context blocks

### 4.1 Dockerfiles

**Priority:** 1 (team’s own repos)

**Relevance:** The Platypus team maintains Dockerfiles and docker-compose files in separate repos. Multi-stage builds create implicit dependencies between stages. Base image version changes have security implications. Environment variable and secret propagation between build stages and compose services is a common source of misconfiguration.

**Crates:**

|Crate                   |Version|What it provides                                                                                                                                     |
|------------------------|-------|-----------------------------------------------------------------------------------------------------------------------------------------------------|
|`tree-sitter-dockerfile`|0.2.0  |Dockerfile grammar — integrates with Prism’s existing tree-sitter infrastructure                                                                     |
|`dockerfile-parser-rs`  |3.3.0  |Full Dockerfile parser with structured representation — resolves `ARG`/`ENV` propagation between stages, identifies `COPY --from=` stage dependencies|
|`docker-compose-types`  |0.23.0 |Typed docker-compose.yml deserialization — service dependency graph, volume/port/env mappings between services                                       |
|`tree-sitter-yaml`      |0.7.2  |YAML grammar — for docker-compose if using tree-sitter consistently; also covers Ansible playbooks, K8s manifests, CI configs                        |

**Context extraction operations:**

- **Multi-stage build dependency:** Given a diff touching a `RUN` command in build stage N, extract the `FROM` chain, `COPY --from=` references, and all `ARG`/`ENV` values that flow into that stage.
- **Base image tracking:** If a `FROM` line changes, extract the downstream stages that inherit from it and any `COPY --from=` that reference it.
- **Compose service graph:** Given a diff to one service definition, extract `depends_on` services, shared volumes/networks, and environment variable cross-references.
- **Security-relevant extraction:** `USER` directives, `EXPOSE` ports, `HEALTHCHECK` definitions, `--privileged` or `--cap-add` in compose — extract these as context when any nearby configuration changes.

**Implementation effort:** 1–2 weeks

-----

### 4.2 Protocol Buffers

**Priority:** 2 (nice-to-have — relevant for gRPC/protobuf for control-plane IPC)

**Relevance:** Modern firmware IPC between management daemon, data-plane agent, and hardware abstraction layer increasingly uses Protocol Buffers. Schema changes break wire compatibility between components that may be on different update schedules.

**Crates:**

|Crate              |Version|What it provides                                                                                                                           |
|-------------------|-------|-------------------------------------------------------------------------------------------------------------------------------------------|
|`tree-sitter-proto`|0.4.0  |Proto2/3 grammar — integrates with Prism’s tree-sitter infrastructure                                                                      |
|`protobuf-parse`   |3.7.2  |Full `.proto` file parser into `FileDescriptorSet` — resolves message references, knows field numbers, can detect wire-compatibility breaks|

**Context extraction operations:**

- **Message reference graph:** Given a diff adding/removing a field, extract the containing `message` definition and all messages that reference it (nested messages, repeated fields, `oneof` members).
- **Field number stability:** Detect reused or removed field numbers that break wire compatibility.
- **Service endpoint context:** If a `service` or `rpc` definition changes, extract the request/response message types and their full definitions.

**Implementation effort:** 1 week

-----

### 4.3 YANG / NETCONF

**Priority:** 3 (skip for now — use existing tools)

**Relevance:** CMTS and OLT devices are managed via NETCONF/YANG. YANG data models define configuration schemas. Changes to YANG models affect configuration validation, default values, and constraint enforcement.

**Crates:**

|Crate             |Version|What it provides                                                                   |
|------------------|-------|-----------------------------------------------------------------------------------|
|`tree-sitter-yang`|0.1.3  |YANG grammar — basic, integrates with tree-sitter                                  |
|`yang-rs`         |0.1.1  |YANG parser library — resolves `augment`, `deviation`, `uses`/`grouping` references|

**Assessment:** YANG model review requires domain-specific schema validation (`pyang`, `yanglint`) more than context extraction. The tree-sitter grammar is immature. Defer unless YANG model changes show up frequently in reviewed MRs.

-----

### 4.4 Makefiles / CMake

**Priority:** 4 (skip for now — use flag audit tools)

**Crates:** `tree-sitter-make` (1.1.1), `tree-sitter-cmake` (0.7.1)

**Assessment:** The highest-value Makefile analysis is security flag auditing (`-fstack-protector`, `-D_FORTIFY_SOURCE`, `-fPIE`) — this is rule-based checking, not context extraction. Prism’s slice-based analysis doesn’t map well. Defer.

-----

### 4.5 Device Tree Source, Linker Scripts, Assembly

**Priority:** Skip

**Assessment per the gap analysis:**

- **Device Tree Source** (.dts/.dtsi): No mature tree-sitter grammar. DTS changes are better served by `dtc` warnings and `dt-validate`. The data model (hardware topology) is fundamentally different from code or config reference graphs.
- **Linker Scripts** (.ld/.lds): No tree-sitter grammar. Memory layout validation requires comparing `MEMORY` region sizes against hardware specs — outside Prism’s analysis model.
- **Assembly** (ARM, MIPS, x86): Structural analysis provides limited value. Jump target resolution is undecidable in general. At most, a display-only mode showing diff context without algorithmic analysis. `tree-sitter-asm` (0.24.0) exists but is less mature.

-----

## 5. Crates Beyond Tree-Sitter

### 5.1 Where Tree-Sitter Falls Short

Tree-sitter is syntactic only — it parses AST structure but knows nothing about types, imports, scoping rules, or semantic meaning. For Prism’s slicing, these gaps manifest as:

|Gap                         |Impact on Prism                                                                                                                                                       |Current mitigation                                                                  |
|----------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------|
|**No type information**     |`dev->name` vs `dev->id` are different taint paths but tree-sitter sees both as `field_expression`. Struct field tracking (P2 item) can’t be done purely syntactically|Name-based tracking with `extract_lvalue_names()`                                   |
|**No import resolution**    |Cross-file taint/membrane analysis uses name-based matching, which conflates same-named functions from different modules                                              |`static_functions` set for C/C++ disambiguation; no equivalent for Python/JS imports|
|**No preprocessor handling**|C/C++ macros produce ERROR nodes, silently degrading analysis quality                                                                                                 |ERROR node counting and `analysis_quality: degraded` reporting                      |
|**No control flow graph**   |Path-sensitive analysis (“this taint path is unreachable because of a guard”) is impossible with AST alone                                                            |Not mitigated — Prism does path-insensitive analysis                                |
|**No semantic scoping**     |Variable shadowing in nested scopes can create false def-use edges                                                                                                    |`find_variable_references_scoped` handles some cases                                |

### 5.2 Crates That Strengthen Prism’s Core Analysis

|Crate                        |Version|What it adds                                                                                                                                         |Where it helps                                                                                                                                                                                                                                                            |Priority                    |
|-----------------------------|-------|-----------------------------------------------------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------|
|`petgraph`                   |0.8.3  |Efficient graph data structures and algorithms — SCC (strongly connected components), dominator trees, topological sort, shortest paths, reachability|Call graph cycle detection in CircularSlice (currently hand-rolled BFS). Dominator analysis for potential future CFG work. More efficient BFS/DFS than hand-rolled `BTreeSet` frontiers in GradientSlice. Topological sort for build-order analysis in declarative formats|High                        |
|`oxc_parser` + `oxc_semantic`|0.123.0|Rust-native JS/TS parser with scope analysis, symbol binding, and type narrowing. 3–5x faster than tree-sitter for JS                                |Replaces tree-sitter-javascript/typescript with semantic-aware parsing. Resolves which `query` refers to which import, eliminating false taint matches. Knows about variable shadowing, hoisting, and module-level bindings                                               |Medium                      |
|`syn`                        |2.0.117|Full-fidelity Rust AST parser used by proc-macros and rust-analyzer                                                                                  |For Rust language support: `unsafe` block boundary detection, lifetime annotations, trait resolution, derive macro expansion are all first-class in `syn`. More precise than tree-sitter for Rust-specific safety analysis                                                |Medium (Rust-specific)      |
|`regex`                      |1.x    |Regular expression matching                                                                                                                          |Tighten sink/source pattern matching beyond the current substring + exact-match model. Word-boundary matching, pattern groups, look-ahead/behind for more precise filtering                                                                                               |Low                         |
|`rayon`                      |1.x    |Data parallelism — parallel iterators                                                                                                                |Parse files in parallel across large firmware trees with hundreds of files. `files.par_iter().map(                                                                                                                                                                        |f                           |
|`ignore`                     |0.4.x  |Gitignore-aware recursive file walking                                                                                                               |For `discover.py` replacement in Rust. Respects `.gitignore` and `.prismignore`, handles symlinks, efficient directory traversal                                                                                                                                          |Medium                      |
|`hcl-rs`                     |0.19.6 |Full HCL2 parser with serde and expression evaluation                                                                                                |Terraform reference resolution — `var.x`, `local.y`, `module.z.output` dependency graph building that tree-sitter-hcl can’t do                                                                                                                                            |High (for Terraform support)|

### 5.3 Crates for Declarative Format Parsing

|Crate                   |Version|Format            |What it provides                                                                                                     |Context extraction value                                                                                     |
|------------------------|-------|------------------|---------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------|
|`dockerfile-parser-rs`  |3.3.0  |Dockerfile        |Full parser with structured representation, `ARG`/`ENV` propagation tracking, multi-stage build dependency resolution|Given a diff touching stage N, extract the `FROM` chain and all values flowing into that stage               |
|`docker-compose-types`  |0.23.0 |docker-compose.yml|Typed deserialization with service dependency graph                                                                  |Given a diff to one service, extract `depends_on` services, shared volumes/networks, and env cross-references|
|`protobuf-parse`        |3.7.2  |Protocol Buffers  |Full `.proto` parser into `FileDescriptorSet`, resolves message references, knows field numbers                      |Given a field change, extract containing message + all referencing messages + wire compatibility impact      |
|`yang-rs`               |0.1.1  |YANG              |YANG parser resolving `augment`, `deviation`, `uses`/`grouping`                                                      |Given a diff to a YANG `leaf`, extract containing hierarchy and constraint context                           |
|`serde_yaml`            |0.9.x  |YAML (general)    |Generic YAML parsing with serde support                                                                              |Docker-compose, Ansible, K8s manifests, CI configs — parse and extract diff-relevant subtrees                |
|`tree-sitter-yaml`      |0.7.2  |YAML              |YAML grammar for tree-sitter                                                                                         |Consistent tree-sitter integration for all YAML-based formats                                                |
|`tree-sitter-dockerfile`|0.2.0  |Dockerfile        |Dockerfile grammar for tree-sitter                                                                                   |Integrates with Prism’s existing tree-sitter infrastructure                                                  |
|`tree-sitter-proto`     |0.4.0  |Protocol Buffers  |Proto2/3 grammar for tree-sitter                                                                                     |Integrates with Prism’s existing tree-sitter infrastructure                                                  |
|`tree-sitter-make`      |1.1.1  |Makefile          |Makefile grammar for tree-sitter                                                                                     |Target dependency extraction                                                                                 |
|`tree-sitter-cmake`     |0.7.1  |CMake             |CMake grammar for tree-sitter                                                                                        |CMake target/library dependency extraction                                                                   |

-----

## 6. Architecture Decision: One Repo or Two?

### Option A: Single Binary with Feature Flags

Add all language support to Prism behind Cargo feature flags:

```toml
[features]
default = ["python", "javascript", "typescript", "go", "java", "c", "cpp"]
rust-lang = ["tree-sitter-rust"]
terraform = ["tree-sitter-hcl", "hcl-rs"]
bash = ["tree-sitter-bash"]
lua = ["tree-sitter-lua"]
docker = ["tree-sitter-dockerfile", "dockerfile-parser-rs", "docker-compose-types"]
protobuf = ["tree-sitter-proto", "protobuf-parse"]
full = ["rust-lang", "terraform", "bash", "lua", "docker", "protobuf"]
```

Two subcommands:

```
prism slice   -- procedural language slicing (existing behavior)
prism context -- declarative format context extraction (new)
```

**Pros:** Single binary deployment, shared infrastructure (diff input, findings output, file discovery), consistent build/test/CI. The reviewer pipeline invokes one tool.

**Cons:** Larger binary when built with `--features full`. More dependencies to compile. Declarative format analysis has a fundamentally different analysis model that may feel shoehorned into Prism’s slicing architecture.

### Option B: Two Repos, Shared Schema

Prism stays focused on slicing. A second tool (e.g., `platypus-context` or `prism-extract`) handles declarative format context extraction. Both share:

- Findings JSON schema (from `agent-knowledge`)
- Diff input model
- GitLab posting pipeline (the Rust orchestrator script invokes both)

**Pros:** Clean separation of concerns. Each tool evolves independently. Prism stays lean.

**Cons:** Two binaries to deploy. Duplicated infrastructure (diff parsing, file discovery, output formatting). Coordination overhead for shared schema changes.

### Recommendation: Option A — Single Binary

The overlap in infrastructure is substantial: diff parsing, file walking, tree-sitter parsing, findings output, CLI interface. Feature flags keep the binary lean for environments that don’t need all languages. The `prism slice` / `prism context` subcommand split keeps the conceptual model clean.

The declarative format analysis is structurally simpler than slicing (parse → find touched units → trace references → emit context), so it won’t create unmanageable complexity in the codebase. Each format’s context extractor is a self-contained module behind a feature flag.

-----

## 7. Implementation Roadmap

### Phase 1 — Team’s Own Languages (Weeks 1–6)

|Week|Item                     |Deliverable                                                                                                                                                                                                 |
|----|-------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
|1–3 |Rust language support    |`Language::Rust`, all algorithm wiring, taint sinks (`unsafe` boundaries), absence pairs (`Mutex::lock`), provenance sources, quantum async (`tokio::spawn`, channels). Integration tests for each algorithm|
|3–5 |Terraform / HCL support  |`Language::Hcl`, tree-sitter-hcl integration, `hcl-rs` reference resolver, taint through `var.`/`local.` to sensitive resource attributes, module membrane analysis. Integration tests                      |
|5–6 |Docker context extraction|`prism context` subcommand, `dockerfile-parser-rs` integration, multi-stage build dependency tracking, `docker-compose-types` service graph extraction. Integration tests                                   |

**Cargo.toml additions (Phase 1):**

```toml
# Procedural language grammars
tree-sitter-rust = "0.24"

# Terraform / HCL
tree-sitter-hcl = "1.1"
hcl-rs = "0.19"

# Docker
tree-sitter-dockerfile = "0.2"
dockerfile-parser-rs = "3.3"
docker-compose-types = "0.23"
tree-sitter-yaml = "0.7"

# Infrastructure improvements
petgraph = "0.8"
ignore = "0.4"
```

### Phase 2 — Firmware Review Languages (Weeks 7–10)

|Week|Item                |Deliverable                                                                                                                                                                            |
|----|--------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
|7–8 |Shell / Bash support|`Language::Bash`, taint sources (`$1`, `$@`, `read`), sinks (`eval`, backtick, `$()`), absence pairs (`trap`), provenance classification. Integration tests                            |
|9–10|Lua support         |`Language::Lua`, taint sources (`ngx.req.*`, `luci.http.*`), sinks (`os.execute`, `io.popen`, `loadstring`), absence pairs (`io.open`/`close`), membrane (`require`). Integration tests|

**Cargo.toml additions (Phase 2):**

```toml
tree-sitter-bash = "0.25"
tree-sitter-lua = "0.5"
```

### Phase 3 — Schema Formats & Analysis Improvements (Weeks 11–14)

|Week |Item                               |Deliverable                                                                                                                                                            |
|-----|-----------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------|
|11–12|Protocol Buffers context extraction|`tree-sitter-proto` or `protobuf-parse` integration, message reference graph, field number stability detection. Integration tests                                      |
|12–13|`oxc_semantic` for JS/TS           |Replace or supplement tree-sitter-javascript with `oxc_parser` + `oxc_semantic` for scope-aware analysis. Measure false positive reduction on existing JS/TS test cases|
|13–14|`petgraph` migration               |Replace hand-rolled BFS/DFS in CircularSlice, GradientSlice with `petgraph` algorithms. Add SCC detection to call graph. Benchmark performance                         |

**Cargo.toml additions (Phase 3):**

```toml
tree-sitter-proto = "0.4"
protobuf-parse = "3.7"
oxc_parser = "0.123"
oxc_semantic = "0.123"
```

### Phase 4 — Future (Unscheduled)

|Item                              |Trigger condition                                                                                     |
|----------------------------------|------------------------------------------------------------------------------------------------------|
|YANG/NETCONF context extraction   |YANG model changes appear frequently in reviewed MRs                                                  |
|Makefile/CMake context extraction |Build system security flag auditing becomes a review requirement                                      |
|`rayon` parallelization           |Performance becomes a bottleneck on large firmware trees (>500 files)                                 |
|`syn`-based Rust semantic analysis|Tree-sitter-only Rust analysis produces unacceptable false positive rate on `unsafe`/lifetime patterns|
|Assembly display mode             |Assembly diff context requested by firmware reviewers                                                 |

-----

## 8. Overlap & Shared Infrastructure

### What procedural and declarative analysis share

|Component                                 |Current location                        |Shared by                                         |
|------------------------------------------|----------------------------------------|--------------------------------------------------|
|Diff input model (`DiffInput`, `DiffInfo`)|`src/diff.rs`                           |All languages and formats                         |
|Findings output schema                    |`src/output.rs`                         |All — same JSON structure, same fields            |
|File discovery and walking                |`discover.py` (planned Rust replacement)|All — `ignore` crate for gitignore-aware traversal|
|Tree-sitter parsing infrastructure        |`src/ast.rs`, `src/languages/mod.rs`    |All formats with tree-sitter grammars             |
|CLI interface and subcommands             |`src/main.rs`                           |`prism slice` and `prism context`                 |
|Findings schema (pipeline integration)    |`agent-knowledge/findings_schema.md`    |Both Prism and the reviewer pipeline              |

### What differs

|Aspect            |Procedural slicing                                                     |Declarative context extraction                                   |
|------------------|-----------------------------------------------------------------------|-----------------------------------------------------------------|
|Analysis model    |Def-use chains, call graphs, data flow                                 |Reference/containment graphs, dependency DAGs                    |
|Hop semantics     |Variable assignments, function calls, data edges                       |Resource references, import/include, field types                 |
|Primary algorithms|Taint, Provenance, Absence, Membrane, Quantum, Circular, DFG, CallGraph|Touched-unit extraction, reference tracing, dependency resolution|
|Output semantics  |“This code is relevant because data flows here”                        |“This config is relevant because it references the changed block”|

-----

## 9. Crate Version Summary

### Procedural Language Support

|Crate             |Version|Purpose                                            |Phase|
|------------------|-------|---------------------------------------------------|-----|
|`tree-sitter-rust`|0.24.2 |Rust AST parsing                                   |1    |
|`tree-sitter-hcl` |1.1.0  |Terraform/HCL AST parsing                          |1    |
|`hcl-rs`          |0.19.6 |HCL2 reference resolution and expression evaluation|1    |
|`tree-sitter-bash`|0.25.1 |Shell/Bash AST parsing                             |2    |
|`tree-sitter-lua` |0.5.0  |Lua AST parsing                                    |2    |

### Declarative Format Support

|Crate                   |Version|Purpose                                              |Phase|
|------------------------|-------|-----------------------------------------------------|-----|
|`tree-sitter-dockerfile`|0.2.0  |Dockerfile AST parsing                               |1    |
|`dockerfile-parser-rs`  |3.3.0  |Dockerfile structured parsing with stage dependencies|1    |
|`docker-compose-types`  |0.23.0 |Docker-compose typed deserialization                 |1    |
|`tree-sitter-yaml`      |0.7.2  |YAML grammar (docker-compose, K8s, Ansible, CI)      |1    |
|`tree-sitter-proto`     |0.4.0  |Protocol Buffers grammar                             |3    |
|`protobuf-parse`        |3.7.2  |Full proto file parsing with field numbers           |3    |

### Analysis Infrastructure

|Crate         |Version|Purpose                                         |Phase |
|--------------|-------|------------------------------------------------|------|
|`petgraph`    |0.8.3  |Graph algorithms (SCC, dominators, reachability)|1+    |
|`ignore`      |0.4.x  |Gitignore-aware file discovery                  |1     |
|`oxc_parser`  |0.123.0|Semantic JS/TS parsing (scope, bindings)        |3     |
|`oxc_semantic`|0.123.0|JS/TS symbol resolution and type narrowing      |3     |
|`regex`       |1.x    |Pattern matching for sinks/sources              |Future|
|`rayon`       |1.x    |Parallel file parsing                           |Future|
|`syn`         |2.0.117|Rust-specific semantic AST (unsafe, lifetimes)  |Future|

-----

## 10. Success Metrics

|Metric                              |Target                                                                           |How measured                                          |
|------------------------------------|---------------------------------------------------------------------------------|------------------------------------------------------|
|Language coverage (team repos)      |100% — Python, Go, Rust, JS/TS, Terraform, Docker all analyzed                   |All team repos have Prism integration                 |
|Test count                          |200+ (from current 151)                                                          |`cargo test` count                                    |
|Taint sink coverage per language    |≥6 language-specific sinks per procedural language                               |Positive + negative test pairs per pattern            |
|False positive rate on new languages|≤30% (matching current Python/JS target)                                         |Manual precision calibration study                    |
|Context extraction precision        |Extracted context is <20% of total file size for typical declarative format diffs|Measured on sample MRs from Terraform and Docker repos|
|Build time (full features)          |<5 minutes on CI                                                                 |GitHub Actions timing                                 |
|Binary size (full features)         |<50MB                                                                            |`ls -la target/release/prism`                         |
