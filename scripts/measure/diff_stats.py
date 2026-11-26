"""Admit two runs and compare the numeric leaves of their counter objects."""

from __future__ import annotations

from dataclasses import dataclass
from decimal import Decimal, InvalidOperation, localcontext
import hashlib
import json
from pathlib import Path
import re
import sys

from . import receipt
from .inputs import ArtifactDrift, unique_object

TABLES = {"dfg": "dfg-stats", "call": "call-stats"}
MAX_DIGITS = 1000
MAX_EXPONENT = 1000
# Unicode White_Space, plus Cc (C0 and C1). This does not depend on Python's
# treatment of the ASCII information separators as whitespace.
ESCAPED_CODE = re.compile(r"[\x00-\x20\x7f-\xa0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]")
SPECIAL = frozenset("\\.*#")
TOKEN = re.compile(r"\\(?:[\\.*#e]|u\{[0-9A-F]{1,6}\})|[^\\.*#]+|\.")


@dataclass(frozen=True)
class Run:
    directory: Path
    receipt: dict
    receipt_sha256: str

    def artifact(self, role, field):
        name = next(command[field] for command in self.receipt["commands"] if command["id"] == role)
        return self.directory / name

    def expected(self, role, field):
        """The receipt's {sha256, bytes} record for one command artifact."""
        name = next(command[field] for command in self.receipt["commands"] if command["id"] == role)
        return self.receipt["artifacts"][name]

    def read(self, role, field):
        """Read an artifact once and verify the very bytes returned."""
        path = self.artifact(role, field)
        data = path.read_bytes()
        record = self.expected(role, field)
        if hashlib.sha256(data).hexdigest() != record["sha256"] or len(data) != record["bytes"]:
            raise ArtifactDrift(f"run binding: {path.name} differs from the receipt")
        return data


def admit_pair(base, head, entries):
    runs = []
    for arm, directory in (("base", base), ("head", head)):
        directory = directory.absolute()
        try:
            record, digest = receipt.admit(directory, entries=entries, with_digest=True)
            runs.append(Run(directory, record, digest))
        except (ValueError, OSError, RecursionError) as error:
            raise ValueError(f"run binding: {arm}: {error}") from error
    left, right = (run.receipt for run in runs)
    if left["corpus"]["input_sha256"] != right["corpus"]["input_sha256"]:
        raise ValueError("run pairing: different corpus input digests")
    if any(left["options"][name] != right["options"][name] for name in ("bytes", "sites", "cache")):
        raise ValueError("run pairing: different analysis options")
    return tuple(runs)


def numeric(text):
    if len(text) > MAX_DIGITS:
        raise ValueError(f"counter number exceeds {MAX_DIGITS} characters")
    try:
        value = Decimal(text)
    except InvalidOperation:
        raise ValueError("counter number is not a representable decimal") from None
    if not value.is_finite() or abs(value.as_tuple().exponent) > MAX_EXPONENT:
        raise ValueError(f"counter number is non-finite or its quantum exceeds {MAX_EXPONENT}")
    return value if "." in text or "e" in text.lower() else int(text)


def leaves(run, role):
    name = run.artifact(role, "output").name
    try:
        obj = json.loads(run.read(role, "output").decode("utf-8"), object_pairs_hook=unique_object,
                         parse_int=numeric, parse_float=numeric, parse_constant=numeric)
        if not isinstance(obj, dict):
            raise ValueError("expected a JSON object")

        def walk(items, prefix):
            for name, value in items.items():
                name.encode("utf-8")
                key = (*prefix, name)
                if isinstance(value, dict):
                    yield from walk(value, key)
                elif type(value) in (int, Decimal):
                    yield key, value

        return dict(walk(obj, ()))
    except ArtifactDrift:
        raise
    except (OSError, ValueError, RecursionError) as error:
        raise ValueError(f"counter input: {name}: {error}") from error


