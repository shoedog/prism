"""Evidence requirements for confirmed report statements."""

from dataclasses import dataclass


@dataclass(frozen=True)
class Record:
    name: str
    side: str | None = None


def _both(*names):
    return tuple(Record(name, side) for name in names for side in ("before", "after"))


_DECLARATIONS = _both("declaration inventory", "route specification")
_FILE_DECLARATIONS = _both("file declaration inventory", "route specification")
_CLIENTS = _both("client inventory")
_SCHEMAS = _both("schema inventory")
_MATCH = tuple(Record(name) for name in ("declaration inventory", "route specification", "request mapping",
                                       "applicable method and path", "match result"))
_CLIENT_DELTA = _CLIENTS + (Record("call span diff"),)
_OPERATION_DELTA = _DECLARATIONS + (Record("declaration span diff"),)
_OPERATION_TOTAL = _DECLARATIONS + _both("operation schema scope") + (Record("operation comparison"),)
_FIELD_TOTAL = _SCHEMAS + (Record("body comparisons"),)
_CLIENT_TOTAL = _CLIENTS + (Record("client comparison"),)
_PAIR = _both("request comparison")


CLAIMS = {
    "Operation removed": _OPERATION_DELTA,
    "Operation added": _OPERATION_DELTA,
    "Operation presence": (Record("declaration inventory"), Record("route specification")),
    "Declaration removed from file": _FILE_DECLARATIONS + (Record("declaration span diff", "before"),),
    "Declaration added to file": _FILE_DECLARATIONS + (Record("declaration span diff", "after"),),
    "Declaration moved": _FILE_DECLARATIONS + (Record("one-to-one declaration correspondence"),),
    "Route declaration changed": _FILE_DECLARATIONS + _both("declaration span diff"),
    "Successful response status added": _SCHEMAS + _both("response status inventory"),
    "Successful response status removed": _SCHEMAS + _both("response status inventory"),
    "Response bodies correspond": _SCHEMAS + _both("body root"),
    "Field declaration changed": _SCHEMAS + _both("body root", "field region"),
    "Client call added": _CLIENT_DELTA,
    "Client call removed": _CLIENT_DELTA,
    "Client target changed": _CLIENT_DELTA + (Record("call correspondence"),),
    "Corresponding call": _CLIENT_DELTA + (Record("call correspondence"),),
    "No call at this site": (Record("proved occurrence absence"),),
    "Matches declaration": _MATCH,
    "No matching declaration": _MATCH,
    "Possible match": _MATCH,
    "Possible match under another prefix": (Record("request mapping"), Record("shared address suffix")),
    "Closest-match population": (Record("client inventory"), Record("declaration inventory"),
                                 Record("route specification"), Record("match population")),
    "Request count": (Record("client inventory"), Record("declaration inventory"),
                      Record("route specification"), Record("match population")),
    "Unknown target count": (Record("client inventory"),),
    "Operation total": _OPERATION_TOTAL,
    "Field total": _FIELD_TOTAL,
    "Client change total": _CLIENT_TOTAL,
    "Empty service report": _OPERATION_TOTAL,
    "Empty client report": _CLIENT_TOTAL,
    "Empty mixed report": tuple(dict.fromkeys(_OPERATION_TOTAL + _CLIENT_TOTAL)),
    "Possible-only count": _CLIENTS + _DECLARATIONS + (Record("applicable match comparisons"),),
    "Existing client call gains a matching declaration": _PAIR + (Record("deployment pair"),),
    "Client calls a missing route": _PAIR + (Record("deployment pair"),),
    "Client target needs confirmation": (Record("request comparison"), Record("deployment pair")),
    "Standing difference timing": _PAIR + (Record("call correspondence"),),
    "Standing schema difference": (Record("schema inventory"), Record("declaration inventory"),
                                  Record("route specification")),
    "Caller search": (Record("enclosing function"), Record("caller search scope"), Record("caller search result")),
    "Schema coverage": _FIELD_TOTAL,
    "Source coverage": (Record("source inventory"),),
    "Address mapping": (Record("request mapping"),),
    "Source location": (Record("tree source location"),),
    "Enclosing function": (Record("enclosing function"),),
}


def failed(check, file, revision, reason, *, side=None, line=None):
    return {"check": check, "file": file, "rev": revision, "side": side, "line": line, "reason": reason}


def explain(failure):
    where = failure["file"]
    if where and failure["line"]:
        where += f":{failure['line']}"
    if failure["rev"]:
        where = f"{where} at {failure['rev'][:8]}" if where else failure["rev"][:8]
    scope = " ".join(part for part in (failure["side"], where) if part)
    scope = f"{scope}: " if scope else ""
    return f"{failure['check']}: Not checked — {scope}{failure['reason'].rstrip('.')}."


def decide(kind, records):
    """Return every unsatisfied requirement, retaining its side and diagnostic."""
    failures = []
    for record in CLAIMS[kind]:
        value = records.get(record)
        if value is True:
            continue
        if not value:
            value = failed(kind, None, None, f"{record.name} was not recorded", side=record.side)
        for failure in value if isinstance(value, (list, tuple)) else [value]:
            failure = dict(failure, record=record.name)
            if record.side is not None and failure["side"] is None:
                failure["side"] = record.side
            if failure not in failures:
                failures.append(failure)
    return {"kind": kind, "confirmed": not failures, "failures": failures}


def reasons(proof):
    return " ".join(dict.fromkeys(map(explain, proof["failures"]))) or None


def wording(proof, confirmed):
    return confirmed if proof["confirmed"] else reasons(proof)
