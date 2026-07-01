# Merged Spec Review — S3 Call-Resolution Precision Floor (rev 1 → rev 2)

**Process (first use of the cleanroom protocol):** both reviewers received a variant of
the spec with the R6 decision **redacted** and replaced by an open-question exercise
(enumerate policies P1–P6, recommend one, state consequent spec changes), reviewed in
a worktree branched from `84ff67c` (pre-spec-commit) so no trace of the chosen policy
was reachable. Reviewers: **codex gpt-5.5 xhigh** (rigor lens, draft→refine via
a2a-bridge, config `examples/a2a-bridge.slicing-spec-review-codex.toml` — codex-only
variant while the bridge claude model override defect is open) and **claude fable
xhigh** (soundness lens, draft→gaps-register→refine as an operator subagent). Raw
outputs: codex `/tmp/s3-spec-review-codex-rigor.md` (transient), fable inline in the
session transcript; the substance is merged below.

**Cleanroom outcome — R6:** both reviewers **independently recommended P6-lite
layered over P2** ("recover what tree-sitter can prove, demote the single-owner
residue, drop the rest"), converging with the operator-side scale analysis (prism:
1% of receiver sites multi-owner; tokio: 50%; caddy collisions are interface-method
shaped) and with the spec's pre-redaction choice on the P2 half. The spec's original
R6 (P2 alone) is **amended** to the three-step policy in rev 2. Fable additionally
quantified the caddy 441-FP split: ≈167 receiver-var (R6-shaped) / ≈148
imported-package-qualified (R3 fall-through — **not** reachable by any R6 policy),
which forced the M1 fix below.

**Independent verification performed during synthesis:** codex's three most
load-bearing claims were spot-checked against code before acceptance (scoped-CPG
bare-name resolution `src/cpg/context.rs:131-134,356`; `CACHE_VERSION` at
`src/cpg_cache.rs:44`; qualifier-discarding helpers `src/call_graph.rs:857,883,925,992`)
— all confirmed.

## Merged findings (BLOCKER → MAJOR → MINOR)

