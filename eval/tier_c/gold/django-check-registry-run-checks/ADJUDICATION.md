# django-check-registry-run-checks — ADJUDICATION

Status: DRAFT — controller+Fable review pending.

## Source And Scope

- Repo: `~/code/bench-repos/django` at `e8cff29`.
- Target: `CheckRegistry.run_checks` in `django/core/checks/registry.py:73`.
- Scope applied: runner/registry collection layer plus registered check callbacks under `django/core/checks` and explicit contrib app check registrations.
- Scope excluded: generic Django `register` decorators and methods outside the system-check registry, tests, and docs.
- Generator of record: grep-per-hop/callback only. No LSP/prism/agent enumeration was used.

## Closure Walk

L0 — `run_checks`

- Probe: `git -C ~/code/bench-repos/django grep -nw run_checks`
- Count: 74 raw lines / 14 files; repo-wide word count is 75.
- Real production runner sites admitted: `CheckRegistry.run_checks`, module alias `run_checks = registry.run_checks`, package reexport in `django/core/checks/__init__.py`, and `BaseCommand.check`.
- Excluded: tests/docs and same-name other receiver `DiscoverRunner.run_checks` in `django/test/runner.py`.

Hop 1 — `get_checks`

- Probe: `git -C ~/code/bench-repos/django grep -nw get_checks`
- Count: 3 raw lines / 1 file.
- Real sites admitted: `CheckRegistry.get_checks`, the `CheckRegistry.run_checks` call to it, and `CheckRegistry.tags_available` as a consumer of collection semantics.
- Termination: `tags_available` consumes tag-list semantics and is not a forwarder into check execution.

Hop 2/3 — scoped system-check registration callbacks

- Raw noisy probe: `git -C ~/code/bench-repos/django grep -nw register`
- Count: 923 raw lines / 169 files, dominated by admin/template/library/register collisions.
- Scoped probes:
  - `git -C ~/code/bench-repos/django grep -n "@register" -- django/core/checks` = 40 decorator lines.
  - `git -C ~/code/bench-repos/django grep -n "checks\\.register" -- django/contrib/{admin,auth,contenttypes,sites,staticfiles}/apps.py` = 10 app registration lines.
- Real callback sites admitted: 50 registered check functions.
- Receiver evidence:
  - Core checks import `register` from `django.core.checks` / `.`, which reexports `registry.register`.
  - Contrib app configs import `checks` from `django.core` and pass concrete check functions to `checks.register(...)`.
- Termination: check function bodies are callback consumers invoked by `CheckRegistry.run_checks`; no further recursion.

## Counts

- Real gold sites: 58.
- D1 site count: 50.
- D1 files: 18 files under `django/core/checks` and contrib check modules.
- Scorer D denominator is file-level: `d_gold_size=18`.
- D2 count: 0. Verified repo-wide `run_checks` word count is 75, below the >100 D2 threshold.
- Admission: PASS (`8 <= 58 <= 60`, D1 sites `50 >= 3`).
- Ceiling caution: this is close to the 60-site cap. Adding separate `AppConfig.ready` registration sites in addition to check functions would exceed the cap; I represented programmatic app registrations as the actual check function sites with cross-file `register@apps.py` anchors.

## Dry Run

Command run from `eval/`:

```text
uv run python -c "from tier_c.structural import score_structural,load_gold; g=load_gold('tier_c/gold/django-check-registry-run-checks/gold.json'); perfect=[{'file':s['file'],'symbol':s['symbol']} for s in g['sites'] if s['adjudication']=='real']; r=score_structural(perfect,g); print('perfect',r.file_f1,r.d_recall,r.gold_size,r.d_gold_size,r.phantom)"
```

Output:

```text
perfect 1.0 1.0 58 18 0
```

## Exclusions

- Same-name other receiver: `django/test/runner.py:1051` / `1132` (`DiscoverRunner.run_checks`).
- Tests/docs: `tests/admin_checks`, `tests/auth_tests`, `tests/check_framework`, `tests/contenttypes_tests`, `tests/test_runner`, `docs/releases/1.11.txt`, `docs/topics/testing/advanced.txt`.
- Non-check `register` collisions: `django/contrib/admin/decorators.py`, `django/contrib/admin/filters.py`, admin/auth/flatpages/humanize template tag libraries, `django/contrib/gis/db/backends/postgis/base.py`, `django/contrib/postgres/signals.py`.

## Uncertain / Review

- Programmatic contrib registrations are represented by the check function definitions, with token anchors at the `checks.register(...)` call in `apps.py`. This matches the prompt's "registered check functions" wording but differs from a strict "enclosing symbol of the register call" interpretation, which would use `AppConfig.ready`.
- I included `CheckRegistry.tags_available` because it consumes `get_checks` collection semantics. It does not invoke checks and could be excluded under a narrower "execution only" reading.
- I included `CheckRegistry.register` and module/package reexports as registry forwarder sites. They are part of the callback dispatch path, but the task is close enough to the 60-site ceiling that controller may prefer a narrower runner+callbacks-only shape.
