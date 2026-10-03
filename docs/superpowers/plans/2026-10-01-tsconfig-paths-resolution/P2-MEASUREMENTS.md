# Lane P, P2-0 measurement checkpoint

ASSUMPTION: **Stop implementation dispatch at this checkpoint.** The measured public opportunity is ten distinct sites targeting six callable spans: eight non-relative export hops and two import-forwarded arrows. That is 0.052% of X's 19,219 sites, or 0.32% of P1's 3,121 gains. I recommend treating that public-only yield as immaterial. The historical 749-row F JS-hop bucket could change the decision substantially; its current resolution-admissible yield is still unmeasured. This is a public measurement result and an F gate, not a completed all-corpus P2-0 or a global recommendation to abandon P2.

READ: Owner authorization is measure-first, build only for material precision-safe yield. S6 keeps refusing; no program-graph ownership computation is authorized. The accepted tolerant ambient-scan cost remains. Plan review cap: **2 rounds**; controller's Opus-5.5 review has not run (**0/2**). No production prototype, SPEC amendment, implementation dispatch, product mutants or Tier-A fixture has been created before the gate.

MEASURED: Checkout is `plan/tsconfig-paths-p2` at `1811d2fed149cd8b5794f8a7e5557f2c915b247b`, directly after main `c50de85a`. The retained r5 executable SHA256 is `907d110c70962063d5fde23acd22b6d92b62b117d837b97a81f6d65ed263aa03`; all **304 production/vendor/build inputs** in its frozen manifest equal this checkout. `target/p2-plan/entry-binding.json` records that comparison. The executable was rerun, not rebuilt here; its embedded version identifies the historical dirty build.

## Public measurement and denominator

MEASURED: `p2-probes/public.py R X installed-X T` reran complete no-cache call-site streams, then the pinned P1 oracle. The final `p2-probes/native.cjs` supplement reran against every remaining associated low row using the **actual caller ProjectService program and checker**, rather than the P1 oracle's independent combined program. `p2-probes/summarize.py` rehashed native inputs and bound all certificates to the final instrument hash. Evidence: `target/p2-plan/summary.json`, `public-callable-sites.json`, `public/*/receipt.json`, complete native rows/traces and native input hashes.

READ: Import/site facts were extracted previously by the retained P0 instrument. They were not freshly extracted for the public corpora. MEASURED: all fact-source bytes were rehashed: X **628**, installed-X **628**, R **50**, T **707** files. Complete fresh P1 streams equal their r5 streams byte-for-byte, including keys and metadata. This admits reuse of those facts for this unchanged input and source population; it is not a claim of fresh extraction. The F wrapper does fresh extraction in the controller's private environment.

MEASURED: Native input rehashes cover **633 / 2,219 / 48 / 606** files for X / installed-X / R / T. Oracle version is **5.9.3**, SHA256 `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`. All ten remaining public candidates have identical native terminal `(file, name, start_line, end_line)` and agreeing ProjectService/root-file ownership. No candidate has been upgraded in Prism.

MEASURED: Counts below concern **remaining non-relative import-associated low call sites and their export hops**, including ineligible guards to preserve the denominator. They do not claim absence of these mechanisms throughout unrelated relative, class, CommonJS or dynamic call sites. X and installed-X are copies of the same source snapshot; their ten remaining sites are not additive.

| Corpus | Complete sites | Associated remaining low rows | Native callable candidates | Distinct spans |
|---|---:|---:|---:|---:|
| X | 19,219 | 648 | 10 | 6 |
| installed-X | 19,219 | 648 | 10 | 6 |
| R | 953 | 168 | 0 | 0 |
| T | 61,712 | 15 | 0 | 0 |

## Complete refusal census

MEASURED: These are the P1 oracle's **ordered independent explanations**, not production refusal telemetry. An explanation can preempt another mechanism. In particular, `BARE_BASEURL_OUTSIDE_P1` is a generic `via != paths` fallback and does **not** demonstrate a successful bare-baseUrl resolution. Counts sum to the associated-low denominator in each public corpus. Callable counts are separated in the next table.

READ: F numbers are the owner's supplied final-P1 aggregates (also retained in `MEASUREMENTS.md` under controller acceptance). They were not rerun here and are not bound to this turn's executable. `pending` means no fresh controller aggregate, not zero.

