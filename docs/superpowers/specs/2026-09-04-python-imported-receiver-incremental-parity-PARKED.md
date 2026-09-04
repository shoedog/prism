# Python imported receiver incremental parity — PARKED successor

**Status:** PARKED; requires explicit owner authorization
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

The parent slice reached its declared two-round cap with a second proof-lifecycle defect. This successor is therefore parked for owner authorization. If authorized, make the bounded guard on the existing branch/artifact, run one explicitly declared review round, and do not broaden into general Python incremental invalidation.
