"""Decode wire populations and select original occurrences before Node starts."""

from __future__ import annotations

from collections import Counter, defaultdict
from pathlib import Path

from . import edges, receipt
from .diff_stats import Run, admit_pair
from .inputs import json_object, jsonl_objects
from .oracle_inputs import RESULTS, digest, name, regular, safe_path, shape

ARMS = ("base", "head")
TARGET_CONFIDENCES = ("exact", "name_only")


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def admit(path, entries):
    directory = path.absolute()
    record, sha256 = receipt.admit(directory, entries=entries, with_digest=True)
    return Run(directory, record, sha256)


def number(value, minimum=0):
    return edges.integer(value, minimum) and value <= 2**53 - 1


def coordinate(value):
    # Invalid/missing coordinates on an otherwise well-formed claim are
    # classified against the authenticated source by the worker.
    require(isinstance(value, dict) and safe_path(value.get("file")), "wire-coordinate: invalid file")
    for key in ("line", "start_byte", "end_byte"):
        current = value.get(key)
        require(current is None or type(current) is int and abs(current) <= 2**53 - 1,
                "wire-coordinate: expected integer or null")


def edge(row):
    require(row.keys() >= {"from", "to", "confidence", "doubt"}, "edge-wire: missing field")
    require(row["confidence"] in ("exact", "nameonly"), "edge-wire: invalid confidence")
    require(row["doubt"] is None or row["confidence"] == "nameonly" and name(row["doubt"]),
            "edge-wire: invalid doubt")
    require(row.get("kill_line") is None or number(row["kill_line"], 1), "edge-wire: invalid kill_line")
    require("relation" not in row or name(row["relation"]), "edge-wire: invalid relation")
    for side in edges.SIDES:
        endpoint = row[side]
        coordinate(endpoint)
        require(endpoint.get("access") in ("def", "use"), "edge-wire: invalid access")
        edges.path_fields(endpoint.get("path"))
    return row


def function_id(value):
    require(isinstance(value, dict) and safe_path(value.get("file")) and name(value.get("name"))
            and number(value.get("start_line"), 1) and
            (value.get("end_line") is None or number(value["end_line"], value["start_line"])),
            "sites-wire: invalid function_id")


def site(row):
    require(row.keys() >= {"caller", "source_span", "callee_text", "origin", "drop",
                            "exact_target", "resolved_targets"}, "sites-wire: missing field")
    function_id(row["caller"])
    coordinate(row["source_span"])
    require(name(row["callee_text"]) and name(row["origin"]) and
            (row["drop"] is None or name(row["drop"])) and isinstance(row["resolved_targets"], list)
            and ("relation" not in row or name(row["relation"])), "sites-wire: invalid call record")
    if row["exact_target"] is not None:
        function_id(row["exact_target"])
    for target in row["resolved_targets"]:
        require(isinstance(target, dict) and target.get("confidence") in TARGET_CONFIDENCES
                and (target.get("kind") is None or name(target["kind"])),
                "sites-wire: invalid target confidence or kind")
        function_id(target.get("function_id"))
    return row


def representable(value):
    pending = [value]
    while pending:
        member = pending.pop()
        if type(member) is int:
            require(abs(member) <= 2**53 - 1, "wire-json: integer exceeds exact Node range")
        elif isinstance(member, str):
            member.encode("utf-8")
        elif isinstance(member, dict):
            pending.extend(member)
            pending.extend(member.values())
        elif isinstance(member, list):
            pending.extend(member)
    return value


def read(run, role):
    require(any(command["id"] == role for command in run.receipt["commands"]),
            f"run binding: missing {role} artifact")
    return map(representable, jsonl_objects(run.artifact(role, "output"), run.expected(role, "output")))


def claim(row, ordinal, arm, *, index=None, change=None):
    return dict(row=row, ordinal=ordinal, arm=arm, index=index, change=change)


CERT_FIELDS = (
    "schema key_mode base_edges head_edges changed_keys collisions line_projection_loss line_reason "
    "attributed unattributed ambiguous occurrences base_residual head_residual by_kind by_class "
    "records unjoined trace_by_tag mechanisms validation failures result gate_sha256 gate_step_id "
    "inputs classifier_sha256 changes_sha256"
).split()
CHANGE_FIELDS = ("id", "key", "kind", "count", "base_bag", "head_bag", "records", "class", "error")
ATTRIBUTE_FIELDS = ("confidence", "doubt", "kill_line", "from_access", "to_access")


