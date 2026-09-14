# Nested-callable execution-owner proof

Verdict: **READY_FOR_BOUNDED_REPAIR**. This proof does not implement that repair.
Production collector bodies, occurrence indexing, parameter admission, call/receiver
ownership, public signatures and caches are unchanged. CPG cache remains 92 and
navigation call cache remains 52.

## Bound identity and custody

The proof ran in `/Users/wesleyjinks/code/slicing` on branch
`feat/js-ts-module-binding-audit`, HEAD
`36aec5f7a16e9eb90002f01235365cdc4ab6395a`, tree
`2ac20fcbf743d54c508f914d685936285f30a549`. That tree equals the supplied
verified main tree at `c1dbc292175f8e93271e7a3699b2ee20fa41e42f`; stale local
main refs were not used. Evidence is under
`/private/tmp/prism-nested-owner-proof-Rjzipk`. Test and executable hashes are in
the adjacent baseline receipt. No commit, fetch, push, rebaseline or production
publication was performed.

## Findings

**WRONG — outer callable rvalue queries emit nested execution-region tokens.** For
the fixed one-line assignment input, querying `outer` returns the nested function
name `inner` at bytes 40–45, nested parameter `nestedParam` at 46–57, and nested
body reads at 59–101 through span query and private manual-span routes. Name and
path routes return the same spellings without byte identity. Call and returned
families retain the eager sibling but also emit the delayed nested tokens. The
same fixed vectors occur in JavaScript, TypeScript and TSX. The aggregate desired
test records 387 consumer/language/multiplicity mismatch rows and then fails: 369
from the four route families plus 18 nested-default rows (six consumers in each
language). These are repeated observations, not 387 unique sites. Its separate
outer-only preservation oracle pins all eight required name/path/span rows for
`outerRead` and returned `kept`, so deleting valid outer reads cannot make it green.

The mechanism is two-part. `rvalue_capture_is_contained` accepts descendants
because byte containment only excludes ancestors. Once a callable-valued capture
is accepted, `collect_identifier_*` recursively crosses its parameter and body
boundaries. The independently authored `TokenAnchor` oracle resolves exact UTF-8
ranges and validates concrete AST parent/field relationships; it does not call the
production collectors or the proposed pruning algorithm to create expectations.

**WRONG — exact call resolution assigns one nested call to two callers.** In source
epochs 1–3, the only `item(...)` call is inside `inner`, yet exact call resolution
returns both `inner` and `outer` relations for the identical byte span (55–66,
61–72 and 60–71 respectively). The proof pins those complete tuples. This is the
separate parked call-ownership defect; the bounded rvalue repair must not change it.

No stronger DFG or FullFlow claim follows automatically. The representative DFG contains
the nested parameter/body Uses under `outer`, including exact duplicate
multiplicity, but the free `seed` capture retains two
`NameOnly(CfgIncomplete)` labels and no `Exact` label. Wrong-token and out-of-span
argument substitutions are rejected; the genuine same-line collision refuses.
An effectful default retains positional signature shape as `[None]` and does not
gain an argument Def. FullFlow is unchanged with and without DFG:
`[app.js:3:true, app.js:5:false]`; the nested body line is absent. Therefore this
proof demonstrates a source-extractor WRONG and a separate exact call-ownership
WRONG, but not a wrong exact DFG edge or public taint result.

**SMELL — phase-sensitive class expressions need their own policy proof.** Root
inventory observed heritage, computed class key, static field, instance field and
static-block expressions. That inventory is not an execution-timing theorem, so
the first repair excludes class policy rather than applying a blanket subtree
prune.

**SMELL — unindexed/recovery callables cannot inherit ownership by name/line.** An
anonymous `generator_function` is a callable boundary but absent from
`all_functions`; two same-name same-line functions have distinct byte ranges; a
recovery fixture has parse errors. The bounded repair must derive ownership from
the queried tree node/ranges or refuse. Absence from an inventory never grants an
ancestor ownership.

## Fixed proof population

