"""deepeval suite: `cd eval && uv run deepeval test run adoption/tests/test_prism_adoption.py
  --identifier prism-adoption-round-N -n 5 -i -s`.
Generation is cached by SKILL.md hash (runner.py) so re-scoring is free; editing
skills/prism-code-navigation/SKILL.md invalidates the cache and re-spends."""
from __future__ import annotations
import os; os.environ.setdefault("OPENAI_API_KEY", "sk-adoption-deterministic-gate")
import pytest
from deepeval import assert_test
from adoption.goldens import load_probes
from adoption.env import build_isolated_config
from adoption.runner import run_trial
from adoption.testcase import build_test_case
from adoption.metrics import GATE_METRICS
from adoption.aggregate import summarize, write_benchmark

K = 5
EVAL_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))  # eval/
REPO_ROOT = os.path.dirname(EVAL_ROOT)
SKILL_SRC = os.path.join(REPO_ROOT, "skills", "prism-code-navigation")
RESULTS = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "results")
SKILL_BYTES = open(os.path.join(SKILL_SRC, "SKILL.md"), "rb").read()
PRISM_BIN = os.path.join(REPO_ROOT, "target", "release", "prism-mcp")

_PROBES = load_probes()
_RESULTS: dict = {}
_CFG_CACHE: dict = {}

def _cfg_for(repo_rel: str):
    if repo_rel not in _CFG_CACHE:
        _CFG_CACHE[repo_rel] = build_isolated_config(skill_src=SKILL_SRC,
                                                     mcp_repo=os.path.join(EVAL_ROOT, repo_rel),
                                                     prism_mcp_bin=PRISM_BIN)
    return _CFG_CACHE[repo_rel]

@pytest.mark.parametrize("probe", _PROBES, ids=[p.id for p in _PROBES])
def test_probe(probe):
    cfg = _cfg_for(probe.repo)
    inv, act = [], []
    for trial in range(K):
        traj = run_trial(probe, trial, cfg=cfg, eval_root=EVAL_ROOT, results_root=RESULTS,
                         skill_bytes=SKILL_BYTES, model="sonnet")
        tc = build_test_case(traj, probe)
        fired = set(traj.prism_nav_calls())
        inv.append(bool(fired & set(probe.expected_tools)) if probe.kind == "nav" else bool(fired))
        act.append(traj.loaded_prism_skill())
        # score with deepeval gate metrics (deterministic); nav probes only
        if probe.kind == "nav":
            assert_test(test_case=tc, metrics=GATE_METRICS, run_async=False)
    _RESULTS[probe.id] = {"kind": probe.kind, "invocation": inv, "activation": act}

def teardown_module(_):
    if _RESULTS:
        s = summarize(_RESULTS)
        path = write_benchmark(s, RESULTS, os.environ.get("ADOPT_ID", "latest"))
        print(f"\nnav invocation pass^{K} = {s['nav_invocation_pass5_rate']:.0%} "
              f"(activation {s['nav_activation_pass5_rate']:.0%})  -> {path}")
