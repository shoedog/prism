from __future__ import annotations


CLASSES = ["fixture", "kill:alias", "alias-d3"]


def parse_trace(line, arm):
    tag, mechanism, file, owner, *places = line.rstrip("\n").split("\t")
    if tag != "rd-census":
        return {"arm": arm, "tag": tag, "kind": "write", "key": {}}
    source, destination = places
    return {
        "arm": arm, "tag": tag, "kind": "edge", "mechanism": mechanism,
        "key": {"from": endpoint(source), "to": {"file": file, **endpoint(destination)}},
    }


def endpoint(place):
    path, coordinates = place.rsplit("@", 1)
    line, byte = coordinates.split(":")
    base, *fields = path.replace("[]", ".[]").split(".")
    return {"base": base, "fields": fields, "line": int(line), "start_byte": int(byte)}


def classify(change, context):
    for record in context.records(change):
        mechanism = record["mechanism"]
        if mechanism in CLASSES:
            return mechanism
    return None


def validate(bound):
    delta = bound.counter_deltas["dfg_label_nameonly"]
    if delta != 1:
        return [f"dfg_label_nameonly: observed {delta:+d}, predicted +1"]
    return []