| # | Sev | Section | Finding (lens) | Resolution in rev 2 |
|---|---|---|---|---|
| 1 | BLOCKER | §2.2 R6 | R6 policy was open (by cleanroom design); both lenses recommend P6-lite + P2 residual (codex B1, fable §3.5) | Adopted: 3-step R6 — qualifier-as-owner rung, P6-lite syntactic receiver recovery (Rust+Go), then drop-multi/demote-single residue; fallback to steps 1+3 as S3 core with P6-lite as S3.1 *before* re-baselining |
| 2 | BLOCKER | §2.2 R4/R5 | Method-exclusion for unqualified calls is language-unsound: Java has **no free functions** (`languages/mod.rs:103`), C++ members call siblings unqualified — both call graphs would be deleted, unmeasured (no Java/C++ corpus), surfacing only as golden drift pre-framed as improvement (fable B1) | New implicit-receiver rung R4b (Java/C++ only): unqualified `f()` inside a method of owner `K` checks `(K, f)` → Exact; Java sibling-call survival fixture added to §5 |
| 3 | BLOCKER | §3 | "Shared resolver" overstated: four traversal helpers re-resolve **dropping the site qualifier** (`call_graph.rs:857,883,925,992`) and `build_skeleton` extracts no qualifiers (`:116-123`) yet `compute_scope` resolves through it (`cpg/context.rs:356`) — under new R5 every method call on those paths looks unqualified → method edges stripped from barrier/spiral/circular/vertical/3D and from diff-review scoping (fable B2; codex M4) | §3 now enumerates: thread `site.qualifier` through all four helpers; scope computation pinned to a recall-biased name-only mode (scope is a superset heuristic, not a truth claim) |
| 4 | BLOCKER | §3.3 | CPG inclusion rule for NameOnly never pinned; "Step 5b stops polluting" is policy-dependent (codex B2; fable M5) | **Pinned: CPG includes Exact + NameOnly; drops excluded everywhere.** Disagreement resolved against codex's Exact-only preference: measured single-owner precision is 0.99/0.89 (U-strata) so exclusion would buy little precision for a real slicing-recall regression; coupling note added — Plan B must not ship boundary verdicts before S2 stores confidence, since NameOnly edges are CPG-visible |
| 5 | BLOCKER | §2.2/§3 | Resolver input contract unspecified: extraction yields only final name + raw qualifier text; Rust extracts **no imports** (`src/ast.rs:388`) and `Type::m()` arrives as `callee_name="Type::m", qualifier=None` (`languages/mod.rs:693`) (codex B3; fable verification) | §2.2 gains a parsed `CallTarget` contract: shapes `T::m`, `mod::T::m`, `mod::f`, `pkg.f`, `Class.m`, `x.m`, `self/this`, Go receiver-var; Rust path heads `crate`/`self`/`super`; `Self::` self-type resolution |
| 6 | MAJOR | §2.2 R3 | R3 "unchanged" preserves the import-narrowing **fall-through** (`call_graph.rs:783`) producing ≈148 caddy FPs (`zap.Error`/`caddyhttp.Error`) that no R6 policy can reach; Go import narrowing matches file stems, not package paths (fable M1) | R3 amended: qualifier in imports map + no in-repo candidate ⇒ **unresolved** (provably external); Go import matching by package directory/path suffix; Go package-path fixture added |
| 7 | MAJOR | §2.2 | Missing qualifier-as-owner rung (`ClassName.method()`, Lua `M.f()`) — resolvable exactly but fell to R6; Lua keying silently changes `"M.f"`→`"f"` (fable M2) | New rung R3b `(qualifier-as-owner)` → Exact; Lua `FunctionInfo.name` change made explicit with golden note |
| 8 | MAJOR | §2.1/§2.2 | Trait dual-key returns all impls at **Exact** — CHA over-approximation mislabeled; `Default::default()`-class fan-out becomes confidently wrong (fable M3; codex M6) | Trait-key lookups returning >1 candidate demote to NameOnly with `kind: trait_cha` |
| 9 | MAJOR | §3.4 | Stem fallback retained nav-side perpetuates two-resolver divergence (substrate F11); Plan B boundaries vs nav would disagree on `mod::free_fn` (fable M4; codex accepted nav-side) | Resolved toward fable: fallback **folds into the shared ladder** as explicit last rung (single stem match Exact, multiple NameOnly — also fixes the duplicate-stem-at-Exact gripe); goldens are being re-blessed this phase anyway |
| 10 | MAJOR | §2.1 | Owner-key normalization undefined (generics, pointers, nested classes, out-of-line C++, same-named types in different modules) (codex M5) | §2.1 defines OwnerKey: bare type name, generics/pointer/ref stripped; same-named-type collision limitation stated (S2/Phase-IP refine) |
| 11 | MAJOR | §3 | CPG `func_index` keyed `(file,name)`, last-writer-wins (`cpg/build.rs:197-212`) — same-file same-name methods collide at edge assembly (codex M7) | Stated exclusion: same-file overloads remain outside S3 guarantees; S2 (span-keyed identity) is the fix |
| 12 | MAJOR | §3 | Step 5b receiver/param binding unspecified for method calls (explicit args vs declared params incl. Python `self`) (codex M8) | Rule pinned: receiver never binds to a parameter in S3; Python binding skips leading `self`/`cls`; per-language tests |
| 13 | MAJOR | §3.5 | Cache bump named the wrong store: `CACHE_VERSION` lives in `src/cpg_cache.rs:44` (serialized payload includes `CallGraph`); index-maintaining mutators unenumerated (codex M9) | §3.5 corrected (v3→v4); mutator list added: `empty`, `build`, `build_skeleton`, `build_direct_subset`, `remove_files`, `merge`, deserialization |
| 14 | MAJOR | §1/§5 | "Near-eliminated"/"zero regressions" too subjective to gate (codex M10) | Concrete gates: matrix flip + zero ok→fail flips; tokio C-method caller corrected FP 390→≤20; caddy C-name caller FP 441→≤30; no anchor-corpus stratum corrected P or R drops >0.02; R6 residue counts reported in PR |
| 15 | MAJOR | §2.4 | Drop-visibility is callees-only; callers queries silently lose dropped R6 sites — trust erodes exactly where the spec aims to build it (fable M6; codex M11) | Callers/ego emit `WarningKind::Collision` (exists, `types.rs:86`) when same-name dropped R6 sites exist |
| 16 | MINOR | §2.1 | C++ `ns::f` vs `Foo::bar` syntactically identical (fable M7) | R1 treats the qualified prefix uniformly as index key; misses fall to stem rung |
| 17 | MINOR | §3.2 | Resolver signature lacks caller identity for R2; owner-by-`FunctionId` side map needed since `FunctionId` is frozen (fable M8) | Signature change named: caller `FunctionId` param + owner side map |
| 18 | MINOR | §3.4 | NameOnly 0.5 collides with hop-decayed Exact (fable M9) | NameOnly base score set to 0.6 |
| 19 | MINOR | §3.4 | `Reason::Resolution { kind }` schema undefined (codex M12) | Kinds enumerated per rung; serde snake_case; attaches alongside `Calls`/`CalledBy` |
| 20 | MINOR | §0 | Spec cites pre-amendment counts (922/26) vs current jsonl (924/37) (fable M10) | Amended-count footnote added |
| 21 | MINOR | §3 | `qualifier: None` must mean *verified receiver-less*; Phase-3 synthesized sites (`call_graph.rs:325-534`) satisfy it today by luck, not contract (fable coupling note) | Stated as a named invariant |

## Verdict

Both lenses: the core architecture (owner index + tiered ladder in the shared
resolver + deferred confidence storage) is right-sized and correctly seamed; rev 1
as written would **not** have met its own acceptance bar (B2/B3/M1 above). With the
rev 2 amendments — all folded in the same commit as this record — **sound to plan**.
