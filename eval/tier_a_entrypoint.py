"""Import-safe Tier-A console bootstrap with a private bytecode cache."""
from __future__ import annotations

import atexit
import os
import shutil
import sys
import tempfile
from pathlib import Path


_PRIVATE_CACHE_ENV = "TIER_A_PRIVATE_PYCACHE"


def _remove_private_cache(path: str) -> None:
    cache = Path(path)
    if cache.is_dir() and cache.name.startswith("tier-a-pycache-"):
        shutil.rmtree(cache)


def main() -> int:
    private_cache = os.environ.get(_PRIVATE_CACHE_ENV)
    if private_cache is None:
        private_cache = tempfile.mkdtemp(prefix="tier-a-pycache-")
        env = dict(os.environ)
        env[_PRIVATE_CACHE_ENV] = private_cache
        env["PYTHONPYCACHEPREFIX"] = private_cache
        os.execve(
            sys.executable,
            [sys.executable, "-m", "tier_a_entrypoint", *sys.argv[1:]],
            env,
        )
        raise AssertionError("os.execve returned")

    configured = sys.pycache_prefix
    if configured is None or Path(configured).resolve() != Path(private_cache).resolve():
        raise RuntimeError("tier_a_private_pycache_mismatch")
    atexit.register(_remove_private_cache, private_cache)

    from tier_a.cli import main as cli_main

    return cli_main()


if __name__ == "__main__":
    raise SystemExit(main())
