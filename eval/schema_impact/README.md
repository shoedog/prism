# schema_impact (lane 5 prototype)

The feasibility prototype and the version-2 report renderer for the schema-aware service impact lane
(design: `docs/superpowers/plans/2026-11-20-lanes/lane5-schema-impact/`). It is its **own uv project**
(`pyproject.toml` and `uv.lock` here) and is not part of the shared eval harness in `eval/`, whose `uv.lock`
is pinned by the Tier-A lock.

```bash
cd eval/schema_impact
uv sync --locked
uv run --locked pytest -q                                # the prototype's tests (tests/)
PYTHONPATH=.. uv run --locked python -m schema_impact.cli --help   # the commands (ids, producer, routes, consumer, mismatch, replay, report)
```

The shared eval project's pytest run ignores this directory (`conftest.py`).
