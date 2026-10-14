"""The decision rule that ranks reduction candidates (imported by memreport.py)."""
from memreport_render import MIB

RULE_FRACTION = 0.15
RULE_ABSOLUTE = 200 * MIB
# candidate -> (estimate metric, base metric). M1 and M3 are judged against the steady state, M2 against the
# peak through the index load, because that is the footprint each one would lower.
CANDIDATES = (('M1', 'm1', 'steady'), ('M2', 'm2', 'peak'), ('M3', 'm3', 'steady'))


def rule_arm(estimate, base):
    """Which arm of the decision rule an estimate meets: absolute, fraction, both, none; None if unjudgeable."""
    if estimate is None or not base:
        return None
    absolute, fraction = estimate >= RULE_ABSOLUTE, estimate / base >= RULE_FRACTION
    if absolute and fraction:
        return 'both'
    return 'absolute' if absolute else 'fraction' if fraction else 'none'


def meets(estimate, base):
    return rule_arm(estimate, base) in ('absolute', 'fraction', 'both')


def decision_points(entry):
    """{metric: (value, n)} the decision rule reads from one corpus entry.

    Per-repetition metrics are medians with the number of runs behind them. m2 is not a series: it is the
    size of the CPG cache file the priming run wrote, one measurement (n is None).
    """
    points = {}
    for metric, stats in (entry.get('metrics') or {}).items():
        points[metric] = (stats['median'], stats['n']) if stats else (None, 0)
    points['m2'] = ((entry.get('cache_files') or {}).get('cpg_cache_bytes'), None)
    return points


def evaluate(entries):
    """entries: {corpus: corpus entry} -> {candidate: {'corpora': [...], 'advance': bool, 'rows': {...}}}"""
    verdict = {}
    for candidate, estimate_key, base_key in CANDIDATES:
        rows = {}
        for name, entry in entries.items():
            points = decision_points(entry)
            (estimate, estimate_n), (base, base_n) = (points.get(estimate_key, (None, 0)),
                                                      points.get(base_key, (None, 0)))
            rows[name] = {'estimate': estimate, 'base': base, 'estimate_n': estimate_n, 'base_n': base_n,
                          'percent': 100.0 * estimate / base if estimate is not None and base else None,
                          'arm': rule_arm(estimate, base), 'meets': meets(estimate, base)}
        hits = sorted(name for name, row in rows.items() if row['meets'])
        verdict[candidate] = {'corpora': hits, 'advance': len(hits) >= 2, 'rows': rows}
    return verdict
