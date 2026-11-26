"""The DESIGN section 7 pair draw and the precision receipt of the I2 admission check (lane 10, Python).

`draw` derives the pair draw from `candidates.jsonl` alone; `evaluate` scores a judgments file against that
draw; `verify` recomputes both from the files a receipt names, so a receipt is never trusted for its own
numbers. Nothing here is hand-written: the draw is a pure function of the candidates and the prepared digest.

Draw: the distinct pair keys (`pair_key`, role last) ordered by sha256 of the key (then the key). The pooled
core is the first `TARGET` keys of that order (all of them when the population is smaller: exhaustive); then
each populated role is topped up, in lexical role order and the same key order, to min(`ROLE_FLOOR`, its
population) members. The draw is core plus top-ups and may exceed `TARGET`. CHOICE: one family (Python), so
the strata are the roles, pooled over corpora; the draw replaces the design's CPython `random.Random(10)`
sampling with a hash order that any reader reproduces from the keys.

Judgments: one JSON row per drawn key, `{key, verdict, witnessed, prepared_sha256}`, `verdict` one of
`valid`, `invalid`, `stale`, `unknown`; a row whose `prepared_sha256` is not the draw's is stale. The key set
must be exactly the draw's. `witnessed` must be a strict boolean (any other value refuses the file) and may be
true only on a `valid` verdict (a witness for a pair judged otherwise contradicts it: refused). A pair is
witnessed when `witnessed` is true and its verdict is `valid`. The core, the exhaustive family and the roles
are all scored on the WITNESSED fraction, never on `valid` verdicts: a judgment of `valid` that no witness
backs counts against precision.

Populations (schema `admission_precision/4`): `a-credit` is `candidates.jsonl`, the pairs the head credits to
A (agreed by both sides, no parent route, unoccupied, in a reconciled owner); `all-survivors` is
`survivors.jsonl`, every pair both sides agree on, the method-call predictions the head excludes included.
Which one section 7 scores is an OWNER decision that is still open, so the receipt names its population
(`population_name`) and `verify` recomputes it from the file that population names; nothing here chooses.
"""

import json
import math
from collections import Counter
from hashlib import sha256
from pathlib import Path

DRAW_SCHEMA = "admission_draw/1"
PRECISION_SCHEMA = "admission_precision/4"
POPULATIONS = {"a-credit": "candidates", "all-survivors": "survivors"}
TARGET, ROLE_FLOOR, FLOOR = 100, 20, 0.85
VERDICTS = ("valid", "invalid", "stale", "unknown")


def digest(data):
    return sha256(data).hexdigest()


def dump(row):
    return json.dumps(row, sort_keys=True, separators=(",", ":"))


def wilson_lower(k, n, z=1.96):
    if not n:
        return 0.0
    p = k / n
    return (p + z * z / (2 * n) - z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n))) / (1 + z * z / n)


def wilson_interval(k, n, z=1.96):
    """`(lower, upper)` Wilson score interval of k successes in n trials; `(0.0, 0.0)` for n == 0."""
    if not n:
        return 0.0, 0.0
    p = k / n
    half = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n))
    mid = p + z * z / (2 * n)
    return (mid - half) / (1 + z * z / n), min(1.0, (mid + half) / (1 + z * z / n))


def derive_draw(candidates, meta):
    """The section 7 draw of this prepared receipt's candidates (a pure function of both)."""
    role_of = {}
    for c in candidates:
        role_of.setdefault(c["key"], c["role"])
    ranked = sorted(role_of, key=lambda k: (digest(k.encode()), k))
    roles = Counter(role_of.values())
    core = ranked[:TARGET]
    chosen, topup = set(core), []
    for role in sorted(roles):
        have = sum(role_of[k] == role for k in chosen)
        for key in ranked:
            if have >= min(ROLE_FLOOR, roles[role]):
                break
            if key not in chosen and role_of[key] == role:
                chosen.add(key)
                topup.append(key)
                have += 1
    return dict(schema=DRAW_SCHEMA, prepared_sha256=meta["prepared_sha256"],
                candidates_sha256=meta["candidates_sha256"], target=TARGET, role_floor=ROLE_FLOOR,
                ordering="sha256(key), then key", population=len(role_of),
                role_population=dict(sorted(roles.items())), exhaustive=len(chosen) == len(role_of),
                core=core, topup=topup, keys={k: role_of[k] for k in sorted(chosen)})


