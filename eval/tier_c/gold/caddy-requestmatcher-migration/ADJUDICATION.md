# ADJUDICATION — caddy-requestmatcher-migration

Repo: `~/code/bench-repos/caddy` @ `77e9ce7` (confirmed via `git log -1`, worktree clean).
Archetype-B interface migration (NOT a single-symbol renaming-forwarder closure): gold =
every site that must change to delete `RequestMatcher` (`modules/caddyhttp/caddyhttp.go:43`)
and consolidate on `RequestMatcherWithError` (`caddyhttp.go:55`).

## Closure walk — two orthogonal enumeration layers + one true renaming hop

**Layer 1 (L0 dispatch, found by `git grep -nw RequestMatcher`, 17 raw hits / 5 files
non-test):** collapses to 9 (file, symbol) dispatch/wiring sites after collapsing multiple
hits per enclosing function (e.g. routes.go:359+380+393+449+453 → 3 functions) and excluding
the def site itself (caddyhttp.go:36-55).

**Layer 2 (D1 impl layer, found by `git grep -nE 'func .*Match\([a-zA-Z_]+ \*?http\.Request\)
bool'` repo-wide non-test, 18 raw hits, 0 in test files):** 16 concrete matcher-module
`Match` implementers that satisfy `RequestMatcher` IMPLICITLY and never spell its name in
their own function bodies (the 18th hit, `MatcherSets.AnyMatch`, is excluded — see below).
This orthogonal grep (method signature, not interface name) is the whole point of the task,
per the task notes.

**One true renaming hop:** `CELMatcherFactory` (`celmatcher.go:438`, deprecated type alias
`= func(data ref.Val) (RequestMatcher, error)`) is a genuine forwarder name — its two
consumers `CELMatcherDecorator` (469) and `CELMatcherRuntimeFunction` (554) contain **zero**
occurrences of the bare token `RequestMatcher` in their own bodies; they're reachable only by
grepping `CELMatcherFactory`. They still score `d_member=none` under the strict per-file rule
(celmatcher.go the FILE contains `RequestMatcher` elsewhere, at 436/438), but the discovery
mechanism is a true hop, illustrating the same renaming-boundary principle as the other Part-D
tasks even though the file-level D-membership doesn't reward it here.

Terminates because: dispatch-layer functions call the interface methods on config-decoded
`any` values; the 16 concrete `Match(r) bool` implementers are leaves — their own callers all
go through `MatcherSet.Match`/`MatcherSet.MatchWithError` (routes.go), which is already gold
as a dispatch site, so there is no further hop to recurse.

## Per-hop grep counts
- `git grep -nw RequestMatcher -- '*.go' ':!*_test.go'` → 17 hits / 5 files (celmatcher.go=2,
  matchers.go=3, routes.go=5, caddyhttp.go=6, httptype.go=1). Repo-wide total (incl. tests) is
  also 17 — confirms zero test-file references and that D2 (>100 threshold) never applies to
  this task.
- `git grep -nE 'func .*Match\([a-zA-Z_]+ \*?http\.Request\) bool' -- '*.go' ':!*_test.go'` (repo-wide,
  not just caddyhttp) → 18 hits, all in `modules/caddyhttp/**`. 16 are matcher-module impls
  (gold, role=consumer); 1 is `MatcherSet.Match` (gold, role=forwarder — double duty, recorded
  once); 1 is `MatcherSets.AnyMatch` (excluded — different method name, already-migrated body).
- `git grep -nw CELMatcherFactory -- '*.go' ':!*_test.go'` → 5 hits, all in `celmatcher.go`
  (decl@436/438, 2 call sites@522/568, 1 error-message string@548) — confirms no external
  module uses the legacy CEL factory type; all 3 enclosing functions already gold.
- Interface-guard regex `git grep -nE '_[[:space:]]+(caddyhttp\.)?RequestMatcher(WithError)?[[:space:]]*='`
  → **0 hits** for bare `RequestMatcher`. Broader guard scan (any `_ ...Matcher... = (`) → 14
  hits, ALL already `RequestMatcherWithError`. See Fable correction #1 below.
