from __future__ import annotations

CLASSES = []


def parse_trace(line, arm):
    tag, mechanism, file, owner, start, *flip = line.rstrip("\n").split("\t")
    if mechanism.startswith("guard-deleted:") and not flip:
        delta = -1
    elif mechanism in {"arrow-body-node", "loop-exit"} and len(flip) == 1:
        delta = {"ok>no-cfg": 1, "no-cfg>ok": -1}[flip[0]]
    else:
        return None
    return {"arm": arm, "tag": tag, "kind": "owner", "key": {},
            "owner": [file, owner, int(start)], "delta": delta}


def classify(change, context):
    return None


def validate(bound):
    predicted = sum(record["delta"] for record in bound.records)
    if predicted != bound.counter_deltas["dfg_rd_functions_without_cfg"]:
        return ["unexpected owner availability delta"]
    return []
