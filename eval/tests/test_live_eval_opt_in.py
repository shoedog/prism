"""Exercise the real live-suite bytes in isolated pytest subprocesses.

Never import real eval dependencies or launch a model, even on the unguarded base.
"""
from pathlib import Path
import os
import subprocess
import sys

import pytest


LIVE_SUITE = Path(__file__).resolve().parents[1] / "adoption/tests/test_prism_adoption.py"

# The fake dependency finder is an import tripwire without opt-in. With opt-in,
# it supplies deterministic interfaces so the original test body can execute.
PROBE_PLUGIN = r'''
import importlib.abc
import importlib.util
import json
import os
from pathlib import Path
import sys
from types import SimpleNamespace

def record(event):
    with Path("events.jsonl").open("a") as f:
        f.write(json.dumps(event) + "\n")

class FakeDependencies(importlib.abc.MetaPathFinder, importlib.abc.Loader):
    def find_spec(self, fullname, path=None, target=None):
        if fullname == "deepeval" or fullname == "adoption" or fullname.startswith("adoption."):
            record(["import", fullname])
            if os.environ.get("PROBE_ALLOW_IMPORTS") != "1":
                raise AssertionError("live dependency imported before opt-in")
            return importlib.util.spec_from_loader(fullname, self, is_package=True)

    def create_module(self, spec):
        return None

    def exec_module(self, module):
        def config(**kwargs):
            record(["config"])
            return object()

        def trial(probe, number, **kwargs):
            record(["trial", probe.id, number, kwargs["model"]])
            return SimpleNamespace(
                prism_nav_calls=lambda: ["nav_callers"] if probe.kind == "nav" else [],
                loaded_prism_skill=lambda: True,
            )

        def benchmark(*args):
            record(["benchmark"])
            return "fake-benchmark"

        exports = {
            "deepeval": {"assert_test": lambda **kwargs: record(["score"])},
            "adoption.goldens": {"load_probes": lambda: [
                SimpleNamespace(id="nav", kind="nav", repo="fixture", expected_tools=["nav_callers"]),
                SimpleNamespace(id="negative", kind="negative", repo="fixture", expected_tools=[]),
            ]},
            "adoption.env": {"build_isolated_config": config},
            "adoption.runner": {"run_trial": trial},
            "adoption.testcase": {"build_test_case": lambda *args: object()},
            "adoption.metrics": {"GATE_METRICS": []},
            "adoption.aggregate": {
                "summarize": lambda results: {"nav_invocation_pass5_rate": 1, "nav_activation_pass5_rate": 1},
                "write_benchmark": benchmark,
            },
        }
        module.__dict__.update(exports.get(module.__name__, {}))

sys.meta_path.insert(0, FakeDependencies())
'''


def run_suite(tmp_path, opt_in=None, *, allow_imports=False, args=()):
    root = tmp_path / "repo"
    suite = root / "eval/adoption/tests/test_prism_adoption.py"
    suite.parent.mkdir(parents=True)
    suite.write_bytes(LIVE_SUITE.read_bytes())
    skill = root / "skills/prism-code-navigation/SKILL.md"
    skill.parent.mkdir(parents=True)
    skill.write_text("fake skill, no live model")
    cwd = root / "eval"
    (cwd / "conftest.py").write_text(PROBE_PLUGIN)
    (cwd / "test_ordinary.py").write_text("def test_ordinary(): pass\n")
    env = dict(os.environ)
    env.pop("PRISM_RUN_LIVE_EVALS", None)
    env.pop("PYTEST_ADDOPTS", None)
    env.pop("PYTEST_PLUGINS", None)
    env.pop("PYTHONPATH", None)
    env["PYTEST_DISABLE_PLUGIN_AUTOLOAD"] = "1"
    env["PROBE_ALLOW_IMPORTS"] = "1" if allow_imports else "0"
    if opt_in is not None:
        env["PRISM_RUN_LIVE_EVALS"] = opt_in
    result = subprocess.run(
        [sys.executable, "-m", "pytest", "-q", "-rs", *args], cwd=cwd,
        env=env, capture_output=True, text=True, timeout=30,
    )
    events = cwd / "events.jsonl"
    import json
    return result, [json.loads(line) for line in events.read_text().splitlines()] if events.exists() else []


@pytest.mark.parametrize("opt_in", [None, "", "0", "true", "yes", " 1", "1 "])
def test_default_collection_skips_before_live_imports(tmp_path, opt_in):
    result, events = run_suite(tmp_path, opt_in)
    assert events == [], (events, result.stdout, result.stderr)
    assert result.returncode == 0, (result.stdout, result.stderr)
    assert "1 passed, 1 skipped" in result.stdout
    assert "PRISM_RUN_LIVE_EVALS=1" in result.stdout


@pytest.mark.parametrize("args", [
    ("adoption/tests/test_prism_adoption.py",),
    ("--collect-only",),
])
def test_explicit_path_and_collection_do_not_grant_opt_in(tmp_path, args):
    result, events = run_suite(tmp_path, args=args)
    assert events == [], (events, result.stdout, result.stderr)
    assert "SKIPPED [1]" in result.stdout
    assert "PRISM_RUN_LIVE_EVALS=1" in result.stdout
    # Explicitly selecting only the skipped module has no runnable tests (exit5).
    assert result.returncode == (5 if args[0] != "--collect-only" else 0)


def test_exact_opt_in_runs_existing_trials_through_fakes(tmp_path):
    result, events = run_suite(tmp_path, "1", allow_imports=True)
    assert result.returncode == 0, (result.stdout, result.stderr)
    assert "3 passed" in result.stdout
    assert [e for e in events if e[0] == "trial"] == [
        ["trial", probe, n, "sonnet"] for probe in ["nav", "negative"] for n in range(5)
    ]
    assert events.count(["config"]) == 1
    assert events.count(["score"]) == 5
    assert events.count(["benchmark"]) == 1


def test_opted_in_collect_only_does_not_run_trials_or_create_config(tmp_path):
    result, events = run_suite(tmp_path, "1", allow_imports=True, args=("--collect-only",))
    assert result.returncode == 0, (result.stdout, result.stderr)
    assert "3 tests collected" in result.stdout
    assert events and all(e[0] == "import" for e in events)
