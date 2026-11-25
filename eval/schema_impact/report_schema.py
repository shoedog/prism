"""Read message coverage separately from the existing five-property field index."""

from collections import defaultdict
from dataclasses import asdict
from hashlib import sha256

import yaml

from . import schema
from .ids import op_id
from .producer_go import _entries
from .report_sources import site


_JSON = "application/json"
_STOPS = {
    "nonlocal_ref": "unreadable definition", "unresolved_ref": "unreadable definition",
    "depth_cut": "depth limit", "recursive_composition": "recursive definition",
    "anyOf_union": "alternative shapes", "oneOf_union": "alternative shapes",
    "anyof_union": "alternative shapes", "oneof_union": "alternative shapes",
    "allof_overlap": "overlapping definitions", "field_id_collision": "ambiguous field paths",
    "unescaped_property_name": "ambiguous field path",
}


def _media(document, operation, message, openapi, direction):
    if openapi:
        content = message.get("content")
        if not isinstance(content, dict):
            return None, None, f"the {direction} media list could not be read"
        return sorted(content), "message content", None
    key = "consumes" if direction == "request" else "produces"
    owner = operation if key in operation else document
    if key not in owner:
        return None, None, f"neither the operation nor the document declares {direction} media types"
    media = owner[key]
    if not isinstance(media, list) or not all(isinstance(entry, str) for entry in media):
        return None, None, f"the {direction} media list could not be read"
    return media, f"{'operation' if owner is operation else 'document'} {key}", None


def _root(document, operation, message, body, openapi, direction):
    if body is None and not message.get("content"):
        return {"body_present": False, "declared_media": None, "media_source": None,
                "selected_schema": None, "coverage": "complete", "reason": None}
    media, source, reason = _media(document, operation, message, openapi, direction)
    if not reason and _JSON not in media:
        reason = "this body declares " + ", ".join(media) if media else "the media list is empty"
    if openapi and not reason:
        body = schema._mapping(message["content"][_JSON]).get("schema")
    if not reason and not isinstance(body, dict):
        reason = "the JSON body schema could not be read"
    return {
        "body_present": True, "declared_media": media, "media_source": source,
        "selected_schema": body, "coverage": "not_checked" if reason else "complete",
        "reason": f"JSON fields: Not checked — {reason}" if reason else None,
    }


def _duplicate_keys(node):
    pending, visited = [node], set()
    while pending:
        current = pending.pop()
        if id(current) in visited:
            continue
        visited.add(id(current))
        if isinstance(current, yaml.MappingNode):
            keys = [key.value for key, _ in current.value if isinstance(key, yaml.ScalarNode)]
            if len(keys) != len(set(keys)):
                return True
            pending.extend(value for _, value in current.value)
        elif isinstance(current, yaml.SequenceNode):
            pending.extend(current.value)
    return False


def _empty(snapshot, file, reason, status):
    return {"schema": {"source": {"path": file, "rev": snapshot["rev"], "tree": snapshot["tree"],
                                  "blob": snapshot["tree_files"].get(file, {}).get("blob")},
                       "sha256": None, "content_hash": None, "status": status, "reason": reason,
                       "operations": None, "message_roots": None, "fields": None,
                       "success_statuses": None, "warnings": None}}


def schema_inventory(snapshot, file, source):
    if file is None or file not in snapshot["tree_files"]:
        return _empty(snapshot, file, "no committed field schema", "missing")
    if source is None:
        return _empty(snapshot, file, "the committed schema file could not be read", "not_checked")
    encoded = source.encode("utf-8")
    try:
        document, openapi = schema._parse(encoded)
        tree = yaml.compose(source, Loader=yaml.CSafeLoader)
        if not isinstance(document.get("paths"), dict) or _duplicate_keys(tree):
            raise ValueError("schema paths missing or duplicate keys")
        index = schema.load(encoded)
        return _read(snapshot, file, source, document, openapi, index, tree)
    except (ValueError, TypeError, KeyError, AttributeError, RecursionError, yaml.YAMLError):
        result = _empty(snapshot, file, "the committed schema could not be parsed or contains ambiguous entries",
                        "not_checked")
        result["schema"]["sha256"] = sha256(encoded).hexdigest()
        return result


