from __future__ import annotations

CLASSES = []


def parse_trace(line, arm):
    tag, mechanism, file, owner, start, *rest = line.rstrip("\n").split("\t")
    if tag != "rd-census-owner":
        return None
    key = {"file": file, "owner": owner, "line": int(start)}
    if mechanism.startswith("guard-deleted:") and not rest:
        delta = -1
    elif mechanism in {"arrow-body-node", "loop-exit"} and len(rest) == 1:
        control, head = rest[0].split(">")
        delta = int(head == "no-cfg") - int(control == "no-cfg")
    elif mechanism in {"param-alias", "param-bare"} and len(rest) == 1:
        key = {"file": file, "owner": owner, "first_line": int(start), "last_line": int(rest[0])}
        delta = 0
    else:
        return None
    return {"arm": arm, "tag": tag, "kind": "owner", "key": key,
            "mechanism": mechanism, "delta": delta}


def classify(change, context):
    return None


def validate(bound):
    predicted = sum(record["delta"] for record in bound.records
                    if record["tag"] == "rd-census-owner")
    if predicted != bound.counter_deltas["dfg_rd_functions_without_cfg"]:
        return ["unexpected owner availability delta"]
    return []
