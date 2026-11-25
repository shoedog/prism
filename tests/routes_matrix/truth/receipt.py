"""Check the independent observations behind a saved projection."""
from collections import Counter

from run_truth import project, tree_sha256
from stage import methods

KINDS = {
    "express": {"route", "use", "verb"},
    "fastapi": {"api_route", "websocket"},
    "flask": {"url_rule"},
}


def validate(row, truth, digest):
    """Yield field, expected and observed triples for inconsistent evidence."""
    framework = truth.get("framework")
    hint = {pin["framework"] for pin in row["expect"] if "framework" in pin}
    if truth.get("id") != row["id"]:
        yield "truth.id", row["id"], truth.get("id")
    if truth.get("fixture_sha256") != digest:
        yield "truth.fixture_sha256", digest, truth.get("fixture_sha256")
    if framework not in KINDS or (hint and framework not in hint):
        yield "truth.framework", sorted(hint or KINDS), framework
        return
    calls = {}
    ordinals = Counter()
    for index, call in enumerate(truth["calls"], start=1):
        site, call_kind = call["site"], call["kind"]
        call_id = call["id"]
        key = site["file"], site["line"]
        ordinals[key] += 1
        origin_id = call.get("origin_call_id")
        args, callbacks = call["args"], call["callbacks"]
        path_key = "rule" if framework == "flask" else "path"
        path = args[path_key]
        written = call["path"]["literal"] if "path" in call else None
        if call_kind not in {"route", "use"}:
            declared = args["methods"]
            if (declared is not None and declared != "ANY"
                    and (not isinstance(declared, list) or not all(isinstance(m, str) for m in declared))):
                yield f"truth.calls[{index - 1}].args.methods", "native method declaration", declared
        if not isinstance(callbacks, list) or (isinstance(written, str) and written != path):
            yield f"truth.calls[{index - 1}]", "raw callback array and compatible path evidence", call
        if any(callback.get("kind") == "javascript" and type(callback.get("anonymous")) is not bool
               for callback in callbacks):
            yield f"truth.calls[{index - 1}].callbacks", "JavaScript anonymity marker", callbacks
        if origin_id in calls:
            origin = calls[origin_id]
            if callbacks != origin["callbacks"] or args["methods"] != origin["args"]["methods"]:
                yield f"truth.calls[{index - 1}].origin_call_id", "copy preserves callbacks and declared methods", origin_id
            if framework == "fastapi" and not path.endswith(origin["args"]["router_prefix"] + origin["args"]["path"]):
                yield f"truth.calls[{index - 1}].args.path", "copy retains the native source path", path
        if call_kind == "use":
            has_path = args["has_path_arg"]
            if (type(has_path) is not bool or not callbacks
                    or (call.get("mount") and call["mount_prefix"] != (path if has_path else ""))
                    or (not has_path and path != "/")):
                yield f"truth.calls[{index - 1}]", "consistent Express use arguments and callback", call
        if call_kind == "route" and callbacks:
            yield f"truth.calls[{index - 1}].callbacks", [], callbacks
        if (type(call_id) is not int or call_id != index or call_kind not in KINDS[framework]
                or site["ordinal"] != ordinals[key] or call["mounted"] is not (origin_id is not None)
                or (framework != "express" and call["framework"] != framework)
                or (origin_id is not None and (origin_id not in calls or calls[origin_id]["kind"] != call_kind))):
            yield f"truth.calls[{index - 1}]", "sequential native call with a valid site and origin", call
        calls[call_id] = call
    if not row["expect"] and row.get("expect_unlisted") and row["id"] not in {"et1-10", "es1-20", "es3-07"}:
        count = sum(call["kind"] == "use" and not call.get("mount") for call in calls.values())
        if count != row["expect_unlisted"]:
            yield "truth.calls", row["expect_unlisted"], count
    for index, served in enumerate(truth["served"]):
        linked = [calls[call_id] for call_id in served["call_ids"]]
        kind = "websocket" if linked[0]["kind"] == "websocket" else "http"
        declared = None
        if framework == "express":
            declared = [call["args"]["methods"] for call in linked if call["kind"] == "verb"]
            declared = "ANY" if "ANY" in declared else methods(m for leg in declared for m in leg)
            if not isinstance(served["path"], str):
                if not any(call["args"]["path"] == served["path"] for call in linked):
                    yield f"truth.served[{index}].path", "linked regular expression", served["path"]
        else:
            previous = None
            for call in linked:
                if call.get("origin_call_id") != previous:
                    yield f"truth.served[{index}].call_ids", "complete ordered copy chain", served["call_ids"]
                    break
                previous = call["id"]
            raw = linked[0]["args"]["methods"]
            declared = methods(raw) if isinstance(raw, list) else raw
            last = linked[-1]["args"]
            path = last["rule"] if framework == "flask" else last["router_prefix"] + last["path"]
            if framework == "flask" and linked[0]["callbacks"] and served["runtime_callback"] != linked[0]["callbacks"][0]:
                yield f"truth.served[{index}].runtime_callback", linked[0]["callbacks"][0], served["runtime_callback"]
            if served["path"] != path:
                yield f"truth.served[{index}].path", path, served["path"]
        if served["kind"] != kind:
            yield f"truth.served[{index}].kind", kind, served["kind"]
        if served["declared_methods"] != declared:
            yield f"truth.served[{index}].declared_methods", declared, served["declared_methods"]

    if any(isinstance(call["args"].get("path"), dict) and call["kind"] == "verb"
           and not any(call_id in served["call_ids"] for served in truth["served"])
           for call_id, call in calls.items()) and any(
               exposure.get("reason") == "regex_path" for pin in row["expect"] for exposure in pin.get("paths", ())
           ):
        yield "truth.served", "served regex evidence for the pinned registration", truth["served"]

    projected = project(truth)
    if truth["projection"] != projected:
        yield "truth.projection", "projection reconstructed from raw calls and served evidence", truth["projection"]
