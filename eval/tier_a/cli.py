"""Argument parsing and console plumbing for Tier-A."""
from __future__ import annotations

import argparse
import sys
import types
from pathlib import Path

from . import commands, runner


class _RunnerCompatibility(types.ModuleType):
    def __getattr__(self, name):
        return getattr(runner, name)

    def __setattr__(self, name, value):
        if hasattr(runner, name):
            setattr(runner, name, value)
        else:
            super().__setattr__(name, value)


sys.modules[__name__].__class__ = _RunnerCompatibility


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(prog="tier-a")
    result.add_argument("--corpus", default=None)
    result.add_argument("--lang", help="filter selected corpora, e.g. ts,js")
    result.add_argument(
        "--sample",
        type=int,
        help="symbols per stratum; overrides quick's smoke sample of 3",
    )
    result.add_argument(
        "--out-dir",
        type=Path,
        help="report directory (keeps exploratory runs separate from baseline)",
    )
    result.add_argument("--query-timeout-s", type=float)
    result.add_argument("--startup-timeout-s", type=float)
    result.add_argument("--oracle-budget-s", type=float)
    result.add_argument("--quick", action="store_true")
    result.add_argument("--matrix-only", action="store_true")
    result.add_argument("--report-only")
    result.add_argument("--sut-bin")
    result.add_argument("--allow-stale-sut", action="store_true")
    result.add_argument(
        "--live", action="store_true", help="measure the working tree, never anchoring"
    )
    result.add_argument("--run-id")
    result.add_argument("--sut-archive")
    result.add_argument(
        "--oracle",
        default=None,
        help=(
            "override per-corpus oracle (e.g. basedpyright); outputs are written "
            "under <corpus>-<oracle> so they do not clobber committed baselines"
        ),
    )
    result.add_argument("--date", default=None)
    return result


def main() -> int:
    return commands.dispatch_or_run(sys.argv[1:], parser())
