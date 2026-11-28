"""Typed stderr transport, occurrence joins and disk-backed callback views."""

from __future__ import annotations

from collections import Counter
from collections.abc import Sequence
from dataclasses import dataclass
from itertools import groupby, islice
import json
import math
from operator import index as integer_index
import sqlite3

from .classifier import describe, line_text
from .edges import BOUNDS, FIELDS, SIDES, canonical, integer, valid_bounds
from .inputs import ArtifactDrift, transport_lines
from .run import OutputError

STATUSES = ("attached", "stray", "ambiguous", "unmatched")
ARM = ("base", "head")
INDEX_FIELDS = (*BOUNDS, "file", "line", "base", "fields")


def json_value(value):
    if value is None or type(value) in (str, int, bool):
        return True
    if type(value) is float:
        return math.isfinite(value)
    if type(value) is list:
        return all(map(json_value, value))
    return type(value) is dict and all(type(k) is str and json_value(v) for k, v in value.items())


def check_endpoint(part):
    if not isinstance(part, dict) or not part.keys() <= set(FIELDS):
        raise ValueError("invalid trace edge endpoint fields")
    for field in ("file", "base"):
        if field in part and not isinstance(part[field], str):
            raise ValueError(f"trace {field} must be a string")
    if "line" in part and not integer(part["line"], 1):
        raise ValueError("trace line must be a positive integer")
    if "fields" in part and (not isinstance(part["fields"], list)
                            or not all(isinstance(field, str) for field in part["fields"])):
        raise ValueError("trace fields must be an ordered string array")
    if not valid_bounds(*(part.get(name) for name in BOUNDS)):
        raise ValueError("invalid trace byte bounds")


def check_record(record, arm):
    if (type(record) is not dict or record.get("arm") != arm or not line_text(record.get("tag"))
            or not line_text(record.get("kind")) or type(record.get("key")) is not dict
            or not json_value(record)):
        raise ValueError("expected a JSON-compatible trace record with its input arm, tag, kind and key")
    key = record["key"]
    if record["kind"] == "edge":
        if not key or not key.keys() <= set(SIDES):
            raise ValueError("edge record must supply from/to endpoint fields")
        for part in key.values():
            check_endpoint(part)
        if not any(key.values()):
            raise ValueError("edge record has no endpoint fields")
    if record["kind"] == "node":
        if (not isinstance(key.get("file"), str) or not integer(key.get("line"), 1)
                or key.get("access") not in ("def", "use")
                or not all(integer(key.get(name), 0) for name in BOUNDS)
                or not valid_bounds(*(key[name] for name in BOUNDS))
                or "owner" in key and not isinstance(key["owner"], str)):
            raise ValueError("node record requires file, line, access and an exact byte span")
    if "legacy_key" in record:
        legacy = record["legacy_key"]
        if (not line_text(record.get("mechanism")) or not isinstance(legacy, list) or len(legacy) != 5
                or not all(isinstance(legacy[i], str) for i in (0, 1, 3))
                or not all(integer(legacy[i], 1) for i in (2, 4))):
            raise ValueError("invalid historical mechanism key")


@dataclass(frozen=True)
class View(Sequence):
    connection: object
    query: str
    params: tuple = ()
    decode: object = lambda row: json.loads(row[0])

    def __iter__(self):
        for row in self.connection.execute(self.query, self.params):
            yield self.decode(row)

    def __len__(self):
        return self.connection.execute(f"SELECT count(*) FROM ({self.query})", self.params).fetchone()[0]

    def __getitem__(self, index):
        if isinstance(index, slice):
            positions = range(*index.indices(len(self)))
            if not positions:
                return []
            first, last = positions[0], positions[-1]
            items = list(islice(self, min(first, last), max(first, last) + 1, abs(positions.step)))
            if positions.step < 0:
                items.reverse()
            return items
        position = integer_index(index)
        if position < 0:
            position += len(self)
        if position >= 0:
            row = self.connection.execute(self.query + " LIMIT 1 OFFSET ?", (*self.params, position)).fetchone()
            if row is not None:
                return self.decode(row)
        raise IndexError(index)