def _read(snapshot, file, source, document, openapi, index, tree):
    nodes = _entries(_entries(tree)["paths"][1])
    lines = source.splitlines()
    reader = schema._Reader(document)
    ops, messages, fields, statuses = {}, {}, {}, {}
    base = schema._base(document, openapi)
    for source_path, raw_path in document["paths"].items():
        if not source_path.startswith("/"):
            continue
        path = reader.message(raw_path, source_path)
        path_node = _entries(nodes[source_path][1])
        for method in sorted(schema._METHODS & path.keys()):
            operation_id = op_id(method, source_path, base)
            if operation_id in ops:
                ops[operation_id]["status"] = "not_checked"
                ops[operation_id]["reason"] = "two schema declarations have the same method and path"
                continue
            raw_operation = path[method]
            operation = schema._mapping(raw_operation)
            suffix = operation_id.removeprefix("op:")
            req = f"req:{suffix}"
            op_node = path_node.get(method)
            op_site = None
            if op_node:
                line = op_node[0].start_mark.line
                op_site = site(snapshot, file, line + 1, text=lines[line], kind="schema operation")
            ops[operation_id] = {"site": op_site, "status": "complete" if isinstance(raw_operation, dict) else "not_checked",
                                 "reason": None if isinstance(raw_operation, dict) else "schema operation could not be read",
                                 "source_path": source_path}
            if openapi:
                message = reader.message(operation.get("requestBody"), req)
                messages[req] = _root(document, operation, message, None, True, "request")
            else:
                body = schema._swagger_body(reader, path, operation, req)
                messages[req] = _root(document, operation, {}, body, False, "request")
            if "responses" not in operation or not isinstance(operation["responses"], dict):
                statuses[operation_id] = None
                continue
            responses = operation["responses"]
            response_nodes = _entries(_entries(op_node[1]).get("responses", (None, None))[1]) if op_node else {}
            statuses[operation_id] = {}
            for key, raw in responses.items():
                status = str(key)
                if not schema._SUCCESS.fullmatch(status) and not (openapi and status == "2XX"):
                    continue
                root = f"resp:{suffix}:{status}"
                response = reader.message(raw, root)
                messages[root] = _root(document, operation, response, response.get("schema"), openapi, "response")
                status_node = response_nodes.get(status)
                status_site = None
                if status_node:
                    line = status_node[0].start_mark.line
                    status_site = site(snapshot, file, line + 1, text=lines[line], kind="schema response status")
                statuses[operation_id][status] = {"site": status_site, "root": root}

    stops = defaultdict(list)
    for warning in sorted(set(index.warnings) | reader.warnings):
        kind, _, detail = warning.partition(" ")
        if kind not in _STOPS:
            continue
        location = detail.removeprefix("field:")
        region = next((root for root in messages
                       if location == root or location.startswith((root + "#", root + " "))), None)
        if region:
            pointer = location.partition("#/")[2].split(" '", 1)[0] or "body"
            stops[region].append(f"Not checked — {pointer} stops at {_STOPS[kind]}")
        elif location.startswith("/"):
            region = location.split(" ", 1)[0]
            for operation, row in ops.items():
                if row["source_path"] == region:
                    row.update(status="not_checked", reason="the referenced definition could not be read")
    for key, column in index.fields.items():
        root, _, pointer = key.removeprefix("field:").partition("#/")
        if column.recursive:
            stops[root].append(f"Not checked — {pointer} stops at a recursive definition")
        if len(column._shapes) != 1:
            stops[root].append(f"Not checked — {pointer} stops at alternative shapes")
        fields[key] = {
            "field_id": key, "operation_id": index._field_operations[key],
            "definition": column.definition, "root": root, "path": pointer,
            "shape": asdict(column._shapes[0]) if len(column._shapes) == 1 else None,
            "site": None, "limits": None, "recursive": column.recursive,
        }
    for root, record in messages.items():
        record["root"] = root
        record["stops"] = sorted(set(stops[root]))
    schema_source = {"path": file, "rev": snapshot["rev"], "tree": snapshot["tree"],
                     "blob": snapshot["tree_files"][file]["blob"], "repo": snapshot["repo"],
                     "committed_at": snapshot["committed_at"]}
    return {
        "sha256": index.sha256, "content_hash": index.content_hash,
        "schema": {"source": schema_source, "sha256": index.sha256, "content_hash": index.content_hash,
                   "info_version": index.info_version, "format": "openapi-3" if openapi else "swagger-2.0",
                   "status": "complete", "reason": None, "operations": ops, "message_roots": messages,
                   "fields": fields, "success_statuses": statuses, "warnings": index.warnings},
    }


