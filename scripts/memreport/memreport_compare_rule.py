"""The pinned rules of the baseline-against-candidate comparison (imported by memreport_compare.py).

Experiments are rows of a table (`EXPERIMENTS`), so another is a new entry. Every judgement is on exact
arithmetic (`Fraction`), so a value exactly on a boundary is decided the same way on every machine.
"""
from fractions import Fraction

from memreport_render import MIB

GATE_MIN_RUNS = 3          # a gate needs at least this many runs on each side
GATE_CORPORA = 2           # and passes on at least this many corpora
GATE_ABSOLUTE = 100 * MIB  # effect: a reduction of at least this ...
GATE_FRACTION = Fraction(5, 100)    # ... or of at least this share of the baseline median
NOT_WORSE_RATIO = Fraction(105, 100)    # worse: candidate median above this times the baseline median ...
NOT_HIGHER_BYTES = MIB     # higher: candidate median more than this above the baseline median
GATE, NOT_WORSE, NOT_HIGHER = 'gate', 'not_worse', 'not_higher'
ROLE_NAMES = {GATE: 'gate', NOT_WORSE: 'not worse', NOT_HIGHER: 'not higher'}
# experiment -> metric rows (key in a corpus entry's `metrics`, label, unit, role)
EXPERIMENTS = {
    'm2': (
        ('peak', 'Load peak (cache-loaded, through the index load)', 'bytes', GATE),
        ('build_peak', 'First-build peak (writes the cache)', 'bytes', GATE),
        ('load_ms', 'Index load time', 'ms', NOT_WORSE),
        ('build_ms', 'Index build time', 'ms', NOT_WORSE),
        ('steady_heap', 'Steady live heap', 'bytes', NOT_HIGHER),
    ),
}


def check_experiments(experiments):
    """Refuse a table whose gate rows are not in bytes (the gate's thresholds are byte counts)."""
    for name, rows in experiments.items():
        for key, label, unit, role in rows:
            if role == GATE and unit != 'bytes':
                raise ValueError('experiment %s: gate metric %s is in %s, a gate needs bytes' % (name, key, unit))


check_experiments(EXPERIMENTS)


def share(part, whole):
    """Whether part >= GATE_FRACTION * whole, exactly (no float rounding at the boundary)."""
    return Fraction(part) >= GATE_FRACTION * Fraction(whole)


def judge_gate(base, cand):
    """One corpus against the gate: separated (candidate max < baseline min) and an effect (reduction of the
    medians of at least GATE_ABSOLUTE or GATE_FRACTION of the baseline median), on at least GATE_MIN_RUNS runs."""
    if base['n'] < GATE_MIN_RUNS or cand['n'] < GATE_MIN_RUNS:
        return {'outcome': 'insufficient runs', 'detail': 'n=%d baseline, n=%d candidate, at least %d needed' % (
            base['n'], cand['n'], GATE_MIN_RUNS)}
    separated = cand['max'] < base['min']
    reduction = base['median'] - cand['median']
    absolute, fraction = reduction >= GATE_ABSOLUTE, reduction > 0 and share(reduction, base['median'])
    effect = 'both' if absolute and fraction else 'absolute' if absolute else 'fraction' if fraction else 'none'
    passed = separated and effect != 'none'
    return {'outcome': 'pass' if passed else 'fail', 'separated': separated, 'reduction': reduction, 'effect': effect,
            'detail': '%s, reduction %.1f MiB, effect %s' % ('separated' if separated else 'not separated',
                                                             reduction / MIB, effect)}


def judge_not_worse(base, cand):
    """Worse only when the candidate median is above NOT_WORSE_RATIO times the baseline median and the two
    ranges do not overlap; a difference inside the overlap is not evidence either way. The ratio is printed."""
    above = Fraction(cand['median']) > NOT_WORSE_RATIO * Fraction(base['median'])
    apart = cand['min'] > base['max']
    ratio = '×n/a' if not base['median'] else '×%.3f' % (Fraction(cand['median']) / Fraction(base['median']))
    return {'outcome': 'worse' if above and apart else 'ok', 'detail': ratio}