class TraceInput(ValueError):
    """A trace callback failed on one input line."""


class TraceStore:
    def __init__(self, connection):
        self.db = connection
        connection.executescript("""
            CREATE TABLE records (id INTEGER PRIMARY KEY, arm INTEGER, line INTEGER,
                                  tag TEXT, payload BLOB, status TEXT, change_id INTEGER);
            CREATE TABLE nodes (arm INTEGER, file BLOB, start BLOB, end BLOB, access TEXT,
                                owner BLOB, record_id INTEGER,
                                PRIMARY KEY(arm, file, start, end, access, record_id)) WITHOUT ROWID;
            CREATE TABLE legacy (tag TEXT, key BLOB, PRIMARY KEY(tag, key)) WITHOUT ROWID;
        """)

    def store(self, sql, params):
        """Write one row. A storage failure is not a defect of the trace input."""
        try:
            return self.db.execute(sql, params)
        except sqlite3.Error as error:
            raise OutputError(f"output or storage: {describe(error)}") from error

    def read(self, runs, parser):
        has_edges = False
        for arm, run in enumerate(runs):
            number = 0
            try:
                for number, raw in enumerate(transport_lines(run.artifact("edges", "stderr"),
                                                              run.expected("edges", "stderr")), 1):
                    line = raw.decode("utf-8")
                    if number == 1 and line.startswith("\ufeff"):
                        raise ValueError("UTF-8 BOM is not admitted")
                    try:
                        record = parser(line, ARM[arm]) if parser is not None else None
                    except (Exception, SystemExit) as error:
                        # Whatever the callback raises, an OutputError included, is its input's defect.
                        raise TraceInput(f"trace input: {ARM[arm]} line {number}: {describe(error)}") from error
                    if record is None:
                        continue
                    check_record(record, ARM[arm])
                    kind, payload = record["kind"], canonical(record)
                    is_edge = kind == "edge"
                    has_edges |= is_edge
                    cursor = self.store(
                        "INSERT INTO records (arm, line, tag, payload, status) VALUES (?, ?, ?, ?, ?)",
                        (arm, number, record["tag"], payload, None if is_edge else "unjoined"))
                    if kind == "node":
                        key = record["key"]
                        self.store("INSERT INTO nodes VALUES (?, ?, ?, ?, ?, ?, ?)",
                                   (arm, canonical(key["file"]), canonical(key["start_byte"]),
                                    canonical(key["end_byte"]), key["access"],
                                    canonical(key.get("owner")), cursor.lastrowid))
                    if arm == 1 and "legacy_key" in record:
                        self.store("INSERT OR IGNORE INTO legacy VALUES (?, ?)",
                                   (record["mechanism"], canonical(record["legacy_key"])))
            except (ArtifactDrift, OutputError, TraceInput):
                raise
            except (Exception, SystemExit) as error:
                raise ValueError(f"trace input: {ARM[arm]} line {number or 1}: {describe(error)}") from error
        return has_edges

    def join(self, mode, has_edges):
        if has_edges:
            candidates = CandidateIndex(self.db)
            query = "SELECT id, arm, payload FROM records WHERE status IS NULL ORDER BY id"
            for ident, arm, payload in self.db.execute(query):
                status, change = candidates.match(arm, json.loads(payload)["key"], mode)
                self.db.execute("UPDATE records SET status=?, change_id=? WHERE id=?",
                                (status, change, ident))
        self.db.execute("CREATE INDEX record_attachment ON records(change_id, id) WHERE change_id IS NOT NULL")

    @property
    def records(self):
        return View(self.db, "SELECT payload FROM records ORDER BY id")

    @property
    def unchanged(self):
        return View(self.db, "SELECT key, bag FROM unchanged ORDER BY key",
                    decode=lambda row: {"key": json.loads(row[0]), "bag": json.loads(row[1])})

    @property
    def changes(self):
        return View(self.db, "SELECT final FROM changes ORDER BY id")

    @property
    def unjoined(self):
        return View(self.db, "SELECT payload FROM records WHERE status='unjoined' ORDER BY id")

    @property
    def joins(self):
        return View(self.db, "SELECT arm, line, status, change_id FROM records ORDER BY id",
                    decode=lambda row: dict(arm=ARM[row[0]], line=row[1], status=row[2], change_id=row[3]))

    def attached(self, change_id):
        return list(iter(View(self.db, "SELECT payload FROM records WHERE change_id=? ORDER BY id", (change_id,))))

    def nodes(self, arm, file, start, end, access, *, owner=None):
        sql = """SELECT payload FROM nodes JOIN records ON record_id = id
                 WHERE nodes.arm=? AND file=? AND start=? AND end=? AND access=?"""
        params = (ARM.index(arm), canonical(file), canonical(start), canonical(end), access)
        if owner is not None:
            sql += " AND owner=?"
            params += (canonical(owner),)
        return View(self.db, sql + " ORDER BY record_id", params)

    def totals(self):
        tags = {}
        totals = Counter(dict.fromkeys((*STATUSES, "unjoined"), 0))
        for tag, status, count in self.db.execute("SELECT tag, status, count(*) FROM records GROUP BY tag, status"):
            tags.setdefault(tag, dict.fromkeys((*STATUSES, "unjoined"), 0))[status] = count
            totals[status] += count
        mechanisms = dict(self.db.execute("SELECT tag, count(*) FROM legacy GROUP BY tag"))
        unjoined = totals.pop("unjoined")
        return dict(totals), unjoined, tags, mechanisms


