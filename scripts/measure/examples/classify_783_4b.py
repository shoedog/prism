"""Classifier for the PR #786 alias-kill and alias-d3 transitions."""

import re

CLASSES = ["kill:alias", "alias-d3"]
STRAYS = "allow"
LEGACY_MECHANISM_ORDER = {
    "X": ["d2c", "capture-body", "capture-region", "kill:update", "loop-exit", "d3-loop",
          "guard-deleted:f", "arrow-body-node", "capture-param"],
    "EX": ["capture-region", "d2c", "capture-body", "loop-exit", "d3-loop", "guard-deleted:a",
           "kill:update"],
    "T": ["capture-body", "capture-region", "d3-loop", "loop-exit", "d2c", "d3-use",
          "capture-param", "kill:update", "guard-deleted:a", "do-cond", "kill:for-increment",
          "arrow-body-node", "owner-refuse:parse", "guard-deleted:f", "owner-refuse:one-node"],
}
PLACE = re.compile(r"(.+)@([1-9][0-9]*):([0-9]+)")
DOUBT = {"alias_unstable": "kill:alias", "sameline": "alias-d3"}


def parse_trace(line, arm):
    columns = line.rstrip("\r\n").split("\t")
    event = columns[0]
    if event in ("rd-alias-cert", "rd-alias-move", "rd-census-owner"):
        return dict(arm=arm, tag=event, kind="owner" if event.endswith("-owner") else "write",
                    key={}, columns=columns[1:])
    if event != "rd-census":
        return None
    if len(columns) != 6:
        raise ValueError("rd-census requires six fields")
    _, mechanism, file, owner, source, destination = columns
    places = [PLACE.fullmatch(place) for place in (source, destination)]
    if not all(places):
        raise ValueError("rd-census place requires path@line:start_byte")
    paths = [re.sub(r"[^A-Za-z0-9_$]", "", match[1]) for match in places]
    source_line, destination_line = (int(match[2]) for match in places)
    legacy = [file, paths[0], source_line, paths[1], destination_line]
    kind, key = "counter", {}
    if mechanism in CLASSES:
        kind = "edge"
        # The producer provides the destination file and two coordinates.
        # Display path spellings cannot certify the structured path fields.
        key = {"from": dict(line=source_line, start_byte=int(places[0][3])),
               "to": dict(file=file, line=destination_line, start_byte=int(places[1][3]))}
    return dict(arm=arm, tag=mechanism, kind=kind, key=key, owner=owner, mechanism=mechanism,
                legacy_key=legacy)


def classify(change, context):
    if change["kind"] != "label_moved":
        return None
    before, after = change["base_bag"][0], change["head_bag"][0]
    if before["confidence"] != "exact" or after["confidence"] != "nameonly":
        return None
    wanted = DOUBT.get(after["doubt"])
    if wanted is not None and {record["tag"] for record in context.records(change)} == {wanted}:
        return wanted
    return None


def validate(bound):
    if bound.head_receipt["corpus"]["key"] != "R4B":
        return []
    count = sum(row["status"] == "stray" for row in bound.joins)
    return [f"R4B forbidden stray edge records: {count}"] if count else []


def historical_trials(expected, primary_identity, fixture_pass, measure_trace):
    """Apply TARGET's ordered trials to observations from a measurement runner.

    measure_trace(corpus, head) returns the binary digest and exact stdout
    line for that attempt. primary_identity is the already established
    primary edge/counter claim per corpus, covering both required key modes.
    """
    heads = ("1f2ad1cc", "32ad7c09b", "3510ed4f2")
    trials = {}
    for corpus in ("X", "EX", "T"):
        trials[corpus] = attempts = []
        for head in heads:
            digest, line = measure_trace(corpus, head)
            matched = line == expected[corpus]
            attempts.append(dict(head=head, sha256=digest, trace_line=line, trace_match=matched,
                                 identity=primary_identity[corpus] if head == heads[0] else None))
            if matched:
                break
    passed = fixture_pass and all(primary_identity[corpus] and values[-1]["trace_match"]
                                  for corpus, values in trials.items())
    return {"pass": passed, "trials": trials}
