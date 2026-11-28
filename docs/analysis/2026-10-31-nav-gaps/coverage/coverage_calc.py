#!/usr/bin/env python3
"""Weighted coverage roll-up for COVERAGE.md (2026-10-31).

Method (stated in COVERAGE.md section 3):
- Each survey class is counted once, with its prevalence weight from the survey's own source.
- Area grades are the numeric form of the per-area verdicts in COVERAGE.md section 2.
- Two class scores: PRIMARY = grade of the class's decisive area (the surveys' exclusive
  attribution); CHAIN = min grade over the first three listed needed areas, J excluded
  (a trace is only as good as its weakest required hop). Both are reported; neither double counts.
- Buckets: covered >= 0.65, partial 0.30..0.65, uncovered < 0.30.
- J-primary classes are reported separately and never as a gap.
"""

GRADE = {  # numeric form of COVERAGE.md section 2 verdicts
    "A": 0.75,  # partial (high)
    "B": 0.50,  # partial (med)
    "C": 0.45,  # partial (med)
    "D": 0.60,  # partial (high) for ESM/TS, low for CJS; blended
    "E": 0.20,  # partial (low)
    "F": 0.25,  # partial (low)
    "G": 0.50,  # partial (med)
    "H": 0.30,  # partial (low)
    "I": 0.05,  # none by design (syntactic annotations only)
}
WS_STATUS = {1: "done", 2: "partial", 3: "partial", 4: "not started", 5: "partial (low)",
             6: "partial", 7: "partial"}

# (class, prevalence weight, needed areas primary first, primary workstream, note)
# CVE space: Kluban 2022 895-set shares (survey-cve-luna.md). The ~22% not separately
# classified and the malicious-package class are excluded from the denominator.
CVE = [
    ("XSS (server/template)",  25.5 * 0.7, "AGBDH", 6, "70/30 server vs React split is a judgment"),
    ("XSS (React-specific)",   25.5 * 0.3, "FABDH", 5, "survey: only a subset of XSS is React"),
    ("Code/command injection", 18.7,       "GABDH", 6, "895-set folds command into code injection"),
    ("ReDoS",                  13.5,       "JBDGH", 0, "J-primary: pattern analysis plus reachability"),
    ("Prototype pollution",    11.3,       "CBDAG", 3, ""),
    ("Path traversal",          4.7,       "GBADH", 6, ""),
    ("Auth / logic",            2.2,       "ABDHG", 2, "I listed last by the survey; excluded from chain"),
    ("SSRF",                    1.9,       "GHABD", 6, ""),
]
# Defect space: arXiv:2601.21186 (633 TS-ecosystem bugs), survey-defects-sonnet.md.
# Splits inside a bucket are judgments and are flagged in COVERAGE.md.
DEF = [
    ("Tooling/Config (J part)",    27.8 * 0.72, "J",    0, "build scripts, versions, config"),
    ("Tooling/Config (E part)",    27.8 * 0.28, "ED",   4, "tsconfig/project ownership share is a judgment"),
    ("API misuse (B/C part)",      14.5 * 0.7,  "BCD",  3, "symptom bucket; 70% param/member, 30% async (judgment)"),
    ("API misuse (H part)",        14.5 * 0.3,  "HBD",  6, ""),
    ("UI bug (React)",             12.5,        "FCH",  5, "Ocariza: DOM/member faults dominate client JS"),
    ("Type error",                 12.4,        "IBC",  0, "I-primary; prism has no types by design"),
    ("Logic error",                 9.7,        "J",    0, "single-expression logic"),
    ("Test fault",                  9.0,        "J",    0, ""),
    ("Missing case (A part)",       8.2 * 0.5,  "ABC",  3, "guards/branches reachable by slicing (judgment split)"),
    ("Missing case (J part)",       8.2 * 0.5,  "J",    0, ""),
    ("Missing feature",             3.8,        "J",    0, ""),
    ("Async/Event",                 3.6,        "HGA",  6, "survey: a floor; causal async bugs hide elsewhere"),
    ("Runtime exception",           1.2,        "CBD",  3, "cannot read property of undefined"),
    ("Error handling",              0.8,        "HAG",  6, ""),
]


def score(areas):
    areas = [a for a in areas[:3] if a != "J"]
    if not areas:
        return None, None
    primary = GRADE[areas[0]]
    chain = min(GRADE[a] for a in areas)
    return primary, chain


def bucket(x):
    return "covered" if x >= 0.65 else ("partial" if x >= 0.30 else "uncovered")


def rollup(name, rows):
    print(f"\n### {name}\n")
    print("| Class | Weight | Areas (primary first) | WS | Primary score | Chain score | Bucket (chain) |")
    print("|---|---:|---|---|---:|---:|---|")
    tot = 0.0
    jw = 0.0
    prim_sum = chain_sum = 0.0
    buckets = {"covered": 0.0, "partial": 0.0, "uncovered": 0.0}
    ws = {}
    for cls, w, areas, wsn, note in rows:
        if areas[0] == "J":
            jw += w
            print(f"| {cls} | {w:.1f} | J | — | n/a | n/a | J (not a tracing problem) |")
            continue
        p, c = score(areas)
        tot += w
        prim_sum += w * p
        chain_sum += w * c
        buckets[bucket(c)] += w
        ws.setdefault(wsn, [0.0, 0.0])
        ws[wsn][0] += w
        ws[wsn][1] += w * c
        print(f"| {cls} | {w:.1f} | {'/'.join(areas)} | {wsn or '—'} | {p:.2f} | {c:.2f} | {bucket(c)} |")
    print()
    print(f"Tracing-relevant weight {tot:.1f}; J weight {jw:.1f} (reported separately).")
    print(f"Weighted PRIMARY coverage {100*prim_sum/tot:.0f}%; weighted CHAIN coverage {100*chain_sum/tot:.0f}%.")
    print("Chain buckets by weight: " + ", ".join(f"{k} {100*v/tot:.0f}%" for k, v in buckets.items()) + ".")
    print("\n| Workstream | Status | Share of tracing-relevant weight | Chain coverage inside that share |")
    print("|---|---|---:|---:|")
    for wsn in sorted(ws):
        w, s = ws[wsn]
        print(f"| {wsn} | {WS_STATUS.get(wsn,'—')} | {100*w/tot:.0f}% | {100*s/w:.0f}% |")
    return tot, prim_sum, chain_sum, buckets


if __name__ == "__main__":
    print("Area grades:", GRADE)
    rollup("CVE space (895-set weights)", CVE)
    rollup("Defect space (TS-ecosystem 633-bug weights)", DEF)