def judge_not_higher(base, cand):
    """A candidate may lower this figure (tighter allocations do); it must not raise it."""
    ok = Fraction(cand['median']) - Fraction(base['median']) <= NOT_HIGHER_BYTES
    return {'outcome': 'ok' if ok else 'higher', 'detail': ''}


JUDGES = {GATE: judge_gate, NOT_WORSE: judge_not_worse, NOT_HIGHER: judge_not_higher}


def corpus_row(role, base, cand):
    """One corpus of one metric: both series, the difference of the medians and the outcome."""
    row = {'baseline': base, 'candidate': cand, 'difference': None, 'percent': None}
    if base is None and cand is None:
        return dict(row, outcome='not measured', detail='')
    if base is None or cand is None:
        return dict(row, outcome='one-sided', detail='series on the %s side only' % (
            'baseline' if cand is None else 'candidate'))
    row['difference'] = cand['median'] - base['median']
    row['percent'] = 100.0 * row['difference'] / base['median'] if base['median'] else None
    return dict(row, **JUDGES[role](base, cand))


def metric_verdict(role, rows):
    """The verdict of one metric from its per-corpus outcomes."""
    outcomes = [row['outcome'] for row in rows.values()]
    if all(outcome == 'not measured' for outcome in outcomes):
        return 'not measured'
    if role == GATE:
        passes = outcomes.count('pass')
        if passes >= GATE_CORPORA:
            return 'pass'
        # Corpora that pass, or could with more data (too few runs, a series on one side, none yet).
        open_ended = passes + sum(outcomes.count(word) for word in ('insufficient runs', 'one-sided', 'not measured'))
        return 'undecided' if len(outcomes) < GATE_CORPORA or open_ended >= GATE_CORPORA else 'fail'
    bad = 'worse' if role == NOT_WORSE else 'higher'
    if bad in outcomes:
        return bad
    return 'incomplete' if 'one-sided' in outcomes else 'ok'


def rule_data():
    """The rule as data (constants and the text of each line), carried in the comparison's JSON."""
    return {
        'constants': {'gate_min_runs': GATE_MIN_RUNS, 'gate_corpora': GATE_CORPORA,
                      'gate_absolute_bytes': GATE_ABSOLUTE, 'gate_fraction': float(GATE_FRACTION),
                      'not_worse_ratio': float(NOT_WORSE_RATIO), 'not_higher_bytes': NOT_HIGHER_BYTES},
        'lines': [
            '- **gate**, per corpus: *separated* = candidate max < baseline min; *reduction* = baseline median - '
            'candidate median; *effect* = reduction of at least %d MiB or at least %g %% of the baseline median. '
            'The corpus passes when both series have at least %d runs and it is separated and has an effect; with '
            'fewer runs it is "insufficient runs", neither a pass nor a fail. The metric passes when at least %d '
            'corpora pass. Otherwise it is "undecided" when fewer than %d corpora exist or when the corpora that '
            'pass or could still pass with more data (pass, insufficient runs, one-sided, not measured) number at '
            'least %d, and "fail" when not.' % (GATE_ABSOLUTE // MIB, 100 * GATE_FRACTION, GATE_MIN_RUNS,
                                                GATE_CORPORA, GATE_CORPORA, GATE_CORPORA),
            '- **not worse**, per corpus: "worse" only when the candidate median is above %g times the baseline '
            'median **and** the ranges do not overlap (candidate min > baseline max); otherwise ok. The ratio of '
            'the medians is always shown. A difference inside the overlap of the two ranges is not evidence either '
            'way.' % NOT_WORSE_RATIO,
            '- **not higher**, per corpus: ok when the candidate median is at most %d MiB above the baseline '
            'median (lower is ok).' % (NOT_HIGHER_BYTES // MIB),
            '- A series on exactly one side is "one-sided" for that corpus; a series on neither side is "not '
            'measured". For not worse and not higher the metric is "incomplete" when any corpus is one-sided and '
            'none is worse/higher ("worse"/"higher" wins); a metric with no series on either side in any corpus is '
            '"not measured" and decides nothing.',
        ],
    }