def encode_component(part):
    if not part:
        return r"\e"
    return "".join("\\" + c if c in SPECIAL else rf"\u{{{ord(c):X}}}" if ESCAPED_CODE.fullmatch(c)
                   else c for c in part)


def render(key):
    return ".".join(encode_component(part) for part in key)


def parse_pattern(text):
    prefix = text.endswith(".*") and not _escaped(text, len(text) - 2)
    if prefix:
        text = text[:-2]
    parts = [""]
    pos = 0
    for token in TOKEN.finditer(text):
        if token.start() != pos:
            raise ValueError("invalid path escape or reserved character")
        pos = token.end()
        value = token[0]
        if value == ".":
            parts.append("")
        elif value.startswith(r"\u{"):
            parts[-1] += chr(int(value[3:-1], 16))
        elif value != r"\e":
            parts[-1] += value[1:] if value.startswith("\\") else value
    key = tuple(parts)
    if pos != len(text) or render(key) != text:
        raise ValueError("expected a canonical counter component path")
    key_text = "".join(parts)
    key_text.encode("utf-8")
    return key, prefix


def _escaped(text, index):
    start = index
    while index and text[index - 1] == "\\":
        index -= 1
    return (start - index) % 2 != 0


def patterns(path):
    if path is None:
        return None
    found = set()
    try:
        for number, raw in enumerate(path.read_bytes().decode("utf-8").split("\n"), 1):
            comment = next((i for i, char in enumerate(raw) if char == "#" and not _escaped(raw, i)),
                           len(raw))
            line = raw[:comment].strip()
            if line:
                try:
                    found.add(parse_pattern(line))
                except ValueError as error:
                    raise ValueError(f"line {number}: {error}") from error
    except (OSError, ValueError) as error:
        raise ValueError(f"allow list: {error}") from error
    return found


def delta(base, head):
    left, right = (0 if value is None else value for value in (base, head))
    if isinstance(left, Decimal) or isinstance(right, Decimal):
        with localcontext() as context:
            # A 1000-character coefficient at +1000 can be subtracted from
            # another value at -1000 without rounding any significant digit.
            context.prec = MAX_DIGITS + 2 * MAX_EXPONENT + 2
            context.Emax = context.prec
            context.Emin = -context.prec
            return right - left
    return right - left


def counter_tables(runs, which="both", allowed=None):
    results = {}
    for kind, role in TABLES.items():
        if which not in ("both", kind):
            continue
        base, head = (leaves(run, role) for run in runs)
        changed = []
        forbidden = []
        # The canonical codec has a round-trip inverse, so its tuple names are injective.
        for name, key in sorted((render(key), key) for key in base.keys() | head.keys()):
            before, after = base.get(key), head.get(key)
            if before == after:
                continue
            row = dict(which=kind, key=name, base=before, head=after, delta=delta(before, after))
            changed.append(row)
            if (allowed is not None and (key, False) not in allowed
                    and not any((key[:n], True) in allowed for n in range(1, len(key)))):
                forbidden.append(row)
        results[kind] = dict(base=len(base), head=len(head), changed=changed, forbidden=forbidden)
    return results


def compare_stats(base, head, *, entries=None, which="both", allow=None):
    return counter_tables(admit_pair(base, head, entries), which, patterns(allow))


def display_stats(results):
    def display(value):
        return "missing" if value is None else str(value)

    for kind, table in results.items():
        print(kind)
        print(f"counters base={table['base']} head={table['head']} changed={len(table['changed'])}")
        for row in table["changed"]:
            amount = row["delta"]
            signed = str(amount)
            if not signed.startswith("-"):
                signed = "+" + signed
            print(f"  {row['key']}: {display(row['base'])} -> {display(row['head'])} ({signed})")
        for row in table["forbidden"]:
            print(f"forbidden counter delta: {kind} {row['key']}", file=sys.stderr)
    return int(any(table["forbidden"] for table in results.values()))
