"""Command line entry point for the schema impact feasibility prototype."""

import argparse
from pathlib import Path

from . import producer_go, producer_route_table
from .schema import load


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="schema-impact")
    commands = parser.add_subparsers(dest="command", required=True)
    ids = commands.add_parser("ids", help="Print sorted operation and field IDs")
    ids.add_argument("schema")
    producer = commands.add_parser("producer", help="Measure Go producer bindings and route drift")
    producer_go.add_arguments(producer)
    routes = commands.add_parser("routes", help="Extract literal JavaScript route tables")
    producer_route_table.add_arguments(routes)
    consumer = commands.add_parser("consumer", help="Measure TypeScript and JavaScript consumer bindings")
    consumer.add_argument("--repo", type=Path, required=True)
    consumer.add_argument("--root", required=True)
    consumer.add_argument("--schema", type=Path, required=True)
    consumer.add_argument("--config", type=Path, required=True)
    consumer.add_argument("--out", type=Path, required=True)
    mismatch = commands.add_parser("mismatch", help="List standing producer and consumer mismatches")
    mismatch.add_argument("--producer", type=Path, required=True)
    mismatch.add_argument("--consumer", type=Path, required=True)
    mismatch.add_argument("--schema", type=Path, required=True)
    mismatch.add_argument("--out", type=Path, required=True)
    replay = commands.add_parser("replay", help="Replay producer changes against historical client calls")
    replay.add_argument("--repo", type=Path, required=True)
    replay.add_argument("--schema", type=Path, required=True)
    replay.add_argument("--configs", type=Path, required=True)
    replay.add_argument("--root", required=True)
    replay.add_argument("--pin", required=True)
    replay.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)

    if args.command == "producer":
        return producer_go.run(args)
    if args.command == "routes":
        return producer_route_table.run(args)
    if args.command == "replay":
        from ._replay_measure import run

        run(args.repo, args.schema, args.configs, args.root, args.pin, args.out)
        return 0
    if args.command == "consumer":
        from .f2 import run

        run(args.repo, args.root, args.schema, args.config, args.out)
        return 0

    index = load(args.schema)
    if args.command == "mismatch":
        from .mismatch import run

        run(args.producer, args.consumer, index, args.out)
        return 0

    for group in (index.operations, index.fields):
        for element in sorted(group):
            print(element)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
