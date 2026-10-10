# PR-B R10 — COMPLETE and clean on admitted roots

[MEASURED] Written 2026-10-10T07:47:02.841108+00:00. Source patch base **96817370ffb65dd3de14fe574789b6bf31e99ac5**; docs patch base **29a1d2d9139590baf3a281d1f32e233f01337026**. Planning branch plan/js-param-defs-prB stays at docs HEAD29a1d2d9; no Git metadata writes. Entry source727/727 equal including untracked files, no extras. R10 frozen source has728 files with one new control fixture; source-manifest SHA256 `12ece9338d27eb411666fb0e08635e2cc2591f32c5ca7a65df349c3095fa3b61`. Static analysis only; no corpus package execution, network/provider use or frontend-portal access.

## Part A: corrected saved-detail baseline, before product edits

586 accounted,579 admitted (X/Xi/T +576 SecBench), seven retained main-side exclusions. Re-scored saved R9b details only; no corpus recapture for Part A. Every result binds saved details, diagnostics and proofs with SHA256. Raw row totals independently reconcile to the previous raw ledger. Bucket1 is `.js`/`.jsx` with at least one TS8000–8999 diagnostic; bucket2 other TS syntactic diagnostics in JS/JSX and no8xxx; bucket3 everything else. Only bucket1 receives the owner's E13 exception. `@flow` is not the test.

A non-allowlisted Node failure is an INADMISSIBLE **proof**, retaining the raw oracle verdict. Only a fixture-backed genuine early error overrides to WRONG. Both worker tools and controller helper/consumer implement the rule. Raw/after-override totals are identical on the saved corpus: zero allowlisted overrides, five failed proofs leave five Pressability rows CORRECT. Opus's expected bucket1 LOST CORRECT11 is **confirmed** (six known + five Pressability). Outside bucket1, every admitted root has LOST CORRECT0 and ADDED WRONG0, raw and after override. UNDECIDED remains separately disclosed.

Allowlist positives: six message families plus Unicode, escaped Greek and astral redeclarations; negative colon/static/super/member cases plus a typed neighbour for each new invalid-target message. Fresh built-in controls9 positive/6 negative. IdentifierName validation covers ECMAScript Unicode and Unicode escapes without accepting punctuation or invalid scalars. Synthetic worker/controller parity22/22. Probe suite71/71, including eight F summary STOP/override controls. Shell syntax valid. Two initial parity fixture expectations incorrectly called invalid-binding sources bucket3; both implementations independently returned bucket2/TS1005. Those expectations were inadmissible, corrected without a product change.

## Confirmation dispositions

| Confirmation finding | Classification | Disposition | Test / record |
|---|---|---|---|
| Opus E1; Sol R9-W2 | WRONG | rule changed | TS8xxx bucket; octal/no-pragma regressions; parity22/22 |
| Opus E2 | WRONG | rule changed | failed proof retains raw CORRECT; outside STOP; r10-controller-summary controls |
| Opus N1; Sol R9-W1 | WRONG | fixed | r10_labelled_callable_body_rows_survive_without_a_seam; PD110 |
| Opus N2 | WRONG | fixed | r10_jsx_member_tags_keep_complete_path_rows; namespace formal collides; PD111 |
| Opus N3; Sol R9-W3 | WRONG | fixed | r10_enum_string_and_merged_members_fence_rows; quoted/escaped/merged keys; PD112/113 |
| Opus commented-eval note (pre-existing) | WRONG | fixed | r10_eval_comments_refuse_formals_and_keep_indirect_rows; PD114 |
| Sol R9-W4 | WRONG | rule changed | Unicode/escaped redeclaration positives and parser-neighbour negatives |
| Opus N4 | WRONG | recorded | recovered body / parser-artefact owners and file-level guard proposal in SPEC follow-ups; classification retained |
| Opus S1(i) ERROR ancestor | SMELL | fixed coverage gap | r10_error_ancestor_walk_preserves_local_rows; clean params/ERROR ancestor; PD115 |
| Opus S1(ii–iv) namespace, bare enum, asserts, row controls | SMELL | fixed coverage gaps | corrected r9 namespace formal; r10_bare_enum_and_asserts_name_arms_have_rows; full row and 48 frozen negative/seam controls |
| Opus S2 | SMELL | rule changed | both invalid-target messages, genuine positives and typed-neighbour negatives |
| Opus S3 | SMELL | recorded | TypeScript class-symbol blind spot; SPEC follow-ups |
| Opus F10/F11 | SMELL | recorded | shared reaching_edges / alias-twin ordering; SPEC follow-ups |
| Sol malformed-arrow note | SMELL (not promoted in review) | recorded | SPEC follow-ups; generic Node failure supplies no early-error credit |
| Confirmation fix-table F1/F2/F3/F5/F6/F7/F8/F9; prior Sol W1/W2/W3 | prior closure statuses retained | fixed /rule changed /recorded as above | F1 member boundary fixed, F2 residue recorded, F3 rules corrected, F6 labelled rows fixed, F8 string/merge fixed; other previously closed mechanisms preserved |

