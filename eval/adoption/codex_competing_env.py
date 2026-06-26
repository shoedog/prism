# eval/adoption/codex_competing_env.py
"""COMPETING codex CODEX_HOME: the real ~/.codex config + skills (auto-loads knowledge-ref's
prism-nav/lsp-nav) + the tuned skill + prism MCP, minus the prism-naming instruction. Pairs with
codex_env.py's isolated builder. Verify-first: the 'no prism hint' condition is recipe-uncertain."""
from __future__ import annotations
import atexit, os, shutil, tempfile

def build_competing_codex_home(*, skill_src: str, mcp_repo: str, prism_mcp_bin: str,
                               root: str | None = None, real_home: str = "~/.codex") -> str:
    base = root or tempfile.mkdtemp(prefix="tc-codex-compete-")
    if root is None:
        atexit.register(shutil.rmtree, base, True)
    os.makedirs(base, exist_ok=True)
    real = os.path.expanduser(real_home)

    # carry the real codex home (auth + skills) so the user's real skills compete; symlink the
    # bulky bits, copy the small ones we must extend.
    ra = os.path.join(real, "auth.json")
    if os.path.exists(ra):
        d = os.path.join(base, "auth.json"); shutil.copy2(ra, d); os.chmod(d, 0o600)
    # skills: copy real skills then add the tuned one
    skills_dir = os.path.join(base, "skills"); os.makedirs(skills_dir, exist_ok=True)
    rs = os.path.join(real, "skills")
    if os.path.isdir(rs):
        for name in os.listdir(rs):
            src = os.path.join(rs, name)
            if os.path.isdir(src):
                shutil.copytree(src, os.path.join(skills_dir, name), symlinks=True,
                                dirs_exist_ok=True)
    dst = os.path.join(skills_dir, os.path.basename(skill_src.rstrip("/")))
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(skill_src, dst)

    # config.toml: copy the real one (keeps user's mcp_servers = competition) + append prism MCP.
    # Deliberately do NOT carry any [projects.*] / instruction keys that name prism (the memory hint).
    lines = []
    rc = os.path.join(real, "config.toml")
    if os.path.exists(rc):
        for ln in open(rc):
            # drop project-instruction blocks that may inject prism hints; keep mcp_servers etc.
            if ln.strip().startswith("[projects."):
                break  # everything after the projects table is instruction config — exclude it
            lines.append(ln)
    lines += ["\n[mcp_servers.prism]\n", f'command = "{prism_mcp_bin}"\n',
              f'args = ["--repo", "{mcp_repo}"]\n']
    with open(os.path.join(base, "config.toml"), "w") as f:
        f.writelines(lines)
    return base
