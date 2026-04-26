# Phase 1 (CWE Coverage — Go) — Design DRAFT

**Date:** 2026-04-25 (draft, mid-brainstorm)
**Status:** Sections 1–2 approved; Sections 3–4 pending
**Driver:** ACK Phase 1 deliverable (`~/code/slicing/ACK-prism-cwe-coverage-handoff.md` §2 Phase 1)
**Architecture:** Locked by ACK §3 Q1–Q5 — Rust modules with `const` arrays + CLI passthrough; `src/frameworks/<name>.rs` per-framework with `FrameworkSpec`; boolean `cleansed_for: BTreeSet<SanitizerCategory>`; quiet-mode default.
**Acceptance criteria:** ACK §1 — taint fires on at least one CWE-78/22 example in C1 fixtures; ≥80% sanitizer suppression rate; framework detection without per-run config; D2 coexistence; no Tier-1 regression.

> **Note:** This is a working draft. Sections 3–4 (sink/sanitizer registries, test strategy + PR cadence) are still being brainstormed in conversation. Final spec will be renamed to drop the `-DRAFT` suffix and committed when all sections are approved.

---

## 1. Scope & deliverables

Three sequential commits on a feature branch, single PR. Same shape as Phase 0.

### Commit 1 — "Framework detection layer (net/http + gin + gorilla/mux)"