## Product RED/GREEN and bounded mechanisms

Four row regressions fail on exact96817370 and pass on R10: labelled callable-body declarations, complete JSX member tag paths, quoted/escaped/same-scope merged enum members, commented direct eval. Six R10 test groups pass; ERROR-ancestor and bare-enum/asserts existing-arm tests pass the base and guard mutants challenge them. ERROR fixture has clean params but a real ERROR ancestor; ordinary duplicate-arrow negative refuses locals. All applicable JS/TS/TSX, named/synthetic, single/chained labels and nested-block controls are covered. The JSX namespace formal actually collides with svg. Enum keys reuse the existing StringValue decoder; no new decoder/design. Cross-file and namespace merging remain out of model.

Forty-eight complete endpoint/confidence/doubt/kill negative/seam tables are frozen R9 controls. Synthetic labelled expression-parameter seams stay refused; named seams retain main. Both class/receiver and type-predicate R9 mechanisms remain unchanged. Initial Vec type and iterator lifetime compile errors earned no behavioral credit. Two negative expected grades/endpoints and a named-seam expectation were corrected using same-environment frozen base outputs, never by changing production behavior. At the two-attempt cap, the bounded test corrections were disclosed; no artifact restart or broader source fold.

Exact final-tests base export:2 existing-arm groups passed /4 repair groups failed on assertions. The first export omitted23 include_bytes script assets and was inadmissible; exporting the base scripts made the control runnable. See red-final-tests-corrected.log and stderr. Full head suite5247 passed /0 failed /1 reserved skip; doctests2/2. PD109/109 killed and admissible; coupled lane-P P2-M11 1/1 killed, other120 lane-P mutants not rerun. Fmt clean. Clippy head/main emitted warning multisets371/371 equal (229 distinct locations each), same environment, clean local-main export c8de720b36c24ae8a7ab274ec10c994f258eb336. No fresh warning delta. Semantic763/763 and grammar2289/2289, kind200/200, full matrix rows equal frozenR9 (semantic10535 /kind396), DESIGN-CHANGE0. Tier-A matrix178/178 after immediate release rebuild; TS and Node quick both VALID (admission, not precision). Rust/full multi-corpus quick remains controller/human work. Receipts record gate and producer wall times.

## Head-to-head certificate and final tables

Collect-all progress: 586/586 terminal root results. New R10 byte and call-site producers use1500s limits. Complete non-JS output is included in every byte capture. Whole symbol inventory (`functions`) and module graph (`repo-map`) compare new R10 with fresh captures of the frozen R9 binary on every admitted root. Call sites compare the saved R9 captures. X241/T81 navigation selectors additionally compare saved R9 outputs. Inputs are hashed before and after; binary/input/output receipts bind every producer. New auxiliary frozen-R9 navigation producers are identified as fresh, rather than invented historical captures.

| Frozen R10 binary | SHA256 |
|---|---|
| prism-head-r10-bytes | `e3657b3745fc4a8ec161adedb4225490f9415f000c779b069c116f3f37f8f3eb` |
| seam-census | `b91f06e4d1cc43443513d83a611dc2e09f7582eff21eb37028d7aae97951eca6` |
| prism-head-r10 | `93cee446370fa1c5b7fe1b60edd1ce7eec6d86158045ee94d9982b93030463bd` |

| Corpus | Accounted | Admitted | LABELLED_FUNCTION | JSX_MEMBER_TAG | ENUM_MEMBER | EVAL_COMMENT | Total differing rows |
|---|---:|---:|---:|---:|---:|---:|---:|
| X | 1 | 1 | 0 | 0 | 0 | 0 | 0 |
| Xi | 1 | 1 | 0 | 0 | 0 | 0 | 0 |
| T | 1 | 1 | 0 | 0 | 0 | 0 | 0 |
| SecBench | 583 | 576 | 0 | 0 | 0 | 0 | 0 |

