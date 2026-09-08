# S2 — module-owned boundary observations

The schema13 candidate adds request and boundary-event provenance without changing
the public compiler outcomes or any closure decision. The disposable candidate
reproduces as valid/unproven; runtime and class authority remain false.

| Fixed public population | Count |
|---|---:|
| Module callback occurrences | 6,893 |
| Boundary encounters | 2,075 |
| Refused, all module-owned | 1,269 |
| Outside, module-owned | 448 |
| Outside, not attributed in this slice | 358 |
| Identical ordered basic-host operations, base/candidate | 91,409 |

The basic-host transcript SHA256 is identical in both runs:
`fc4cfa95458d5521bab94c2e9bcf809c674e67acd5e62b119fd4a07a5b3d2eca`.
Every previous packet field except schema/producer compares equal. Matching final
targets alone would not establish this callback/search equivalence. New request
anchors reuse the existing observer's exact records rather than reading sources
again during the resolver callback.

The358 unowned public events are the source-backed type/lib population from S1:
30 type boundary encounters and328 lib boundary encounters, not distinct search
executions. S3-T and D1-L will attribute them separately
while preserving their distinct cache behavior. No classification clears a refusal,
proves outside absence, installs dependencies or admits a runtime edge. react-scripts
and the14 module-literal gaps remain unchanged. No receiver recall gain is claimed.

## RED, controls and review findings

The initial new-channel test fails against exact S1 with a missing-ledger assertion.
Expanded producer controls fail13/13 at that assertion against the same base and
pass13/13 on the candidate. The expanded population covers shared outside paths,
duplicate anchors, virtual refusal, synthetic JSX, source/configured type and lib
non-ownership, resolved-package peer probes, Unicode spans, parser rejection,
genuine owner/digest swaps, event omission, source/config epoch changes and legacy
packet compatibility. Helper controls separately cover caps and context restoration;
these new-helper tests are not claimed as production RED.

Final review requested positive non-null mode coverage. Two additional NodeNext
import/require controls fail on exact S1 at the missing-ledger assertion and pass
on the candidate; the final expanded file passes15/15. The complete disposable
observer suite passed393/393 before those two additions; the final repository
suite must therefore cover395 tests. Do not confuse that interim total with final gates.

WRONG found and corrected during candidate review: the schema migration's source-
reference version guard omitted historical schema12. The genuine PR280 packet
parsed on exact base, failed on the initial candidate and parses after correction.
The old refusal is preserved; historical packets still cannot validate as current.

One expanded JSX fixture incorrectly assumed the first synthetic request came from
the .tsx file. Same-environment base and candidate both synthesize requests for
app.ts and view.tsx under that configuration. The fixture now selects the intended
source occurrence. Its initial12/13 candidate result was a fixture failure, not a
production regression; the corrected control passes13/13.

Final independent review ACCEPT: WRONG0, two SMELLs. Positive mode coverage was
added as above. Out-of-root resolved-target projection is helper-tested and its
worker placement statically reviewed; no real worker fixture produces that target
through the refusing host, so that integration case is explicitly not claimed.
The independent reviewer also matched all6,893 request records and2,075 events in
order against S1's diagnostic capture, including anchors/owners/digests/operations.
Two review rounds converged, no open-class issue or review-cap extension.
All1,229 public source files were freshly rechecked unchanged.

Full repository gates and publication are pending. The [contract](../../superpowers/specs/2026-09-08-callable-module-search-provenance.md)
and active [handoff](../../superpowers/handoffs/2026-09-08-callable-autonomous-sequence.md)
define scope. S1's diagnostic capture/audit remains pinned to its exact worker base;
use that checkout for historical raw-capture replay, not a newer changed worker.
