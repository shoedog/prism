# guava-forwardingmap-standard-containskey

Status: DRAFT — controller+Fable review pending; ADMISSION FAIL under the prompt's `guava/src` production scope.

## Why This Task Exists

The requested second task was `guava-converter-doforward`. The prompt said to verify admission for that contested target and, if it failed either bound, swap to `guava-forwardingmap-standard-containskey`.

Source verification found that `guava-converter-doforward` fails the D1 bound under `guava/src` only:

- `git -C ~/code/bench-repos/guava grep -nw doForward -- guava/src` -> 23 lines.
- Real family: abstract `Converter.doForward`, wrappers `correctedDoForward`/`unsafeDoForward`, and converter implementations in `CaseFormat`, `Converter`, `Enums`, `Maps`, and primitive converters.
- `git -C ~/code/bench-repos/guava grep -n "\\.convert(" -- guava/src` -> 21 lines, but real production Converter consumers are not D1; the apparent `CaseFormat` calls are `CaseFormat.convert`, and `Stopwatch`/`AbstractFutureState` are `TimeUnit.convert`.
- `git -C ~/code/bench-repos/guava grep -n "\\.apply(" -- guava/src` -> 163 lines, receiver-verified as `Function`, `Predicate`, stream/function helpers, or implementation internals; no production D1 callers of `Converter.apply`.
- `convertAll` has no production call sites in `guava/src`.
- Result: `D1=0`; contested task fails admission, so the instructed swap was attempted.

## Scope

- Repo: `~/code/bench-repos/guava` at `2b2452a`.
- Included source root: `guava/src` only.
- Excluded: `android/guava/src` mirror and `guava-tests`.
- Fallback root symbol used for closure: `Maps.containsKeyImpl` at `guava/src/com/google/common/collect/Maps.java:3816`.
- Renaming forwarder: `ForwardingMap.standardContainsKey` at `guava/src/com/google/common/collect/ForwardingMap.java:214`.

## Closure Walk

### L0: `containsKeyImpl`

Command:

```bash
git -C ~/code/bench-repos/guava grep -nw containsKeyImpl -- guava/src
```

Result: 3 lines.

- `ForwardingMap.java:19`: static import.
- `ForwardingMap.java:215`: real direct call from `standardContainsKey`.
- `Maps.java:3816`: target helper declaration.

Forwarder: `ForwardingMap.standardContainsKey` is thin. It returns `containsKeyImpl(this, key)` and adds no independent logic.

### Hop 1: `standardContainsKey`

Command:

```bash
git -C ~/code/bench-repos/guava grep -n "standardContainsKey(" -- guava/src
```

Result: 2 lines.

- `ForwardingMap.java:214`: the real forwarder definition.
- `ForwardingSortedMap.java:133`: same-name sibling override, excluded because it does not call `containsKeyImpl` and has sorted-map-specific logic.

Full-repo check:

```bash
git -C ~/code/bench-repos/guava grep -n "standardContainsKey("
```

Result: 10 lines, but the additional callers are in `guava-tests` or the `android/guava` mirror, both out of scope.

## D-Membership

- Repo-wide `containsKeyImpl` is far below 100 occurrences, so no D2 sites.
- D1 sites under `guava/src`: 0.
- Distinct D-file scorer denominator: 0.

## Admission

- Real gold sites: 2.
- D1 sites: 0.
- Gate: `8 <= |gold| <= 60` and `D1 >= 3`.
- Admission: FAIL.

This is not an acceptable Part-D measurement gold set without changing scope or choosing a different weak target.

## Dry Run

Command:

```bash
cd eval && uv run python -c "from tier_c.structural import score_structural,load_gold; g=load_gold('tier_c/gold/guava-forwardingmap-standard-containskey/gold.json'); perfect=[{'file':s['file'],'symbol':s['symbol']} for s in g['sites'] if s['adjudication']=='real']; r=score_structural(perfect,g); print('perfect',r.file_f1,r.d_recall,r.gold_size,r.d_gold_size,r.phantom)"
```

Output:

```text
perfect 1.0 0.0 2 0 0
```

## Exclusions

- `ForwardingSortedMap.standardContainsKey`: same-name sibling implementation, not a caller of `Maps.containsKeyImpl`.
- `guava-tests/test/com/google/common/collect/Forwarding*Test.java`: out-of-scope tests.
- `android/guava/src/com/google/common/collect/Forwarding*.java`: out-of-scope mirror.

## Unsure / Review Flags

- The prompt described this as the clean weak alternate, but source verification under the same `guava/src`-only scope shows it fails both the size and D1 gates.
- If controller wants this fallback, scope must likely be relaxed to include tests, or a different production weak target should be selected.