- New `src/frameworks/mod.rs` defining `FrameworkSpec`, `ALL_FRAMEWORKS`, and the `detect_for` dispatch function.
- New `src/frameworks/{nethttp,gin,gorilla_mux}.rs` — one per framework. Each module exports the `FrameworkSpec` const for that framework: name, detection signals (imports, function signatures, decorator patterns), source patterns (route-boundary tainted accessors), framework-gated sinks (e.g., `http.ServeFile` is path-traversal sink only when net/http is active), and (initially empty) framework-specific sanitizers.
- Add `parsed.framework()` accessor backed by `OnceCell<Option<&'static FrameworkSpec>>` — populated lazily on first call. No integration with `CpgContext::build()` (lazy avoids build-pipeline coupling).
- Tests: per-framework detection unit tests (verify `FrameworkSpec::detect` fires on positive fixtures, doesn't fire on look-alikes). No algorithm-level tests yet (no sinks or sources active in algorithms).
- *No user-visible algorithm output change.* This commit is pure infrastructure.

### Commit 2 — "Wire framework sources, add Go CWE-78 + CWE-22 sinks"

- Hook framework-detected sources into `taint.rs` source inference: when a file's framework is detected and a function's parameters match the framework's source-shape, tainted sources are produced at route boundaries.
- Extend `taint.rs::SINK_PATTERNS` with Go CWE-78 sinks (`exec.Command("sh","-c",X)`, `exec.Command("bash","-c",X)`, `exec.CommandContext(...,sh,-c,X)`, `syscall.Exec(argv0,argv,envv)`) and CWE-22 sinks (`os.Open`, `os.OpenFile`, `os.Create`, `os.ReadFile`, `os.WriteFile`, `os.Remove`, `os.RemoveAll`, `os.Rename`, `os.Mkdir`, `os.MkdirAll`, `ioutil.ReadFile`, `ioutil.WriteFile`, `http.ServeFile`).
- CWE-78 has two argument-shape variants, both included per handoff §2.5:
  - **Shell-wrapped:** `exec.Command("sh"/"bash","-c",X)`, `exec.Command("cmd.exe","/c",X)`, `exec.CommandContext(...,sh,-c,X)`. Tainted arg index `[2]`; `semantic_check` verifies arg[0] is a shell literal and arg[1] is `-c`/`/c`.
  - **Tainted-binary:** `exec.Command(taintedPath, args...)` where the binary path itself is tainted. Tainted arg index `[0]`; `semantic_check: None` (any tainted first arg fires).
  - Both encoded as separate `SinkPattern` entries on `call_path: "exec.Command"`. Either fires independently.
- `exec.Command("ls", arg)` (non-shell, literal binary, non-tainted args[0]) does *not* fire either pattern.
- Tests: per-CWE positive (taint flows from gin `c.Param` to `exec.Command("sh","-c",X)`), per-CWE negative (no taint = no finding), framework-disabled (`http.ServeFile` outside a net/http context = no framework-gated sink).
- *User-visible:* Phase 1 acceptance criterion 1 (taint fires on at least one CWE-78 + CWE-22 example) achievable here.

### Commit 3 — "Sanitizer registry + shell-escape + path-validation cleansers"

- New `src/sanitizers/mod.rs` defining `SanitizerCategory` enum (`{Xss, Sqli, Ssrf, Deserialization, OsCommand, PathTraversal}` per the ACK), `cleansed_for: BTreeSet<SanitizerCategory>` field on the taint-value type, propagation hook (when a value flows through a recognized cleanser, add its category), and the suppression check (sink-side: if `Y ∈ value.cleansed_for`, skip the finding).
- New `src/sanitizers/{shell,path}.rs` — one file per category. Each exports `Recognizer` patterns: shell-escape recognizes `exec.Command("ls", arg)` (no shell, list-form args), `exec.CommandContext(ctx, "ls", arg)`; path-validation recognizes `filepath.Clean(p)` followed by `strings.HasPrefix(cleaned, base)`, `filepath.Rel(base, p)` followed by `!strings.HasPrefix(rel, "..")`.
- Tests: per-cleanser positive (taint passes through cleanser → finding suppressed), per-cleanser negative (taint passes through fake cleanser like `filepath.Base` → not suppressed), regression (already-tested unsanitized cases still fire).
- *User-visible:* Phase 1 acceptance criterion 2 (≥80% suppression rate) achievable here.

### Out of scope for Phase 1

Per ACK §5:
- No new Prism algorithms.
- No inter-procedural escape analysis.
- No perf optimization.
- No format-pass changes.

Also excluded:
- CWE-79, CWE-89, CWE-918, CWE-502 (Phases 2–3).
- Python, JS, or Java framework detection.
- HARDCODED_SECRET LHS patches (deferred per ACK §4.1; subsumed by handoff registry redesign).

---

## 2. Framework detection layer design (Commit 1)

The most architecturally novel piece — no equivalent exists today. Detailed design below.

### 2.1 New `src/frameworks/` module structure

```
src/frameworks/
├── mod.rs              (FrameworkSpec, ALL_FRAMEWORKS, detect_for dispatch)
├── nethttp.rs          (Go net/http FrameworkSpec const)
├── gin.rs              (Go gin FrameworkSpec const)
└── gorilla_mux.rs      (Go gorilla/mux FrameworkSpec const)
```

`mod.rs` defines the shape; per-framework modules export const data only. Mirrors `src/languages/<lang>.rs` structure per ACK §3 Q2.

### 2.2 `FrameworkSpec` struct (concrete shape)

```rust
pub struct FrameworkSpec {
    pub name: &'static str,
    pub detect: fn(&ParsedFile) -> bool,
    pub sources: &'static [SourcePattern],
    pub sinks: &'static [SinkPattern],
    pub sanitizers: &'static [SanitizerRecognizer],  // empty in Commit 1; populated by Commit 3
}

pub struct SourcePattern {
    /// Identifier path matched against call expressions, e.g. "c.Param", "r.URL.Query().Get".
    pub call_path: &'static str,
    /// Origin classification (existing Origin enum from provenance_slice.rs).
    pub origin: Origin,
    /// Which call argument *receives* taint as a side-effect. `None` = the call result itself is
    /// the tainted value. `Some(i)` = the i-th argument becomes tainted (e.g., `c.BindJSON(&v)`
    /// taints `&v` at index 0).
    pub taints_arg: Option<usize>,
}

pub struct SinkPattern {
    pub call_path: &'static str,                       // e.g. "http.ServeFile"
    pub category: SanitizerCategory,                   // PathTraversal, OsCommand, etc.
    pub tainted_arg_indices: &'static [usize],         // arg positions checked for tainted values
    pub semantic_check: Option<fn(&CallSite) -> bool>, // optional argument-shape refinement
}
```

`SanitizerRecognizer` is defined in Commit 3; in Commit 1 the field is `&[]`.

### 2.3 Detection function shape

Each framework's `detect: fn(&ParsedFile) -> bool` runs:

1. Parse import paths (cheap — top-of-file scan).
2. Look for the framework's import signature (e.g., `"net/http"`, `"github.com/gin-gonic/gin"`, `"github.com/gorilla/mux"`).
3. If imports match, check for a corroborating signal — function with parameter typed `*http.Request` (net/http), `*gin.Context` (gin), or call to `mux.Vars(r)` (gorilla/mux). Imports alone aren't enough; a vendored framework that's imported but never used shouldn't trigger.

Detection is per-file. **First-match wins** with the registry ordered `[gin, gorilla_mux, nethttp]`. Router frameworks (gin, gorilla/mux) take precedence; net/http is the fallback for any file with `*http.Request` parameters that isn't already claimed by a more-specific router.

### 2.4 Detection dispatch

A free function in `mod.rs` walks the static framework list in order; first match wins. No enum — `&'static FrameworkSpec` references are sufficient (the `name: &'static str` field on `FrameworkSpec` carries the telemetry-label use case the enum would have served).

```rust
const ALL_FRAMEWORKS: &[&'static FrameworkSpec] = &[
    &gin::SPEC,
    &gorilla_mux::SPEC,
    &nethttp::SPEC,
];

/// Detect the active framework for a file, if any. First match wins.
pub fn detect_for(parsed: &ParsedFile) -> Option<&'static FrameworkSpec> {
    for spec in ALL_FRAMEWORKS {
        if (spec.detect)(parsed) {
            return Some(spec);
        }
    }
    None
}
```

### 2.5 `ParsedFile` metadata caching

Add a single field:

```rust
pub framework: OnceCell<Option<&'static FrameworkSpec>>,
```

Populated lazily on first call to `parsed.framework()`. `OnceCell` keeps it cheap and thread-safe. Algorithm dispatch path is `parsed.framework().and_then(...)` — O(1) after first call.

Alternative considered: compute eagerly at `CpgContext::build()`. Eager is fine but lazy is simpler (no integration with build pipeline).

### 2.6 Per-framework `SourcePattern` lists

**`nethttp.rs` `sources` const** (sample):

```rust
const SOURCES: &[SourcePattern] = &[
    SourcePattern { call_path: "r.URL.Query", origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "r.FormValue", origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "r.PostFormValue", origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "r.Header.Get", origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "r.Cookie", origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "r.URL.Path", origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "r.URL.RawQuery", origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "r.PathValue", origin: Origin::UserInput, taints_arg: None },
    // io.ReadAll(r.Body) / json.NewDecoder(r.Body).Decode(...) — handled separately as compositions
];
```

The `r.` prefix matches any variable holding an `*http.Request`. Detection helper: a Go function whose parameter list contains `*http.Request` — that parameter name is the matching prefix (commonly `r`, sometimes `req`). Detection captures the local name; source matching uses it. **Multiple `*http.Request` parameters** (e.g., `func compareRequests(want, got *http.Request)`): bind *all* matching parameter names — each becomes a candidate prefix and is checked independently. Pathological in handlers, common in test utilities; binding all is the conservatively-correct behavior (no false negatives) at negligible cost.

**`gin.rs` `sources` const** (sample):

```rust
const SOURCES: &[SourcePattern] = &[
    SourcePattern { call_path: "c.Param", origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "c.Query", origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "c.PostForm", origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "c.GetHeader", origin: Origin::UserInput, taints_arg: None },
    SourcePattern { call_path: "c.Request.URL.Path", origin: Origin::UserInput, taints_arg: None },
    // c.BindJSON(&v) — taints v; handled via tainted_arg=Some(0) plus an aliasing rule
];
```

Same prefix-binding logic — detection captures the `*gin.Context` param name (commonly `c`, sometimes `ctx`).

**`gorilla_mux.rs` `sources` const** (smaller — gorilla/mux mostly relies on the underlying net/http request):

```rust
const SOURCES: &[SourcePattern] = &[
    SourcePattern { call_path: "mux.Vars", origin: Origin::UserInput, taints_arg: None },
];
```

`mux.Vars(r)` returns `map[string]string`; values are tainted. Plus all net/http sources still apply (gorilla/mux files always have `*http.Request` too — detection chain).

### 2.7 Framework-gated sinks (Commit 1 stub, populated in Commit 2)

In Commit 1, `sinks: &[]` for all three frameworks. In Commit 2, populate:

- `nethttp::SINKS` — `http.ServeFile(w, r, path)` as a `PathTraversal` sink with `tainted_arg_indices: &[2]`.
- `gin::SINKS` — `c.File(path)` as `PathTraversal`.
- `gorilla_mux::SINKS` — empty (no framework-specific sinks beyond what net/http provides).

CWE-78 sinks (`exec.Command("sh","-c",X)`, etc.) are *not* framework-gated — they apply across all Go code regardless of framework. Those go in `taint.rs::SINK_PATTERNS` extension in Commit 2.

### 2.8 Pull-model integration with `taint.rs`

When taint analysis processes a function:

1. `parsed.framework()` → `Option<&FrameworkSpec>`.
2. If `Some`, scan function parameters for the framework's expected parameter types (e.g., `*http.Request`).
3. Bind the param's local name (e.g., `r`).
4. For each `SourcePattern` in `spec.sources`, substitute the bound name into `call_path` (e.g., `"r.URL.Query"` becomes a concrete pattern for this function).
5. Match the substituted pattern against call expressions in the function body. Matched call results become tainted.

Pull model: the framework data is queried at analysis time, not pre-tagged. Simpler than the push model (no precompute, no cache invalidation). Performance is fine because `parsed.framework()` is `OnceCell`-cached.

### 2.9 Tests for Commit 1 (infrastructure-only)

Per-framework detection unit tests under `tests/frameworks/`:

- **Positive:** minimal Go file with the framework's import + signature → `detect` returns `true`.
- **Negative — wrong import:** file imports a different module that mentions the framework's tokens but isn't the framework → returns `false`.
- **Negative — vendored:** file imports the framework but never uses it (no matching parameter type / no `mux.Vars` call) → returns `false`.
- **Negative — no framework (baseline):** plain Go file with no web imports (e.g., `main.go` with `package main; func main() { fmt.Println("hi") }`) → `detect_for` returns `None`. Pins the quiet-mode default per ACK §3 Q5; prevents a future regression that auto-tags every Go file as a framework.
- **Disambiguation:** file imports both `net/http` and `gin` → `Gin` wins. Verified by detection-order assertion.

Cargo.toml additions: 4 new `[[test]]` entries (`frameworks_nethttp`, `frameworks_gin`, `frameworks_gorilla_mux`, `frameworks_registry`).

---

## 3. Sink registry + sanitizer registry design (Commits 2 and 3)

### 3.1 Where new sinks live

Two storage locations, keyed by what gates the sink:

- **Cross-cutting Go sinks** (apply regardless of framework — `exec.Command`, `os.Open`, etc.) live as new `const` arrays in `src/algorithms/taint.rs` alongside existing `SINK_PATTERNS`. Existing `SINK_PATTERNS` stays untouched for backward compatibility; new structured Go sinks live in `GO_CWE78_SINKS: &[SinkPattern]` and `GO_CWE22_SINKS: &[SinkPattern]` consts. The taint analysis pass consults both registries (existing flat list + new structured list). *Future migration: existing `SINK_PATTERNS` could be promoted to `SinkPattern` shape for uniform handling — deferred to keep this PR scoped; Phases 2/3 will revisit once additional CWE families exercise the same registry.*

- **Framework-gated sinks** (only meaningful when a specific framework is detected — `http.ServeFile`, `c.File`) live in each framework's `FrameworkSpec.sinks: &[SinkPattern]` field per §2.2. Consulted only when `parsed.framework()` returns `Some(matching framework)`.

`SinkPattern` shape is defined in §2.2 and reused verbatim for both storage locations.

### 3.2 Go CWE-78 sink patterns (cross-cutting)

```rust
pub const GO_CWE78_SINKS: &[SinkPattern] = &[
    // Shell-wrapped: exec.Command("sh","-c",X), exec.Command("bash","-c",X), exec.Command("cmd.exe","/c",X)
    SinkPattern {
        call_path: "exec.Command",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[2],
        semantic_check: Some(check_shell_wrapper),
    },
    SinkPattern {
        call_path: "exec.CommandContext",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[3],  // ctx, sh, -c, X
        semantic_check: Some(check_shell_wrapper_ctx),
    },
    // Tainted-binary: exec.Command(taintedPath, ...) — first arg itself is tainted.
    // semantic_check: None because per-arg taint resolution at sink-eval time
    // (taint.rs::arg_is_tainted_in_path) is the structural gate — a literal binary
    // has no identifier and is never in any FlowPath's tainted set.
    SinkPattern {
        call_path: "exec.Command",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "exec.CommandContext",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[1],  // ctx, taintedPath
        semantic_check: None,
    },
    // syscall.Exec(argv0, argv, envv) — argv0 or the argv slice tainted (whole-slice analysis)
    SinkPattern {
        call_path: "syscall.Exec",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[0, 1],
        semantic_check: None,
    },
];

fn check_shell_wrapper(call: &CallSite) -> bool {
    let arg0 = call.literal_arg(0).unwrap_or("");
    let arg1 = call.literal_arg(1).unwrap_or("");
    matches!(arg0, "sh" | "bash" | "cmd.exe" | "/bin/sh" | "/bin/bash")
        && matches!(arg1, "-c" | "/c")
}

fn check_shell_wrapper_ctx(call: &CallSite) -> bool {
    let arg1 = call.literal_arg(1).unwrap_or("");
    let arg2 = call.literal_arg(2).unwrap_or("");
    matches!(arg1, "sh" | "bash" | "cmd.exe" | "/bin/sh" | "/bin/bash")
        && matches!(arg2, "-c" | "/c")
}

```

`CallSite::literal_arg(i)` returns `Option<&str>` if arg `i` is a string literal, `None` otherwise. Defined as a helper in `taint.rs`.

**Tainted-binary `semantic_check: None` (Phase 1.5+).** Earlier Phase 1 used a Path C `semantic_check` (`check_command_taintable_binary`) to syntactically gate the tainted-binary pattern on arg[0] being non-literal. Phase 1.5 (item #1) replaced this with proper per-arg taint resolution at sink-eval time via `arg_is_tainted_in_path` in `taint.rs`. The structural gate is now: "is arg[0]'s identifier in this FlowPath's tainted set?" A literal binary has no identifier and is never tainted; a variable bound to a non-tainted source isn't reached by any FlowPath edge at the call line. Path C's semantic_check is therefore redundant and removed.

**Slice-taint behavior for `syscall.Exec`:** Prism's existing DFG models slices conservatively — any tainted element taints the slice as a whole, and reads of `slice[i]` produce tainted values regardless of which element was assigned the taint. The `tainted_arg_indices: &[0, 1]` entry means: arg 0 (`argv0`) is checked directly; arg 1 (`argv` slice) fires the sink if the slice itself is taint-flagged in the DFG, which captures the case where any element was assigned from a tainted source. Per-element tracking (knowing exactly *which* `argv[i]` is tainted) is not modeled today and is out of scope for Phase 1.

**Shell-wrapper binary list — scope:** `sh`, `bash`, `cmd.exe`, `/bin/sh`, `/bin/bash` cover common Linux/Windows shell invocations. PowerShell (`pwsh`, `powershell.exe`) and exotic absolute paths (`/usr/local/bin/bash` etc.) are deferred — add to the literal list in a follow-up if C1 fixtures exercise them.

### 3.3 Go CWE-22 sink patterns (cross-cutting)

```rust
pub const GO_CWE22_SINKS: &[SinkPattern] = &[
    // Read sinks
    SinkPattern { call_path: "os.Open",          category: PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    SinkPattern { call_path: "os.OpenFile",      category: PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    SinkPattern { call_path: "os.ReadFile",      category: PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    SinkPattern { call_path: "ioutil.ReadFile",  category: PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    // Write sinks
    SinkPattern { call_path: "os.Create",        category: PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    SinkPattern { call_path: "os.WriteFile",     category: PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    SinkPattern { call_path: "ioutil.WriteFile", category: PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    // Mutation sinks
    SinkPattern { call_path: "os.Remove",        category: PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    SinkPattern { call_path: "os.RemoveAll",     category: PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    SinkPattern { call_path: "os.Mkdir",         category: PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    SinkPattern { call_path: "os.MkdirAll",      category: PathTraversal, tainted_arg_indices: &[0], semantic_check: None },
    SinkPattern { call_path: "os.Rename",        category: PathTraversal, tainted_arg_indices: &[0, 1], semantic_check: None },
];
```

Framework-gated CWE-22 sinks (per §2.7):

- `nethttp::SINKS` — `http.ServeFile(w, r, path)` at `tainted_arg_indices: &[2]`.
- `gin::SINKS` — `c.File(path)` at `tainted_arg_indices: &[0]`.

Note: `filepath.Join(base, userPart)` is *not* a sink — it's a path-construction primitive. The downstream `os.*` call on the joined result is the sink. Taint flows through `filepath.Join` (composes) but the join itself doesn't fire.

### 3.4 Sanitizer registry — module structure and types

```
src/sanitizers/
├── mod.rs              (SanitizerCategory, SanitizerRecognizer, propagation/suppression hooks)
├── shell.rs            (OsCommand cleansers — list-form exec.Command etc.)
└── path.rs             (PathTraversal cleansers — Clean+HasPrefix, Rel+!startswith-..)
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SanitizerCategory {
    Xss,
    Sqli,
    Ssrf,
    Deserialization,
    OsCommand,
    PathTraversal,
}

pub struct SanitizerRecognizer {
    /// Call expression that performs the cleansing, e.g. "filepath.Clean".
    pub call_path: &'static str,
    /// What category the cleansing applies to.
    pub category: SanitizerCategory,
    /// Optional argument-shape refinement (e.g., "the first arg is a literal binary, not a shell wrapper").
    pub semantic_check: Option<fn(&CallSite) -> bool>,
    /// For pattern-pair cleansers (Clean→HasPrefix, Rel→check), the recognizer is the *first half*;
    /// the second-half check is named here and verified by `cleanser_pair_satisfied`.
    pub paired_check: Option<&'static str>,
}
```

The `paired_check` field handles the path-validation case where cleansing requires both a transform call AND a guard:

- `paired_check: Some("strings.HasPrefix")` for `filepath.Clean` recognizer.
- `paired_check: None` for one-call cleansers.

**Resolution model:** `paired_check` is a string name resolved at suppression time by **textual co-occurrence in the same function body** — not a type-safe binding to a Rust function. The implementation walks the function body looking for the literal token; presence anywhere in the body (regardless of placement relative to the cleanser call or the sink) is enough to satisfy the pair. See §3.8 for the heuristic limitations this implies.

### 3.5 Taint-value extension — `cleansed_for` field

Prism's existing taint engine doesn't have a literal `TaintValue` struct today. Taint propagation works through `crate::data_flow::FlowPath` (a sequence of `FlowEdge`s over `VarLocation`s). The `cleansed_for: BTreeSet<SanitizerCategory>` field needs a concrete home — three plausible choices:

1. **New `TaintValue` wrapper** in `taint.rs` that pairs `VarLocation` with metadata. Largest refactor; existing taint engine threads `TaintValue` instead of bare `VarLocation` everywhere.
2. **Augment `VarLocation`** with `cleansed_for`. Per-location, not per-flow-path — two flows reaching the same use-site couldn't disagree on cleansing status.
3. **Side-table** `BTreeMap<(FlowPath, VarLocation), BTreeSet<SanitizerCategory>>` keyed by flow + location; threaded through the engine. Smaller refactor; clutters call sites.

**Choice: option (1), integrated as a `FlowPath` augmentation.** Cleansing is path-sensitive — two flows reaching the same use-site may disagree on cleansing status (one flow passes through `filepath.Clean(...)` + `strings.HasPrefix(...)`, another doesn't). Option (2) loses this precision. Option (3) achieves the same precision but at the cost of pervasive call-site clutter.

Concrete shape: augment `FlowPath` in `data_flow.rs` with `cleansed_for: BTreeSet<SanitizerCategory>` (or, equivalently, introduce a thin `TaintValue` newtype wrapping `(FlowPath, BTreeSet<SanitizerCategory>)` for ergonomics — implementation detail). A new flow starts with empty set; cleansers add their category as the flow extends through them; sinks check the flow's set when evaluating suppression.

`BTreeSet` (not `HashSet`) for deterministic iteration order in test snapshots.

Constructor default: empty set (newly tainted values are not cleansed).

### 3.6 Propagation hook (cleanser application)

When taint analysis processes a call expression `f(args)`:

1. For each `SanitizerRecognizer` in the active registries (Go shell + Go path + framework-active sanitizers):
2. If `f`'s call_path matches and `semantic_check` (if any) passes:
3. Find tainted values in `args` that match the recognizer's input position.
4. The call's *result* (or arg-by-reference, for output-cleanser shapes) inherits all `cleansed_for` from the input *plus* the recognizer's category.
5. For paired-check recognizers: only mark cleansed-for-category if a corresponding `paired_check` call appears in the same function body acting on the result. Heuristic — see §3.8.

### 3.7 Suppression check (sink-side)

When taint analysis evaluates a sink call:

1. Get the matched `SinkPattern` from the call.
2. For each tainted value in the sink's `tainted_arg_indices` positions:
3. If `value.cleansed_for.contains(sink.category)` → **suppress** (no finding).
4. Otherwise → **fire** (emit finding).

A value cleansed-for-`Sqli` but not cleansed-for-`PathTraversal` would suppress a SQLi sink but still fire a path-traversal sink. This is the category-aware property the ACK §3 Q3 specifies.

### 3.8 Path-validation recognizer — heuristic detail

Path-validation is the harder case because cleansing is a *guarded check* not a transform. Per handoff §3.3: "recognize the textual pattern and trust that it cleanses, accepting some FNs on unusual structures."

Recognizer entry:

```rust
SanitizerRecognizer {
    call_path: "filepath.Clean",
    category: SanitizerCategory::PathTraversal,
    semantic_check: None,
    paired_check: Some("strings.HasPrefix"),
}
```

Heuristic for the paired check: when taint analysis sees a sink consume `cleaned := filepath.Clean(tainted)`, before firing it walks back through the function body looking for `strings.HasPrefix(cleaned, ...)` or `strings.HasPrefix(cleaned + ..., ...)`. If found anywhere in the function body, mark `cleaned` as `cleansed_for(PathTraversal)`.

Limitations explicitly accepted:

- Doesn't distinguish guard-true branch from guard-false branch (textual co-occurrence is enough). False negatives possible if the guard is in a different function reachable only by the guard-failure path; false positives possible if the guard is dead code. Per handoff, acceptable.
- Doesn't model `strings.HasPrefix(cleaned, base)` where `base` is itself tainted (a contrived case where the prefix being checked is attacker-controlled). Document, defer.
- Doesn't model `filepath.Rel(base, p)` followed by `strings.HasPrefix(rel, "..")` *negation* check — the Rel-form needs its own paired_check entry pointing to a custom function `check_rel_no_dotdot` rather than just `strings.HasPrefix`.

Recognizer list for path-validation:

```rust
pub const PATH_RECOGNIZERS: &[SanitizerRecognizer] = &[
    SanitizerRecognizer {
        call_path: "filepath.Clean",
        category: SanitizerCategory::PathTraversal,
        semantic_check: None,
        paired_check: Some("strings.HasPrefix"),
    },
    SanitizerRecognizer {
        call_path: "filepath.Rel",
        category: SanitizerCategory::PathTraversal,
        semantic_check: None,
        paired_check: Some("strings.HasPrefix"),  // negated check on result starting with "..": same matcher
    },
];
```

A future Phase 1.5 / Phase 2 enhancement: CFG-aware version using existing CFG infrastructure (Phase 6 done) to check guard-true vs guard-false branch placement. Out of scope for Phase 1.

**Rel-form direction ambiguity (Phase 1 limitation):** the `filepath.Rel` recognizer's `paired_check: Some("strings.HasPrefix")` matches by textual co-occurrence — it does not distinguish whether the prefix check is used as a *negative* guard (`if HasPrefix(rel, "..") { return error }`) or a *positive* guard (`if HasPrefix(rel, "..") { use rel }`). Both shapes contain the literal `strings.HasPrefix(rel, "..")` in the function body and both will suppress the finding under the textual heuristic. The negative-guard shape is correct usage; the positive-guard shape is a real bug that Phase 1's recognizer would mis-suppress. This is a known false-negative; CFG-aware enhancement (deferred above) resolves it.

### 3.9 Shell-escape — no cleanser needed in Phase 1

Initially this section proposed an explicit `SHELL_RECOGNIZERS` list with `check_not_shell_wrapper` mirroring the sink's `check_shell_wrapper`. **That cleanser is dropped** — analysis showed it would be a no-op in Phase 1.

Reasoning: for `exec.Command("ls", tainted)`:
- Tainted-binary sink (`tainted_arg_indices: &[0]`): `arg[0] == "ls"` (not tainted) → no fire.
- Shell-wrapper sink (`tainted_arg_indices: &[2]`, `check_shell_wrapper`): `arg[0] == "ls"` (not in shell list) → `semantic_check` fails → no fire.
- Result: no sink fires. Cleansing the *result* (`*exec.Cmd`) is moot because no Phase 1 sink consumes `*exec.Cmd` and checks for `OsCommand` cleansing.

The sink's existing `semantic_check` exclusion already encodes the safe-form-doesn't-fire behavior. A separate cleanser is redundant and would add code without changing outcomes.

A future phase that adds a sink consuming `*exec.Cmd` (e.g., if `(*exec.Cmd).Output()` becomes a sink for some downstream reason) would justify reintroducing a cleanser. Documented forward-looking scope; not in Phase 1.

`SHELL_RECOGNIZERS` consequently is empty in Phase 1:

```rust
pub const SHELL_RECOGNIZERS: &[SanitizerRecognizer] = &[];
```

The const exists for symmetry with `PATH_RECOGNIZERS` (which is non-empty) and for forward-extension; future phases populate it as new sinks consuming cleansed `*exec.Cmd` arrive.

### 3.10 Tests for Commits 2 and 3

**Commit 2 tests** (Go CWE-78/22 sinks):

- Per CWE × per framework × per source path = ~12 positive tests (CWE-78 net/http, CWE-78 gin, CWE-78 gorilla/mux, CWE-22 net/http, etc.).
- Per CWE negative: same shape, no taint flow → no finding.
- Argument-shape negative: `exec.Command("ls", arg)` (literal non-shell binary, tainted arg) → no finding (because cleanser hasn't been added yet in Commit 2; this becomes a *suppression* test in Commit 3).
- Framework-gated negative: `http.ServeFile` outside a net/http context → no finding (no framework, no source).
- Total: ~24 tests.

**Commit 3 tests** (sanitizer registry):

- Shell-escape positive: tainted gin source flows through `exec.Command("ls", arg)` → no finding emitted. (In Commit 2 this would have fired; Commit 3 suppresses.)
- Shell-escape negative: tainted gin source flows through `exec.Command("sh", "-c", arg)` → still fires (cleanser doesn't apply to shell-wrapper form).
- Path-validation positive: `cleaned := filepath.Clean(tainted); if strings.HasPrefix(cleaned, base) { ... os.ReadFile(cleaned) }` → no finding.
- Path-validation negative — fake cleanser: `_ = filepath.Base(tainted); os.ReadFile(tainted)` → fires (Base is not a cleanser).
- Path-validation negative — paired-check missing: `cleaned := filepath.Clean(tainted); os.ReadFile(cleaned)` (no HasPrefix check) → fires.
- Regression: all Commit 2 unsanitized cases still fire after Commit 3 lands.
- Total: ~18 tests including suppression-rate measurements.

For acceptance criterion #2 (≥80% suppression rate) — the test suite will include a curated `tests/fixtures/sanitizer-suite-go/` directory with 10 sanitized + 10 unsanitized examples. Asserts: ≥8 of the 10 sanitized examples produce no findings; all 10 unsanitized examples produce findings. Pinned by an integration test.

**C1 fixture validation is external.** Acceptance criterion 1 (taint fires on at least one CWE-78 + CWE-22 example *in C1's Go fixtures*) is validated post-merge by the eval team via the validation cadence in ACK §6 / handoff §9. Phase 1 in-tree tests pin synthetic minimal examples; the curated `~/code/agent-eval/cache/prism-cwe-fixtures/` set is the eval team's validation surface, not this repo's.

---

## 4. Test strategy, PR cadence, branch strategy

The procedural close-out. Most test detail lives in §2.9 and §3.10 already; this section ties them together and adds branch / PR / status-doc shape.

### 4.1 Test strategy summary

Three test layers, all in-tree:

| Layer | Location | Purpose | Total tests |
|---|---|---|---|
| **Unit — framework detection** | `tests/frameworks/{nethttp,gin,gorilla_mux,registry}_test.rs` | `FrameworkSpec::detect` positive + 3 negatives + disambiguation per §2.9. Pure detection logic, no taint flow. | ~16 (4 per framework × 3 frameworks + 4 registry) |
| **Unit — sinks + recognizers** | `tests/algo/taxonomy/taint_sink_lang_test.rs` extension + new `tests/algo/taxonomy/sanitizers_test.rs` | Per-CWE × per-framework × per-source taint-flow positive + negatives per §3.10. Includes shell-wrapper / tainted-binary / argument-shape negatives, paired-check positives, fake-cleanser negatives. | ~42 (24 Commit 2 + 18 Commit 3) |
| **Integration — suppression-rate fixture suite** | `tests/fixtures/sanitizer-suite-go/` (10 sanitized + 10 unsanitized examples) + `tests/integration/cwe_phase1_suppression_test.rs` | Asserts ≥8/10 sanitized produce no findings; all 10 unsanitized produce findings. Pins acceptance criterion #2. | 1 integration test asserting against 20 fixtures |

**Local minimal fixtures only** during implementation. No dependency on `~/code/agent-eval/cache/prism-cwe-fixtures/` for the in-tree test suite — that's external validation post-merge per §3.10.

**Coverage matrix update** in `coverage/matrix.json`: no new algorithm rows (Phase 1 extends `taint`, doesn't add an algorithm), but the `taint` row's `go` cell stays at `full` (already 3+ tests) — verify after Commit 2 that depth is preserved or increased. Run `python3 scripts/generate_coverage_badges.py` if cells change.

**No regression on Tier 1** (acceptance criterion #5): the existing `1,406` tests must all still pass after each commit. CI catches this.

**`Cargo.toml` `[[test]]` additions** — total 6 new entries across the three commits:
- 4 from §2.9 (Commit 1): `frameworks_nethttp`, `frameworks_gin`, `frameworks_gorilla_mux`, `frameworks_registry`.
- 2 from §3.10 / §4.1 (Commits 2 + 3): `algo_taxonomy_sanitizers`, `integration_cwe_phase1_suppression`.

The Commit 2 sink tests extend the existing `algo_taint_sink_lang` target rather than adding a new one — no Cargo.toml change for those.

**`tests/integration/coverage_test.rs` 3-copy update** (per CLAUDE.md): the file has the `all_test_files` array in three places — `test_algorithm_language_matrix`, `test_language_coverage_minimum`, and `test_coverage_matrix_validation`. All three need the new test file paths (`tests/algo/taxonomy/sanitizers_test.rs`, `tests/integration/cwe_phase1_suppression_test.rs`, plus the four `tests/frameworks/*` entries). PR #71 hit this; Phase 1 must too. Run `cargo test --test integration_coverage` to verify the matrix doesn't under-report.

### 4.2 Branch + PR shape

**Branch name:** `claude/cwe-phase1-go` — matches the `claude/<topic>` convention (Phase 0 used `claude/t1-cleanup-pre-cwe-handoff`).

**Single PR, three commits** matching the §1 deliverable boundaries:

1. `Add framework detection layer (net/http + gin + gorilla/mux)` — Commit 1 from §1.
2. `Wire framework sources, add Go CWE-78 + CWE-22 sinks` — Commit 2.
3. `Add sanitizer registry: shell-escape + path-validation cleansers` — Commit 3.

Each commit independently green: `cargo build && cargo test && cargo fmt --check && cargo clippy` clean. CI re-runs at every push; reviewer checks intermediate green-ness via PR commit-list view.

**Sub-agent dispatch** (post-handoff session): one sub-agent per commit, two-stage review (spec-compliance + code-quality) between commits, same pattern as Phase 0. Final-branch review before push + PR.

### 4.3 Off-ramps and risk mitigations

The handoff §9 / ACK §6 establish that we file `~/code/slicing/QUESTIONS-prism-cwe-phase1.md` rather than guess when implementation diverges from spec. Specific risks worth pre-deciding:

| Risk | Trigger | Action |
|---|---|---|
| `FrameworkSpec` shape needs additional fields surfaced by Commit 2 implementation | Implementer hits "I need to express X but the struct doesn't have a slot" | Extend `FrameworkSpec` in Commit 1 amend (pre-push), document the extension in the QUESTIONS doc. Don't ship Commit 1 with a known-too-narrow shape. |
| `FlowPath` augmentation per §3.5 turns out to require deeper `data_flow.rs` refactor than expected | Commit 3 stalls on threading `cleansed_for` through every flow propagation site | Surface in QUESTIONS doc; consider splitting Commit 3 into "infrastructure (cleansed_for plumbing) + Phase 1 recognizers" if needed. |
| Path-validation paired-check heuristic produces unacceptable FP/FN rate on local fixture suite | Suppression-rate test fails (<80%) on local 10+10 suite | Tighten the heuristic (e.g., require `paired_check` token within N statements rather than function-wide) before shipping; document the tightening. |
| C1 fixtures land mid-implementation | Eval team publishes `~/code/agent-eval/cache/prism-cwe-fixtures/` before our PR merges | Integrate a sample subset into local fixtures (with attribution); use as additional tests but don't take a hard dependency on the external cache path. |
| `gin` / `gorilla/mux` framework detection FP rate higher than expected on real Go code | Detection fires on files that aren't actually using the framework | Tighten the corroborating-signal requirement (§2.3 — currently requires both import + signature; may need additional signals like specific function calls). |

### 4.4 STATUS doc + post-merge handoff

Per ACK §6 communication protocol, on PR merge:

1. File `~/code/slicing/STATUS-prism-cwe-phase1.md` summarizing what shipped, what tests pin which acceptance criterion, and any deferred items rolled to Phase 1.5.
2. Notify eval team. Their reply convention is `~/code/agent-eval/analysis/RE-*.md` per the explicit statement in `~/code/agent-eval/analysis/RE-prism-cwe-coverage-handoff-go-signal.md` ("Reply docs from us: file under `~/code/agent-eval/analysis/RE-*.md` (this doc establishes the pattern)"). Notification mechanism (ping vs. doc-mention) is implicit — likely whatever the eval team monitors; no explicit channel specified in the ACK or reply.
3. Eval team runs Prism against `~/code/agent-eval/cache/prism-cwe-fixtures/` plus 2-3 Tier 1 fixtures. Feedback comes back via a `RE-*.md` doc.
4. Phase 2 (Python) starts on their go-signal, same shape.

Acceptance criterion 1 (taint fires on at least one CWE-78 + CWE-22 example in C1 fixtures) and 5 (no Tier-1 regression) are validated *by the eval team* in step 3. Phase 1 is "shipped" when the PR merges; "accepted" when the eval team's reply confirms criteria 1 + 5.

### 4.5 Time estimate (ACK §2 alignment)

ACK §2 estimated 1-2 weeks. Breakdown:

| Commit | Scope | Estimate |
|---|---|---|
| Commit 1 | Framework detection layer + 16 unit tests | ~3 days |
| Commit 2 | Cross-cutting + framework-gated sinks + 24 unit tests | ~3-4 days |
| Commit 3 | Sanitizer registry + 18 unit tests + 10+10 fixture suite + integration test | ~3 days |
| Buffer | CI fixes, fmt drift, sub-agent review iterations, fixture tuning | ~1-2 days |
| **Total** | | **~10-12 days = ~1.5-2 weeks** |

Aligns with the lower end of the ACK estimate. Faster if `FlowPath` augmentation goes cleanly; slower if the path-validation heuristic needs tuning.

### 4.6 Out of scope (recap from §1)

Per ACK §5 and §1 of this design:

- No new Prism algorithms.
- No CWE-79/89/918/502 (Phases 2-3).
- No Python/JS/Java framework detection.
- No `HARDCODED_SECRET` LHS patches (deferred per ACK §4.1).
- No CFG-aware path-validation refinement (Phase 1.5+).
- No PowerShell / exotic shell binaries in CWE-78 sink list (deferred).

---

*End of design draft. All four sections present; ready for final review pass and rename from `-DRAFT` to ship.*
