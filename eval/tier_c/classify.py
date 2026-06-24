"""Classify an arm's recorded commands (spec §3): did it reach a dedicated LSP/type-checker
(lsp_leak — should be impossible for lsp-off arms via the shim, flags bypass) or a compiler
type-check (compiler_assisted — the 'no dedicated LSP' caveat, reported per-protocol)."""
from __future__ import annotations
import re
from .lspshim import DENIED

_LSP = re.compile(r"\b(" + "|".join(re.escape(t) for t in DENIED if t not in {"npx", "uvx", "mise"}) + r")\b")
_COMPILER = re.compile(r"\b(cargo\s+(check|clippy|build)|go\s+(vet|build)|rustc|tsc)\b")

def classify_tools(commands: list[str]) -> dict:
    joined = "\n".join(commands)
    return {"lsp_leak": bool(_LSP.search(joined)),
            "compiler_assisted": bool(_COMPILER.search(joined))}
