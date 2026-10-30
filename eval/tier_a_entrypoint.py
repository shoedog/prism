"""Import-safe Tier-A console bootstrap with a private bytecode cache."""
from __future__ import annotations

import atexit
import os
import shutil
import sys
import tempfile
from pathlib import Path


_PRIVATE_CACHE_ENV = "TIER_A_PRIVATE_PYCACHE"
_PRIVATE_CHILD_FD_ENV = "TIER_A_PRIVATE_PYCACHE_FD"
_PRIVATE_CHILD_ARG = "--_tier-a-private-pycache-child"
_PRIVATE_CHILD_TOKEN = b"tier-a-private-pycache-v1"


def _remove_private_cache(path: str) -> None:
    cache = Path(path)
    if cache.is_dir() and cache.name.startswith("tier-a-pycache-"):
        shutil.rmtree(cache)


def _exec_private_child() -> None:
    private_cache = tempfile.mkdtemp(prefix="tier-a-pycache-")
    read_fd, write_fd = os.pipe()
    try:
        os.set_inheritable(read_fd, True)
        os.write(write_fd, _PRIVATE_CHILD_TOKEN)
        os.close(write_fd)
        write_fd = -1
        env = dict(os.environ)
        env[_PRIVATE_CACHE_ENV] = private_cache
        env[_PRIVATE_CHILD_FD_ENV] = str(read_fd)
        env["PYTHONPYCACHEPREFIX"] = private_cache
        os.execve(
            sys.executable,
            [
                sys.executable,
                "-B",
                "-m",
                "tier_a_entrypoint",
                _PRIVATE_CHILD_ARG,
                *sys.argv[1:],
            ],
            env,
        )
        raise AssertionError("os.execve returned")
    finally:
        if write_fd >= 0:
            os.close(write_fd)
        os.close(read_fd)
        _remove_private_cache(private_cache)


def _authorized_private_cache() -> str | None:
    if len(sys.argv) < 2 or sys.argv[1] != _PRIVATE_CHILD_ARG:
        return None
    private_cache = os.environ.get(_PRIVATE_CACHE_ENV)
    raw_fd = os.environ.pop(_PRIVATE_CHILD_FD_ENV, None)
    if private_cache is None or raw_fd is None:
        raise RuntimeError("tier_a_private_pycache_unauthorized_child")
    try:
        read_fd = int(raw_fd)
        token = os.read(read_fd, len(_PRIVATE_CHILD_TOKEN) + 1)
    except (OSError, ValueError) as exc:
        raise RuntimeError("tier_a_private_pycache_unauthorized_child") from exc
    finally:
        if "read_fd" in locals():
            os.close(read_fd)
    if token != _PRIVATE_CHILD_TOKEN:
        raise RuntimeError("tier_a_private_pycache_unauthorized_child")
    del sys.argv[1]
    return private_cache


def main() -> int:
    private_cache = _authorized_private_cache()
    if private_cache is None:
        _exec_private_child()

    configured = sys.pycache_prefix
    if configured is None or Path(configured).resolve() != Path(private_cache).resolve():
        raise RuntimeError("tier_a_private_pycache_mismatch")
    cache = Path(private_cache)
    if not cache.is_dir() or any(cache.iterdir()):
        raise RuntimeError("tier_a_private_pycache_not_fresh")
    atexit.register(_remove_private_cache, private_cache)

    from tier_a.cli import main as cli_main

    return cli_main()


if __name__ == "__main__":
    raise SystemExit(main())
