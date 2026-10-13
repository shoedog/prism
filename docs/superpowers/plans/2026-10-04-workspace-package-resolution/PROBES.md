> Historical prototype record at 92c1d0bc / packet c89bc5b7. R1 current state, repairs, receipt source limits and scheme/status contract supersede operational claims below; see R1-REPORT, SPEC and VERIFICATION. This history does not certify final R1.

# Probe contract

The census precedes design; results are in CENSUS.md. Each diagnostic's expected observation/falsifier and result belong in PROBE-LOG. Empty populations, compiler/setup errors, source/config/binary drift, partial streams and oracle-schema errors are inadmissible.

`probes/dump-facts.rs` is Cargo-compiled as a temporary example to avoid mismatched extern profiles; the temporary example is removed. `census.cjs TS_JS ROOT BASE_SITES FACTS OUT` binds source and every native read, uses actual ProjectService and each usage's resolution mode, and reports main proof/candidate/installed/scheme outcomes separately. Workspace names are inventory, not resolver causality.

`compare.cjs TS_JS ROOT BASE_SITES HEAD_SITES HEAD_FACTS OUT` validates complete populations and metadata, then certifies every changed row with the writer's checker and module/owner/span. Its independent accepted React memo/forwardRef terminal check follows the existing static-binding model; an unsupported terminal is UNPROVEN, not guessed correct. Every base target must be retained.

`controls.py NEW_DIRECTORY` retains synthetic linked/uninstalled/declaration/metadata/order/mode/subpath cases in both writer grammars. Native expected module targets are checked independently even for unchanged/refused rows; changed positives also need callable/ownership certification. Scope coverage is bounded, not exhaustive TS conformance.

Controller F execution is CONTROLLER-pkg.sh BASE_BIN HEAD_BIN BASE_FACTS_BIN HEAD_FACTS_BIN TS_JS with CORPUS_F_ROOT and a new PRIVATE_EVIDENCE_ROOT supplied privately. It validates manifest binary/probe hashes and emits only fixed aggregate counts. Do not execute it on F here. Syntax-check locally and exercise the underlying checker on public controls.


Final reproduction:

```bash
python3 probes/measure.py BASE_BIN HEAD_BIN HEAD_FACTS TS_JS NEW_OUT --public
python3 probes/controls.py NEW_CONTROLS
python3 probes/measure.py BASE_BIN HEAD_BIN HEAD_FACTS TS_JS NEW_OUT --controls NEW_CONTROLS/manifest.json
python3 probes/s2-measure.py MAIN_ROWS_ROOT CENSUS_ROOT S2_BASE S2_PKG BASE_FACTS PKG_FACTS TS_JS NEW_OUT
```

Run from packet directory or prefix probe paths. Public refresh can optionally use `--reuse-base PREVIOUS_PUBLIC_DIRECTORY`: every original census read is rehashed, retained base streams are copied with hashes, and head/facts/native checker run fresh. Fresh reproduction needs no old output.

S2 source/helper overlay is scratch-only. s2-census.cjs is copied from the S2 packet oracle; s2-compare.py extracts its complete-row comparator without private defaults. s2-measure.py fixes roots to public X/installed-X, enables the parent diagnostic flag, compares full populations, records actual native ownership, and deduplicates repeated writer/spec/member joins. It stores both the last observed gain-class set and conservative union across graph-build emissions. Diagnostic risk counts overlap; complete changed-row output is the actual gain measurement. Trace events lack graph IDs, so no isolated causal credit or unique final-graph binding is inferred. Reference class weights 3/52/34/43 (132) remain inherited from supplied R3b evidence.
