"""Evidence for changes between two frozen source inventories."""

from collections import Counter, defaultdict
from difflib import SequenceMatcher


def failed(check, file, revision, reason, *, side=None, line=None):
    return {"check": check, "file": file, "rev": revision, "side": side, "line": line, "reason": reason}


def explain(record):
    where = record["file"]
    if where and record["line"]:
        where += f":{record['line']}"
    if where and record["rev"]:
        where += f" at {record['rev'][:8]}"
    side = f"{record['side']} " if record["side"] else ""
    scope = f"{side}{where}: " if where else side
    return f"{record['check']}: Not checked — {scope}{record['reason'].rstrip('.')}."


def source_gap(check, snapshot, side=None, *, file=None):
    affected = {file, snapshot["scope"]}
    if file is not None and snapshot["role"] == "consumer":
        dependencies = defaultdict(set)
        for dependency in snapshot["dependencies"]:
            if dependency["path"]:
                dependencies[dependency["importer"]].add(dependency["path"])
        pending = [file]
        while pending:
            added = dependencies[pending.pop()] - affected
            pending.extend(added)
            affected.update(added)
    for gap in snapshot["unaccounted_scopes"]:
        if file is None or gap["file"] in affected:
            return failed(check, gap["file"], snapshot["rev"], gap["reason"], side=side, line=gap["line"])
    return None


def diff(before, after):
    a, b = (before or "").splitlines(), (after or "").splitlines()
    return a, b, list(SequenceMatcher(None, a, b, autojunk=False).get_opcodes())


def overlaps_change(opcodes, start, end, side):
    return any(tag != "equal" and lo < end and start - 1 < hi
               for tag, *bounds in opcodes for lo, hi in [bounds[2 * side:2 * side + 2]])


def _signature(site):
    return site["entry_digest"], site["handler_ref"]


def _by_file(sites):
    result = defaultdict(list)
    for site in sites:
        result[site["file"]].append(site)
    return result


def _site_delta(old, new, equal_lines):
    """Match unchanged positions first, then remaining exact entries as a multiset."""
    remaining = list(new)
    removed = []
    unmatched = []
    for entry in old:
        index = next((i for i, candidate in enumerate(remaining)
                      if candidate["line"] == equal_lines.get(entry["line"])
                      and equal_lines.get(entry["end_line"]) == candidate["end_line"]
                      and _signature(entry) == _signature(candidate)), None)
        if index is None:
            unmatched.append(entry)
        else:
            remaining.pop(index)
    for entry in unmatched:
        index = next((i for i, candidate in enumerate(remaining) if _signature(entry) == _signature(candidate)), None)
        if index is None:
            removed.append(entry)
        else:
            remaining.pop(index)
    return removed, remaining


def equal_lines(opcodes):
    return {i + offset + 1: j + offset + 1
            for tag, i, end, j, _ in opcodes if tag == "equal" for offset in range(end - i)}


def _entry_failure(frozen, snapshots, inventories, file, entry, side, opcodes):
    check = "Removal" if side == 0 else "Addition"
    for index in (side, 1 - side):
        snap = snapshots[index]
        label = ("before", "after")[index]
        gap = source_gap(check, snap, label, file=file)
        if gap:
            return gap
        sources = (frozen.before_sources, frozen.after_sources)[index]
        status = inventories[index].get(file)
        if status == "complete" or (index != side and file in frozen.changed_paths and file not in sources):
            continue
        return failed(check, file, snap["rev"], "no complete declaration inventory was recorded", side=label)
    if file not in frozen.changed_paths or not overlaps_change(opcodes, entry["line"], entry["end_line"], side):
        return failed(check, file, snapshots[side]["rev"],
                      "the declaration span does not overlap a changed block in the selected diff",
                      side=("before", "after")[side], line=entry["line"])
    return None


def declaration_changes(frozen, old_sites, new_sites, inventories, diffs):
    snapshots = frozen.before["service"], frozen.after["service"]
    by_file = _by_file(old_sites), _by_file(new_sites)
    scoped, failures, kinds = [], [], []
    for file in sorted(by_file[0].keys() | by_file[1].keys()):
        opcodes, mapped = diffs[file]
        removed, added = _site_delta(*[index[file] for index in by_file], mapped)
        local_changes = []
        for side, entries in enumerate((removed, added)):
            for entry in entries:
                failure = _entry_failure(frozen, snapshots, inventories, file, entry, side, opcodes)
                if failure:
                    failures.append(failure)
                else:
                    local_changes.append({"kind": ("removal", "addition")[side], "site": entry,
                                          "proof": "complete inventories and changed source lines"})
        if (len(by_file[0][file]) == len(by_file[1][file]) == len(removed) == len(added) == 1
                and len(local_changes) == 2):
            kinds.append("Route declaration changed")
        else:
            scoped.extend(local_changes)
        if (by_file[0][file] and by_file[1][file] and not removed and not added and
                (file in frozen.changed_paths or any(
                    (site["handler_site"] or {}).get("file") in frozen.changed_paths
                    for side in by_file for site in side[file]))):
            failures.append(failed("Validation or handler edits", file, snapshots[1]["rev"],
                                   "the two source bodies were not compared"))
    presence = ["present" if sites else "unknown" if snap["unaccounted_scopes"] else "absent"
                for sites, snap in zip((old_sites, new_sites), snapshots)]
    for side, (owned, other) in enumerate(((old_sites, new_sites), (new_sites, old_sites))):
        if not owned or other:
            continue
        word = ("removal", "addition")[side]
        gap = source_gap(f"Operation {'removed' if side == 0 else 'added'}", snapshots[1 - side],
                         ("before", "after")[1 - side])
        if not failures and not gap and any(row["kind"] == word for row in scoped):
            kinds.append("Operation removed" if side == 0 else "Operation added")
        else:
            presence[1 - side] = "unknown"
            kinds.append(f"Listed only {('before', 'after')[side]} among the declarations read")
            if gap and not any(row["file"] == gap["file"] and row["rev"] == gap["rev"] for row in failures):
                failures.append(gap)
    if not {"Operation added", "Operation removed"}.intersection(kinds):
        kinds.extend(f"Declaration {'removed from' if row['kind'] == 'removal' else 'added to'} {row['site']['file']}"
                     for row in scoped)
    return kinds, scoped, presence, failures


def paired_functions(before, after, mapped):
    """Require a unique declaration in each file and unchanged mapped header lines."""
    old, new = ({tuple(row["span"]): row for row in file} for file in (before, after))
    identities = [Counter(tuple(row["identity"]) for row in file.values()) for file in (old, new)]
    after_by_key = {tuple(row["identity"]): row for row in new.values()}
    result = {}
    for span, row in old.items():
        key = tuple(row["identity"])
        if not key or identities[0][key] != 1 or identities[1][key] != 1:
            continue
        candidate = after_by_key[key]
        start, stop = row["header_lines"]
        after_start, after_stop = candidate["header_lines"]
        if [mapped.get(line) for line in range(start, stop + 1)] == list(range(after_start, after_stop + 1)):
            result[span] = tuple(candidate["span"])
    return result