def certificate_arms(args, entries, gate):
    path = args.changes.absolute().with_name("certificate.json")
    raw = regular(path)
    cert = json_object(raw)
    require(shape(cert, CERT_FIELDS) and cert["schema"] == "prism.measure.certificate/1"
            and cert["key_mode"] in ("bytes", "line") and cert["result"] in ("pass", "fail")
            and receipt.is_digest(cert["changes_sha256"])
            and receipt.is_digest(cert["classifier_sha256"])
            and shape(cert["inputs"], ARMS), "certificate: invalid shape")
    receipt.gate_binding(cert["gate_sha256"], cert["gate_step_id"])
    require(gate is None or cert["gate_sha256"] == gate, "certificate: gate binding differs")
    paths = []
    for arm in ARMS:
        entry = cert["inputs"][arm]
        require(shape(entry, ("run", "receipt_sha256")) and name(entry["run"])
                and not Path(entry["run"]).is_absolute() and receipt.is_digest(entry["receipt_sha256"]),
                "certificate: invalid run reference")
        paths.append((path.parent / entry["run"]).resolve())
    pair = dict(zip(ARMS, admit_pair(*paths, entries)))
    require(all(pair[arm].receipt_sha256 == cert["inputs"][arm]["receipt_sha256"] for arm in ARMS),
            "certificate: receipt digest differs")
    return cert, pair, digest(raw)


def project_attribute(attribute):
    require(shape(attribute, ATTRIBUTE_FIELDS) and attribute["confidence"] in ("exact", "nameonly")
            and (attribute["doubt"] is None or attribute["confidence"] == "nameonly" and name(attribute["doubt"]))
            and (attribute["kill_line"] is None or number(attribute["kill_line"], 1))
            and all(attribute[side + "_access"] in ("def", "use") for side in edges.SIDES),
            "changes-wire: invalid bag attribute")
    return edges.canonical(attribute)


def validate_change(change, key_mode=None):
    require(shape(change, CHANGE_FIELDS) and number(change["id"], 1)
            and change["kind"] in edges.KINDS and isinstance(change["records"], list)
            and all(isinstance(row, dict) for row in change["records"])
            and (change["class"] is None or name(change["class"]))
            and (change["error"] is None or name(change["error"]))
            and all(isinstance(change[a + "_bag"], list) for a in ARMS)
            and number(change["count"], 1)
            and change["count"] == max(len(change[a + "_bag"]) for a in ARMS),
            "changes-wire: invalid change")
    kind = change["kind"]
    left, right = (change[a + "_bag"] for a in ARMS)
    require((kind == "removed" and left and not right)
            or (kind == "added" and right and not left)
            or (kind == "label_moved" and len(left) == len(right))
            or (kind == "ambiguous" and left and right),
            "changes-wire: bag incompatible with kind")
    key = change["key"]
    require(shape(key, edges.SIDES), "changes-wire: invalid key")
    for endpoint in key.values():
        require(shape(endpoint, edges.FIELDS) and number(endpoint["line"], 1)
                and edges.valid_bounds(endpoint["start_byte"], endpoint["end_byte"])
                and (key_mode != "line" or endpoint["start_byte"] is endpoint["end_byte"] is None),
                "changes-wire: invalid projected endpoint")
        coordinate(endpoint)
        edges.path_fields(endpoint)
    bags = {a: Counter(map(project_attribute, change[a + "_bag"])) for a in ARMS}
    return key, bags


def requests(args, cert, arm):
    require(isinstance(cert["by_class"], dict) and all(name(k) for k in cert["by_class"])
            and set(args.classes) <= cert["by_class"].keys(), "selector: unknown class")
    wanted = defaultdict(list)
    path = args.changes
    raw = regular(path)
    require(digest(raw) == cert["changes_sha256"], "certificate: changes digest differs")
    changes = (representable(json_object(line)) for line in raw.split(b"\n") if line.strip(b" \t\r"))
    for ident, change in enumerate(changes, 1):
        key, bags = validate_change(change, cert["key_mode"])
        require(change["id"] == ident and
                (change["class"] is None or change["class"] in cert["by_class"]),
                "changes-wire: invalid change id or unregistered class")
        kind = change["kind"]
        if ((not args.kind or kind in args.kind) and
                (not args.classes or change["class"] in args.classes)):
            for attribute, count in bags[arm].items():
                wanted[(edges.canonical(key), attribute)].append((change, count))
    return wanted


