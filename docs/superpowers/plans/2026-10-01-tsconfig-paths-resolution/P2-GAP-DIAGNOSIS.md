# P2 gap diagnosis packet

[INHERITED] Controller supplied **32** changed rows, all `CORRECT_STATIC_BINDING` and `JS_EXPORT_HOP`, **24** member-written, no key changes; relative JS-resolution ceiling **33**. The native-callable, owner-agreeing bucket has **749** rows, so **717** are unrecovered by this prototype. The roughly **716** figure is 749 minus the resolution ceiling, not the exact unrecovered-row denominator. P2 as built is not material. No private input was read by this planner.

[MEASURED] Checkout `proto/tsconfig-paths-p2` at `e80fbf541d53ace5c547d5f26c9135e46e7be12b`; plan predecessor `plan/tsconfig-paths-p2` at `1cb46b80`. Production inputs equal the pinned 304-file manifest. This round adds diagnostics and docs only, with no Git writes.

## Controller command

From this checkout, set the controller-owned `CORPUS_F_ROOT`, a **new, nonexistent** `PRIVATE_EVIDENCE_ROOT`, and pinned offline `TS_JS` (TypeScript 5.9.3), then run:

```bash
bash docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/p2-probes/CONTROLLER-p2-gap.sh \
  /Users/wesleyjinks/code/prism-paths-impl/target/repair-r5/head/prism \
  /Users/wesleyjinks/code/prism-paths-impl/target/repair-r1/base/dump_imports \
  /Users/wesleyjinks/code/prism-paths-p2-plan/target/p2-plan/bin/prism-p2
```

Return stdout only: one JSON object. Raw source-derived keys, paths, traces, facts, diagnostics, rows and input hashes remain in the private evidence directory. The output omits corpus names, root paths, site keys and corpus-derived hashes. Binary and diagnostic-source hashes identify instruments only. An inadmissible run prints one fixed `INADMISSIBLE` object with the failed stage, exits nonzero, and publishes no counts.

The packet requires Python 3.9+, Node, Cargo/rustc and the existing offline Cargo dependencies. It builds the unchanged library with `cargo build --release --lib --offline --locked`, copies the six actual resolver modules into the evidence directory, and compiles a measurement-only sidecar. Production files are never patched. No installs or network are used. Allow several minutes; time depends on the native ProjectService programs and whether the release library is warm.

## What the partition proves

The denominator is exactly P1-low rows whose prior reason is `JS_EXPORT_HOP`, native callable is true, and caller-project ownership agrees. A row is recovered only if P2 emits the one native exact `import_member` terminal, including its span. Every remaining row receives one primary class and an exact gate code. Primary class totals must equal `p2_unrecovered`; counters for later gates and TypeScript diagnostics are not additive. The complete P1/P2 comparison retains key/metadata checks and changed-row native certification.

Entry resolution is evaluated first. The unchanged kernel returns the actual target; the explainer must reproduce that Option on every entry and every raw export-hop request. The sidecar extracts actual prism export facts, forwardable names, exact spans, conflicts and indexed files. Export replay retains named-before-star precedence, the two-hop limit, active-stack cycles, forwardability after child resolution, distinct terminal competition and full skipped-star provenance. It must match the actual P2 export table and identities. Neither a JS terminal nor a successful TS hop is used as a shortcut to explain a prism refusal.

TypeScript outcomes are recomputed in the real caller ProjectService program with that project’s options: module resolution pass, source/declaration/unresolved/outside result, literal shape, package/typesVersions/index/suffix redirect, checker export-symbol presence, callable shape and semantic diagnostic codes. No diagnostic message or source name is printed. Export and site gates use the checker export result and terminal shape rather than pretending that they are module-resolution failures. A native symbol in a program with duplicate-export diagnostics is not promoted to precision-safe recovery.