The canonical raw vector has 143 rows per language: assignment 46, initializer
16, call 58 and returned 23. Each route's complete JS vector is byte-equal to TS
and TSX and is pinned by SHA-256 in the baseline receipt (429 repeated language
observations). The exact eight-row outer-only preservation vector and positive
fixtures have zero parse errors. Separate controls cover
own nested-body reads, eager call siblings, a computed object-method key, root
inventory, erased TS types, augmented LHS, Unicode, callable kinds, effectful
defaults, optional/default/rest/destructured signatures, an unparenthesized arrow
occurrence gap, same-name/same-line ranges and recovery.

The graph epoch sequence is immediate outer read → nested body → nested body plus
eager sibling → shadowed nested parameter → restored source. Exact full and
incremental DFG Use/edge rows, multiplicity and call-resolution tuples agree after
a bincode cache round trip at every epoch; DFG row counts are 5, 10, 14, 11 and 5.
Epochs 1–3 deliberately retain the duplicate exact `inner`/`outer` caller tuples as
a parked refusal control. Only `app.js` is changed and the imported callee is not.

## Hypothesis–probe–result log

| Hypothesis | Expected / falsifier / alternative | Result |
|---|---|---|
| Source identity is the pinned main tree | expected equal tree; falsifier unequal tree; alternative stale refs | HEAD tree exactly matched the supplied main tree |
| Containment admits nested captures | expected accepted nested ranges; falsifier no nested spans; alternative recursive descent alone | accepted callable descendants and wrong spans observed |
| Recursion crosses callable boundaries | expected parameter/body children after accepted capture; falsifier capture only; alternative query-pattern overmatch | all query/manual names, paths and spans cross the boundary |
| Raw WRONG creates an exact DFG/public failure | expected wrong exact DFG edge/FullFlow line; falsifier conservative/refused/unchanged output; alternative separate capture path | falsified: no wrong exact DFG edge or public FullFlow result demonstrated; exact call ownership is separately wrong |
| Capture doubt survives | expected NameOnly labels and fallback controls; falsifier Exact upgrade/loss; alternative raw span as sole source | two NameOnly(CfgIncomplete), no Exact; all four zero-width fixture occurrences pinned |
| Incremental state can retain stale ownership | expected full/incremental mismatch; falsifier exact semantic equality; alternative count-only coincidence | falsified at all five epochs with full tuples and resolution |
| Gate failure is caused by this slice | expected base green/candidate red; falsifier identical base failure; alternative missing external compiler | owner-audit base and candidate fail the identical 21+2 NotPresent tests |

Compilation failures, zero-selection probes, a malformed counterfactual and the
initial sandbox-denied `uv`/pytest invocations were treated as inadmissible. The
helper's retained behavioral RED selected one test and showed an incorrect second
occurrence resolving to the first; the repaired helper test is green. The desired
production RED selected one ignored test and fails only after printing every row.
Direct raw-span removal at private `cpg::reaching::scope::use_byte` is not proved:
the plan permits no production-adjacent test seam, so this remains an explicit
repair-slice test gap rather than an inferred guarantee.

## Bounded repair contract

The next slice may add private, rvalue-specific scope plumbing in `src/ast.rs`:

```rust
enum RvalueQueryScope {
    LegacyInventory,
    CallableBody {
        owner_start_byte: usize,
        owner_end_byte: usize,
        body_start_byte: usize,
        body_end_byte: usize,
    },
    Unresolved { reason: &'static str },
}

enum OwnerDecision { Owned, Nested, Signature, Outside, Unsupported }

fn rvalue_capture_owner(
    &self,
    scope: &RvalueQueryScope,
    capture: Node<'_>,
) -> OwnerDecision;

fn collect_identifier_path_spans_scoped(
    &self,
    node: Node<'_>,
    scope: &RvalueQueryScope,
    out: &mut Vec<PathSpan>,
);
```

Equivalent scoped walkers retain the existing `Vec<(AccessPath, usize)>` and
`Vec<(String, usize)>` outputs. They are private implementation seams, not new
public API. `rvalue_capture_owner` governs capture acceptance, while each scoped
walker independently enforces recursion barriers; one cannot substitute for the
other. General-purpose `collect_identifier_*` helpers remain intact until their
other consumers are separately proved compatible.