def compare_schema(before, after):
    old, new = before["schema"], after["schema"]
    if old["status"] != "complete" or new["status"] != "complete":
        if old["status"] == new["status"] == "missing":
            return {}, ["Changed fields: Not checked — no committed field schema"]
        return {}, [f"Changed fields: Not checked — {label} {side['source']['path'] or 'schema'}: {side['reason']}"
                    for label, side in (("before", old), ("after", new)) if side["status"] != "complete"]
    groups, limits = {}, []
    indexed = []
    for side in (old, new):
        by_root = defaultdict(dict)
        for field in side["fields"].values():
            by_root[field["root"]][field["path"]] = field
        indexed.append(by_root)
    for identifier in sorted(old["operations"].keys() | new["operations"].keys()):
        old_op, new_op = old["operations"].get(identifier), new["operations"].get(identifier)
        suffix = identifier.removeprefix("op:")
        if any(op and op["status"] != "complete" for op in (old_op, new_op)):
            limits.extend(f"{suffix}: Not checked — {op['reason']}" for op in (old_op, new_op)
                          if op and op["status"] != "complete")
            continue
        a = old["success_statuses"].get(identifier) if old_op else {}
        b = new["success_statuses"].get(identifier) if new_op else {}
        group = {"fields": [], "status_additions": [], "status_removals": [], "response_root_pairs": [],
                 "limits": [], "schema_sites": [old_op["site"] if old_op else None,
                                                new_op["site"] if new_op else None], "message_coverage": []}
        changed_scope = (a is None) != (b is None)
        for label, statuses in (("before", a), ("after", b)):
            if statuses is None:
                group["limits"].append(f"Status change: Not checked — {label} response status list is missing.")
        if a is None or b is None:
            for side, statuses in ((old, a), (new, b)):
                for status in (statuses or {}).values():
                    record = side["message_roots"][status["root"]]
                    group["limits"].extend(_message_limits(status["root"], record))
        pairs = [(f"req:{suffix}", f"req:{suffix}")]
        if a is not None and b is not None:
            removed, added = sorted(a.keys() - b.keys()), sorted(b.keys() - a.keys())
            group["status_removals"] = [{"status": status, **a[status]} for status in removed]
            group["status_additions"] = [{"status": status, **b[status]} for status in added]
            pairs.extend((a[status]["root"], b[status]["root"]) for status in sorted(a.keys() & b.keys()))
            range_overlaps = ("2XX" in a or "2XX" in b) and any(
                status != "2XX" for status in removed + added
            )
            if range_overlaps:
                group["limits"].append("Body comparison: Not checked — the status range overlaps")
            elif len(removed) == len(added) == 1:
                ra, rb = a[removed[0]]["root"], b[added[0]]["root"]
                if all(rec["body_present"] and rec["coverage"] == "complete"
                       for rec in (old["message_roots"][ra], new["message_roots"][rb])):
                    group["response_root_pairs"].append({"before": ra, "after": rb,
                                                         "before_status": removed[0], "after_status": added[0]})
                    pairs.append((ra, rb))
                else:
                    pairs.extend([(ra, None), (None, rb)])
            elif removed and added:
                group["limits"].append("Body comparison: Not checked — several bodies could correspond")
            else:
                pairs.extend((a[status]["root"], None) for status in removed)
                pairs.extend((None, b[status]["root"]) for status in added)

        for ra, rb in pairs:
            records = [side["message_roots"].get(root) for side, root in ((old, ra), (new, rb))]
            af, bf = indexed[0][ra], indexed[1][rb]
            changed = records[0] != records[1] or af != bf
            changed_scope |= changed
            gaps = [message for root, record in zip((ra, rb), records) for message in _message_limits(root, record)]
            if gaps:
                group["limits"].extend(gaps)
                continue
            if changed:
                for label, record in zip(("before", "after"), records):
                    if record and record["body_present"]:
                        group["message_coverage"].append({"side": label, **record})
            for path in sorted(af.keys() | bf.keys()):
                lhs, rhs = af.get(path), bf.get(path)
                previous, following = lhs["shape"] if lhs else None, rhs["shape"] if rhs else None
                if previous != following:
                    group["fields"].append({"before": lhs, "after": rhs, "before_shape": previous,
                                            "after_shape": following, "path": path})
        group["limits"] = sorted(set(group["limits"]))
        if any(group[key] for key in ("fields", "status_additions", "status_removals")) or (
                changed_scope and group["limits"]):
            groups[identifier] = group
        else:
            limits.extend(f"{suffix} · {gap}" for gap in group["limits"])
    return groups, limits


def _message_limits(root, record):
    if record is None:
        return []
    if root.startswith("req:"):
        message = f"{root[4:]} request body"
    else:
        operation, status = root[5:].rsplit(":", 1)
        message = f"{operation} response body ({status})"
    reasons = ([record["reason"]] if record["reason"] else []) + record["stops"]
    return [f"{message} · {reason}" for reason in reasons]
