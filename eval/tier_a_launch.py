"""Validate the outer launch before importing any ``tier_a`` module."""
from __future__ import annotations

import atexit
import shutil
import sys
from pathlib import Path


PROTOCOL = "tier-a-isolated-launch-v2"
NONCE_FILE = ".tier-a-launch-nonce"


def validate_launch(prefix: str, protocol: str, nonce: str) -> dict:
    cache = Path(prefix)
    if not sys.flags.isolated:
        raise RuntimeError("tier_a_launch_not_isolated")
    if protocol != PROTOCOL:
        raise RuntimeError("tier_a_launch_protocol_mismatch")
    configured = sys.pycache_prefix
    if configured is None or Path(configured).resolve() != cache.resolve():
        raise RuntimeError("tier_a_launch_prefix_mismatch")
    nonce_file = cache / NONCE_FILE
    if not cache.is_dir() or sorted(cache.iterdir()) != [nonce_file]:
        raise RuntimeError("tier_a_launch_prefix_not_owned")
    if nonce_file.read_text() != f"{PROTOCOL}:{nonce}\n":
        raise RuntimeError("tier_a_launch_prefix_not_owned")
    nonce_file.unlink()
    if any(cache.iterdir()):
        raise RuntimeError("tier_a_launch_prefix_not_fresh")
    return {"isolated": True, "prefix_fresh": True, "protocol": PROTOCOL}


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        raise RuntimeError("tier_a_launch_protocol_missing")
    prefix, protocol, nonce, *cli_argv = argv
    provenance = validate_launch(prefix, protocol, nonce)
    atexit.register(shutil.rmtree, prefix, ignore_errors=True)
    from tier_a import launch

    launch.authorize(provenance)
    sys.argv = ["tier-a", *cli_argv]
    from tier_a.cli import main as cli_main

    return cli_main()