`counterfactual_gates_after_native_entry` means the entry was actually refused, but the native target was hypothetically admitted so later export gates could be enumerated. These are explicitly not executed prism paths and not additional recovered rows. `unresolved_star_branch` reports its underlying exact branch gates in `encountered_gate_rows`.

Plain singleton `folder/index.js` is already supported. An explicit `folder.js` directory, a package redirect or file/index competition can still be refused. The script reports the redirect used by TS separately from prism’s gate, so “directory index” cannot silently become a catch-all refusal class.

If `post_export_site_gate` is nonzero, that count is explicitly **unresolved**: module/export replay passed but the supplied site did not recover. Do not call that a diagnosed resolver gap or dispatch a fix from it; a narrower site-query trace is required. Kernel/export disagreement, byte drift or uncertified changed output invalidates the run.

## Class meanings, port feasibility and forecast

All sizes below are **ASSUMPTION**, source LOC / behavioral-test LOC for a future bounded implementation. They are not additive: candidate ordering, explicit suffixes, redirects and first-pass evaluation share a kernel. A resolver-only change cannot fix export, custody or site policy. Null sizes mean a separate contract/design is required before a defensible implementation estimate.

| Class | Exact meaning | Faithful TypeScript port | Source / tests; risk | Public shape |
|---|---|---|---|---|
| `entry_js_first_pass` | Alias substitution selects JS; P1/P2 refuse because the complete local/ancestor/@types/typeRoots priority-pass absence proof fails. | YES for a readable indexed native JS winner; distinguish an actual TS/declaration winner from conservative occupancy. | 120–250 / 200–400; medium-high | `entry-js-first-pass` |
| `hop_js_first_pass` | A relative JS candidate exists, but its local Node10 priority-pass absence proof fails. | CONDITIONAL: can prove the native source winner; declaration winners must remain refusals. | 80–160 / 150–300; medium | `hop-js-first-pass` |
| `nonrelative_hop` | Export/import-forward module literal is bare or aliased; P2's relative-only callback rejects it before lookup. | YES for supported paths/baseUrl/package lookup with source custody; retain caller project options across barrels. | 160–320 / 250–500; high | `nonrelative` |
| `package_directory_redirect` | prove_path sees package.json at the candidate directory and refuses before reading main/types/typesVersions or index. | YES for bounded readable Node10 redirects; types/declaration-only results are not executable callable authority. | 120–240 / 200–400; high | `package-main` |
| `explicit_extension_literal` | Entry prove_path rejects a known non-TS extension, or a relative explicit JS/TS literal is unavailable. Relative indexed literal JS is already admitted by P2. | YES with separate paths-literal and relative-suffix rules; substitutions and appended extensions can win. | 60–140 / 150–300; medium | `explicit-redirect` |
| `directory_literal` | Specifier or alias substitution ends in slash or slash-dot; refused before directory lookup. | YES for readable bounded package/index resolution. | 40–100 / 100–200; medium | `directory-literal` |
| `candidate_competition` | More than one file/index/extension candidate exists, or an explicit TS hop has a competing sibling; prism requires a singleton instead of applying TS precedence. | YES; ordered file-before-directory and suffix precedence can prove a source winner. | 100–200 / 200–400; medium-high | `file-index-competition` |
| `declaration_priority` | A declaration or unknown-suffix declaration competitor blocks prove_path. | CONDITIONAL: a native declaration winner cannot certify a source callable; source winners require ordered resolution. | 60–120 / 150–250; medium | `declaration-priority-negative` |
| `source_custody` | Snapshot incomplete, opaque/outside path, or winner not indexed; exact gate histogram distinguishes them. | NO by resolver port alone; custody/index coverage is a separate contract. | design first / design first; high; design required | `unindexed-package-negative` |
| `candidate_absent` | Bounded candidate list is empty, or extensionless literal file blocks lookup; native may resolve an unsupported extension/redirect. | CONDITIONAL on an indexed source winner; no callable proof for true absence. | 60–160 / 150–300; medium-high | `missing-negative` |
| `config_or_alias_gate` | Prism cannot select/admit config or alias pattern before candidate lookup; exact gate histogram names the refusal. | CONDITIONAL; pattern/options ports can help, ownership doubt stays refused. | 100–300 / 200–500; high; split by exact gate | `ordered-substitution-negative` |
| `ambient_competition` | Captured ambient module pattern matches the alias and vetoes entry. | NO by filesystem resolution alone; requires program-scoped declaration/binding proof. | design first / design first; high; separate design | `ambient-negative` |
| `allow_js_off` | The selected config lacks literal allowJs=true at a JS entry/hop. | CONDITIONAL; TS checker knowledge of JS does not itself grant the existing indexing/allowJs contract. | 30–80 / 100–200; high; policy decision | `allow-off-negative` |
| `export_depth_limit` | A third export/import-forward/star hop exceeds MAX_REEXPORT_DEPTH=2 before module resolution. | YES for finite unique export graphs using memoized symbol/identity closure; increasing the constant is not a TS port. | 100–220 / 200–350; medium-high | `depth-three` |
| `export_cycle` | An active file/export-name pair repeats in bounded export lookup. | CONDITIONAL: SCC/fixed-point export closure can prove cycles with a unique terminal; empty/ambiguous cycles remain refused. | 160–300 / 250–450; high | `cycle-positive` |
| `star_competition` | Conflicted raw named claims or multiple distinct star terminal identities poison export lookup. | NO for a real TS2308 ambiguity; YES only where TS precedence or identical-symbol deduplication proves uniqueness. | 80–180 / 180–350; high | `star-competition-negative` |
| `unresolved_star_branch` | A callable was found but skipped_star marks a missing/opaque/unresolved branch; alias query rejects via_unresolved_star. | CONDITIONAL: prove branch absence under TS export semantics; checker first-wins lookup alone is insufficient. | 80–180 / 180–350; high | `star-hole` |
| `import_forwardability` | ImportForward reaches a terminal outside forwardable_function_locals, e.g. an arrow instead of a top-level function declaration. | NO by module resolver alone; YES with source-backed callable and unwritten-binding proof across forwarding. | 80–180 / 180–350; medium-high | `forward-arrow` |
| `export_fact_gap` | Missing file/name fact, UnprovenLocal, or non-callable class projection prevents an export-table terminal. | NO by module resolver alone; an AST/export proof extension may prove supported syntax. | 100–300 / 200–500; high; split by syntax | `default-expression` |
| `terminal_span_gate` | Export resolves to Local without an exact callable span; alias projection refuses name-only authority. | NO by module resolver alone; source-backed span capture can prove a unique callable. | 60–140 / 150–300; medium-high | `binding-write-negative` |
| `binding_or_site_guard` | Import binding eligibility/kind/ESM provenance or site Unproven(import) guard refuses before alias use. | NO by module resolver alone; preserve binding writes/shadows and prove any new site form separately. | 60–180 / 150–350; high | `shadow-negative` |
| `wrapped_non_jsx` | Wrapped React callable is projected at an ordinary call site; current contract admits it only at JSX sites. | NO; this is a site policy, not module resolution. | design first / design first; contract decision | `wrapper-call-negative` |
| `terminal_function_lookup` | Proven export span has zero or multiple matching FunctionIds; callable projection cannot select one. | NO by module resolver alone; repair callable capture only after source-specific diagnosis. | 60–180 / 150–350; high | `span-lookup-contract-negative` |
| `post_export_site_gate` | Entry, export closure, provenance and callable lookup pass, but supplied P2 site output is not native exact binding. This is explicitly unresolved query-level diagnosis, not an explained resolver refusal. | UNKNOWN; requires a narrower site-query trace before planning. | design first / design first; unresolved; do not dispatch | `site-output-contract-negative` |