def edge_population(args, entries, gate):
    run = admit(args.run, entries)
    selection = dict(arm="census", kind=list(dict.fromkeys(args.kind)),
                     **{"class": list(dict.fromkeys(args.classes))})
    hashes = {}
    if args.all:
        require(not (args.side or args.kind or args.classes), "selector: side/kind/class require --changes")
        occurrences = [claim(edge(row), ordinal, "census") for ordinal, row in enumerate(read(run, "edges"))]
        return {"census": run}, selection, hashes, occurrences
    cert, pair, cert_sha = certificate_arms(args, entries, gate)
    matching = [arm for arm in ARMS if run.receipt_sha256 == pair[arm].receipt_sha256]
    arm = args.side or (matching[0] if len(matching) == 1 else None)
    require(arm in matching, "selector: run does not uniquely identify selected arm")
    pair[arm] = run
    selection["arm"] = arm
    hashes.update(changes_sha256=cert["changes_sha256"], certificate_sha256=cert_sha,
                  certificate_result=cert["result"])
    wanted = requests(args, cert, arm)
    candidates = defaultdict(list)
    for ordinal, row in enumerate(read(run, "edges")):
        try:
            key, attribute, _ = edges.decode_edge(edge(row), line_mode=cert["key_mode"] == "line")
        except (ValueError, KeyError, TypeError) as error:
            raise ValueError(f"edge-wire: {error}") from error
        if (key, attribute) in wanted:
            candidates[key, attribute].append((ordinal, row))
    occurrences = []
    for key, requests_for_key in wanted.items():
        group = candidates[key]
        count = sum(count for _, count in requests_for_key)
        require(count <= len(group), "selector: residual requests absent occurrences")
        if count < len(group) or len(requests_for_key) != 1:
            require(all(edges.canonical(row) == edges.canonical(group[0][1])
                        for _, row in group), "ambiguous-occurrence: unequal edge contexts")
        first = 0
        for change, copies in requests_for_key:
            occurrences.extend(claim(row, ordinal, arm, change=change)
                               for ordinal, row in group[first:first + copies])
            first += copies
    return pair, selection, hashes, occurrences


def site_key(row):
    span, caller = row["source_span"], row["caller"]
    return (span["file"], span.get("start_byte"), span.get("end_byte"), row["callee_text"],
            row["origin"], caller["file"], caller["name"], caller["start_line"])


def target_key(target):
    func = target["function_id"]
    return ((func["file"], func["name"], func["start_line"]), target["confidence"], target.get("kind"))


def ordered(items):
    return sorted(items, key=edges.canonical)


def target_bag(counter):
    return [dict(target=dict(file=key[0][0], name=key[0][1], start_line=key[0][2]),
                 confidence=key[1], kind=key[2], count=counter[key]) for key in ordered(counter)]


def project(records):
    contexts = defaultdict(list)
    drops = Counter()
    for ordinal, row in records:
        if row["drop"] is not None:
            drops[row["drop"]] += 1
        for index, target in enumerate(row["resolved_targets"]):
            contexts[target_key(target)].append((ordinal, row, index))
    return contexts, Counter({key: len(value) for key, value in contexts.items()}), drops


def delta_for(key, base, head, left, right, base_drops, head_drops):
    if not base:
        kind = "new"
    elif not head:
        kind = "lost"
    else:
        other_changes = (base_drops, len(base), sum(not row["resolved_targets"] for _, row in base)) != (
            head_drops, len(head), sum(not row["resolved_targets"] for _, row in head))
        if left == right and not other_changes:
            return None
        li, ri, lk, rk = Counter(), Counter(), Counter(), Counter()
        for bag, identities, kinds in ((left, li, lk), (right, ri, rk)):
            for (identity, _, resolution), count in bag.items():
                identities[identity] += count
                kinds[identity, resolution] += count
        if len(li) == len(ri) == 1 and li.keys() != ri.keys() and sum(li.values()) == sum(ri.values()) == 1:
            kind = "retargeted"
        elif left != right and lk == rk and not other_changes:
            kind = "confidence-only"
        else:
            kind = "multi-target"
    arms = [dict(site_records=len(records), dropped=sum(drops.values()), targets=target_bag(bag))
            for records, drops, bag in ((base, base_drops, left), (head, head_drops, right))]
    if base and head and (base_drops - head_drops or head_drops - base_drops) and (
            base_drops and head_drops or left == right
    ):
        arms[0]["drop_reasons"], arms[1]["drop_reasons"] = base_drops, head_drops
    return dict(site_key=key, kind=kind, base=arms[0], head=arms[1],
                lost=target_bag(left - right), gained=target_bag(right - left))