**Difference set empty. The R9b certificate against f8c768b3 and Part A tables carry over to R10, adjusted by exactly zero classed rows.** Historical certificate497 differences remain classified (SEAM3,EVAL41,ARGS386,TYPE_PREDICATE62,INADMISSIBLE5); none is new R10 incidence. Call sites, non-JS output and navigation are identical on all admitted roots; X/T sampled selectors also equal. No independent AST class proof was needed because no differing row exists.

| Corpus | Bucket | Raw ADDED C/W/U/I | After ADDED | Raw LOST C/W/U/I | After LOST | Raw RELABELLED C/W/U/I | After RELABELLED | Raw RE-OWNED | After RE-OWNED |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| X | 1 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 |
| X | 2 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 |
| X | 3 | 15510/0/0/0 | 15510/0/0/0 | 0/3415/2/0 | 0/3415/2/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 |
| Xi | 1 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 |
| Xi | 2 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 |
| Xi | 3 | 15510/0/0/0 | 15510/0/0/0 | 0/3415/2/0 | 0/3415/2/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 |
| T | 1 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 |
| T | 2 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 |
| T | 3 | 19434/0/0/0 | 19434/0/0/0 | 0/374016/0/0 | 0/374016/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 |
| SecBench | 1 | 885/84/0/0 | 885/84/0/0 | 11/395/26/0 | 11/395/26/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 |
| SecBench | 2 | 194/0/0/0 | 194/0/0/0 | 0/31/0/0 | 0/31/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 | 0/0/0/0 |
| SecBench | 3 | 287144/0/0/0 | 287144/0/0/0 | 0/947096/18/0 | 0/947096/18/0 | 1/0/1/0 | 1/0/1/0 | 0/0/0/0 | 0/0/0/0 |

Tuples are CORRECT/WRONG/UNDECIDED/INADMISSIBLE; explicit zeros. These are final R10 tables, carried from the corrected 96817370 baseline through complete fresh row equality. All RE-OWNED are zero. Seven exclusions receive no semantic or navigation credit.

## Corrected E13 extent per root

| Root | Raw ADDED | After ADDED | Raw LOST | After LOST |
|---|---:|---:|---:|---:|
| prototype-pollution/aurelia-path_1.1.7 | 0/0/0/0 | 0/0/0/0 | 0/10/0/0 | 0/10/0/0 |
| redos/react-native_0.63.0-rc.0 | 885/84/0/0 | 885/84/0/0 | 11/385/26/0 | 11/385/26/0 |

All other admitted roots have zero bucket1 differing rows, including X/Xi/T. The five Pressability losses are value-flow losses from parser-artefact refusal, separately disclosed from the earlier six type-name artefacts. All84 ADDED WRONG remain bucket1 React Native. The owner decision wording is unchanged, measured extent rewritten and earlier figures marked superseded; E13-owner-wording-check.json proves exact decision-line equality. Owner reaffirmation/revision remains a landing prerequisite.

Bucket2 (outside E13), raw and after identical:

| Root | ADDED C/W/U/I | LOST C/W/U/I |
|---|---:|---:|
| code-injection/kmc_1.2.2 | 4/0/0/0 | 0/0/0/0 |
| command-injection/buns_1.1.6 | 82/0/0/0 | 0/27/0/0 |
| command-injection/google-cloudstorage-commands_0.0.1 | 1/0/0/0 | 0/0/0/0 |
| command-injection/pdfinfojs_0.3.6 | 4/0/0/0 | 0/3/0/0 |
| command-injection/pidusage_1.0.0 | 2/0/0/0 | 0/0/0/0 |
| command-injection/roar-pidusage_1.1.6 | 1/0/0/0 | 0/0/0/0 |
| path-traversal/crud-file-server_0.7.0 | 38/0/0/0 | 0/1/0/0 |
| path-traversal/node-srv_2.0.0 | 5/0/0/0 | 0/0/0/0 |
| prototype-pollution/fabiocaccamo-utils.js_0.17.0 | 25/0/0/0 | 0/0/0/0 |
| prototype-pollution/jquery_1.11.0 | 8/0/0/0 | 0/0/0/0 |
| redos/htmlparser_1.7.7 | 24/0/0/0 | 0/0/0/0 |

## Controller F command

