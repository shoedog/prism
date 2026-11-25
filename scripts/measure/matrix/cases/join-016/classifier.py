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


EXPECTED = {"alias_unstable": "kill:alias", "cfg_incomplete": "alias-d3"}


def classify(change, context):
    expected = {EXPECTED.get(label["doubt"]) for label in change["head_bag"]}
    mechanisms = {record["mechanism"] for record in context.records(change)}
    if mechanisms == expected and len(mechanisms) == 1:
        mechanism, = mechanisms
        return mechanism
    return None
