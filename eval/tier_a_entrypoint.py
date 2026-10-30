"""Source-minimal outer shim for an isolated Tier-A child."""
from __future__ import annotations

import os
import secrets
import shutil
import sys
import tempfile
from pathlib import Path


_PROTOCOL = "tier-a-isolated-launch-v2"
_NONCE_FILE = ".tier-a-launch-nonce"
_CHILD_CODE = (
    "import sys; root=sys.argv.pop(1); sys.path.insert(0, root); "
    "from tier_a_launch import main; raise SystemExit(main(sys.argv[1:]))"
)


def _remove_private_cache(path: str) -> None:
    cache = Path(path)
    if cache.is_dir() and cache.name.startswith("tier-a-pycache-"):
        shutil.rmtree(cache)


def _exec_private_child() -> None:
    private_cache = tempfile.mkdtemp(prefix="tier-a-pycache-")
    nonce = secrets.token_hex(32)
    (Path(private_cache) / _NONCE_FILE).write_text(f"{_PROTOCOL}:{nonce}\n")
    env = dict(os.environ)
    env.pop("PYTHONPATH", None)
    env.pop("PYTHONPYCACHEPREFIX", None)
    env.pop("TIER_A_PRIVATE_PYCACHE", None)
    env.pop("TIER_A_PRIVATE_PYCACHE_FD", None)
    env["PYTHONDONTWRITEBYTECODE"] = "1"
    try:
        os.execve(
            sys.executable,
            [
                sys.executable,
                "-I",
                "-X",
                f"pycache_prefix={private_cache}",
                "-B",
                "-c",
                _CHILD_CODE,
                str(Path(__file__).resolve().parent),
                private_cache,
                _PROTOCOL,
                nonce,
                *sys.argv[1:],
            ],
            env,
        )
        raise AssertionError("os.execve returned")
    finally:
        _remove_private_cache(private_cache)


def main() -> int:
    _exec_private_child()
    raise AssertionError("isolated child exec returned")
