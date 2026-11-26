"""DFG wire admission and counted edge projection."""

from __future__ import annotations

from collections import Counter
from itertools import groupby
import json
import sqlite3

from .inputs import ArtifactDrift, NumberToken, jsonl_objects

SIDES = ("from", "to")
BOUNDS = ("start_byte", "end_byte")
FIELDS = ("file", "line", "base", "fields", *BOUNDS)
KINDS = ("label_moved", "removed", "added", "ambiguous")


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
                      allow_nan=False).encode("utf-8")


def integer(value, minimum):
    return type(value) is int and value >= minimum


def path_fields(path):
    if (not isinstance(path, dict) or not isinstance(path.get("base"), str)
            or not isinstance(path.get("fields"), list)
            or not all(isinstance(field, str) for field in path["fields"])):
        raise ValueError("expected structured path with base and ordered string fields")
    return {"base": path["base"], "fields": path["fields"]}


def valid_bounds(start, end):
    return ((start is None or integer(start, 0)) and (end is None or integer(end, 0))
            and (start is None or end is None or start <= end))


def decode_edge(wire, *, line_mode):
    confidence, doubt, kill = wire["confidence"], wire["doubt"], wire.get("kill_line")
    if (confidence not in ("exact", "nameonly") or
            doubt is not None and (confidence == "exact" or not isinstance(doubt, str) or not doubt)
            or kill is not None and not integer(kill, 1)):
        raise ValueError("invalid confidence, doubt or kill line")
    key = {}
    attributes = dict(confidence=confidence, doubt=doubt, kill_line=kill)
    original = []
    for side in SIDES:
        endpoint = wire[side]
        if not isinstance(endpoint, dict):
            raise ValueError(f"{side} endpoint must be an object")
        file, line, access = endpoint["file"], endpoint["line"], endpoint["access"]
        if not isinstance(file, str) or not integer(line, 1) or access not in ("def", "use"):
            raise ValueError(f"invalid {side} endpoint file, line or access")
        start, end = (endpoint.get(name) for name in BOUNDS)
        if not valid_bounds(start, end):
            raise ValueError(f"invalid {side} endpoint byte bounds")
        original.extend((start, end))
        key[side] = dict(file=file, line=line, **path_fields(endpoint["path"]),
                         start_byte=None if line_mode else start, end_byte=None if line_mode else end)
        attributes[side + "_access"] = access
    return canonical(key), canonical(attributes), original


def residual_rows(base, head):
    """Return cancellations and ordered (kind, base counts, head counts) rows."""
    common = base & head
    return common, list(residual(base - common, head - common))


def residual(base, head):
    if not base or not head:
        if base or head:
            yield ("removed" if base else "added"), base, head
        return

    b, h = sum(base.values()), sum(head.values())
    if (b >= h and len(base) == 1) or (h >= b and len(head) == 1):
        # On an equal-size tie, select the homogeneous side, whichever it is.
        reverse = len(head) == 1 and h >= b
        single, other = (head, base) if reverse else (base, head)
        attr, count = next(iter(single.items()))
        for member in sorted(other):
            used = other[member]
            pair = (Counter({attr: used}), Counter({member: used}))
            yield "label_moved", *(pair[::-1] if reverse else pair)
            count -= used
        if count:
            remainder = Counter({attr: count})
            if reverse:
                yield "added", Counter(), remainder
            else:
                yield "removed", remainder, Counter()
    else:
        yield "ambiguous", base, head


def bag_bytes(counts):
    return b"[" + b",".join(member for member in sorted(counts) for _ in range(counts[member])) + b"]"


def make_database(path):
    database = sqlite3.connect(str(path))
    # Private scratch storage. A failed comparison discards the entire database.
    database.executescript("""
        PRAGMA journal_mode = OFF;
        PRAGMA cache_size = -4096;
        PRAGMA temp_store = FILE;
        CREATE TABLE edges (
            key BLOB, arm INTEGER, attr BLOB, bounds BLOB, n INTEGER,
            PRIMARY KEY (key, arm, attr, bounds)
        ) WITHOUT ROWID;
        CREATE TABLE unchanged (key BLOB PRIMARY KEY, bag BLOB) WITHOUT ROWID;
        CREATE TABLE changes (id INTEGER PRIMARY KEY, key BLOB, kind TEXT,
                              base BLOB, head BLOB, count INTEGER, final BLOB);
        CREATE INDEX changes_key ON changes(key);
        CREATE TABLE assignments (key BLOB, arm INTEGER, change_id INTEGER,
                                  PRIMARY KEY(key, arm)) WITHOUT ROWID;
    """)
    return database


