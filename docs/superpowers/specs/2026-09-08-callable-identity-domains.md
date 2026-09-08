# H1 — compiler identity domains and synthetic lookup coordinates

Hardening reserve H1 is allocated before S3 integration. S3's existing artifact is
parked, not discarded. Two ordinary review rounds and one disclosed numeric-order
verification extension exposed another identity-domain case; primary escalated to
this finite source-backed design audit instead of another open-ended patch round.
No closure, acquisition, runtime/class, application or dependency expansion.

## Counterexample and controls

With `links:in-root`, select `alias/tsconfig.json`, where directory alias points to
real. A configured type makes pinned TypeScript search from
`/__prism__/project/alias/__inferred type names__.ts`. This is an invented lookup
address, not a source file. S3's general canonical projector changed it to real,
while its parser correctly derived the lexical alias address from scope.config.
Exact S2 retains the normal packet; S3 returns worker_failed. Its raw worker packet
is normal and the parser rejects it, ruling out a compiler or inventory failure.
Evidence: task-root/probe-s3-config-link.mjs and s3-config-link-result.json.

Previously corrected cases remain controls: repeated source callback visits need
separate batches; duplicate occurrences need exact per-batch coverage; batch indices
are numeric even though older serialized source/entry rows use lexical key order.

## Closed coordinate contract

| Field | Domain | Projection |
|---|---|---|
| Module from, source-type from, all source anchors | Actual source identity | Existing bounded canonical file/link/case projection |
| Module/type resolver targets | Actual selected file identity | Existing bounded canonical projection, nullable |
| Type-entry batch/request/search from | Compiler-generated lookup address | Pure normalized lexical project address; no links/case folding or membership claim |
| Planned library-search from | Compiler-generated lookup address | Same lexical projector; derive from selected config directory and lib filename |
| scope.config | Caller-selected config spelling | Lexical safe project ID, unchanged |
| S4 config cache key | Compiler cache address | Lexical selected config path, case-normalized exactly as the pinned host requires; never substitute a link target |
| S4 root/option/extends source anchors | Actual config bytes | Canonical source identity, kept distinct from the lexical config address |

Pinned TS127131–127137 derives inferred type from options.configFilePath's directory;
TS126698–126700 does the same for library searches. TS43633–43635 case-normalizes
config cache keys when host.useCaseSensitiveFileNames is false. The SourceFile name
retains selected spelling. Neither synthetic-address derivation resolves links.
Current inventory.resolve deliberately follows link prefixes for actual file IDs;
that operation is not interchangeable with recording a synthetic lookup address.

H1 adds a small pure lexical projector and compiler-source controls. It is not wired
into the current producer in H1, so the current packet and producer digest must
remain unchanged. S3 will add it to its producer hash and use it only for non-source
type from coordinates; searches reuse their batch's from ID. D1 will use it for lib
from coordinates. Existing source/target canonicalization must not change.

The projector accepts a normalized safe virtual project address and returns its
lexical project ID. It never opens or resolves a file, follows links, lowercases,
consults a manifest or creates legacy boundary events. Refused/unsafe/outside and
compiler-root addresses fail closed. Existing inventory controls still reject unsafe
links, cycles and link-step overflow before compiler execution; the lexical projector
must not add a competing link traversal.

## Finite acceptance matrix

1. Direct, mixed-case and nested config spellings preserve lexical addresses.
2. Config-file alias, directory alias and multi-hop directory alias preserve selected
   spelling for synthetic from, but canonicalize actual source/target IDs.
3. Alias-selected and real-selected configs with identical bytes retain different
   scope/from/options epochs; cross-substitution fails reproduction.
4. Type and lib synthetic names need not exist in inventory or Program.
5. Module/source-type from and selected target aliases still collapse canonically.
6. Source revisits through import/root aliases retain batches; no cross-batch reuse.
7. Source/configured/automatic indices0–11 remain numeric within each callback;
   duplicate omission and index10 swaps reject.
8. Unsafe .git, outside-root, refused colon and compiler-root synthetic inputs reject
   without I/O. Existing inventory link cycle/step controls remain unchanged.
9. S4 case-sensitive/insensitive cache-key selection stays distinct from source/link
   identity, including wrong-key and unrelated-alias substitution negatives.

H1's helper/compiler tests characterize these domains; they are not S3 production
RED. S3 requires captured exact-base alias regressions and full final observer,
public/cache transcript, Rust/MCP/helper/authority gates. D1/S4 integration controls
remain separate slices; their future behavior is not claimed here. Two H1 review
rounds maximum. Resume the same S3 artifact only after this contract is reviewed.