| Ordered refusal reason | X MEASURED | installed-X MEASURED | R MEASURED | T MEASURED | F READ / pending |
|---|---:|---:|---:|---:|---:|
| JS_EXPORT_HOP | 0 | 0 | 0 | 0 | 749 READ |
| NONRELATIVE_EXPORT_HOP | 8 | 9 | 0 | 0 | 1 READ |
| IMPORT_FORWARD_NOT_FORWARDABLE | 2 | 2 | 0 | 0 | pending |
| UNCLASSIFIED_P1_PROOF | 0 | 0 | 0 | 0 | 21 READ |
| BINDING_OR_SITE_GUARD | 0 | 0 | 0 | 0 | 22 READ |
| INVALID_CONFIG_BARRIER | 2 | 2 | 0 | 0 | pending |
| TSSERVER_OWNERSHIP_DISAGREEMENT | 6 | 6 | 144 | 15 | pending |
| BARE_BASEURL_OUTSIDE_P1, generic explanation | 568 | 568 | 0 | 0 | pending |
| MODULE_RESOLUTION_OUTSIDE_P1 | 23 | 23 | 0 | 0 | pending |
| TERMINAL_VALUE_ALIAS_OR_NONCALLABLE | 24 | 24 | 0 | 0 | pending |
| UNPROVEN_STAR_BRANCH | 1 | 0 | 0 | 0 | pending |
| NO_OWNING_CONFIG | 14 | 14 | 0 | 0 | pending |
| DELEGATED_CONFIG_BARRIER | 0 | 0 | 24 | 0 | pending |
| Public denominator | **648** | **648** | **168** | **15** | not measured |

MEASURED: Mechanisms and actual native results for every nonzero public bucket:

| Bucket | TypeScript binding / remaining yield |
|---|---|
| NONRELATIVE_EXPORT_HOP | X: eight callable rows; installed-X: the same eight plus one noncallable value. Four callable identities: `getSceneVersion`, `getSelectedElements`, `convertToExcalidrawElements`, `getCommonBounds`. Entry aliases resolve to TS barrels; named non-relative `@excalidraw/element` forwarding reaches TS arrows, sometimes through a relative star. |
| IMPORT_FORWARD_NOT_FORWARDABLE | Two rows bind `FooterCenter` and `WelcomeScreen` arrow spans in both X copies. The hops are **relative** import forwarding. READ: `js_exports.rs:562–566` requires `forwardable_function_locals`, populated by the declaration model; these arrows do not pass it. This corrects the older P0 list, which labeled all ten rows non-relative. |
| BARE_BASEURL_OUTSIDE_P1 | X: 568 unavailable native terminals. Installed-X: 466 unavailable + 102 noncallable values. Actual successful `via=baseUrl` rows: **0** in both copies. Installing packages resolves many declarations/value aliases; it adds no callable implementation span in this bucket. |
| TERMINAL_VALUE_ALIAS_OR_NONCALLABLE | 24 values in each X copy; native checker does not supply a function/arrow/wrapper implementation span. No P2 callable yield. |
| INVALID_CONFIG_BARRIER | X: two unavailable terminals; installed-X: one value and one unavailable terminal. No callable yield. Membership/config doubt keeps base. |
| TSSERVER_OWNERSHIP_DISAGREEMENT | X: six unavailable; installed-X: two values + four unavailable; R: 144 unavailable; T: 15 unavailable. No callable implementation span. Ownership disagreement independently keeps base under S6. |
| MODULE_RESOLUTION_OUTSIDE_P1 | 23 unavailable terminals in each X copy; native mode is Bundler. No callable yield. |
| UNPROVEN_STAR_BRANCH | One X row has no native callable terminal. Installed dependencies turn its ordered explanation into the ninth non-relative-hop row, which remains a value. It is not the ninth callable gain. |
| NO_OWNING_CONFIG | 14 unavailable terminals in each X copy. No callable yield. |
| DELEGATED_CONFIG_BARRIER | 24 R rows without native callable terminals. Solution/inferred ownership is not an implementation opportunity. |

## Re-measured parked mechanisms

MEASURED: Cells are **remaining mechanism rows / native callable rows**, with overlap allowed. A reference/config/package attribute is not an additive gain. `package_or_other` is the oracle's broad routing classification; it is not proof that package exports or workspaces caused the result.

