# Handoff — parameter real-site value audit

**Written:** 2026-09-10 · **By:** root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/slicing · audit/typescript-parameter-real-sites · **Measured state:** `[MEASURED]` base013d4bf014abe1c33449629854f8f8915d4237ef; tree DIRTY with audit-only example/docs; git status.
**Predecessor:** PR309 required-parameter repair, merged.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by root. `[MEASURED]` rechecked this turn; `[INHERITED]` explicitly named.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[MEASURED]` root owns native/design; parameter_comparison completed two comparator files only — **RESOLVED**.
**(b) Custody exposure** — `[MEASURED]` native6/comparator10 GREEN; identical observers built/frozen against both artifacts; checkpoint follows — **OPEN** until commit.
**(c) In flight / irreversible** — `[MEASURED]` sequential bounded corpus runs active via run-audit.mjs; full gates/review pending — **OPEN** before completion.
**(d) Authorization granted but not exercised** — “merged - proceed to next”; standing commit/push/open PR; no auto-merge or private-source publication.

## 1. Resume order

1. `git status --short --branch`; preserve this lane's example and comparator files.
2. Run native example tests and comparator tests; build/freeze identical observer code against f369f20c and current production.
3. Run fixed public/private snapshots; validate comparison, review up to2 rounds, full suites, then publish aggregates only.

**STOP conditions:** private source in publication, source population drift, new runtime authority, open-class review at cap2, unclassified failures.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Merge | done | `[MEASURED]` gh PR309 MERGED013d4bf0; clean start |
| Snapshot custody | done | `[MEASURED]` explicit Git archives of fixed commits; original private checkout clean |
| Native/tooling | done | native-final-green.log6 pass; comparator-green.log10 pass; raw RED retained |
| Corpus/full gates/review | pending | Not run |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| PR309 handoff | Merge pending | `[MEASURED]` merged013d4bf0; approved successor is this audit |
| Initial comparator | Occurrences part of static equality | Removed: empty-to-populated occurrences are the measured effect; regression added |
| Native recovery test | One recovered parameter node | Actual tree retains required_identifier plus sibling ERROR; whole-list flag remains true |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Corpus measurement | next | Frozen before/after native runs | Focused GREEN | Raw local only |
| 2 | Review/gates/publication | pending | Review then clean checkpoint suites | Corpus results | cap2 |

## 5. Invariants and traps — do not do these

- Do not count body assignment targets as parameter-token endpoints.
- Do not call all_functions a complete raw callable census.
- Do not freeze occurrences into static-population equality.
- Do not publish private per-file rows, identifiers or raw graphs.
- No runtime/cache/closure/React.FC/react-scripts change; no Tier-A production trigger.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Evidence | `/private/tmp/prism-parameter-audit-rhxJcA` |
| Public snapshot | `0642e72cfa2d9a71198200e52f37399384610ee3` |
| Pre-repair baseline | `f369f20cdcd3463bae86b191bcdd8ec9efd8f6fd` |
| Plan | `docs/superpowers/plans/2026-09-10-typescript-parameter-real-sites.md` |

## 7. Refutation verdict and owner questions

**§2c verdict:** NOT RUN — measurement underway · claim: "the comparison separates recovered parameter definitions from positional or body fallback flow" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: native-red.log and comparator tests only.

**Questions the owner owes an answer to:** None.
