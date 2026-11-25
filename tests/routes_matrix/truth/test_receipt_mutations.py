"""Single-field corruption checks over every executed route-matrix receipt."""
from collections import Counter
from copy import deepcopy
from pathlib import Path
import unittest

from reconcile import RUNTIME_ONLY, compare_receipt, read_jsonl
from receipt import tree_sha256


def edits(value, path=(), pattern=""):
    if isinstance(value, dict):
        for key, child in value.items():
            name = f"{pattern}.{key}".lstrip(".")
            yield (*path, key), name, True, None
            yield from edits(child, (*path, key), name)
    elif isinstance(value, list):
        yield path, pattern, False, [] if value else [None]
        for index, child in enumerate(value):
            yield from edits(child, (*path, index), pattern + "[]")
    else:
        if isinstance(value, bool):
            replacement = not value
        elif isinstance(value, int):
            replacement = value + 100_000
        elif isinstance(value, str):
            replacement = value + "--corrupted--"
        else:
            replacement = "--corrupted--"
        yield path, pattern, False, replacement


def corrupt(entry, path, drop, replacement):
    changed = deepcopy(entry)
    parent = changed
    for key in path[:-1]:
        parent = parent[key]
    if drop:
        del parent[path[-1]]
    else:
        parent[path[-1]] = replacement
    return changed


def static_only(row, truth, path, field):
    """Identify observations that require the source oracle or native replay."""
    if field in ("served[].effective_methods", "served[].effective_methods[]"):
        return "effective methods"
    if field in ("served[].endpoint", "calls[].args.endpoint", "calls[].args.name"):
        return "native endpoint names"
    calls, served = truth["calls"], truth["served"]
    if field.startswith("served[].runtime_callback"):
        linked = next(call for call in calls if call["id"] == served[path[1]]["call_ids"][0])
        if not linked["callbacks"]:
            return "endpoint-only runtime callback"
    if path[0] != "calls" or len(path) < 2:
        return None

    call = calls[path[1]]
    call_id = call["id"]
    projection = next((record for record in truth["projection"] if record["call_id"] == call_id), None)
    mounted = any(call_id in record.get("mount_call_ids", ()) for record in served)
    copied = any(other.get("origin_call_id") == call_id for other in calls)
    installed = any(record["call_ids"][-1] == call_id for record in served)
    if field in ("calls[].site.column", "calls[].site.column_byte", "calls[].site.end_column_byte",
                 "calls[].site.end_line") and row["id"] not in RUNTIME_ONLY:
        return "unpinned call-frame columns and end line"
    if field in ("calls[].site.file", "calls[].site.line") and projection is None and not mounted:
        return "unpaired call-frame file and line"
    if field == "calls[].args.router_prefix" and not installed and not copied:
        return "unconsumed router prefix"
    if field == "calls[].args.has_path_arg" and not mounted and "path" not in call:
        return "untraced middleware path presence"
    if field in ("calls[].path", "calls[].path.raw", "calls[].path.literal") and projection is None and not any(
            pin["prefix"]["status"] == "unevaluated"
            and (pin["site"]["file"], pin["site"]["line"]) == (call["site"]["file"], call["site"]["line"])
            for pin in row.get("expect_mounts", ())):
        return "unprojected source-path helper"
    if field in ("calls[].callbacks[].column", "calls[].callbacks[].source"):
        return "raw callback column and source"
    if (field.startswith("calls[].callbacks[].") and "callbacks" not in (projection or {})
            and not (copied or call["mounted"]) and not (truth["framework"] == "flask" and installed)):
        return "unprojected callback identity"
    return None


class ReceiptMutations(unittest.TestCase):
    def test_every_checked_saved_truth_field_rejects_generated_corruption(self):
        root = Path(__file__).resolve().parent.parent
        rows = read_jsonl(root / "rows.jsonl")
        receipts = read_jsonl(root / "truth/truth.jsonl")
        counts, exceptions = Counter(), Counter()
        survivors = []
        for row_id, (_, row) in rows.items():
            if row["truth_source"] != "executed":
                continue
            truth = receipts[row_id][1]
            digest = tree_sha256(root / row["dir"])
            self.assertFalse(compare_receipt(row, truth, digest).differences, row_id)
            counts["rows"] += 1
            row_fields = set()
            for path, field, drop, replacement in edits(truth):
                reason = static_only(row, truth, path, field)
                if reason is not None:
                    exceptions[reason] += 1
                    continue
                row_fields.add(path)
                counts["tested"] += 1
                if compare_receipt(row, corrupt(truth, path, drop, replacement), digest).differences:
                    counts["rejected"] += 1
                else:
                    survivors.append((row_id, path, "drop" if drop else replacement))
            counts["fields"] += len(row_fields)
        print(f"{counts['rows']} executed rows, {counts['fields']} field instances, "
              f"{counts['rejected']}/{counts['tested']} corruptions rejected"
              f"; static-only: {dict(sorted(exceptions.items()))}")
        self.assertFalse(survivors, f"{len(survivors)} accepted corruptions: {survivors}")

    def test_native_call_ordinal_rejects_a_json_boolean(self):
        root = Path(__file__).resolve().parent.parent
        row_id = "ex02"
        row = read_jsonl(root / "rows.jsonl")[row_id][1]
        truth = read_jsonl(root / "truth/truth.jsonl")[row_id][1]
        digest = tree_sha256(root / row["dir"])
        self.assertEqual(truth["calls"][0]["site"]["ordinal"], 1)
        changed = corrupt(truth, ("calls", 0, "site", "ordinal"), False, True)
        self.assertTrue(compare_receipt(row, changed, digest).differences)
