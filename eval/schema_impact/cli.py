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
    args = parser.parse_args(argv)

    if args.command == "producer":
        return producer_go.run(args)
    if args.command == "routes":
        return producer_route_table.run(args)
    if args.command == "consumer":
        from .f2 import run

        run(args.repo, args.root, args.schema, args.config, args.out)
        return 0

    index = load(args.schema)
    for group in (index.operations, index.fields):
        for element in sorted(group):
            print(element)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
