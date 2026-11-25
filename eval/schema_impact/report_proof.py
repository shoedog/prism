"""Evidence for changes between two frozen source inventories."""

from collections import Counter, defaultdict
from difflib import SequenceMatcher

from .report_claims import Record, decide, explain as explain, failed as failed


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


def source_record(check, snapshot, side=None, *, file=None):
    if any(snapshot.get(key) is None for key in ("file_status", "expected_files", "unaccounted_scopes")):
        return failed(check, file or snapshot["scope"], snapshot["rev"],
                      "the source inventory was not recorded", side=side)
    return source_gap(check, snapshot, side, file=file) or True


def route_record(snapshot, side=None):
    if snapshot["operation_authority"] != "route table":
        return True
    receipt = next((row for row in snapshot["config_sources"] if row.get("kind") == "route specification"), None)
    if receipt and receipt.get("mapping") is not None and receipt.get("revision") and not receipt.get("error"):
        return True
    detail = receipt or {}
    return failed("Route specification", detail.get("file"), snapshot["rev"],
                  detail.get("error") or "an admitted route specification was not recorded", side=side)


def declaration_records(snapshots, check):
    return {record: value for side, snap in zip(("before", "after"), snapshots)
            for record, value in ((Record("declaration inventory", side), source_record(check, snap, side)),
                                  (Record("route specification", side), route_record(snap, side)))}


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


def _entry_proof(frozen, snapshots, inventories, file, entry, side, opcodes):
    label = ("before", "after")[side]
    check = "Removal" if side == 0 else "Addition"
    records = {}
    for index in (0, 1):
        snap = snapshots[index]
        scope = ("before", "after")[index]
        evidence = source_record(check, snap, scope, file=file)
        sources = (frozen.before_sources, frozen.after_sources)[index]
        status = inventories[index].get(file)
        if evidence is True and status != "complete" and not (
            index != side and file in frozen.changed_paths and file not in sources
        ):
            evidence = failed(check, file, snap["rev"], "no complete declaration inventory was recorded", side=scope)
        records[Record("file declaration inventory", scope)] = evidence
        records[Record("route specification", scope)] = route_record(snap, scope)
    records[Record("declaration span diff", label)] = (
        file in frozen.changed_paths and overlaps_change(opcodes, entry["line"], entry["end_line"], side)
    ) or failed(check, file, snapshots[side]["rev"],
                "the declaration span does not overlap a changed block in the selected diff",
                side=label, line=entry["line"])
    return decide("Declaration removed from file" if side == 0 else "Declaration added to file", records)


def declaration_changes(frozen, old_sites, new_sites, inventories, diffs):
    snapshots = frozen.before["service"], frozen.after["service"]
    by_file = _by_file(old_sites), _by_file(new_sites)
    scoped, failures, kinds, notices, proofs = [], [], [], [], {}
    for file in sorted(by_file[0].keys() | by_file[1].keys()):
        opcodes, mapped = diffs[file]
        removed, added = _site_delta(*[index[file] for index in by_file], mapped)
        local_changes = []
        for side, entries in enumerate((removed, added)):
            for entry in entries:
                proof = _entry_proof(frozen, snapshots, inventories, file, entry, side, opcodes)
                if not proof["confirmed"]:
                    failures.extend(proof["failures"])
                else:
                    local_changes.append({"kind": ("removal", "addition")[side], "site": entry,
                                          "proof": "complete inventories and changed source lines", "claim": proof})
        if (len(by_file[0][file]) == len(by_file[1][file]) == len(removed) == len(added) == 1
                and len(local_changes) == 2):
            kind = "Route declaration changed"
            records = {Record(name, side): True for name in ("file declaration inventory", "declaration span diff")
                       for side in ("before", "after")}
            records.update({Record("route specification", side): route_record(snap, side)
                            for side, snap in zip(("before", "after"), snapshots)})
            proofs[kind] = decide(kind, records)
            if proofs[kind]["confirmed"]:
                kinds.append(kind)
        else:
            scoped.extend(local_changes)
        if by_file[0][file] and by_file[1][file] and not removed and not added:
            affected = set()
            for index, entries in enumerate(by_file):
                for site in entries[file]:
                    handler = (site["handler_site"] or {}).get("file")
                    if not handler and site["kind"] == "annotation" and site["function"]:
                        handler = file
                    if handler in frozen.changed_paths:
                        affected.add(handler)
                    if overlaps_change(opcodes, site["line"], site["end_line"], index):
                        affected.add(file)
            notices.extend(failed("Validation or handler edits", path, snapshots[1]["rev"],
                                  "the two source bodies were not compared") for path in sorted(affected))
    presence = ["present" if sites else "unknown" if snap["unaccounted_scopes"] else "absent"
                for sites, snap in zip((old_sites, new_sites), snapshots)]
    for side, (owned, other) in enumerate(((old_sites, new_sites), (new_sites, old_sites))):
        if not owned or other:
            continue
        word = ("removal", "addition")[side]
        kind = "Operation removed" if side == 0 else "Operation added"
        records = declaration_records(snapshots, kind)
        records[Record("declaration span diff")] = failures or any(row["kind"] == word for row in scoped)
        proof = proofs[kind] = decide(kind, records)
        if proof["confirmed"]:
            kinds.append(kind)
        else:
            presence[1 - side] = "unknown"
            kinds.append(f"Listed only {('before', 'after')[side]} among the declarations read")
            affected = {(row["file"], row["rev"], row["side"], row["reason"]) for row in failures}
            failures.extend(row for row in proof["failures"]
                            if (row["file"], row["rev"], row["side"], row["reason"]) not in affected)
    if not {"Operation added", "Operation removed"}.intersection(kinds):
        kinds.extend(f"Declaration {'removed from' if row['kind'] == 'removal' else 'added to'} {row['site']['file']}"
                     for row in scoped)
    removed = [row["site"] for row in scoped if row["kind"] == "removal"]
    added = [row["site"] for row in scoped if row["kind"] == "addition"]
    if len(old_sites) == len(new_sites) == len(removed) == len(added) == 1:
        lhs, rhs = removed[0], added[0]
        if lhs["file"] != rhs["file"] and _signature(lhs) == _signature(rhs):
            records = {Record("file declaration inventory", side): True for side in ("before", "after")}
            records.update({Record("route specification", side): route_record(snap, side)
                            for side, snap in zip(("before", "after"), snapshots)})
            records[Record("one-to-one declaration correspondence")] = True
            proofs["Declaration moved"] = decide("Declaration moved", records)
    return kinds, scoped, presence, failures, notices, proofs


def measured_operation_sides(group):
    sides = set()
    for kind, side in (("Operation removed", "before"), ("Operation added", "after")):
        if group["claims"].get(kind, {}).get("confirmed"):
            sides.add(side)
    if group["claims"].get("Route declaration changed", {}).get("confirmed"):
        sides.update(("before", "after"))
    sides.update("before" if row["kind"] == "removal" else "after" for row in group["file_changes"])
    sides.update(side for side in ("before", "after") if any(field[side] for field in group["fields"]))
    if group["status_removals"]:
        sides.add("before")
    if group["status_additions"]:
        sides.add("after")
    return {side for side in sides if group[f"{side}_presence"] == "present"}


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
