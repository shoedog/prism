"""Invoke `claude -p --model sonnet` (or `codex exec --json -m gpt-*`) in the isolated env,
k trials per probe, and cache each parsed Trajectory keyed by (SKILL.md bytes, probe, trial,
model) so re-scoring during the iteration loop is free and only a real skill edit re-spends
(spec §Cost controls).

Model dispatch:
  - model starts with "gpt" → codex path (build_isolated_codex_home + CODEX_HOME env +
    --ignore-user-config + parse_codex_stream)
  - otherwise              → claude path (existing CLAUDE_CONFIG_DIR + parse_stream_json)
"""
from __future__ import annotations
import hashlib, json, os, subprocess
from .model import Probe, Trajectory
from .trajectory import parse_stream_json, parse_codex_stream
from .env import build_isolated_config, IsolatedConfig

_TIMEOUT = 600
_CODEX_TIMEOUT = 600  # codex is slower; keep same ceiling

def build_claude_cmd(*, prompt: str, mcp_cfg: str, model: str = "sonnet") -> list[str]:
    return ["claude", "-p", "--output-format", "stream-json", "--verbose",
            "--model", model, "--mcp-config", mcp_cfg, "--strict-mcp-config", prompt]


def build_codex_cmd(*, prompt: str, model: str = "gpt-5.5") -> list[str]:
    """Build a `codex exec --json` command.

    CODEX_HOME must be set by the caller via env; --ignore-user-config prevents the
    user's ~/.codex/config.toml from injecting extra skills.
    The -C (cwd) flag is NOT embedded here; the caller sets cwd via subprocess.run cwd=.
    """
    return [
        "codex", "exec",
        "--json",
        "--ignore-user-config",
        "-m", model,
        "-s", "read-only",
        prompt,
    ]


def cache_key(*, skill_bytes: bytes, probe_id: str, prompt: str, repo: str,
              trial: int, model: str) -> str:
    h = hashlib.sha256()
    h.update(skill_bytes)
    h.update(f"|{probe_id}|{prompt}|{repo}|{trial}|{model}".encode())
    return h.hexdigest()[:24]

def _cache_dir(results_root: str) -> str:
    d = os.path.join(results_root, "cache"); os.makedirs(d, exist_ok=True); return d


def _run_codex_trial(probe: Probe, *, eval_root: str, skill_src: str,
                     prism_mcp_bin: str, model: str) -> Trajectory:
    """Run one codex trial: build an isolated CODEX_HOME, exec, parse."""
    from .codex_env import build_isolated_codex_home
    repo_path = os.path.join(eval_root, probe.repo)
    codex_home = build_isolated_codex_home(
        skill_src=skill_src,
        mcp_repo=repo_path,
        prism_mcp_bin=prism_mcp_bin,
    )
    env = dict(os.environ)
    env["CODEX_HOME"] = codex_home
    cmd = build_codex_cmd(prompt=probe.prompt, model=model)
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=_CODEX_TIMEOUT,
                          cwd=repo_path, env=env)
    return parse_codex_stream(proc.stdout)


def run_trial(probe: Probe, trial: int, *, cfg: IsolatedConfig, eval_root: str,
              results_root: str, skill_bytes: bytes, model: str = "sonnet",
              # codex-only kwargs (ignored for claude)
              skill_src: str | None = None,
              prism_mcp_bin: str | None = None) -> Trajectory:
    key = cache_key(skill_bytes=skill_bytes, probe_id=probe.id, prompt=probe.prompt,
                    repo=probe.repo, trial=trial, model=model)
    cpath = os.path.join(_cache_dir(results_root), key + ".json")
    if os.path.exists(cpath):
        d = json.load(open(cpath))
        return Trajectory(final_text=d["final_text"], skill_loads=d["skill_loads"],
                          tool_calls=[tuple(x) for x in d["tool_calls"]])

    if model.startswith("gpt"):
        # Codex path
        if skill_src is None or prism_mcp_bin is None:
            raise ValueError("skill_src and prism_mcp_bin are required for codex (gpt*) models")
        traj = _run_codex_trial(probe, eval_root=eval_root, skill_src=skill_src,
                                prism_mcp_bin=prism_mcp_bin, model=model)
    else:
        # Claude path (original)
        env = dict(os.environ); env["CLAUDE_CONFIG_DIR"] = cfg.config_dir
        cmd = build_claude_cmd(prompt=probe.prompt, mcp_cfg=cfg.mcp_cfg, model=model)
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=_TIMEOUT,
                              cwd=os.path.join(eval_root, probe.repo), env=env)
        traj = parse_stream_json(proc.stdout)

    with open(cpath, "w") as f:
        json.dump({"final_text": traj.final_text, "skill_loads": traj.skill_loads,
                   "tool_calls": [list(t) for t in traj.tool_calls]}, f)
    return traj