| Queried context / visited region | First-repair policy |
|---|---|
| Whole-file root | `LegacyInventory`; preserve nested inventory exactly |
| Supported callable own body | `Owned`; preserve existing rvalues and bytes |
| Own parameter declaration | `Signature`; not an ordinary rvalue |
| Own default initializer | separate parameter environment; no ordinary body read or argument edge admission |
| Nested callable name/parameters/body/default | `Nested` or `Signature`; exclude from the outer result; its own query still works |
| Eager sibling around a callback | `Outside` relative to callback but owned by outer expression; retain |
| Computed object-method key | visit key in the enclosing eager context before refusing method parameters/body |
| Erased type | preserve existing erased-value refusal |
| Unindexed/recovery/ambiguous callable | `Unsupported`; refuse ownership-sensitive collection, never guess by name/line |
| Class heritage/computed/static/instance regions | excluded pending phase-specific proof |

The first production edit is limited to the three rvalue query routes, their
private manual fallbacks and return-span recursion in `src/ast.rs`. It must keep
the names/paths versus span-only return asymmetry. It must not change
`compute_param_def_nodes`, `argument_var_node_in_span`, occurrence indexing,
parameter syntax admission, call/receiver resolution, public signatures, or the
separate scoped-reference/capture pathway. The enabled repair tests must assert
that capture doubt is retained and never promoted to Exact by deleting a warning
source.

Because stored DFG semantics will change, the repair slice must review and bump
CPG cache 92→93. Navigation cache stays 52 unless its serialized facts change.
Enable `ast::nested_execution_owner_tests::nested_execution_owner_desired_contract`
and update the historical six-row namespace audit only when the production repair
lands. Genuine same-line occurrence identity; optional/default/rest/destructured
and unparenthesized-arrow admission; class phases; IIFE/higher-order resolution;
and broader member shapes remain separate work.

## Verification and limits

Focused frozen audit: 7 passed, 0 failed, 1 desired ignore. Revised desired RED:
0 passed, 1 failed after 387 rows on both unchanged base and candidate; normalized
streams are identical at SHA-256 `b96b7274a05149039d85672a3fac343ddc84838220fa74f1756c5cf17f83320b`.
Namespace-flow 5 passed; contained-rvalue 3 passed. Full Rust:
default 4,508 passed/0 failed/2 ignored; MCP 4,701/0/2. The widest owner-audit
configuration has 4,701 passed/23 failed/2 ignored: the unchanged base reproduces
the same 21 library plus 2 integration `NotPresent` compiler refusals. Examples:
32 passed. Formatting, clippy and diff-check completed; repository lint warnings
remain outside this proof scope.

Node: 786 passed across 46 runnable modules at concurrency 2. One module with
three source-custody tests is excluded because its five pinned inputs, including
`/private/tmp/prism-imported-alias-O4d6E1/real-sites.jsonl`, are unavailable.
Authority: 40 passed. Python: initial sandbox configuration was inadmissible
because `pytest-rerunfailures` could not bind localhost; the one environmental
retry outside that restriction passed 940 with one explicit live-adoption skip.

Fresh release Tier-A matrix emitted 159 successful rows and no failure markers;
the tool display lost the original pipeline status, so no status value is
invented. Tier-A quick used the frozen binary and timed out after 300,013 ms with
no report (`INVALID`, not passed); its generated snapshot is preserved in custody.
No full multi-corpus, live adoption or real-site census ran, and no baseline was
changed.

The proof modules remain above the plan's approximate
700–1,000 target. The excess is executable cross-subsystem refusal, epoch and
FullFlow evidence, not a reusable framework or production expansion; independent
review round 1 retained this as a SMELL rather than requiring a context-discarding
restart. Production repair remains the following slice.

## Independent review

The declared two-round cap converged on the same artifact. Terra-high round 1 was
`FIX-FIRST` with three WRONG and three SMELL findings. The targeted fixes added
the exact positive vector, nested-default desired rows and pinned duplicate-call
ownership; they narrowed DFG and fallback claims. Round 2 returned `APPROVE` with
0 WRONG and two retained non-blocking SMELLs: the private `use_byte` counterfactual
gap and the disclosed 1,349-line proof size. No closed-enumerable or open-class
finding remains. The independent production-design verdict is
`READY_FOR_BOUNDED_REPAIR`; it does not approve or implement the runtime repair.
