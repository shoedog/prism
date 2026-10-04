# S2 verification — owner fail-closed repair

Planning HEAD: `6e4e0ef19de578d4865d7297eb5cf72a5b6a27a6`.
Immutable product base: `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`.
The final source, test and frozen-tool hashes are in `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/BUILD-MANIFEST.json`.

## Verified

The project's full MCP suite ran unfiltered after the last semantic edit: **5,154 passed, zero failed, one existing ignored test**, 170.696 seconds. MCP doctests add **2 passed**. No unrelated suite failure occurred; nothing was re-baselined or silently repaired. A fresh hash audit confirms all **770 recorded source/test inputs remain unchanged** since that run. The stop-hook audit added the missing namespace receiver regression and repaired its carrier mapping before this full run. All subsequent edits are documentation only; previous final suites are superseded for these bytes.

Commands below ran from `/Users/wesleyjinks/code/prism-s2-plan`, except the matrix command, which ran from its `eval` directory. Receipts are under `target/s2-plan/`. Redirections shown are the actual receipt destinations.

| Exact command | Pass totals / result |
|---|---|
| `cargo nextest run --offline --features mcp > target/s2-plan/namespace-final-nextest.log 2>&1` | **5,154 passed /0 failed /1 existing skipped** |
| `cargo test --offline --features mcp --doc > target/s2-plan/namespace-final-doctests.log 2>&1` | **2 passed /0 failed** |
| `cargo test --offline --test integration js_import_qualifiers_test > target/s2-plan/namespace-whitelist-targeted.log 2>&1` | **17 passed /0 failed**; aggregates the complete escape population before asserting |
| `cargo fmt --all -- --check > target/s2-plan/namespace-final-fmt.log 2>&1` | PASS |
| `cargo clippy --offline --features mcp --all-targets --message-format=json > target/s2-plan/namespace-final-clippy.jsonl 2> target/s2-plan/namespace-final-clippy.stderr` | PASS; **371 warning instances, zero new** against the same-environment control |
| `CARGO_TARGET_DIR=/Users/wesleyjinks/code/prism-s2-plan/target cargo clippy --offline --features mcp --all-targets --message-format=json --manifest-path target/s2-plan/failclosed-base-clippy-source/Cargo.toml > target/s2-plan/failclosed-clippy-base.jsonl 2> target/s2-plan/failclosed-clippy-base.stderr` | Pre-change main control: PASS; **371 warning instances**; exact warning multiset agrees after normalizing paths and shifted line offsets, retaining exact primary source text |
| `CARGO_NET_OFFLINE=true python3 scripts/mutgate/mutgate.py --lane mutants/lane-s2-import-qualifiers.json --since 6e4e0ef1 --scope file --jobs 1 --out target/s2-plan/namespace-final-mutgate > target/s2-plan/namespace-final-mutgate.log 2>&1` | ADVISORY: **8/14 selected /8 admissible /7 killed**; S2-14 whitelist KILLED; S2-02 survives, exit 1 |
| `cargo build --offline --release > ../target/s2-plan/namespace-final-matrix-build.log 2>&1`<br>`PYTHONPATH=. /Users/wesleyjinks/.local/share/uv/python/cpython-3.12.13-macos-aarch64-none/bin/python3.12 -m tier_a.cli --matrix-only --allow-stale-sut --sut-bin ../target/release/prism > ../target/s2-plan/namespace-final-matrix.log 2>&1` | From `eval`: **182 OK /0 regression /0 skip**, immediate preceding release rebuild |
| `python3 docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/e5-alias-boundary.py target/s2-plan/bin/head-prism target/s2-plan/bin/head-dump_imports target/s2-plan/namespace-e5-green > target/s2-plan/namespace-e5-green.log 2>&1` | **218 scenarios /218 keep base /0 wrong admissions** |
| `python3 docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/e5-alias-boundary.py target/s2-plan/bin/pre-whitelist-prism target/s2-plan/bin/pre-whitelist-dump_imports target/s2-plan/namespace-e5-red > target/s2-plan/namespace-e5-red.log 2>&1` | Same-environment pre-change prototype: **196/218 scenarios RED**, including **195 statically CORRECT wrong changed rows**; exit 1 is expected |
| `python3 docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/prototype-controls.py target/s2-plan/bin/head-prism target/s2-plan/bin/head-dump_imports target/s2-plan/namespace-final-native-controls > target/s2-plan/namespace-final-native-controls.log 2>&1` | **92 scenarios /27 new CORRECT /0 loss /0 unproven** against actual immutable main |
| `python3 docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/compare-head.py target/s2-plan/bin/head-prism target/s2-plan/bin/head-dump_imports target/s2-plan/namespace-final-public --reuse-base target/s2-plan/final-failclosed-public --jobs 4 > target/s2-plan/namespace-final-public.log 2>&1` | **+132/+132/0/0** on X/installed-X/R/T; all changed rows CORRECT with module/owner/full-span agreement, no changed populated base rows |
| `python3 docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/s1b-parity.py target/s2-plan/bin/head-prism target/s2-plan/namespace-final-s1b.json > target/s2-plan/namespace-final-s1b.log 2>&1` | **411 controls /639 sites /822 byte-identical comparisons /stderr zero** |

The Clippy control source was captured with `mkdir -p target/s2-plan/failclosed-base-clippy-source && git archive HEAD | tar -x -C target/s2-plan/failclosed-base-clippy-source`. HEAD's product source equals immutable main; only its docs differ. No Git write occurred. Generated Cargo targets/control source were removed after verified snapshots to keep disk lean; frozen base/head tools and all receipts remain.

