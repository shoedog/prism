# S3 Call-Resolution Precision Floor — Deferred Work

Items intentionally **not** done in the S3 branch (`s3-precision`), recorded so future
implementors don't rediscover them. Each: priority · why deferred · production impact ·
fix sketch. Companion to the spec (`2026-06-12-prism-s3-call-resolution-precision-design.md`)
and plan (`docs/superpowers/plans/2026-06-12-prism-s3-call-resolution-precision.md`).

## 1. Type-confirmed dispatch (Go embedding, Go interface, Python inheritance) — Phase-IP

**Priority:** Important (recall). **Why deferred:** spec §2.4 explicitly sequences
receiver-dispatch-via-type-hierarchy to Phase-IP (E12 `DispatchProvider`); it needs a
type/method-set model S3 does not build. **Impact:** three capability-matrix cases are
`expected_gap` (owner-approved 2026-06-13): `go/embedded_method` (promoted methods on an
embedded field), `go/interface_dispatch` (structural interface satisfaction — no syntactic
`impl` to dual-key, unlike Rust traits which pass), `python/inherited_override` (untyped
receiver + same-name method on base & child → multi-owner collision drop). Real-world Go/
Python receiver calls of these shapes resolve to nothing in nav/CPG. **Fix sketch:** wire
receiver-type inference through `DispatchProvider`; for Go, model embedding promotion
(embedded field types contribute their method sets) and interface satisfaction
(method-set compatibility); index them into `methods` so `owner_lookup` hits.

## 2. P6-lite "recover-then-external-drop" is stricter than R6 demote for inherited methods

**Priority:** Medium. **Why deferred:** the correct fix needs a struct/type registry to
distinguish "external type" from "in-repo type that inherits/embeds the method," which is
the Phase-IP work in item 1. **Impact:** when P6-lite recovers a receiver type `T` that is
in-repo but does not itself *own* method `m` (embedding/interface/inheritance), the call is
dropped as `ExternalReceiver` — whereas an *unrecovered* receiver would have been kept by
R6 single-owner demote. So recovery can lose an edge that the dumber path keeps (observed
on `go/embedded_method` / `go/interface_dispatch`). Conversely it correctly drops genuine
external types (`Vec::truncate` class). **Fix sketch:** on `owner_lookup(T, m)` miss where
`T` is a known in-repo type, fall back to R6 single-owner demote instead of
`ExternalReceiver` drop; requires a set of in-repo type names (struct/enum/class decls).

## 3. S3.1 — struct-field / return-typed receiver index (the named follow-on)

**Priority:** Medium, **gated**: promote to S3.1 (before re-baselining) **iff** the
acceptance corpus rerun shows material callee-recall loss; otherwise Phase-IP. **Why
deferred:** spec §7.4 named candidate; P6-lite deliberately covers only typed params +
constructor locals (the provable subset). **Impact:** `self.field.m()` and chained/
return-bound receivers fall to R6 (demote single / drop multi) instead of resolving
exactly — the largest unrecovered multi-owner class on tokio-shaped code (~18% of
multi-owner sites; ~0% on the prism anchor). **Fix sketch:** a Rust struct-field→type
index (fields are declared typed); extend `receiver_type_in_fn` to resolve `self.field`
receivers via it. Return-type propagation (chained `.m()`, fn-return locals) is a further,
larger step (needs a function-return-type table).

## 4. Nav cache staleness — root-cause the invalidation gap

**Priority:** Important (correctness/reliability). **Why deferred:** S3 worked around it
(the matrix SUT now passes `--no-cache`, Task 13) rather than fixing the cache key, to keep
the acceptance instrument trustworthy without a cache-format investigation mid-branch.
**Impact:** a stale per-repo nav cache silently served pre-change results across binary
versions — it faked **6 capability regressions** during S3 execution until traced. Any nav
consumer (MCP, CLI) that keeps a warm cache across a prism upgrade can read stale edges.
`CACHE_VERSION` was bumped to 4 (Task 4) yet stale fixture caches persisted, so the bump +
content-hash key is insufficient. **Fix sketch:** include `GIT_SHA` (already a build
identity via `build.rs`, commit 6dc2336) in the nav cache key, or a resolver-behavior
version distinct from the serialized-format version; add a test that a binary rebuilt at a
new SHA invalidates a warm nav cache.

## 5. C++ receiver typing not covered by P6-lite

**Priority:** Low (C++ is review-only, unplanned per the roadmap). **Why deferred:** P6-lite
is Rust/Go only; C++ receiver typing needs `type_db` (compile_commands.json). **Impact:**
C++ `obj.method()` / `obj->method()` receiver calls fall to R6 (demote/drop) rather than
type-resolved. C++ `::`-qualified and in-class calls work (R1/R4b). **Fix sketch:** extend
P6-lite receiver recovery for C++ via `type_db` when available, mirroring the Rust path.

## 6. Already-sequenced elsewhere (not new S3 debt, listed for completeness)

- **Confidence on `CpgEdge::Call`** → S2 (spec §7.1): the resolver returns confidence; the
  CPG stores none this phase. S2's batched type churn stores it; first consumers are Plan B
  boundary honesty and `gradient_slice`. **Binding:** because NameOnly edges are CPG-
  included, Plan B must not ship boundary verdicts before S2.
- **Same-file same-name overload identity** (`func_index` last-writer-wins) → S2 span-keyed
  identity (spec §4).
