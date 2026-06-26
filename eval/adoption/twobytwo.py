# eval/adoption/twobytwo.py
"""2x2 driver (prompt-realism x skill-competition). Builds each cell's env, runs its samples x k
trials via run_trial (cached), scores with prism_invoked, writes results/twobytwo-<id>.json.
Cells: 1=micro+isolated (reuse cache), 2=micro+competing, 3=realistic+isolated, 4=realistic+competing."""
from __future__ import annotations
import json, os
from .goldens import load_probes, load_realistic_probes
from .env import build_isolated_config
from .competing_env import build_competing_config
from .codex_env import build_isolated_codex_home
from .codex_competing_env import build_competing_codex_home
from .runner import run_trial
from .aggregate import prism_invoked, summarize_cells

K = 5
def _skill_basename(s: str) -> str:
    """Extract the skill dir name from a skill_load string: '.../<name>/SKILL.md' or '<name>'."""
    s = s.strip()
    head = s.split("/SKILL.md")[0] if "/SKILL.md" in s else s
    return head.rstrip("/").split("/")[-1].lower()

def loaded_skill_name(traj) -> str | None:
    """The prism-related skill the model loaded (for attribution); else the first loaded skill."""
    names = [_skill_basename(s) for s in traj.skill_loads]
    for n in names:
        if "prism" in n:
            return n
    return names[0] if names else None

def run_cell(cell_id, probes, *, env_kind, model, eval_root, results_root, skill_src,
             prism_mcp_bin, repo="tier_c"):
    """env_kind in {'isolated','competing'}. Returns {sample_id: [(invoked, skill_name)] * K}."""
    skill_bytes = open(os.path.join(skill_src, "SKILL.md"), "rb").read()
    repo_path = os.path.join(eval_root, repo)
    is_codex = model.startswith("gpt")
    cfg = codex_home = None
    if is_codex:
        codex_home = (build_competing_codex_home if env_kind == "competing" else build_isolated_codex_home)(
            skill_src=skill_src, mcp_repo=repo_path, prism_mcp_bin=prism_mcp_bin)
    else:
        cfg = (build_competing_config if env_kind == "competing" else build_isolated_config)(
            skill_src=skill_src, mcp_repo=repo_path, prism_mcp_bin=prism_mcp_bin)
    out = {}
    for p in probes:
        trials = []
        for t in range(K):
            traj = run_trial(p, t, cfg=cfg, eval_root=eval_root, results_root=results_root,
                             skill_bytes=skill_bytes + env_kind.encode(),  # env in the cache key
                             model=model, skill_src=skill_src, prism_mcp_bin=prism_mcp_bin,
                             codex_home=codex_home)
            trials.append((prism_invoked(traj), loaded_skill_name(traj)))
        out[p.id] = trials
    return out

def run_2x2(*, model, eval_root, results_root, skill_src, prism_mcp_bin, cells=("1","2","3","4"),
            identifier="latest"):
    micro, real = load_probes(), load_realistic_probes()
    spec = {"1": (micro, "isolated"), "2": (micro, "competing"),
            "3": (real, "isolated"), "4": (real, "competing")}
    data = {}
    for c in cells:
        probes, env_kind = spec[c]
        data[f"cell{c}"] = run_cell(c, probes, env_kind=env_kind, model=model, eval_root=eval_root,
                                    results_root=results_root, skill_src=skill_src,
                                    prism_mcp_bin=prism_mcp_bin)
    summary = summarize_cells(data)
    os.makedirs(results_root, exist_ok=True)
    path = os.path.join(results_root, f"twobytwo-{identifier}.json")
    json.dump({"model": model, "cells": summary, "raw": data}, open(path, "w"), indent=2)
    return summary, path
