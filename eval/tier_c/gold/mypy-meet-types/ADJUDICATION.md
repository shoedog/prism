# mypy-meet-types — ADJUDICATION

Status: DRAFT — controller+Fable review pending.

## Source And Scope

- Repo: `~/code/bench-repos/mypy` at `5ef0902`.
- Target: `meet_types` in `mypy/meet.py:75`.
- Scope applied: direct production callers of `meet_types`, internal `mypy/meet.py` recursive/helper callers, thin wrapper `meet_type_list`, thin internal wrapper `TypeMeetVisitor.meet`, and task-scoped wrapper `narrow_declared_type` consumers.
- Tests and fixtures excluded: `mypy/test/testtypes.py`, `test-data/unit/check-typeform.test`.
- Generator of record: grep-per-hop only. No LSP/prism/agent enumeration was used.

## Closure Walk

L0 — `meet_types`

- Probe: `git -C ~/code/bench-repos/mypy grep -nw meet_types`
- Count: 47 raw lines / 12 files.
- Definition excluded: `mypy/meet.py:75`.
- Import/comment/test surfaces excluded: import-only hits, `expandtype.py`/`types_utils.py` comments, `join.py:838` comment, `mypy/test/testtypes.py`, and `test-data/unit/check-typeform.test`.
- Real L0 sites: 21 sites after collapsing by enclosing function/method.
- Forwarders:
  - `meet_type_list` at `meet.py:1269`, token `meet_types@1276`; thin homogeneous aggregation over a list.
  - `TypeMeetVisitor.meet` at `meet.py:1227`, token `meet_types@1228`; exact method wrapper.
  - `narrow_declared_type` at `meet.py:117`, token `meet_types@232`; admitted as task-scoped wrapper to reach requested optional consumers, but thinness is borderline.
- Consumers include checker narrowing, conditional map merging, join visitor parameter handling, `safe_meet`, attrs/dataclasses plugins, constraint solving, callable argument merging, and internal visitor/helper methods.

Hop 1 — `meet_type_list`

- Probe: `git -C ~/code/bench-repos/mypy grep -nw meet_type_list`
- Count: 5 raw lines / 3 files.
- Excluded: definition at `meet.py:1269`; imports at `solve.py:13` and `suggestions.py:40`.
- Real consumers: `solve.py:choose_free` and `suggestions.py:SuggestionFinder.get_args`.
- D1: `suggestions.py` only; `solve.py` also imports/calls `meet_types`.

Hop 1 — `TypeMeetVisitor.meet`

- Probe used: `git -C ~/code/bench-repos/mypy grep -n "self\\.meet" -- mypy/meet.py`
- Count: 10 raw lines / 1 file.
- Excluded: `self.meet_tuples` sibling hit at `meet.py:1086`.
- New real consumers after collapse/dedup: `TypeMeetVisitor.visit_type_var` and `TypeMeetVisitor.visit_type_type`.
- Other receiver-verified hits collapse into already-admitted direct `meet_types` sites: `visit_instance` and `meet_tuples`.

Hop 1 — `narrow_declared_type`

- Probe: `git -C ~/code/bench-repos/mypy grep -nw narrow_declared_type`
- Count: 17 raw lines / 4 files.
- Excluded: definition, recursive self-calls inside `narrow_declared_type`, imports, and tests.
- Real D1 consumers: `checkexpr.py:ExpressionChecker.narrow_type_from_binder` and `checkpattern.py:PatternTypeVisitor.visit_class_pattern`.
- Thinness caveat: `narrow_declared_type` has substantial narrowing logic and is not a pure forwarder under the strict methodology. I included the hop because the task-specific scope explicitly mentions optional `narrow_declared_type` consumers and the prompt asks for helpers that wrap the meet operation.

## Counts

- Real gold sites: 25.
- D1 site count: 3.
- D1 files: `mypy/checkexpr.py`, `mypy/checkpattern.py`, `mypy/suggestions.py`.
- Scorer D denominator is file-level: `d_gold_size=3`.
- D2 count: 0. Verified repo-wide `meet_types` word count is 47, below the >100 D2 threshold.
- Admission: PASS by stated gate (`8 <= 25 <= 60`, D1 sites `3 >= 3`).
- Methodology caution: `d_gold_size=3` is below the do-not-trust headline denominator of 5; controller should consider this when using the task in headline reporting.

## Dry Run

Command run from `eval/`:

```text
uv run python -c "from tier_c.structural import score_structural,load_gold; g=load_gold('tier_c/gold/mypy-meet-types/gold.json'); perfect=[{'file':s['file'],'symbol':s['symbol']} for s in g['sites'] if s['adjudication']=='real']; r=score_structural(perfect,g); print('perfect',r.file_f1,r.d_recall,r.gold_size,r.d_gold_size,r.phantom)"
```

Output:

```text
perfect 1.0 1.0 25 3 0
```

## Exclusions

- Definition: `mypy/meet.py:75`.
- Import-only: `checker.py:111`, `join.py:334`, `join.py:839`, `plugins/attrs.py:15`, `plugins/dataclasses.py:10`, `solve.py:13`, `typeops.py:569`.
- Comment-only: `expandtype.py:50`, `join.py:838`, `types_utils.py:3`.
- Tests/fixtures: `mypy/test/testtypes.py:11`, `mypy/test/testtypes.py:1396`, `test-data/unit/check-typeform.test:465`.
- Receiver/sibling collision from `self.meet` probe: `meet.py:1086` (`self.meet_tuples`, not `TypeMeetVisitor.meet`).

## Uncertain / Review

- `narrow_declared_type` is the main adjudication question. I admitted it as a task-scoped wrapper to include the optional consumers requested in the task entry, but it is not strictly thin.
- `TypeMeetVisitor.meet` hop used the receiver-specific textual probe `self.meet` rather than raw `git grep -nw meet`, because raw `meet` is polluted by comments/local variables. The receiver evidence is confined to `TypeMeetVisitor`.
- The D-file denominator is only 3. This passes the local admission gate but should be reviewed against the methodology's denominator caution.