The executable catalog is `p2-probes/gap-class-catalog.json`; `exact_gate_histogram` retains the individual source guard when several guards share a forecast class. Public program shapes are generated by `gap-controls.py`; kernel preservation shapes by `gap-kernel-controls.py`; wrapper/span/lookup/site-query guard shapes by `gap-guard-controls.cjs`. Guard tests vary actual public export data at explicitly named seams: they are not alleged production failures. Some preservation controls are intentionally excluded from the native-callable bucket (declarations, shadows, missing sources, unsupported config shape and explicitly extended entries). These do not fabricate private opportunity counts.

## Expectation before the controller run

**ASSUMPTION: `nonrelative_hop` dominates.** The prototype only expands relative callbacks, while the supplied native relative-hop ceiling is 33 of 749. The other rows must have a native non-relative hop, unresolved/unindexed/opaque closure, allowJs/ownership restrictions, or a combination. This constrains the hypothesis but does not identify the first prism blocker. Entry JS first-pass refusal may mask a later non-relative hop; the primary/counterfactual split will discriminate. Depth and forwarding are real independent limits but cannot by themselves explain why the native relative-resolution ceiling is so small.

## Verification and custody

MEASURED: **96 scenarios /192 sites**, with **136** native callable, owner-agreeing JS-hop rows: **16** recovered and **120** partitioned into **14** exercised primary classes; **6** adversarial checks and **11** guard contract checks passed. Both grammars and member-written variants are covered. Kernel preservation controls outside that denominator add **12** exact-kernel checks and **12** TypeScript outcome checks. Latest classifier replay preserves the sealed partition with zero unresolved site rows. Receipts: `target/p2-gap-controls-sealed/summary.json`, `aggregate-final.json`, `evidence/gap-guard-checks.json`, `target/p2-gap-kernel-controls/summary.json`.

