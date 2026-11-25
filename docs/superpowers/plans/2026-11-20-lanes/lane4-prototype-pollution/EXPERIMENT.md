# Lane 4 experiment memo (stage M)

Build: `prism` at 9d1a7d06f (SHA-256 3877b1d1...a70), harness of PR #849. Evidence: `~/prism-evidence/pp/` (e1, m).
Date: 2026-11-24. Parent issue #603.

## Result and gate

**C (strict) = 29, C (liberal) = 42.** The pre-stated gate (owner-confirmed 2026-11-20) is
C >= 25 under the strict reading: **continue to stages T and E without waiting.**
The contradicted-premise check passes: of the 20 traced cases that do not convert under the strict
reading, 3 fail because the key is not reached by name (expand-hash_1.0.1, mixin-deep_2.0.0, which also has no class,
and think-config_1.0.0) and 17 because no strict class applies: 16 for a shape the classes do not cover and 1
(objnest_5.0.0) because the write on the ground-truth line cannot pollute (a fresh local object). At most 3 of the 20 could be
matching artefacts of how the key's arrival is read, so they cannot be most of them; the premise check cannot fire.
Disclosure on the margin: the gate is applied by name, as DESIGN section 6 says. Of the 29 converted cases only 6 have a byte-exact
(strict E2) Use in the index span (arr-flatten-unflatten, assign-deep, dot-object, grunt-util-property, json8-merge-patch,
paypal-adaptive); the margin over the gate is 4 and the readers' coincidental-credit flag was not audited, so 5 coincidental
credits would move the gate to "ask the owner".

## E1: how many cases trace today

49 of the 95 cases with ground truth trace (the 190 acquired entries partition as below; `gt_unavailable` is the other 95) (baseline of 2026-10-05: 41). Outcomes now (baseline): traced 49 (41),
reached_function_only 24 (26), partial 9 (8), prism_error 11 (17), not_reached 2 (3), gt_unavailable 95 (95),
acquisition_excluded 2 (2). 20 case outcomes changed; the list is in `e1/SUMMARY.md`.

## E2: is the key reached (49 traced cases)

