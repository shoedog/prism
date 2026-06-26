"""Invoke `claude -p --model sonnet` in the isolated env, k trials per probe, and cache
each parsed Trajectory keyed by (SKILL.md bytes, probe, trial, model) so re-scoring during
the iteration loop is free and only a real skill edit re-spends (spec §Cost controls)."""
from __future__ import annotations
import hashlib, json, os, subprocess
from .model import Probe, Trajectory
from .trajectory import parse_stream_json
from .env import build_isolated_config, IsolatedConfig

_TIMEOUT = 600

def build_claude_cmd(*, prompt: str, mcp_cfg: str, model: str = "sonnet") -> list[str]:
    return ["claude", "-p", "--output-format", "stream-json", "--verbose",
            "--model", model, "--mcp-config", mcp_cfg, "--strict-mcp-config", prompt]

def cache_key(*, skill_bytes: bytes, probe_id: str, prompt: str, repo: str,
              trial: int, model: str) -> str:
    h = hashlib.sha256()
    h.update(skill_bytes)
    h.update(f"|{probe_id}|{prompt}|{repo}|{trial}|{model}".encode())
    return h.hexdigest()[:24]

def _cache_dir(results_root: str) -> str:
    d = os.path.join(results_root, "cache"); os.makedirs(d, exist_ok=True); return d

def run_trial(probe: Probe, trial: int, *, cfg: IsolatedConfig, eval_root: str,
              results_root: str, skill_bytes: bytes, model: str = "sonnet") -> Trajectory:
    key = cache_key(skill_bytes=skill_bytes, probe_id=probe.id, prompt=probe.prompt,
                    repo=probe.repo, trial=trial, model=model)
    cpath = os.path.join(_cache_dir(results_root), key + ".json")
    if os.path.exists(cpath):
        d = json.load(open(cpath))
        return Trajectory(final_text=d["final_text"], skill_loads=d["skill_loads"],
                          tool_calls=[tuple(x) for x in d["tool_calls"]])
    env = dict(os.environ); env["CLAUDE_CONFIG_DIR"] = cfg.config_dir
    cmd = build_claude_cmd(prompt=probe.prompt, mcp_cfg=cfg.mcp_cfg, model=model)
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=_TIMEOUT,
                          cwd=os.path.join(eval_root, probe.repo), env=env)
    traj = parse_stream_json(proc.stdout)
    with open(cpath, "w") as f:
        json.dump({"final_text": traj.final_text, "skill_loads": traj.skill_loads,
                   "tool_calls": [list(t) for t in traj.tool_calls]}, f)
    return traj