def evaluate(draw, rows, population_name="a-credit"):
    """`(scored body, problems)` of the judgments against the draw; a key set other than the draw's raises."""
    keys = draw["keys"]
    seen = Counter(str(r.get("key")) for r in rows)
    extra = sorted(k for k in seen if k not in keys)
    missing = sorted(k for k in keys if k not in seen)
    duplicated = sorted(k for k, n in seen.items() if n > 1)
    if extra or missing or duplicated:
        raise ValueError(f"judgment keys are not the draw's keys: {len(extra)} extra, {len(missing)} missing, "
                         f"{len(duplicated)} duplicated of {len(keys)} drawn" + "".join(
                             f"; {name} e.g. {group[0]}" for name, group in
                             (("extra", extra), ("missing", missing), ("duplicated", duplicated)) if group))
    verdict = {}
    for row in rows:
        if not isinstance(row.get("witnessed"), bool):
            raise ValueError(f"judgment {row['key']}: witnessed {row.get('witnessed')!r} is not a boolean")
        v = "stale" if row.get("prepared_sha256") != draw["prepared_sha256"] else row.get("verdict")
        if v not in VERDICTS:
            raise ValueError(f"judgment {row['key']}: verdict {v!r} is not one of {VERDICTS}")
        if row["witnessed"] and row.get("verdict") != "valid":
            raise ValueError(f"judgment {row['key']}: witnessed is true but the verdict is {row.get('verdict')!r}")
        verdict[row["key"]] = (v, row["witnessed"] and v == "valid")
    core = set(draw["core"])
    roles = {r: dict(n=0, witnessed=0) for r in sorted(set(keys.values()))}
    for key, role in keys.items():
        roles[role]["n"] += 1
        roles[role]["witnessed"] += verdict[key][1]
    count = Counter(v for v, _ in verdict.values())
    body = dict(schema=PRECISION_SCHEMA, population_name=population_name, prepared_sha256=draw["prepared_sha256"],
                population=draw["population"],
                n=len(keys), valid=count["valid"], invalid=count["invalid"], stale=count["stale"],
                unknown=count["unknown"], exhaustive=draw["exhaustive"],
                core=dict(n=len(core), valid=sum(verdict[k][0] == "valid" for k in core),
                          witnessed=sum(verdict[k][1] for k in core)), roles=roles,
                witnessed=sum(w for _, w in verdict.values()))
    body["wilson_core"] = round(wilson_lower(body["core"]["witnessed"], body["core"]["n"]), 6)
    return body, thresholds(body)


