> Historical R0 record. R1 changes ground truth, attribution and conversions; use [R1.md](R1.md) for the current result and verification. Original evidence remains immutable.

# Hypothesis / probe / result log

1. Expect clean declared branch/base and 600 manifest entries; falsifier: mismatch.
   `git status --short`, branch, rev-parse; manifest census. Confirmed clean base
   4e592daa, SecBench 5d362353, statuses 583/11/6. Tool output in active transcript.
2. Expect usable sink coordinates in acquired sources; falsifier: absent or
   ambiguous paths/invalid locations. Census: 475 exact files, 45 unique basename
   candidates, 7 ambiguous, 31 absent, 24 empty, 1 malformed. This is file/line
   validation only; no semantic sink validation yet. GT coverage requires reporting.
3. Expect parameter-selective CLI; falsifier: line-only seed flags. CLI and taint
   source read confirm line-only external seeds; framework target seeds are internal.
   Alternative cause of apparent reachability: another parameter or same-line
   expression. Inspect witness variables before any traced credit.
4. File lookup `src/algorithms/data_flow.rs` and navigation-specific filename guesses
   were inadmissible reads (files absent). Correct location: `src/data_flow.rs`;
   use rg-discovered query implementation, no belief update from failed reads.
5. Handoff template lookup initially searched too broad and was interrupted;
   targeted steering-repo lookup found canonical template. No corpus content from
   outside the authorized inputs is used. Never repeat broad home-directory scan.
6. Expect witness roots to distinguish payload from callback; falsifier: no per-root
   results. aaptjs source index.js:30 -> sink :18: overall Reached, payload parameter
   apkfilePath NotReached, callback Reached. Harness rejects callback-only credit.
   All eight comparison invocations parsed successfully; evidence probe/aaptjs.json.
7. Corpus inspector attempt 1 stopped at 7 rows: strict UTF-8 failed. Inadmissible
   partial measurement. Alternative: acquisition drift. Exhaustive archive SHA
   checks found zero mismatches; encoding-census.json enumerates all 66 invalid files.
   Census records these files as unparsed instead of silently dropping them.
8. Attempt 2 stopped at 47 rows: constructor has no property name. Inadmissible
   partial measurement. Enumerated all prop-helper consumers; guarded optional
   names and added constructor/object-spread regression. Attempt 3 completed 583.
9. Expect exact-root filtering, errors/exclusions and syntax parser edge cases to
   pass; falsifier: wrong callback credit or syntax in comments counted. 11 Node
   and 8 Python tests passed. Full unmodified-product Rust suite: 4946 passed,
   zero failed, one ignored (cargo-test.log). No failure attribution to product.

10. Three GT validation passes: HTTP setup calls, sink byte/member identity, then
    factory-returned API inputs. Each exposed a bounded harness failure. At cap,
    the final enumerable ambiguity guard was folded; broader API recovery is
    parked. No artifact restart or product edit. Unmarked multiple data parameters
    are unavailable. Four such cases move from traced to GT unavailable.
11. Expect member-only parameters to lack base Defs; falsifier: ordinary parameter
    registration. src/data_flow.rs:602 explicitly skips parameters with no bare
    references. locutus frontier has only module.exports; npm-user-validate
    nodes-at/functions inventory loads email but has no em variable. Whole-file
    rejection alternative is ruled out for npm-user-validate. C-member attribution.
    Expect callback frontier to stop before downstream body; falsifier: callback
    uses already present. lycwed frontier includes waterfall callback uses, so
    CJS/member/callback explanations remain unresolved; no guessed H attribution.
12. Expect stale manual-label detection to reject changed outcomes. Replay rejected
    local-devices. Initial comparison probe raised KeyError (inadmissible: entry-only
    map included acquisition exclusions). Correct class+entry comparison shows
    only the four intended GT exclusions. Manual-label join had collided with an
    excluded command-injection entry. Corrected the label; retained raw measurement
    is unchanged. Replay now applies 30 frozen labels: 7/13 exact agreement; 3/9
    agreement on resolved categories. Same worker, not independent validation.
13. Expect pre-run harness to accept HTTP setup/factory and ambiguous input sources
    and wrong sink occurrences; falsifier: rejection. Preserved snapshot run in the
    same environment with current tests: three Node assertions fail (GT reason null
    instead of exclusion), two Python assertions fail (wrong occurrence/colliding
    synthetic sink incorrectly traced). Current tests pass. control-tests.log
    contains behavioral assertion output; return codes alone are not evidence.
    This attributes harness corrections only, not a product regression.
14. Expect compiler-clean syntax absence to prove a negative within-package path
    feature; falsifier: diagnostics/unparsed files or syntax elsewhere. Summarizer
    retains those as unknown unless manually inspected. Candidate callable-span
    prevalence is separate from actual payload-path evidence. hangersteak options
    spread is a concrete false positive for a naive whole-function path count.

15. Expect the fresh corrected invocations to agree with replay; falsifier: any
    changed outcome. Live aaptjs and thenify remain reached_function_only, and
    hangersteak remains traced. live-controls/results.json records all three.
    Expect a per-root frontier for partial attribution; falsifier: union only.
    Unreached witnesses have no root-specific frontier. Partial fallback and
    attribution remain explicitly union-seed screens, not a proved payload path.
16. Expect repeated replay to produce identical artifacts; falsifier: any byte
    difference in JSONL/summary/binding/Markdown. All four identical; SHA256s in
    determinism.json. This validates replay, not a second fresh full Prism run.
17. Entry parameter syntax is directly on the syntactic input path regardless
    of taint success. Complete eligible parameter census finds nine ...rest
    inputs (all prototype pollution), zero destructured inputs. Source-binding
    presence + whole-package clean negatives + manual path labels yield
    destructure 0 present / 165 absent / 27 unknown; rest 9 / 159 / 24.
    A test keeps unrelated syntax and parser gaps unknown.
18. Final validation: 18 Node tests; 16 Python harness tests; full eval 954 passed,
    3 skipped, 2 subtests passed. Full unchanged-product Rust suite: 4946 passed,
    0 failed, 1 ignored in 29 groups. Exact exclusions in VERIFICATION.md.
    git diff --check passes; source/Cargo/build/vendor diff is empty.

19. Verification hook audit cap two runs. Expect new edge tests to pass and the
    original module command to be absent at Git base; falsifier: failing current
    behavior or pre-change help/options present. Current 22 Node/26 Python pass.
    Exact git archive base: two CLI assertions fail with absent benchmark output/
    module. This establishes feature availability only. Expanded pre-run controls:
    Node 14 pass/8 assertion failures; classifier 6 pass/2 behavioral failures/
    3 errors (null-callee crash and two absent new summary fields). No unselected
    tests or invocation errors are counted as algorithm evidence.
20. Full verification-gate rerun: Rust 4946 passed/0 failed/1 reserved ignore across
    29 groups; full configured eval 964 passed/0 failed/3 skipped, 7 subtests passed.
    Exclusions and exact commands in root VERIFICATION.md, marked environment-limited.
    No unexpected/out-of-scope failures, no source changes and no re-baselining.

21. Final signature-line probe expectation: multiline parameters could expose a
    source-seed/Def-line mismatch; falsifier: all selected declaration lines equal
    their function starts. All 192 eligible entries satisfy that equality; no
    measurement correction or implementation change. Last attribution branch tests
    cover default/rest/local/CJS/callback cases and visited-callable negative cases.
    Second/final verification pass: 965 eval passed, 3 skipped, 10 subtests passed;
    27 Python harness tests pass. Rust and Node remain 4946/0/1 and 22/0.