class CandidateIndex:
    def __init__(self, connection):
        self.db = connection
        self.indexed = set()
        self.columns = tuple(f"{side}_{field}" for side in SIDES for field in INDEX_FIELDS)
        connection.execute(f"""
            CREATE TABLE candidates (
                arm INTEGER, key BLOB, n INTEGER, bounds BLOB, {', '.join(self.columns)},
                PRIMARY KEY (arm, key)
            ) WITHOUT ROWID
        """)
        placeholders = ", ".join("?" for _ in range(4 + len(self.columns)))
        connection.executemany(f"INSERT INTO candidates VALUES ({placeholders})", self.rows())

    def rows(self):
        query = "SELECT key, arm, sum(n), min(bounds) FROM edges GROUP BY key, arm ORDER BY key, arm"
        for key, same_key in groupby(self.db.execute(query), key=lambda row: row[0]):
            decoded = json.loads(key)
            fields = tuple(canonical(decoded[side][field]) for side in SIDES for field in INDEX_FIELDS)
            for _, arm, count, bounds in same_key:
                yield arm, key, count, bounds, *fields

    def match(self, arm, key, mode):
        terms = [(f"{side}_{field}", canonical(part[field]))
                 for field in INDEX_FIELDS for side in SIDES if (part := key.get(side, {}))
                 and field in part and part[field] is not None
                 and (mode == "bytes" or field not in BOUNDS)]
        if not terms:
            return "unmatched", None
        pivot = terms[0][0]
        if pivot not in self.indexed:
            self.db.execute(f"CREATE INDEX candidate_{pivot} ON candidates(arm, {pivot})")
            self.indexed.add(pivot)
        conditions = " AND ".join(f"{column}=?" for column, _ in terms)
        found = self.db.execute(f"SELECT key, n, bounds FROM candidates WHERE arm=? AND {conditions} LIMIT 2",
                                (arm, *(value for _, value in terms))).fetchall()
        if not found:
            return "unmatched", None
        if len(found) > 1 or found[0][1] > 1:
            return "ambiguous", None

        wire, _, original = found[0]
        if original:
            for index, bound in enumerate(json.loads(original)):
                supplied = key.get(SIDES[index // 2], {}).get(BOUNDS[index % 2])
                if bound is not None and supplied is not None and bound != supplied:
                    return "unmatched", None

        change = self.db.execute("SELECT change_id FROM assignments WHERE key=? AND arm=?",
                                 (wire, arm)).fetchone()
        return ("attached", change[0]) if change is not None else ("stray", None)


def parse_subscript(line, arm):
    """Optional parser for the structured subscript inventory wire."""
    from .edges import path_fields
    from .inputs import json_object

    def unsigned(text):
        if not text.isascii() or not text.isdecimal():
            raise ValueError("expected unsigned decimal")
        return int(text)

    def inventory_path(text):
        value = json_object(text.encode())
        if value.keys() != {"base", "fields"}:
            raise ValueError("inventory path requires exactly base and fields")
        return path_fields(value)

    tag, *fields = line.rstrip("\r\n").split("\t")
    if tag not in ("subscript-site", "subscript-inventory-end"):
        return None
    record = dict(arm=arm, tag=tag, key={})
    try:
        if tag == "subscript-inventory-end":
            mode, defs, uses = fields
            if mode != "all":
                raise ValueError("terminal mode must be all")
            return dict(record, kind="inventory-end", mode=mode,
                        defs=unsigned(defs), uses=unsigned(uses))

        file, owner, wire_line, start, end, access, recorded, chain = fields
        key = dict(file=file, owner=owner, line=unsigned(wire_line), start_byte=unsigned(start),
                   end_byte=unsigned(end), access=access)
        node = dict(record, kind="node", key=key,
                    recorded_path=inventory_path(recorded),
                    chain_path=chain if chain in ("refused", "-") else
                    inventory_path(chain))
        check_record(node, arm)
        canonical(node)
        return node
    except (ValueError, RecursionError) as error:
        return dict(record, kind="inventory-error", error=describe(error))


def inventory_failures(bound, required_arms):
    """Validate completion only on the arms selected by the caller's policy."""
    observations = {arm: {"terminals": 0, "terminal": None, "early": False,
                          "malformed": None, "bad_terminal": False, "def": 0, "use": 0} for arm in required_arms}
    for record, join in zip(bound.records, bound.joins):
        arm, kind = record["arm"], record["kind"]
        if arm not in observations:
            continue
        item = observations[arm]
        if kind == "inventory-error":
            item["malformed"] = f"line {join['line']}: {record['error']}"
            item["bad_terminal"] |= record["tag"] == "subscript-inventory-end"
        elif kind == "node" and record["tag"] == "subscript-site":
            item[record["key"]["access"]] += 1
            item["early"] |= item["terminal"] is not None
        elif kind == "inventory-end" and record["tag"] == "subscript-inventory-end":
            item["terminals"] += 1
            item["terminal"] = (record["defs"], record["uses"])

    failures = []
    for arm, item in observations.items():
        rec = bound.base_receipt if arm == "base" else bound.head_receipt
        mode = rec["options"]["trace_env"].get("PRISM_SUBSCRIPT_TRACE")
        parsed = (item["def"], item["use"])
        if item["terminal"] is None and not item["bad_terminal"]:
            reason = "inventory-terminal-missing"
        elif item["malformed"] is not None:
            reason = f"inventory-record: {item['malformed']}"
        elif mode != "all":
            reason = "inventory-mode requires all"
        elif item["terminals"] != 1:
            reason = f"inventory-terminal-count: {item['terminals']}"
        elif item["early"]:
            reason = "inventory-terminal-order"
        elif parsed != item["terminal"]:
            reason = f"inventory-count-mismatch: counted {parsed}, terminal {item['terminal']}"
        else:
            continue
        failures.append(f"{arm}: {reason}")
    return failures
