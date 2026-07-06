# guava-equivalence-doequivalent

Status: DRAFT — controller+Fable review pending.

## Scope

- Repo: `~/code/bench-repos/guava` at `2b2452a`.
- Included source root: `guava/src` only.
- Excluded: `android/guava/src` mirror, `guava-tests`, `android/guava-tests`, and non-code/doc-only hits.
- Root symbol: `Equivalence.doEquivalent` at `guava/src/com/google/common/base/Equivalence.java:100`.

## Closure Walk

### L0: `doEquivalent`

Command:

```bash
git -C ~/code/bench-repos/guava grep -nw doEquivalent -- guava/src
```

Result: 7 lines in 3 files.

- `Equivalence.java:75`: true direct call from `Equivalence.equivalent`.
- `Equivalence.java:100`: abstract SPI declaration.
- `Equivalence.java:384`: `Equals.doEquivalent` implementation.
- `Equivalence.java:405`: `Identity.doEquivalent` implementation.
- `FunctionalEquivalence.java:47`: implementation.
- `PairwiseEquivalence.java:34`: implementation.
- `Equivalence.java:44`: comment-only, excluded in the table.

Forwarder: `Equivalence.equivalent(a, b)` is thin. It adds only identity/null guards before calling `doEquivalent(a, b)`; if the pairwise equivalence contract changes, this public API necessarily changes with it.

### Hop 1: `equivalent`

Raw command:

```bash
git -C ~/code/bench-repos/guava grep -nw equivalent -- guava/src
```

Result: 457 raw word hits, mostly prose. Receiver-verified call-token subset:

```bash
git -C ~/code/bench-repos/guava grep -n "\\.equivalent(" -- guava/src
```

Result: 38 lines; 35 are real call tokens after excluding docs/annotation text. The same-class `test(t, u)` bridge calls `equivalent(t, u)` without a dot at `Equivalence.java:87`, so it is included separately as a checked thin bridge.

Real hop-1 families:

- `Equivalence.Wrapper.equals` and `EquivalentToPredicate.apply`.
- `FunctionalEquivalence.doEquivalent` and `PairwiseEquivalence.doEquivalent`, already represented by their L0 implementation entries.
- `LocalCache` key/value equivalence sites.
- `MapMakerInternalMap` key/value equivalence sites.
- `Maps.doDifference`.

Bridge check:

```bash
git -C ~/code/bench-repos/guava grep -n "\\.test(" -- guava/src/com/google/common
```

Result: 11 lines; all receiver-verified as `Predicate`, `BiPredicate`, or `MillerRabinTester`, not `Equivalence.test`. These are excluded phantom-bait entries.

## D-Membership

- Repo-wide `doEquivalent` token count: 24, so no D2 sites.
- D1 files: `LocalCache.java`, `MapMakerInternalMap.java`, `Maps.java`.
- D1 site count: 27.
- Distinct D-file scorer denominator: 3.

## Admission

- Real gold sites: 36.
- D1 sites: 27.
- Gate: `8 <= 36 <= 60` and `D1 >= 3`.
- Admission: PASS.

## Dry Run

Command:

```bash
cd eval && uv run python -c "from tier_c.structural import score_structural,load_gold; g=load_gold('tier_c/gold/guava-equivalence-doequivalent/gold.json'); perfect=[{'file':s['file'],'symbol':s['symbol']} for s in g['sites'] if s['adjudication']=='real']; r=score_structural(perfect,g); print('perfect',r.file_f1,r.d_recall,r.gold_size,r.d_gold_size,r.phantom)"
```

Output:

```text
perfect 1.0 1.0 36 3 0
```

## Exclusions

- Comment/doc-only `doEquivalent`/`equivalent` hits in `Equivalence.java`.
- `.test(...)` same-name collisions in `LocalCache`, `CollectSpliterators`, `Collections2`, `Iterables`, `Lists`, and `LongMath`.
- `android/guava/src` mirror is out of scope and recorded in the exclusion table.
- `guava-tests` and `android/guava-tests` are out of production scope.

## Unsure / Review Flags

- Java overload/nested-class symbols: several `LocalCache` and `MapMakerInternalMap` entries have the same bare method name. I used signature or class qualifiers in `symbol` where useful, while keeping token anchors line-specific.
- `Equivalence.test` is a true thin bridge, but has no in-scope production consumers; included as a real forwarder and then terminated after receiver-verifying `.test(...)` collisions.
- D-file denominator is 3, below the methodology's do-not-trust flag for headline aggregation, though the task admission rule itself only requires D1 count >= 3.