By name: 46 yes, 3 no (expand-hash_1.0.1, mixin-deep_2.0.0, think-config_1.0.0).
Strict (the Use's byte span inside the index expression): 8 yes over the 49 (6 of them among the 29 converted). As predicted by DESIGN 2, nearly every
conversion rests on a zero-width or right-hand-occurrence Use, so the sink must match by name.
Coincidental credit: the readers flagged none (E3 `coincidental_credit` false in all 49; the field is the least
reliable one, readers had no access to the matching script's internals).

## E3: classes of the 49 ground-truth writes (reader's labels; script cross-check)

Strict class: path_walk 21, recursive_merge 8, nested_keys 2, none 18.
Shape labels: path_walk 21, recursive_merge 5, alias_arg_recursion 5, indirect_cursor 5, helper_walk 3, property_recursion 3, return_recursion 3, nested_keys 2, alias_two_step 1, no_polluting_write 1.
Converted (strict) by payload shape: path_setter 22, object_merge 3, flatten_unflatten 3, other 1; (liberal): path_setter 29, object_merge 8, flatten_unflatten 3, other 2.
Converted, strict, by class: path_walk 19, recursive_merge 8, nested_keys 2.
Converted ids (strict, 29): arr-flatten-unflatten_1.1.4, assign-deep_1.0.0, cached-path-relative_1.0.1, confucious_0.0.12, deeps_1.4.5, doc-path_2.0.0, dot-notes_3.2.0, dot-object_2.1.2, dotty_0.0.1, eivindfjeldstad-dot_0.0.1, field_1.0.1, flat_5.0.0, grunt-util-property_0.0.2, json-pointer_0.6.0, json-pointer_0.6.1, json8-merge-patch_1.0.1, jsonpointer_4.0.0, just-safe-set_1.0.0, keyd_1.3.4, keyget_2.2.0, libnested_1.5.0, madlib-object-utils_0.1.6, minimist_1.0.0, nodee-utils_1.2.2, object-path-set_1.0.0, paypal-adaptive_0.4.1, set-deep-prop_1.0.0, set-value_3.0.0, tiny-conf_1.1.0.
Added by the liberal reading (13): component-flatten_1.0.1, decal_2.1.3, getsetdeep_4.15.0, hoek_5.0.0, merge-change_1.0.1, mpath_0.4.1, objection_2.0.0, putil-merge_3.0.0, safe-obj_1.0.0, strikeentco-set_1.0.0, think-helper_1.1.0, undefsafe_2.0.2, viking04-merge_1.0.0.

Script cross-check (`classify_proto.mjs`, strict prototype): agrees with the reader on 46 of 49. Disagreements: assign-deep_1.0.0
(script none, reader recursive_merge), dot-object_2.1.2 (script nested_keys + path_walk, reader nested_keys), undefsafe_2.0.2
(script recursive_merge, reader none/indirect_cursor). The reader's label counts.

Guards on the 29 converted cases: 3 carry any guard (deeps_1.4.5, dot-object_2.1.2, json-pointer_0.6.1) and all three are
insufficient or misplaced (a presence test, a final-key gap, a skip instead of a reject).

## Traced cases that do not convert (strict reading)

| case | reason | converts under liberal |
|---|---|---|
| component-flatten_1.0.1 | no strict class (shape: alias_two_step) | yes |
| convict_6.0.0 | no strict class (shape: helper_walk) | no |
| decal_2.0.0 | no strict class (shape: helper_walk) | no |
| decal_2.1.3 | no strict class (shape: alias_arg_recursion) | yes |
| expand-hash_1.0.1 | key not reached by name | no |
| getobject_0.1.0 | no strict class (shape: helper_walk) | no |
| getsetdeep_4.15.0 | no strict class (shape: indirect_cursor) | yes |
| hoek_5.0.0 | no strict class (shape: property_recursion) | yes |
| merge-change_1.0.1 | no strict class (shape: property_recursion) | yes |
| mixin-deep_2.0.0 | key not reached by name | no |
| mpath_0.4.1 | no strict class (shape: indirect_cursor) | yes |
| objection_2.0.0 | no strict class (shape: indirect_cursor) | yes |
| objnest_5.0.0 | no strict class (shape: no_polluting_write) | no |
| putil-merge_3.0.0 | no strict class (shape: alias_arg_recursion) | yes |
| safe-obj_1.0.0 | no strict class (shape: property_recursion) | yes |
| strikeentco-set_1.0.0 | no strict class (shape: indirect_cursor) | yes |
| think-config_1.0.0 | key not reached by name | no |
| think-helper_1.1.0 | no strict class (shape: alias_arg_recursion) | yes |
| undefsafe_2.0.2 | no strict class (shape: indirect_cursor) | yes |
| viking04-merge_1.0.0 | no strict class (shape: alias_arg_recursion) | yes |

## P: potential (reach function only or partially, with a class)

33 cases reach the function only or partially; by the script's strict classification 20 have a class (path_walk 12,
recursive_merge 7, nested_keys 1) and 13 none. Not gated; reader labels for these were not made (script only).
First breaks of the 20: unresolved 11, B-plain-argument 7, C-member 1, H-callback/promise/event 1.

## E4: ordinary code

Corpora and commits: Excalidraw 0642e72c (source tree), TypeScript `src` 7964e22f2, express 18e5985, directus `api` c40ab79f2c,
calcom `apps/api` f972feefa1 (superset 7519fa8b6f and answer 6744295f were run and excluded by rule: the first five gave 32 hits >= 20).
Class hits outside test directories: Excalidraw 8 (nested_keys), TypeScript 4 (nested_keys), express 0, directus 19
(nested_keys 10, recursive_merge 9), calcom 1 (nested_keys); path_walk 0 in every corpus. 32 hits, all labelled.
Reader tally: can_pollute 6, guarded 3, cannot 23. **Share able to pollute 6/32 = 19% (9/32 = 28% with guarded)**, below the 70% line:
the misjudged shapes become matrix rows and the class definitions are tightened in stage D before any slice. None of the 6 can-pollute hits has a
confirmed attacker-reachable selector (two of the three guarded directus hits are reachable from a `fields=` query parameter).
False-positive shapes named by the reader: write-back to the iterated object (`for (k in o) o[k] = f(o[k])`, hits 11-20, includes
all 9 directus recursive_merge hits); numeric or array-index selectors; templated selectors with a constant prefix; constant or
fresh-object targets; guards the token scan does not see (`Object.create(null)` on another line, a `throw` guard).
Aliased-receiver exposure: benchmark 3 of 49 traced (minimist_1.0.0, nodee-utils_1.2.2, set-deep-prop_1.0.0); corpora 0 of 32 hits.

## E5: guard forms in fixed versions (20 packages, archives only, never run)

The 20 were taken from the 49 traced cases that name a fixed version (14 of them are strict-converted; convict, getobject, hoek, mixin-deep, mpath and objection are not). All 20 fixes found. Forms: strict equality on literal names, inline or in a helper (12: assign-deep, mixin-deep, object-path-set,
just-safe-set, nodee-utils, dotty, hoek, json8-merge-patch, objection, jsonpointer, convict, madlib-object-utils);
`includes`/`indexOf` on a name list (dot-object, getobject, libnested, mpath, keyget); `startsWith` on the remaining path (doc-path);
a value test `=== Object.prototype` (keyd); a container change to `Object.create(null)` (cached-path-relative, json8-merge-patch).
Several fixes cover `__proto__` only (dotty, getobject, hoek, objection, json8-merge-patch) or test the first or the whole parent
path only (madlib-object-utils, convict). at least 13 test every walk key and the final key (or the whole path before the walk; the reader's count, criterion not re-applied), none only the
final key; most reject with `throw` or `return`, six skip. Each distinct form is an input row for task T-2 (`m/e5_guards.jsonl`).
Fixed versions are available, so DESIGN decision 7's fallback does not apply: guards may suppress once backed by executed rows.

## Notes on method

Not in this memo, deferred to the stage-D brief: P by payload shape and by shape label (reader labels for the 33 were not made), C by shape
label beyond the table above, and the classifier's guard-token report (`m/classify_on_traced.json`).

Readers are Claude seats (Sonnet-5.5); labels are single-reader. The E3 `strict` flag of the key-reach script requires a non-zero-width
Use inside the index; a zero-width anchor never converts strict. Benchmark source was read, never executed.
