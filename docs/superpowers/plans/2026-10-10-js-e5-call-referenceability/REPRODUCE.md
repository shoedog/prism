# Public replay and bindings

All captures are static. Never load/execute corpus packages, use providers/network, access a frontend-portal directory, or write Git state. Every CLI invocation is bound to `/Users/wesleyjinks/prism-evidence/js-e5/cache`; direct CPG/fact helper APIs use no disk cache. The controller script's explicit private mode is for the controller only.

The evidence root is `/Users/wesleyjinks/prism-evidence/js-e5/step0`; the packet probes are adjacent to this document. `base/binding.json` and `head/binding.json` freeze the actual CLI bytes and capture selection. `final-product-binding.json` freezes all final product/test/pin sources and immutable helper binaries. Base is the exact40ad8390 product, not a later main. Source archives and build logs retain base/head/helper and experimental construction. `probe-build-sources.tar.gz` and `probe-format-equivalence.json` preserve both packet Rust probes and their rustfmt-equivalent compiled example copies.

The Rust helper sources are intended to be copied to `examples/e5_call_facts.rs`, `examples/e5_graph_dump.rs`, and (for PR-B normalized language controls) `examples/e5_byte_dump.rs` on separate base/head archives, compiled offline there, and copied to immutable evidence binaries. They are measurement tools, not production changes. Graph output contains every Call/Return/DataFlow/ReturnFlow/Input/Contains row plus a sorted serialized non-call-node/edge digest. There is no projection of resolution changes into graph changes.

To replay against the already frozen binaries, use a new public output directory:

```bash
python3 probes/capture.py /Users/wesleyjinks/prism-evidence/js-e5/step0/bin/base-prism NEW_BASE_OUTPUT --jobs 3 --timeout 1500
python3 probes/capture-facts.py NEW_BASE_OUTPUT /Users/wesleyjinks/prism-evidence/js-e5/step0/bin/base-facts-v2
python3 probes/census-all.py NEW_BASE_OUTPUT /Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js
python3 probes/capture.py /Users/wesleyjinks/prism-evidence/js-e5/step0/bin/head-prism NEW_HEAD_OUTPUT --paired-base NEW_BASE_OUTPUT --jobs 3 --timeout 1500
python3 probes/census-all.py NEW_HEAD_OUTPUT /Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js --facts-dir NEW_BASE_OUTPUT
python3 probes/compare-all.py NEW_BASE_OUTPUT NEW_HEAD_OUTPUT NEW_COMPARISON /Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js
python3 probes/paired-graphs.py /Users/wesleyjinks/prism-evidence/js-e5/step0/bin/base-graph /Users/wesleyjinks/prism-evidence/js-e5/step0/bin/head-graph NEW_HEAD_OUTPUT NEW_GRAPHS --jobs 3
python3 probes/adjudicate-all.py NEW_GRAPHS NEW_COMPARISON /Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js
```

`capture.py` enumerates X/T and **all583** manifest-ok SecBench roots, admits completed JSONL artifacts, and records each actual producer wall time/reason. Check admitted counts, not just the driver exit code. `census-all.py` counts the JS caller denominator separately from the producer's all-language total and writes zero-row census files explicitly. `compare-all.py` adjudicates every changed target, including new rungs; an unavailable changed-target oracle stops admission. `paired-graphs.py` compares full serialized multisets. `adjudicate-all.py` admits every differing graph row with the primary callee proof plus PR-B physical binding/owner/read-role/timing semantics. Copied PR-B helpers remain secondary; their generic failure does not override primary E5 verdicts.

`report-e5.py EVIDENCE PACKET` verifies complete graph/adjudication receipts, renders all cross-tabs and samples, and pairs label halves into RELABELLED rows while retaining the raw base/head verdicts. `navigation-check.py` admits whole JSON query output and binds every lost relationship/dependency or added unresolved placeholder to a measured target refusal. `variant-navigation.py` exercises real sampled caller/callee queries for each experimental rule and corpus. Experimental selection JSON and source/binary bindings live in `step0/variants`; repaired R-inferred retains its original rejected-scope artifact and measurements.

Run synthetic native/controller checks with:

```bash
node --test probes/census-e5.test.cjs probes/controller-parity.test.cjs
```

The current full-suite command is `cargo nextest run --offline --all-targets --features mcp`; it selects35 binaries/5293 tests, including34 example tests beyond the prior matched30-binary runs. Result:5292 passed, one inherited example failure, one skip. CI doctests ran separately. Exact hook commands and failure controls are in root VERIFICATION.md. Tier-A used installed `/Users/wesleyjinks/code/prism-pd-plan/eval/.venv/bin/python`, `PYTHONPATH` pointing to this repo's eval directory, `E5_SUT_BIN` pointing to the immediately rebuilt immutable head, and `--sut-bin probes/prism-bound.py`. Matrix arguments: `--matrix-only --allow-stale-sut`. Authorized quick arguments: `--quick --lang ts,js --allow-stale-sut`. The adapter binds every SUT command to the required cache, replacing harness no-cache defaults. Exact outcomes and exclusions are in VALIDATION.md.

The final fresh quick controls are stronger than the earlier frozen-base comparison: step0/fresh-quick-control.py snapshots every owned product/test/pin byte, restores exact tracked40ad sources without Git writes, builds/runs base quick, restores all prototype bytes in a finally block, then builds/runs head quick. The restoration/hash/build receipt is fresh-quick-control.receipt.json; both sides are VALID and all measured quick metrics agree. The source backup is durable in fresh-control-source-backup.