| Mechanism | X | installed-X | R | T | F |
|---|---:|---:|---:|---:|---|
| Relative JS-family export hops | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 749 historical reason rows; actual hops pending |
| Non-relative callable export paths | 8 / 8 | 8 / 8 | 0 / 0 | 0 / 0 | 1 historical reason row; actual paths pending |
| Import-forwarded callable arrows | 2 / 2 | 2 / 2 | 0 / 0 | 0 / 0 | pending |
| Successfully resolved bare baseUrl | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | pending |
| `.js` to `.ts`, entry or requested export hop | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | pending |
| Project-reference attribute | 0 / 0 | 0 / 0 | 139 / 0 | 15 / 0 | pending |
| Package/other routing | 0 / 0 | 592 / 0 | 0 / 0 | 0 / 0 | pending |
| Multiple paths substitutions | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | pending |
| Tied patterns / empty captures | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | pending |
| Explicit substitution extension refusal | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | pending |
| NodeNext/Node16/Bundler | 23 / 0 | 23 / 0 | 139 / 0 | 15 / 0 | pending |
| Package/array extends attribute | 6 / 0 | 6 / 0 | 0 / 0 | 0 / 0 | pending |

READ: Native rules for any later port, all in the pinned `typescript.js`:

| Rule | Source lines |
|---|---|
| Node10 priority TS/declaration pass, then secondary pass | 45240–45243 |
| Relative hop: local file/directory lookup; non-relative: optional settings, ancestors/@types/custom roots | 45287–45336 |
| Relative normalization and file-before-directory lookup | 45341–45394 |
| Known suffix replacement and implicit extensions, including `.js` to `.ts` | 45423–45503 |
| Directory/package types, typings, main, typesVersions and index | 45745–45813 |
| Selected paths pattern suppresses baseUrl fallback, including a miss | 44970–45001 |
| Bare baseUrl lookup | 45062–45074 |
| Ordered paths substitutions; explicit known-extension `tryFile` shortcut | 46417–46440 |
| Pattern precedence | 3638–3649, 22753–22762 |
| Extends arrays / ordered option merge | 43455–43549 |
| Real default project ownership | 190331–190337, followed by `ensureDefaultProjectForFile` |

## What blocks JS_EXPORT_HOP

READ: `js_paths.rs` has two separate blockers. `relative()` ignores `_allow_js` and finally rejects every JS-family result. Earlier, `prove_path()` rejects explicit `.js/.jsx/.mjs/.cjs` spellings. Therefore removing the final rejection alone cannot recover explicit-JS hops. `call_graph.rs:2257–2259` routes the alias export projection through that relative resolver, and `repo_loader.rs:282` primes the same route. The alias entry resolver already has the non-relative Node10 first-pass proof; the relative export path does not use its local counterpart.

READ: A faithful **relative** hop uses the same Node10 two-pass rule at that hop's containing file, but its first pass searches local relative file/directory candidates. It does **not** search importing ancestors, node_modules/@types or custom typeRoots. Reusing the entire non-relative `absent()` helper for a relative hop would introduce unrelated refusals. Reuse/port the local `Pass::relative` semantics and suffix/package rules, retaining captured no-follow readability, indexed-source identity, caller-project allowJs, complete export provenance and every existing binding/span guard. Native module resolution alone does not authorize Exact.

MEASURED: Both grammars' extensionless/named/star/import-forward and explicit-relative-JS probe cases bind the JS implementation after the priority lookup misses. A sibling declaration binds first and preserves refusal; explicit relative `.js` with a sibling `.ts` binds that TS implementation. A missing star branch prevents a complete-hop proof. The probe checks cover these outcomes without editing production.

MEASURED: A separate witness shows that **non-relative explicit paths substitutions are different**: `@p -> ./leaf.js` resolves directly to `leaf.js` with an occupied `leaf.d.ts` and **no secondary-pass trace**. This follows `typescript.js:46431–46435`. The instrument records JS-family and observed secondary-pass separately. Do not assert that every JS winner implies first-pass absence. Receipt: `target/p2-plan/explicit-path-witness/result.json`.

READ: The controller script measures F's actual named/star/import-forward paths, JS spellings, allowJs-off population, non-relative overlaps, native terminal/ownership disagreements, physical/closure doubt and unclassified detail codes. Its `relative_only_resolution_ceiling` is a **resolution ceiling**, not a certified Prism gain: the requested-member traversal does not reproduce all Prism star-opacity, write, wrapper, depth, cache or extraction guards. Those require the conditional prototype and changed-row oracle. Until F returns, the exact mechanisms within its 749/21/22/1 buckets remain OPEN.

## Ranked scope and forecast