### Failing-before / passing-after coverage

The coverage audit joins old/head records by case and grammar. All **43 newly refused use families** have at least one old-prototype RED control and a repaired-head GREEN control; both grammars have complete head controls. Every new productive mechanism also has an actual-main base/head control, with negative or edge cases. Controls already handled by pre-change code remain preservation tests and are not claimed as new failures.

| Behavior / path | Regression and negative or edge evidence |
|---|---|
| Class static method/field, named/default/paths imports | `class_statics_and_fields_are_new_exact_in_both_grammars`, native class/static-field/default/paths cases; actual main drops positives. Instance, decorator, duplicate/computed key and declaration-priority negatives keep base. |
| Declared namespace, re-exported namespace and constant object members | Declared namespace/reexport/object integration tests and native main/head positives; duplicate/merged namespace, bodyless/wrapped/unspanned members, accessors, spreads and unknown object values refuse. |
| Closed value-use whitelist | `qualifier_identity_whitelist_refuses_all_value_escapes`; 218 source-bound E5 controls across provider/caller/importer/forwarder. S2-W1, aliases/destructuring, arguments/returns, stored/spread values, reflection/computed accesses and renamed/default-plus-use exports are RED on old prototype and GREEN on head. S2-14 removing the predicate is KILLED. |
| Cross-file and unproved writer identity | `qualifier_whitelist_namespace_paths_reexports_and_this_keep_base`; separate importers, namespace paths, source re-exports and writer with unproved owner have old/head controls. Alias, object-this and class-self member-write/read edges are included. |
| Implicit receiver carriers | `namespace_this_uses_require_the_same_closed_whitelist`: six refusal shapes × three valid grammar/carrier combinations are RED before /GREEN after, plus three positive literal direct-call controls. Declared namespaces in JSX are invalid-syntax negatives. `stop-hook-namespace-this` binds actual main/pre-namespace/repaired rows and a same-object runtime mutation. Unknown lexical ownership uses the existing module-namespace identity marker and refuses closure. |
| Dynamic namespace property/destructuring tokens | Dynamic property, renamed destructuring and shorthand-destructuring cases are RED on pre-change prototype in both grammars and GREEN on head; `dynamic-identity-red/summary.json` also pins the intermediate gap. |
| Permitted declaration/call/new/type/own-export contexts | `qualifier_whitelist_allows_declarations_calls_new_types_and_own_export`; actual-main S2 positives fail before feature admission. Intermediate TSX `pattern` declaration and opaque-unrelated-namespace regressions are recorded in `whitelist-positive-red.log` and matrix/facts controls, then GREEN in `namespace-whitelist-targeted.log`. Shadow writes retain base conservatively. |
| Whole populated-base fence and landed projections | `any_already_bound_row_is_byte_preserved` (including a multi-target row); S2-01 fence mutant KILLED. Every populated public main row and all prior lane-P rows remain unchanged; `namespace-final-lane-p.json` includes 3,129 prior gain rows per X snapshot. |
| Cache semantic invalidation | CPG/navigation version-pinning tests require **108/64** and fail at pre-whitelist **107/63**. Existing round-trip/corruption/invalidation tests ran in the full suite. |
| Controller head comparison | Public positive/refusal wrapper controls certify **1 CORRECT /0 changed**, fixed aggregate schema and full-row module/owner/span checks. `namespace-controller-public-selftest.json` is public fixture evidence; it is not an F result. |

The two Tier-A alias-write fixtures are RED on the frozen pre-change prototype and GREEN on actual main and repaired head in both grammars; `namespace-final-tier-a-red-green.json` records the full outcomes. Existing Tier-A baselines are unchanged. A first namespace filter used an absent field and was inadmissible; the corrected full rows establish the failure. A matrix log redirection used the wrong cwd and yielded no matrix evidence; the listed correct eval command supplied the gate. The earlier same-name arrow replacement did not discriminate the intended seed, so that probe was classified inadmissible and replaced by a distinct named replacement before claiming RED/GREEN evidence.

**132 of the old 179 X rows survive**. The 47 refusals have source witnesses and actual head facts: **25 member writes, 11 member-value/chained reads (including conservative this), 11 namespace argument escapes**. X snapshots are not additive.

## Not verified

- The existing ignored `resolution_test::slice_elem_variant_reserved` was not executed; it is reserved for a future SliceElem increment. All other tests in the requested full MCP run executed. No out-of-scope suite failure was found or excluded.
- **SMELL:** S2-02-position-proof survives its advisory selector. This is a coverage gap, not demonstrated wrong output and not proved equivalent. Six new-source anchors are omitted by scoped selection; authoritative all-14 mutation coverage remains a controller gate after commit.
- Private **F** head acceptance, independent review/adoption and O1/O2 owner confirmation of controller interim adoption positions remain controller-owned. The planner never opened F or wrote Git.
- Tier-A quick remains required before independent review; full multi-corpus Tier-A remains human-triggered. Neither ran here.
- Optional detached-owner/all-feature sweeps and Linux/case-sensitive/concurrent-tree/performance checks were not run.

The full MCP suite was runnable and completed; no environmental limitation prevented it. No installs, pushes, merges or publication occurred. Current packet details are in MEASUREMENTS/HANDOFF/FILES, with local source/evidence snapshots and frozen tool hashes in BUILD-MANIFEST and owner-final-custody.json. `VERIFICATION.md` is locally excluded by `.git/info/exclude` and retained in the source snapshot; the exclusion is unchanged.
