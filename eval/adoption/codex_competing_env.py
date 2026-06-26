# eval/adoption/codex_competing_env.py
"""COMPETING codex CODEX_HOME: carries the ENTIRE real ~/.codex/config.toml (keeps
[mcp_servers.*], [plugins.*], [marketplaces.*], and [projects.*] = real competition),
then APPENDS [mcp_servers.prism]. The prism-naming hint is absent from config.toml
itself so no instruction key injects it.

No-hint control: ONLY auth.json + skills/ + config.toml are copied.  memories*.sqlite
and any AGENTS.md are deliberately excluded — the competing home carries config/auth/skills
competition but never the memory-based prism hint.

Pairs with codex_env.py's isolated builder."""
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

    # config.toml: carry the ENTIRE real config (mcp_servers, plugins, marketplaces, projects
    # trust tables — all real competition), then APPEND [mcp_servers.prism].
    # B1 fix: read ALL lines without truncating at [projects.*]; the real config.toml has no
    # prism-naming hint so the no-hint control is inherent in the file itself.
    # B2 control (explicit): we ONLY copy auth.json + skills/ + config.toml.
    # memories*.sqlite and AGENTS.md are NOT copied — they carry the memory-based prism hint.
    lines = []
    rc = os.path.join(real, "config.toml")
    if os.path.exists(rc):
        with open(rc) as fh:
            lines = fh.readlines()  # carry the full file — no filtering by section position
    lines += ["\n[mcp_servers.prism]\n", f'command = "{prism_mcp_bin}"\n',
              f'args = ["--repo", "{mcp_repo}"]\n']
    with open(os.path.join(base, "config.toml"), "w") as f:
        f.writelines(lines)
    return base
