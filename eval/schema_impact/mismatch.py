"""Join the F1 producer residues and F2 client census into the four F8 lists."""

from collections.abc import Iterator
from dataclasses import fields, replace
import json
from pathlib import Path
from typing import Any

from .consumer_ts import ClientCall
from .match import bind
from .schema import SchemaIndex


type Row = dict[str, Any]

_CLIENT_FIELDS = tuple(field.name for field in fields(ClientCall))


def _receipt(path: Path, idx: SchemaIndex) -> dict:
    receipt = json.loads(path.read_text(encoding="utf-8"))
    if receipt["schema_sha256"] != idx.sha256:
        raise ValueError(f"{path.name} was produced with a different schema")
    return receipt


def _client_rows(path: Path) -> Iterator[Row]:
    with path.open(encoding="utf-8") as stream:
        for line in stream:
            if line.strip():
                yield json.loads(line)


def _other_method_operations(row: Row, methods: list[str], idx: SchemaIndex) -> list[Row]:
    call = ClientCall(**{name: row[name] for name in _CLIENT_FIELDS})
    if call.method is None or call.pattern is None:
        return []
    alternatives = []
    for method in methods:
        if method == call.method:
            continue
        # Keep the strongest matches under each alternative method, including candidates.
        _, elements, _ = bind(replace(call, method=method), idx)
        alternatives.extend(
            {"element": element, "source_path": idx.operations[element].source_path}
            for element in elements
        )
    return alternatives


def standing(producer_out: Path, consumer_out: Path, idx: SchemaIndex) -> dict[str, list[Row]]:
    """Return lists keyed by their section 6.1 numbers as strings."""
    producer = _receipt(producer_out / "f1.json", idx)
    _receipt(consumer_out / "f2.json", idx)
    lists: dict[str, list[Row]] = {str(number): [] for number in range(1, 5)}

    for row in producer["residue"]:
        if row["list"] in (2, 3, 4):
            lists[str(row["list"])].append(row | {"f1_row_id": row["row_id"]})

    methods = sorted({element.removeprefix("op:").split(" ", 1)[0] for element in idx.operations})
    for row in _client_rows(consumer_out / "consumer.jsonl"):
        if (row["grade"], row["match_reason"]) != ("unbound", "no_operation"):
            continue
        alternatives = _other_method_operations(row, methods, idx)
        if alternatives:
            lists["4"].append(row | {"other_method_operations": alternatives})
        else:
            lists["1"].append(row)

    for number, rows in lists.items():
        for position, row in enumerate(rows, 1):
            row.update(row_id=f"f8-{number}-{position:04d}", list=int(number))
    return lists


def run(producer_out: Path, consumer_out: Path, idx: SchemaIndex, out: Path) -> None:
    lists = standing(producer_out, consumer_out, idx)
    out.mkdir(parents=True, exist_ok=True)
    (out / "f8.json").write_text(json.dumps(lists, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
