"""LSP-off enforcement (spec §2.2): a temp dir of failing stub executables for dedicated
type-intelligence binaries + launchers, prepended to an arm's PATH so lsp=False arms cannot
use them. Symmetric across claude/codex. Each stub logs the attempt and exits non-zero.
Compilers (cargo/go/tsc-via-build) are intentionally NOT denied — see spec §2.2 compiler caveat."""
from __future__ import annotations
import os, stat, tempfile

LAUNCHERS = ["npx", "uvx", "mise", "pnpm", "yarn"]  # bypass bare-name shims (spec §2.2 codex new-4)

DENIED = [
    "rust-analyzer", "gopls", "pyright", "pyright-langserver", "basedpyright", "pylsp",
    "ruff-lsp", "typescript-language-server", "tsserver", "tsc", "clangd", "mypy",
] + LAUNCHERS

def make_lsp_deny_shim(log_path: str) -> str:
    """Create the deny-shim dir; return its path (prepend to PATH for lsp-off arms)."""
    d = tempfile.mkdtemp(prefix="tc-lspdeny-")
    for name in DENIED:
        p = os.path.join(d, name)
        with open(p, "w") as f:
            f.write(
                "#!/bin/sh\n"
                f'name=$(basename "$0")\n'
                f'printf \'{{"tool":"%s","argv":"%s"}}\\n\' "$name" "$*" >> {log_path!r}\n'
                'echo "$name disabled (Tier-C lsp=off)" >&2\n'
                "exit 127\n"
            )
        os.chmod(p, os.stat(p).st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    return d