def ingest(database, sources, requested, reason, receipt_bytes):
    minimum, maximum = 4, 0
    total = [0, 0]
    line_mode = requested == "line"

    def rows(arm, source):
        nonlocal minimum, maximum
        path, expect = source
        # Members the projection ignores may hold any finite JSON number.
        for number, wire in enumerate(jsonl_objects(path, expect, NumberToken), 1):
            try:
                key, attr, bounds = decode_edge(wire, line_mode=line_mode)
            except (KeyError, ValueError, TypeError) as error:
                raise ValueError(f"record {number}: {error}") from error
            # Track capabilities per endpoint, independently of the arm.
            left = (bounds[0] is not None) + (bounds[1] is not None)
            right = (bounds[2] is not None) + (bounds[3] is not None)
            minimum = min(minimum, left, right)
            maximum = max(maximum, left, right)
            total[arm] += 1
            yield key, arm, attr, canonical(bounds) if line_mode and (left or right) else b""

    for arm, source in enumerate(sources):
        try:
            database.executemany("""
                INSERT INTO edges VALUES (?, ?, ?, ?, 1)
                ON CONFLICT (key, arm, attr, bounds) DO UPDATE SET n = n + 1
            """, rows(arm, source))
        except ArtifactDrift:
            raise
        except (ValueError, RecursionError) as error:
            raise ValueError(f"edge input: {'base' if arm == 0 else 'head'}: {error}") from error

    if requested == "bytes" and minimum < 2:
        raise ValueError("key mode: bytes requires both bounds on every endpoint")
    if requested is None and maximum and minimum != 2:
        raise ValueError("key mode: mixed or partial bounds require explicit line projection")
    discarded = line_mode and maximum > 0
    if discarded and (reason is None or not reason.strip()):
        raise ValueError("key mode: discarding byte bounds requires a nonblank --line-reason")
    mode = requested or ("bytes" if maximum or (minimum == 4 and receipt_bytes) else "line")
    return mode, discarded, total


def partition(database, mode, *, track_assignments):
    """Partition on disk and check reconstruction of every attribute per arm."""
    counts = dict(changed_keys=0, collisions=0, added=0, removed=0, moved=0)
    source = database.execute("""
        SELECT key, arm, attr, sum(n) FROM edges GROUP BY key, arm, attr ORDER BY key, arm, attr
    """)
    ident = 0
    conservation_failed = False
    for key, rows in groupby(source, key=lambda row: row[0]):
        arms = [Counter(), Counter()]
        for _, arm, attr, count in rows:
            arms[arm][attr] = count
        base, head = arms
        b, h = sum(base.values()), sum(head.values())
        counts["added"] += max(0, h - b)
        counts["removed"] += max(0, b - h)
        counts["collisions"] += mode == "line" and max(b, h) > 1

        if base == head:
            database.execute("INSERT INTO unchanged VALUES (?, ?)", (key, bag_bytes(base)))
            continue
        counts["changed_keys"] += 1
        counts["moved"] += bool(b and h)
        common, changes = residual_rows(base, head)
        # Wire order: kind, then both serialised bags, then one-based IDs.
        changes = sorted(((kind, left, right, bag_bytes(left), bag_bytes(right))
                          for kind, left, right in changes),
                         key=lambda row: (KINDS.index(row[0]), row[3], row[4]))
        if common:
            database.execute("INSERT INTO unchanged VALUES (?, ?)", (key, bag_bytes(common)))
        rebuilt = [common.copy(), common.copy()]
        for kind, left, right, left_bytes, right_bytes in changes:
            ident += 1
            count = max(sum(left.values()), sum(right.values()))
            database.execute("INSERT INTO changes VALUES (?, ?, ?, ?, ?, ?, NULL)",
                             (ident, key, kind, left_bytes, right_bytes, count))
            for index, (population, bag) in enumerate(zip((b, h), (left, right))):
                rebuilt[index].update(bag)
                if track_assignments and population == 1 and bag:
                    database.execute("INSERT INTO assignments VALUES (?, ?, ?)", (key, index, ident))
        conservation_failed |= rebuilt != arms
    return counts, conservation_failed


def conserved(database, totals):
    """Rebuild every key from the stored unchanged and final rows.

    Compares each arm's stored multiplicity with the decoded total, then each
    key's stored occurrences with unchanged plus the published bags per arm.
    """
    for arm in (0, 1):
        stored = database.execute("SELECT coalesce(sum(n), 0) FROM edges WHERE arm=?", (arm,)).fetchone()[0]
        if stored != totals[arm]:
            return False
    if database.execute("""
            SELECT count(*) FROM (SELECT key FROM unchanged UNION SELECT key FROM changes)
            WHERE key NOT IN (SELECT key FROM edges)""").fetchone()[0]:
        return False
    source = database.execute("""
        SELECT key, arm, attr, sum(n) FROM edges GROUP BY key, arm, attr ORDER BY key, arm, attr
    """)
    for key, rows in groupby(source, key=lambda row: row[0]):
        decoded = [Counter(), Counter()]
        for _, arm, attr, count in rows:
            decoded[arm][attr] = count
        rebuilt = [Counter(), Counter()]
        row = database.execute("SELECT bag FROM unchanged WHERE key=?", (key,)).fetchone()
        if row is not None:
            for member in json.loads(row[0]):
                rebuilt[0][canonical(member)] += 1
                rebuilt[1][canonical(member)] += 1
        for (final,) in database.execute("SELECT final FROM changes WHERE key=? ORDER BY id", (key,)):
            change = json.loads(final)
            for index, bag in enumerate(("base_bag", "head_bag")):
                rebuilt[index].update(canonical(member) for member in change[bag])
        if rebuilt != decoded:
            return False
    return True