def head_surplus(left, right, count):
    def context(item):
        return edges.canonical([item[1], item[2]])
    common = Counter(map(context, left))
    remainder = []
    for item in right:
        key = context(item)
        if common[key]:
            common[key] -= 1
        else:
            remainder.append(item)
    require(len(remainder) == count or len({context(item) for item in remainder}) == 1,
            "ambiguous-occurrence: unequal call contexts")
    return remainder[:count]


def population(populations, confidence, kind):
    by_confidence = populations.setdefault(confidence, {"kinds": {}})
    counts = dict.fromkeys((*ARMS, "census", "lost", "gained", "eligible", "graded", *RESULTS), 0)
    if kind is None:
        return by_confidence.setdefault("missing_kind", counts)
    return by_confidence["kinds"].setdefault(kind, counts)


def call_population(args, entries):
    require(bool(args.census) != bool(args.base or args.head) and
            (bool(args.census) or bool(args.base and args.head)), "selector: use census or both arms")
    confidences = list(dict.fromkeys(args.target_confidence or ["exact"]))
    kinds = list(dict.fromkeys(args.target_kind))
    require(all(name(kind) for kind in kinds), "selector: invalid target kind")
    require("name_only" not in confidences or args.question and args.relation,
            "selector: name_only requires question and relation")
    runs = ({"census": admit(args.census, entries)} if args.census else
            dict(zip(ARMS, admit_pair(args.base, args.head, entries))))
    selection = dict(arm="census" if args.census else "head", target_confidence=confidences,
                     target_kind=kinds, aggregate_only=args.aggregate_only)
    populations, groups, places, stats = {}, {}, {}, dict.fromkeys((*ARMS, "census"))
    for confidence in confidences:
        for kind in kinds:
            population(populations, confidence, kind)
    for arm, run in runs.items():
        groups[arm] = group = defaultdict(list)
        places[arm] = []
        other = dropped = exact = nonexact = 0
        nonexact_sites = set()
        for ordinal, row in enumerate(read(run, "sites")):
            record_kind = row.get("record_kind")
            if record_kind == "go_level3_b1_candidate":
                other += 1
                continue
            require(record_kind == "call_site", "sites-wire: unknown record_kind")
            site(row)
            key = site_key(row)
            group[key].append((ordinal, row))
            places[arm].append(dict(ordinal=ordinal, source_span=row["source_span"], caller=row["caller"]))
            dropped += row["drop"] is not None
            for target in row["resolved_targets"]:
                _, confidence, kind = target_key(target)
                population(populations, confidence, kind)[arm] += 1
                if confidence == "exact":
                    exact += 1
                else:
                    nonexact += 1
                    nonexact_sites.add(key)
        stats[arm] = dict(sites=len(group), site_records=len(places[arm]), exact_claims=exact,
                          nonexact_claims=nonexact, nonexact_sites=len(nonexact_sites),
                          dropped=dropped, other_records=other)
    keys = set().union(*groups.values())
    occurrences = []
    deltas = None if args.census else []
    selected = 0

    def eligible(projected):
        return projected[1] in confidences and (not kinds or projected[2] in kinds)

    for key in ordered(keys):
        if args.census:
            contexts, bag, _ = project(groups["census"][key])
            selected += sum(bag.values())
            for projected, originals in contexts.items():
                if eligible(projected):
                    population(populations, *projected[1:])["eligible"] += len(originals)
                    occurrences.extend(claim(row, ordinal, "census", index=index)
                                       for ordinal, row, index in originals)
        else:
            base, head = (groups[arm].get(key, []) for arm in ARMS)
            left_context, left, base_drops = project(base)
            right_context, right, head_drops = project(head)
            delta = delta_for(key, base, head, left, right, base_drops, head_drops)
            if delta is not None:
                deltas.append(delta)
            for projected, count in (left - right).items():
                population(populations, *projected[1:])["lost"] += count
                selected += count
            for projected, count in (right - left).items():
                counts = population(populations, *projected[1:])
                counts["gained"] += count
                selected += count
                if eligible(projected):
                    counts["eligible"] += count
                    original = head_surplus(left_context.get(projected, ()), right_context[projected], count)
                    occurrences.extend(claim(row, ordinal, "head", index=index)
                                       for ordinal, row, index in original)
    return runs, selection, occurrences, places, deltas, dict(
        mode="call-census" if args.census else "call-diff", sites=len(keys), selected=selected,
        out_of_scope=0, populations=populations, **stats)