def thresholds(body):
    """Why a scored draw does not clear section 7 (empty: it does)."""
    n, out = body["n"], []
    if not n:
        return ["precision draw is empty (no candidate pairs to judge)"]
    for what in ("stale", "unknown"):
        if body[what]:
            out.append(f"precision draw has {body[what]} {what} judgments (any {what} fails)")
    if body["exhaustive"]:
        if body["witnessed"] != n:
            out.append(f"exhaustive family: {body['witnessed']}/{n} judgments witnessed, every one must be")
    elif wilson_lower(body["core"]["witnessed"], body["core"]["n"]) < FLOOR:
        out.append(f"Wilson lower bound {body['wilson_core']:.4f} of the core's {body['core']['witnessed']}/"
                   f"{body['core']['n']} is below {FLOOR}")
    for role, row in sorted(body["roles"].items()):
        need = max(1, row["n"] - max(1, row["n"] // 10))
        if row["witnessed"] < need:
            out.append(f"role {role}: {row['witnessed']} of {row['n']} witnessed, needs {need}")
    return out


def population_meta(meta, name):
    """The prepared receipt as the draw reads it for population `name`: `candidates_sha256` is the sha of that
    population's file. Raises ValueError for an unknown population or a prepared receipt without its file."""
    if name not in POPULATIONS:
        raise ValueError(f"unknown precision population {name!r} (one of {', '.join(POPULATIONS)})")
    stem = POPULATIONS[name]
    if f"{stem}_sha256" not in meta:
        raise ValueError(f"the prepared receipt has no {stem}.jsonl (re-run `prepare`) for population {name}")
    return dict(meta, candidates_sha256=meta[f"{stem}_sha256"])


def load_prepared(prepared, population="a-credit"):
    """`(meta, rows)` of one population of a prepared directory (the file must match its recorded sha256)."""
    prep = Path(prepared)
    meta = population_meta(json.loads((prep / "prepared.json").read_text()), population)
    path = prep / f"{POPULATIONS[population]}.jsonl"
    data = path.read_bytes()
    if digest(data) != meta["candidates_sha256"]:
        raise ValueError(f"{path.name} does not match prepared.json")
    return meta, [json.loads(x) for x in data.decode().splitlines() if x.strip()]


def rows_of(data):
    return [json.loads(x) for x in data.decode("utf-8").splitlines() if x.strip()]


def score(meta, candidates, draw_path, judgments_path, population="a-credit"):
    """The receipt of two files, after the draw is checked against the derived one (raises ValueError).
    `meta` is the population's own (`population_meta`)."""
    draw_bytes, judgment_bytes = Path(draw_path).read_bytes(), Path(judgments_path).read_bytes()
    draw = json.loads(draw_bytes)
    if draw != derive_draw(candidates, meta):
        raise ValueError("the draw is not the one derived from this prepared receipt's candidates")
    body, problems = evaluate(draw, rows_of(judgment_bytes), population)
    receipt = dict(body, frame_sha256=meta["frame_sha256"], draw_path=str(Path(draw_path).resolve()),
                   draw_sha256=digest(draw_bytes), judgments_path=str(Path(judgments_path).resolve()),
                   judgments_sha256=digest(judgment_bytes))
    return receipt, problems


def find(path, base):
    """A recorded path, else the file of that name beside the receipt (the evidence directory moved)."""
    p = Path(path)
    return p if p.is_file() else Path(base) / p.name


def verify_scored(receipt, meta, candidates, base, survivors=None):
    """`(recomputed body or None, problems)`: why a receipt does not clear I2, every binding recomputed from the
    files it names. `survivors` are the all-survivors rows (None: the prepared directory has none)."""
    got = receipt.get("schema") if isinstance(receipt, dict) else None
    if got != PRECISION_SCHEMA:
        if isinstance(got, str) and got.startswith("admission_precision/"):
            return None, [f"precision receipt schema mismatch: the receipt is {got}, this tool verifies "
                          f"{PRECISION_SCHEMA} (regenerate it with `precision`)"]
        return None, [f"precision receipt is not an {PRECISION_SCHEMA} document"]
    name = receipt.get("population_name")
    if name not in POPULATIONS:
        return None, [f"precision receipt population_name {name!r} is not one of {', '.join(POPULATIONS)}"]
    rows = survivors if name == "all-survivors" else candidates
    if rows is None:
        return None, [f"precision receipt names population {name} but the prepared directory has no survivors.jsonl"]
    try:
        meta = population_meta(meta, name)
    except ValueError as e:
        return None, [f"precision receipt cannot be recomputed: {e}"]
    out = []
    if receipt.get("prepared_sha256") != meta["prepared_sha256"]:
        out.append("precision receipt is not bound to this prepared digest")
    if receipt.get("frame_sha256") != meta["frame_sha256"]:
        out.append("precision receipt is not bound to this frame (frame_sha256)")
    try:
        draw_file = find(receipt.get("draw_path", ""), base)
        judgment_file = find(receipt.get("judgments_path", ""), base)
        if digest(draw_file.read_bytes()) != receipt.get("draw_sha256"):
            out.append("precision receipt draw_sha256 is not the hash of its draw file")
        if digest(judgment_file.read_bytes()) != receipt.get("judgments_sha256"):
            out.append("precision receipt judgments_sha256 is not the hash of its judgments file")
        if out:
            return None, out
        scored, problems = score(meta, rows, draw_file, judgment_file, name)
    except (OSError, ValueError, KeyError, TypeError) as e:
        return None, out + [f"precision receipt cannot be recomputed: {e}"]
    differs = sorted(k for k in scored if k not in ("draw_path", "judgments_path") and scored[k] != receipt.get(k))
    if differs:
        out.append(f"precision receipt disagrees with its recomputation in {', '.join(differs)}")
    return scored, out + problems


def verify(receipt, meta, candidates, base, survivors=None):
    """Why a receipt does not clear I2 (empty: clear); see `verify_scored`."""
    return verify_scored(receipt, meta, candidates, base, survivors)[1]