ASSUMPTION: Ranking uses measured public marginal yield and READ historical F potential. The first row's realized F yield is not established; it cannot yet authorize a build. Forecasts are review sizing estimates, not LOC caps.

| Rank | Mechanism / decision | Yield evidence | Precision risk and forecast |
|---:|---|---|---|
| 1 | Relative JS export-hop proof, only if fresh F shows material resolution ceiling | MEASURED: public 0. READ: F reason bucket 749. | ASSUMPTION: bounded local two-pass port; explicit-JS suffix mapping may be inseparable. Preserve allowJs, physical/declared occupancy and all export guards. About 150–300 source lines, 300–500 control/test lines, 30–70 cache/fixture lines. Start with the measured spelling population; do not add an unmeasured blanket refusal. |
| 2 | Non-relative export hops | MEASURED: eight public sites, four spans. READ: F reason bucket 1. | ASSUMPTION: project option/context identity must accompany the projection; current allowJs-only partition is insufficient when two importing projects map the same barrel specifier differently. About 250–450 source lines plus 350–550 tests/cache controls. Public-only yield does not justify it. |
| 3 | Import-forwarded arrows | MEASURED: two public sites, two spans. | ASSUMPTION: changing shared forwardability risks legacy consumers. Prefer an alias-only span/provenance addition if ever material. About 60–120 source lines plus 150–250 tests. Park at two rows. |
| 4 | Bare baseUrl, general `.js` mapping, ordered substitutions, package/array extends, other modes | MEASURED: zero public marginal callable yield. | ASSUMPTION: no implementation now; each has precedence/config risks and would enlarge review without measured return. |
| 5 | References / program ownership, package exports/workspaces | MEASURED: zero public callable yield; ownership disagreements explicitly present. | READ: S6 remains refused. ASSUMPTION: no scope expansion; do not add program ownership or modern package machinery. |

ASSUMPTION: If the controller certifies a substantial portion of the 749 F rows as resolution-admissible, relative JS-hop proof is a plausible bounded P2. That would be roughly 32% of the historical 2,313 F P1 gains at the full ceiling. The actual gain must be measured with a prototype; neither 749 nor that ratio is a promised recovery. Otherwise recommend stopping. The owner supplied no numeric materiality threshold; [OQ-paths-p2.md](OQ-paths-p2.md) records the decision.

## Verification, custody and next boundary

MEASURED: Complete public no-cache dumps and both oracles ran; **22/22 synthetic instrument checks pass**, **0 failures**, across TS/TSX. Wrapper stdout was checked for absence of corpus, caller and terminal paths. Shell/Node syntax checks pass. Source/input/instrument hashes and bucket denominators pass. `hypothesis-probe-result.log` records the inadmissible first setup attempt and subsequent instrument corrections; unavailable terminal fields are no longer counted as agreement. No Git writes, private F access, network/install or cargo build directory was created.

READ: Not verified: fresh F aggregates, every F changed row, a P2 production prototype, product RED/GREEN, product cache behavior/resource gates, product mutants, full Rust test suites, Tier-A matrix/quick/full, independent Opus review, Linux/case-sensitive behavior or concurrent-tree semantics. Rust suites/Tier-A were not rerun because this checkpoint changes only measurement probes and documentation and makes no production implementation done-claim. P1 test totals remain historical, not this turn's execution.

ASSUMPTION: No buildable production work is awaiting completion. The next authorized boundary depends on missing F information. If material yield is established, retain the prototype as a patch plus exact source/binary hashes in an isolated copy or obtain a controller worktree; finish all four public head/base comparisons, every changed-row oracle, both-grammar controls, scoped incremental mutants, full suites and Tier-A, SPEC amendment and P2 implementation dispatch before handing off a buildable slice. Do not reopen any P1 refusal population without X/installed-X/F yield measurements.

READ: Controller command, with `CORPUS_F_ROOT`, a new private `PRIVATE_EVIDENCE_ROOT`, and pinned `TS_JS` already set:

```bash
bash docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/p2-probes/CONTROLLER-p2.sh \
  /Users/wesleyjinks/code/prism-paths-impl/target/repair-r5/head/prism \
  /Users/wesleyjinks/code/prism-paths-impl/target/repair-r1/base/dump_imports
```

READ: Return stdout JSON only. Raw rows, paths, sources, traces, diagnostics and the actual ProjectService program remain private. A setup failure yields no measurement; its diagnostics are private. Proposed controller commits and the exact file inventory are in [P2-FILES.md](P2-FILES.md).