MEASURED: `PRISM_TYPESCRIPT=<pinned TS_JS> PYTHONDONTWRITEBYTECODE=1 cargo test --offline` completed the **full default suite: 4,935 passed /0 failed /1 existing ignore /0 filtered**, 29 groups including doctests. Receipt: `target/p2-gap-controls-final/rust-default-summary.json` and complete log. The existing ignore is `resolution_test::slice_elem_variant_reserved`. Shell/Node/Python syntax, `git diff --check`, and all304 immutable production-input hashes passed. No production files or tests were edited.

Not run this round: full MCP/all-features suites, Tier-A matrix/quick/full, independent review, Linux/case-sensitive or concurrent-tree checks, and private corpus classification. The current changes do not touch production resolution/navigation/CPG, so no Tier-A trigger or new product-accuracy claim is made. The prior quick exclusion remains historical and unresolved. No historical full-suite result is presented as this turn's run.

Diagnostic correction cap: **2/2**, no extension. Initial missing rustc dependency search directory was inadmissible setup, not gate evidence. The first behavioral run exposed shared-alias ambient contamination; only that control's alias was corrected on the existing artifact. The second partition agreed with the known public shapes. Later runs verified added member/guard/drift coverage. The hypothesis/probe/result record is retained in `target/p2-gap-controls-sealed/hypothesis-probe-result.md`.

Local custody: source/docs hashes and `diagnostic-owned-snapshot.tar.gz` under `target/p2-gap-controls-sealed/`; no Git or external-backup claim. The root VERIFICATION.md is ignored and included in that snapshot. Preserve the three immutable input binaries and pinned compiler; the copied/build sidecar is reproducible from this packet.

Public reproduction, with a new output root:

```bash
P2_GAP_CONTROL_OUT="$PWD/target/p2-gap-review" python3 docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/p2-probes/gap-controls.py P1_BIN FACTS_BIN P2_BIN "$TS_JS"
python3 docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/p2-probes/gap-kernel-controls.py target/p2-gap-review/evidence/gap-driver-build/gap-driver "$TS_JS" target/p2-gap-kernel-review
```

No actual private classification is claimed. Controller execution, independent review and any future production proposal remain separate steps.
