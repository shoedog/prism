"""Command line for the corpus registry and guarded measurements."""

from __future__ import annotations

import argparse
from pathlib import Path

from .corpora import DEFAULT_MANIFEST, list_corpora, load_manifest, verify_corpora
from .receipt import trace_settings
from .run import DEFAULT_GUARD, run


def positive_integer(value):
    if not value.isascii() or not value.isdecimal() or int(value) < 1:
        raise argparse.ArgumentTypeError("expected a positive integer")
    return int(value)


def corpus_roots(parser):
    parser.add_argument("--corpora-root", action="append", type=Path, default=[])


def main(argv=None, *, gate_sha256=None, gate_step_id=None, prediction_sha256=None):
    parser = argparse.ArgumentParser(prog="measure")
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    commands = parser.add_subparsers(dest="command", required=True)
    corpora = commands.add_parser("corpora").add_subparsers(dest="operation", required=True)
    corpora.add_parser("list")
    verification = corpora.add_parser("verify")
    corpus_roots(verification)
    verification.add_argument("keys", nargs="*")

    measurement = commands.add_parser("run")
    measurement.add_argument("--binary", type=Path, required=True)
    measurement.add_argument("--corpus", required=True)
    measurement.add_argument("--out", type=Path, required=True)
    corpus_roots(measurement)
    measurement.add_argument("--trace-env", action="append", default=[])
    measurement.add_argument("--no-bytes", action="store_true")
    measurement.add_argument("--sites", action="store_true")
    measurement.add_argument("--prediction", type=Path)
    measurement.add_argument("--memguard", type=Path, default=DEFAULT_GUARD)
    measurement.add_argument("--limit-gb", type=positive_integer, default=8)
    measurement.add_argument("--expect-version")
    args = parser.parse_args(argv)

    try:
        entries = load_manifest(args.manifest)
        if args.command == "corpora":
            if args.operation == "list":
                return list_corpora(entries)
            keys = list(dict.fromkeys(args.keys)) or sorted(entries)
            if any(key not in entries for key in keys):
                raise ValueError("unknown corpus key(s): " + ", ".join(k for k in keys if k not in entries))
            return verify_corpora(entries, keys, args.corpora_root)
        if args.corpus not in entries:
            raise ValueError(f"unknown corpus key: {args.corpus}")
        overrides = trace_settings(args.trace_env)
    except (ValueError, OSError) as error:
        parser.error(str(error))
    return run(args, entries[args.corpus], overrides, gate_sha256=gate_sha256,
               gate_step_id=gate_step_id, prediction_sha256=prediction_sha256)