- Repo-wide `func.*) Match(` sweep (to hunt phantom bait) → found 5 same-name collisions in
  `modules/caddytls/matchers.go` (incl. a `MatchRemoteIP` name-collision with
  `caddyhttp.MatchRemoteIP`, a different type in a different package) plus `encode.go:278` and
  `responsematchers.go:40`. All excluded — see exclusion table.

## |gold| / D1 numbers
- `|gold(real)|` = **25** (16 impl sites + 9 dispatch/wiring sites).
- D1 = **5** sites, in **3 distinct files**: `fileserver/matcher.go` (MatchFile), `ip_matchers.go`
  (MatchRemoteIP, MatchClientIP), `vars.go` (VarsMatcher, MatchVarsRE). Matches Fable's
  "D1≥5" claim exactly once celmatcher.go and matchers.go are correctly excluded from D1 (both
  contain `RequestMatcher` text elsewhere in the file).
- Admission: D1 count (5) ≥ 3 → **PASS** on the primary clause. ((D1+D2)/|gold| = 5/25 = 0.20,
  below 0.3, but the D1≥3 clause alone is sufficient per the methodology's OR condition.)
- **Caveat (do-not-trust flag #6 adjacent):** the scorer's `d_gold_size` (distinct D-subset
  FILES, the `d_recall` denominator) is **3**, below the flag's <5 quantization-risk threshold.
  This is because the 5 D1 sites cluster into only 3 files (2+2+1). `d_recall` on this task will
  swing in large discrete steps (0, 0.33, 0.67, 1.0) — real but coarse. Flagging for the
  controller; this did not block admission (admission is defined on D1 site count, not file
  count) but should be read as a per-task caveat alongside the "WEAKEST/STRONGEST instrument"
  heterogeneity note in the corpus file.

## Dry-run scorer output
```
PERFECT ARM:              file_f1=1.0                d_recall=1.0  gold_size=25  d_gold_size=3  phantom=0
GREP-RequestMatcher-ONLY: file_f1=0.7272727272727273 d_recall=0.0  gold_size=25  d_gold_size=3  phantom=0
```
The grep-`RequestMatcher`-only arm (claims = the 7 sites whose own text contains the bare
token, i.e. all dispatch/wiring sites EXCEPT the two true-hop CEL consumers) recovers 4/7 gold
files (missing `fileserver/matcher.go`, `ip_matchers.go`, `vars.go` entirely) and **zero** D1
sites — exactly the intended contrast: naive name-grep cannot see the implicit-interface-
satisfaction layer, which is the entire point of this archetype-B task.

## Fable corrections (verified by source, per instructions to not trust the enumeration)
1. **Interface guards do NOT need migrating.** Fable's notes claim `var _ RequestMatcher = ...`
   guards exist and must change. Verified FALSE: all 14 interface guards in the repo already
   read `var _ RequestMatcherWithError = (*T)(nil)` (celmatcher.go:838, fileserver/matcher.go:737,
   ip_matchers.go:356/361, matchers.go:1632/1634/1635/1637/1638/1639/1640/1642/1643,
   vars.go:466/468). This component of the enumeration is empty at this SHA — excluded, not gold.
   (First regex attempt used `\s` under `git grep -E`, which is POSIX ERE and doesn't support
   `\s` — produced a false negative for the wrong reason; redone with `[[:space:]]` and run
   standalone, confirmed genuinely zero, rc=1.)
2. **CEL factories are NOT at vars.go:236/369/392.** Those vars.go closures are already typed
   `(data ref.Val) (RequestMatcherWithError, error)` — fully migrated, no change needed. The
   real legacy CEL dispatch is in `celmatcher.go`: the `CELMatcherFactory` type alias (438) and
   its two consumers `CELMatcherDecorator` (469) and `CELMatcherRuntimeFunction` (554).
3. **routes.go line numbers drift ~7-9.** Fable said 352/373/445; actual `RequestMatcher`
   type-assertions are at 359 (in `MatcherSet.Match`, def 350), 380 (in
   `MatcherSet.MatchWithError`, def 371), 449 (in `MatcherSets.FromInterface`, def 441).
   httptype.go dispatch is inside `parseMatcherDefinitions` (def 1537), assertions at 1572/1577
   (Fable said "1572" as if it were the whole site; it's one of two arms in one function).
4. **"~17 legacy Match impls" reconciled.** = 16 pure matcher-module impls + `MatcherSet.Match`
   (routes.go:350) itself, which both satisfies the legacy signature AND is one of the 3
   dual-dispatch functions — recorded once, as a dispatch/forwarder site, not double-counted.
   Fable's separately-named "deprecated MatcherSet.Match (routes.go:411)" is `MatcherSet.Match`
   (def line 350, not 411); `routes.go:412` is actually `MatcherSets.AnyMatch`, a *different*
   method name whose body already calls only `MatchWithError` — considered and excluded.

## Exclusions (phantom bait / considered-and-excluded) — 17 sites
- **Same-name, different signature, same package:** `matchers.go:1488` `MatchRegexp.Match(input
  string, repl *caddy.Replacer) bool` (string/placeholder helper, not `*http.Request`).
- **Same-name, different package (`caddytls`), `ConnectionMatcher` not `RequestMatcher`):**
  `matchers.go:58` (MatchServerName), `:151` (MatchRegexp — name-collides with
  `caddyhttp.MatchRegexp`), `:231` (MatchServerNameRE), `:306` (MatchRemoteIP — name-collides
  with `caddyhttp.MatchRemoteIP`, a real D1 gold site in a different package), `:422`
  (MatchLocalIP). All take `*tls.ClientHelloInfo`, not `*http.Request`.
- **Same-name, unrelated domain:** `encode/encode.go:278` (`Encode.Match(rw *responseWriter)
  bool`), `responsematchers.go:40` (`ResponseMatcher.Match(statusCode int, hdr http.Header)
  bool` — has "Matcher" in the type name and touches `http.Header`, the closest phantom bait
  in the set, but wrong signature/interface).
- **Already-migrated interface guards (14 sites, not gold):** see Fable correction #1.
- **Def site (not a consequence site):** `caddyhttp.go:36-55`, the `RequestMatcher` /
  `RequestMatcherWithError` interface declarations themselves.
- **Considered borderline, excluded (flagged below as uncertain):** `celmatcher.go:391`
  (`CELMatcherImpl`, thin `any`-passthrough plumbing — doesn't itself dispatch on
  `RequestMatcher`); `matchers.go:1623` (`MatcherErrorVarKey` const, doc-comment-only mention);
  `routes.go:412` (`MatcherSets.AnyMatch`, deprecated but body only calls `MatchWithError`).

`excluded_test_helpers`: **none**. Both the signature grep and `RequestMatcher`/
`CELMatcherFactory` greps returned zero hits in `*_test.go` — no test-only matcher impls exist
for this interface at this SHA.

## Uncertain sites for controller review (top 3)
1. **`modules/caddyhttp/routes.go:412` `MatcherSets.AnyMatch`** — deprecated
   ("Use AnyMatchWithError instead") and textually/thematically part of the same migration
   story, but its body calls only `m.MatchWithError(req)` — no `RequestMatcher` or `.Match()`
   reference anywhere. I excluded it (nothing to textually change here for *this* interface
   deletion), but a controller favoring "everything deprecated in this cluster" scope might
   want it included as a stretch site.
2. **`modules/caddyhttp/matchers.go:1623` `MatcherErrorVarKey` const decl** — its doc comment
   explicitly says "...matchers cannot return errors via the `RequestMatcher` interface," but
   it's prose on a const, not dispatch code; the const's real *use* sites are already gold
   (routes.go Match/MatchWithError, ip_matchers.go Match impls). Excluded, flagged as borderline.
3. **The interface-guards correction itself (Fable correction #1)** — not a single site but a
   structural disagreement with the task-specific notes: I'm confident in the source evidence
   (double-checked with a working regex after catching my own first-attempt regex bug), but
   since it zeroes out an entire component of the pre-registered enumeration, it's worth a
   second pair of eyes before freezing.
