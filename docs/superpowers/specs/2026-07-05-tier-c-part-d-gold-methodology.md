# Part-D gold-set construction methodology (renaming-forwarder closure)

Date: 2026-07-05. Operational procedure for building the Part-D adjudicated gold sets.
Companion to the design-of-record (`2026-07-05-tier-c-part-d-structural-tasks-design.md`)
and the corpus (`eval/tier_c/issues/structural.toml`). Source: Fable consultation,
code-grounded on prometheus@505095b, ruff@44f6d18, caddy@77e9ce7. **This supersedes the
design's §5 "LSP∪prism candidates" as the gold GENERATOR** (LSP/prism are demoted to
cross-checks; grep-per-hop is the generator of record).

## Why (the crux)
build-gold's direct-caller candidates are all grep-findable (`d_member=none`) because a
direct caller of S textually contains S's name. The name-absent (D1) sites — the point of
the task — live one **renaming hop** out: callers spell the *forwarder's* name (`Matches`,
`is_list`, `RequestMatcherWithError`), not S's. Same-name hops (plain `iface.M()`→`(*T).M()`)
produce D2/noise, never D1. **A valid Part-D target has a forwarding chain that crosses ≥1
renaming boundary.** Because at every hop the frontier's callers textually contain the
*frontier's* name, a grep-complete, prism-INDEPENDENT enumeration always exists — you grep a
different name at each hop.

## The rule — gold(S) = renaming-forwarder closure
1. **Level 0.** `git grep -nw <S>` at the SHA. Source-verify each hit's receiver type. True
   direct callers of S → gold. Same-name/other-receiver hits → **exclusion table** (this IS
   the phantom-adjudication table at scoring time).
2. **Classify each caller** CONSUMER (gold, stop) or **FORWARDER** (gold, recurse). Thinness
   test, recorded per forwarder W: *if S's doc contract changes, W's necessarily changes, and
   W adds none of its own* — trivial adaptation only (negation, tag/variant dispatch,
   monomorphization, arg plumbing, boolean aggregation over homogeneous forwards). A caller
   with its own domain logic that insulates its contract from S is a CONSUMER — stop.
3. **Recurse per forwarder** via `git grep -nw <W>` (per-hop textual completeness),
   receiver-verify, classify. Terminate when the frontier has no forwarders. Hop count is not
   fixed (prometheus 1, ruff match_annotation 2) — chains are naturally short; that IS the
   boundedness.
4. **Bound = admission, not truncation.** Closure >60 sites → the target FAILS admission
   (pick another S). Never truncate — a truncated gold makes recall meaningless.
5. **Scope declarations**, mirrored verbatim into the arm prompt (in/out lines). Sibling fast
   paths / decoys get an explicit in/out. Borderline aggregators get a recorded include-site-
   but-don't-hop decision.

## Prism-independence (anti-circularity — pre-register these)
- **Generator of record = grep-per-hop.** Every gold site carries a **token anchor**: the
  file:line of the textual call token grep found it by (S's name at L0, the forwarder's name
  at each subsequent hop). The human never relies on prism/LSP to know a site *exists*.
- **LSP (gopls/rust-analyzer call-hierarchy on S AND each forwarder W) + prism (`nav_callers`
  on both) = cross-checks only**, provenance-tagged. A tool-surfaced site absent from the
  grep enumeration is an ALARM: fix the forwarder map or reject the site — source-first, and
  name the textual token that should have anchored it before admitting.
- **HARD INVARIANT: no gold site may have prism-only provenance without an independent textual
  re-derivation.** Prism provenance alone is never sufficient for gold membership. (Kills
  circularity structurally, not statistically.)
- **Precision = receiver-type verification.** Each site records an **evidence line**: the
  declaration that types the receiver (e.g. `m.re : *FastRegexMatcher` via the Matcher struct
  field). Dual-rater κ on the forwarder classifications + a sample of receiver verdicts (that's
  where raters disagree), not just the lsp-vs-prism band.
- **Hop-completeness check** per target (recorded on the task card): no method values,
  reflection, or macro-generated calls hide a call from grep (method-value tokens still contain
  the name, so grep catches them).

