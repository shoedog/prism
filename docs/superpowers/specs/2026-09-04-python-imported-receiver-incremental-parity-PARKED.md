# Python imported receiver incremental parity — authorized successor

**Status:** implemented and locally accepted at `a48c3db`; publication not yet authorized
**Recorded:** 2026-09-04
**Parent slice:** `py-imported-receiver-owner`, focused-green implementation `d896430`
**Scope:** reclassify unchanged Python call sites only when imported-class proof changes across an incremental rebuild

## 1. Confirmed WRONG

Given unchanged `app.py`:

```python
from pkg.models import Client

def run(client: Client):
    client.send()
```

and a change only to `pkg/models.py` from no clean `Client` declaration to:

```python
class Client:
    def send(self):
        pass
```

a fresh build records `receiver_type = Some("Client")` and resolves one Exact `TypedParam` target in `pkg/models.py`. The incremental build leaves the unchanged call site at `receiver_type = None`. The focused parity regression demonstrates `left: None`, `right: Some("Client")`.

The mechanism is bounded: incremental construction removes and rebuilds facts for changed files, then reconstructs selected whole-program indexes, but does not reclassify call sites in unchanged Python files after the imported-class proof set changes.

## 2. Successor decision

Keep the existing artifact and add a proof-set mismatch guard; do not restart the slice.

1. Before incremental mutation, derive the set of proven imported-class routes from the old `CallGraph` using the same `python_imported_class_route` authority as full construction.
2. After changed-file facts have been merged and whole-program Python ownership indexes are coherent, derive the new set using the same helper.
3. If the sets differ, fall back to a full `CodePropertyGraph` build from the supplied complete file map so unchanged Python call sites are reclassified.
4. If the sets are equal, retain the existing incremental path. Direct-method additions/removals do not require call-site reclassification when class proof is unchanged because resolution consults the rebuilt method indexes at query time.

The compared key must include caller file, local imported spelling, defining file, and original class owner. It must not compare only class names or only defining files.

## 3. Required RED/GREEN evidence

- Existing RED: proof absent to proof present in a changed defining file, with the caller unchanged; incremental must equal fresh build for receiver state and Exact resolution.
- Inverse edge: proof present to proof absent, with the caller unchanged; incremental must equal fresh build and must not retain imported Exact authority.
- Stable-proof control: changing only a direct method body or unrelated Python declaration must preserve full/incremental parity without requiring proof-set churn.
- Existing changed-caller and file-add/delete behavior remains green.

The first two tests must fail on `d896430`. Production changes require the normal full suite and Tier-A gates from the parent design.

## 4. Convergence boundary

The parent slice reached its declared two-round cap with a second proof-lifecycle defect. The owner authorized this bounded successor on 2026-09-04. It was implemented on the existing artifact without broadening into general Python incremental invalidation.

The successor's single declared review round found no WRONG. One test-fixture SMELL was corrected: Rust string-continuation escapes had stripped indentation from the stable-key Python fixture. Focused verification after correction passed all three incremental parity cases and the stable-key unit control.

## 5. Acceptance evidence

- Absent-to-present and present-to-absent parity tests both failed on the pre-fix code with the predicted stale receiver states, then passed with the mismatch guard.
- Stable proof-key unit control and stable-proof full/incremental integration control passed.
- Full default suite: `3,551 passed, 0 failed, 1 ignored` across 28 summaries.
- Full `mcp` suite: `3,737 passed, 0 failed, 1 ignored` across 30 summaries.
- Formatting, `git diff --check`, `cargo check --all-targets`, and configured Clippy passed; Clippy reported only the existing repository warning population and no warning at the successor changes.
- Two immediate release rebuilds passed. Tier-A matrix-only reported 104/104 `ok`.
- Tier-A quick reported 104/104 `ok`, oracle error `0.0`, SUT error `0.0`, and `oracle_not_quiescent = false`. Its exit 2 was solely `corpus_sha_drift: a48c3db2dc78 != pinned 20c8490591a3`; no baseline was updated.
