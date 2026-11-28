"""Decode and compare complete finding bags within one review ordinal."""

from collections import Counter, defaultdict

from .edges import canonical, integer, residual_rows
from .inputs import NumberToken, json_object, wire_int

ANCHOR = ("file", "line", "algorithm", "category")
METRICS = ("presence_changes", "payload_changes", "ambiguous_bags")
SET_FIELDS = ("rules", "related_files", "related_lines")
TEXT_FIELDS = {"description", "message"}


def review_object(raw):
    """Parse finding payloads with every number kept as its spelling: binary floats
    can round two serialized payloads together and `-0` is an int equal to `0`."""
    return json_object(raw, numbers=NumberToken)


def structure_object(raw):
    """Parse receipt structure, whose integers (ordinals, counts, exits) are plain ints."""
    return json_object(raw, numbers=lambda lexeme: NumberToken(lexeme))


def review_json(value):
    if isinstance(value, NumberToken):
        return value.text.encode("ascii")
    if isinstance(value, dict):
        return b"{" + b",".join(canonical(k) + b":" + review_json(value[k]) for k in sorted(value)) + b"}"
    if isinstance(value, list):
        return b"[" + b",".join(map(review_json, value)) + b"]"
    return canonical(value)


def _rule_identity(rule):
    if isinstance(rule, str) and rule.strip():
        return rule, b"{}"
    if isinstance(rule, dict) and isinstance(rule.get("id"), str) and rule["id"].strip():
        return rule["id"], review_json({k: v for k, v in rule.items() if k != "id"})
    raise ValueError("invalid-review-rule")


def _canonical_finding(finding):
    if (not isinstance(finding, dict) or not isinstance(finding.get("file"), str)
            or not finding["file"].strip() or not integer(wire_int(finding.get("line")), 1)):
        raise ValueError("review-finding-location-missing")
    finding = finding.copy()
    for field in SET_FIELDS:
        values = finding.get(field)
        if values is None:
            continue
        if not isinstance(values, list):
            raise ValueError(f"invalid-review-{field}")
        if field == "rules":
            for rule in values:
                _rule_identity(rule)
        members = {review_json(value): value for value in values}
        finding[field] = [members[key] for key in sorted(members)]
    review_json(finding)
    return finding


def _text(value):
    return isinstance(value, str) and bool(value)


def _multi_marker(envelope):
    run, results = envelope.get("algorithms_run"), envelope.get("results")
    return (("algorithms_run" not in envelope
             or isinstance(run, list) and bool(run) and all(map(_text, run)))
            and ("results" not in envelope
                 or isinstance(results, list) and all(isinstance(r, dict) for r in results)))


def decode(raw, *, mode=None):
    """Read only the array selected by explicit or observed envelope mode."""
    envelope = review_object(raw)
    if mode is None:
        single = "algorithm" in envelope
        multi = "algorithms_run" in envelope or "results" in envelope
        if single == multi:
            raise ValueError("review-envelope-contradictory" if single else "review-envelope-unmarked")
        if single and not _text(envelope["algorithm"]):
            raise ValueError("review-envelope-malformed")
        if multi and not _multi_marker(envelope):
            raise ValueError("review-envelope-malformed")
        mode = "single" if single else "multi"
    field = {"single": "findings", "multi": "all_findings"}.get(mode)
    if field is None or not isinstance(envelope.get(field), list):
        raise ValueError("review-findings-unusable")
    return [_canonical_finding(finding) for finding in envelope[field]]


def _bucket(findings, payloads):
    buckets = defaultdict(Counter)
    for finding in findings:
        anchor = {key: finding[key] for key in ANCHOR if key in finding}
        key = review_json(finding)
        payloads[key] = finding
        buckets[review_json(anchor)][key] += 1
    return buckets


def _rule_ids(payload):
    if "rules" not in payload:
        return "missing", None
    if payload["rules"] is None:
        return "null", None
    identities = {_rule_identity(rule) for rule in payload["rules"]}
    return "list", sorted(ident for ident, _ in identities)


def _record_pair(left, right, count, transitions):
    fields = sorted(k for k in left.keys() | right.keys()
                    if k not in left or k not in right or review_json(left[k]) != review_json(right[k]))
    from_state, from_ids = _rule_ids(left)
    to_state, to_ids = _rule_ids(right)
    if (from_state, from_ids) != (to_state, to_ids):
        transitions.append({"from": from_ids, "to": to_ids, "count": count,
                            "from_state": from_state, "to_state": to_state})
    return {"from": left, "to": right, "count": count,
            "fields": fields, "text_only": set(fields) <= TEXT_FIELDS}


def compare(base, head):
    payloads = {}
    base, head = _bucket(base, payloads), _bucket(head, payloads)
    anchors, transitions = [], []
    totals = dict.fromkeys(METRICS, 0)
    for key in sorted(base.keys() | head.keys()):
        left, right = base[key], head[key]
        cancelled, differences = residual_rows(left, right)
        if not differences:
            continue
        left, right = left - cancelled, right - cancelled
        b, h = sum(left.values()), sum(right.values())
        pairs = []
        for kind, lost, gained in differences:
            if kind == "label_moved":
                before, count = next(iter(lost.items()))
                after = next(iter(gained))
                pairs.append(_record_pair(payloads[before], payloads[after], count, transitions))
        ambiguous = differences[0][0] == "ambiguous"
        decoded = payloads[next(iter(left or right))]
        anchors.append({
            "anchor": {field: decoded[field] for field in ANCHOR if field in decoded},
            "base_bag": [payloads[p] for p in sorted(left) for _ in range(left[p])],
            "head_bag": [payloads[p] for p in sorted(right) for _ in range(right[p])],
            "added": max(h - b, 0), "removed": max(b - h, 0),
            "payload_changes": min(b, h), "ambiguous": ambiguous, "pairs": pairs,
        })
        totals["presence_changes"] += abs(b - h)
        totals["payload_changes"] += min(b, h)
        totals["ambiguous_bags"] += ambiguous
    return {**totals, "rule_transitions": transitions, "anchors": anchors}