## gold.json — per-site schema
`{file, symbol, line, token_anchor:"<name>@file:line", receiver_evidence:"<decl>",
hop_distance:int, role:"consumer|forwarder", d_member:"D1|D2|none",
provenance:["grep","lsp","prism"], adjudication:"real|excluded", reason}`
gold = sites with `adjudication=="real"`. Freeze it arm-blind.

## Read-out corrections (fold into the design's §6/§7)
- **§6(iii) BUG FIX:** off-arm grep-recovery of a D1 site falsifies its D1 classification
  ONLY IF the grep was on **S's own name**. Recovery via a **forwarder's** name does NOT
  falsify D1 (the file genuinely lacks S's name). Without this you wrongly discard valid D1
  sites and mis-call INSTRUMENT-FAIL.
- **Attribution audit:** for each D1 gold site the off-arm found, classify the surfacing
  command — grep-S / grep-forwarder / file-read / no-command (memorization tell).
- **Third pre-registered outcome:** off-arm D1-recall high AND dominated by grep-forwarder =
  "shallow renaming chains are grep-recoverable" — a REFUTED-for-this-shape finding (real,
  acceptable).
- **Report ΔD-recall PER TASK**, never pooled-only. Depth heterogeneity is expected:
  prometheus (1 hop) smallest Δ; ruff match_annotation (2 hops) largest. Keep **phantom
  asymmetry** as a co-primary on prometheus (prism value may be precision, not recall).

## Do-not-trust flags (any one → discount the Part-D result)
1. A D1 gold site with prism-only provenance and no textual re-derivation (circularity).
2. A forwarder name in the arm prompt (instant collapse) — audit the final prompt vs the gold's forwarder list.
3. Gold/prompt scope mismatch (SetMatches, tests, Selector hop).
4. Closure truncated instead of target-failed when >60.
5. Off-arm high D-recall with a near-empty command log on famous code (memorization).
6. A D-subset denominator <5 feeding the headline (quantization — why R3 was rejected).
7. Pooled-only reporting across 4 tasks × 2 models (per-task heterogeneity is the signal).

## Per-task starting enumerations (Fable, to be source-verified + frozen)
- **prometheus MatchString** (1 hop): L0 true sites `matcher.go:115,117` + `regexp.go`
  internals; exclusion table = stdlib `*regexp.Regexp` (config.go, discovery/*, promqltest,
  azuread, cors.go, dedupe.go, template.go, api.go:1904) + `relabel.Regexp` (embeds stdlib,
  relabel.go:195). Hop-1 forwarder `(*Matcher).Matches` callers (labels.Matcher receiver):
  promql/info.go:65,470; promql/parser/parse.go:915; rules/group.go:172,1164,1188,1196;
  tsdb/exemplar.go:209; tsdb/querier.go:277,285,349,459,490; web/api/v1/api.go:1268,2311;
  test_utils.go:36. `Selector.Matches` include-site-don't-hop. D1≥6 (only api.go has any
  MatchString text, and it's stdlib @:1904). ≈30-40 sites.
- **ruff match_annotation** (2 hops): S ← `check_type::<T>` ← ~10 is_* wrappers ← rule
  consumers. Decoy `match_annotation_to_complex_bool` (flake8_boolean_trap) = phantom check.
- **caddy RequestMatcher migration** (archetype-B): 17 legacy `Match(r) bool` impls (matchers.go,
  ip_matchers.go, vars.go, celmatcher.go, routes.go, fileserver/matcher.go) + dual-dispatch
  type-switches routes.go:352/373/445 + httptype.go:1572 + CEL factories vars.go:236/369/392 +
  deprecated MatcherSet.Match routes.go:411. D1≥5 (grep -lw RequestMatcher = 5 files; impl files
  never spell it).
- **ruff Imported::qualified_name** (D2/precision): ~40 `.qualified_name()` sites/16 non-test
  files; exclusion = QualifiedName type + same-name fields (binding.rs:500/511/520) + same-name
  methods (imports.rs:60, definition.rs:63, ty class.rs:693/1012, type_alias.rs:302).