```bash
PACKET="$HOME/code/prism-pd-plan/docs/superpowers/plans/2026-10-06-js-param-defs"
R10="$HOME/prism-evidence/js-param-defs/prB/repair-r10"
TS_JS="$HOME/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js"
SEAM_CENSUS_BIN="$R10/bin/seam-census" \
CORPUS_F_ROOT="$CONTROLLER_PRIVATE_F_ROOT" \
PRIVATE_EVIDENCE_ROOT="$CONTROLLER_NEW_PRIVATE_EVIDENCE_DIR" \
bash "$PACKET/CONTROLLER-pd.sh" diff "$TS_JS" \
  "$HOME/prism-evidence/js-param-defs/prB/bin/prism-base-da0604b3" \
  "$R10/bin/prism-head-r10" \
  "$HOME/prism-evidence/js-param-defs/prB/bin/prism-base-da0604b3-bytes" \
  "$R10/bin/prism-head-r10-bytes"
```

Controller supplies the private root and a NEW evidence directory. Detailed private rows stay private; preserve per-producer timings. Worker/controller classifier parity22/22 and F aggregate consumer controls8/8 pass; no F run or frontend-portal read occurred.

## Controller checks on the committed head (2026-10-10)

Source `wip/js-param-defs-prB-r3` at `fc96688e` (the R10 patch applied to `96817370`; every `src/`, `tests/` and `mutants/` file equals the worker's tree). Docs base `c3ee366b`.

- **Tier-A quick, outside the sandbox, after an immediate release rebuild in the source worktree:** `prism-quick` (rust-analyzer), `excalidraw-ts` and `secbench-node` (tsserver) are all VALID with no invalid reasons. Every metric leaf equals the same-environment run on `96817370`; only commit ids and rust-analyzer session metadata differ. The `96817370` run in turn equals the `2431cb10` run.
- **Private corpus F, frozen R10 binaries, corrected three-bucket rules (aggregates only):** status COMPLETE, no stops.
  - ADDED 9,923 rows, all CORRECT, all under synthetic owners; 6,556 Exact and 3,367 NameOnly (cfg_incomplete 3,211; killed 119; alias_unstable 26; sameline 11).
  - LOST 891 rows, all WRONG on main; 157 of them were Exact.
  - Call sites identical; both byte-to-wire projections identical.
  - 0 type-annotated JavaScript files and 0 files with other TypeScript syntactic diagnostics, so every row is in bucket 3 and E13 does not apply to this corpus. 0 overrides and 0 inadmissible proofs: raw and after-override verdicts are equal.
  - These totals equal the controller's run on `2431cb10`.
- **Opus check of the four R10 hunks:** APPROVE, no MATERIAL finding. B1, B2, B4 and B5 CLOSED; B3 PARTIAL. Four WRONG / IMMATERIAL instances are recorded in SPEC-prB "R10 follow-ups"; one of them (R10-1) was created by the R10 enum hunk. Cache 114 and both mutant registries are consistent.

Not done by the controller: a gpt-6.1-sol review of the R10 delta (the fold-confirmation round was declared as one round; R10 was a disclosed extension checked by Opus only); the full multi-corpus Tier-A run (human-triggered).

## Custody, unverified work and STOP

R10-src.patch is relative to96817370; R10-docs.patch relative to29a1d2d9, Git-diff format. Clean exports have no Git metadata; both git apply --check and full-byte reconstruction receipts pass. Final custody rechecks all live source/docs, patches, frozen binaries, old/new captures and producer output hashes before delivery. Source/binary manifests, producer receipts, checked patches and snapshots preserve custody. Controller owns committing and landing; no merge/adoption.

Not verified: private F; owner E13 reaffirmation; controller Rust/full quick; human-triggered full multi-corpus Tier-A; seven excluded roots (six historical main timeouts and clean-css exit-9); general parse-recovery/grammar completeness, cross-file/namespace enum merging, performance/O1/sample/MCP live behavior, other120 lane-P mutants, F10/F11, post-landing state or adoption. Historical missing producer wall times are not reconstructed. No memory write. This implementation used no delegated agents.

**STOP: none.**

The sole skipped test is `slice_elem_variant_reserved`: SliceElem is reserved (spec §5/§10), with no classifier recovery until a future slice. Cargo manifests/build.rs and compiled callable-observation assets match all 68 source-base blobs; see build-input-custody.json.

Retained exclusions (no semantic or navigation credit):

- command-injection/total.js_3.4.6: historical main producer timeout.
- path-traversal/atropa-ide_0.2.2-2: historical main producer timeout.
- prototype-pollution/swiper_6.5.0: historical main producer timeout.
- prototype-pollution/total.js_3.4.6: historical main producer timeout.
- redos/clean-css_4.1.10: historical main nonzero exit -9.
- redos/natural_5.1.0: historical main producer timeout.
- redos/three_0.122.0: historical main producer timeout.
