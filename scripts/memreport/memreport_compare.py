"""Baseline-against-candidate comparison of two run directories under a pinned rule (imported by memreport.py).

`compare` is pure: two summary.json contents and an experiment name in, a result dict out; it raises ValueError
naming the difference when the two runs are not comparable. `to_markdown` renders the result and `cmd_compare`
does the file and argument handling. Experiments are rows of a table (`EXPERIMENTS`), so another is a new entry.
"""
import json
from fractions import Fraction
from pathlib import Path

from memreport_render import MIB

GATE_MIN_RUNS = 3          # a gate needs at least this many runs on each side
GATE_CORPORA = 2           # and passes on at least this many corpora
GATE_ABSOLUTE = 100 * MIB  # effect: a reduction of at least this ...
GATE_FRACTION = Fraction(5, 100)    # ... or of at least this share of the baseline median
NOT_WORSE_RATIO = Fraction(105, 100)    # candidate median at most this times the baseline median
UNCHANGED_BYTES = MIB      # medians differ by at most this
GATE, NOT_WORSE, UNCHANGED = 'gate', 'not_worse', 'unchanged'
ROLE_NAMES = {GATE: 'gate', NOT_WORSE: 'not worse', UNCHANGED: 'unchanged'}
# experiment -> metric rows (key in a corpus entry's `metrics`, label, unit, role)
EXPERIMENTS = {
    'm2': (
        ('peak', 'Load peak (cache-loaded, through the index load)', 'bytes', GATE),
        ('build_peak', 'First-build peak (writes the cache)', 'bytes', GATE),
        ('load_ms', 'Index load time', 'ms', NOT_WORSE),
        ('build_ms', 'Index build time', 'ms', NOT_WORSE),
        ('steady_heap', 'Steady live heap', 'bytes', UNCHANGED),
    ),
}
HEADLINE = ('build_identity', 'git_sha', 'cpg_cache_version', 'allocator')


def provenance_of(summary):
    """The settings and build header of one run that a comparison reports and checks."""
    provenance = summary.get('provenance') or {}
    build = provenance.get('build') or {}
    header = {key: build.get(key) for key in HEADLINE}
    return dict(header, threads=provenance.get('threads'), env=provenance.get('env'),
                reps=provenance.get('reps'), settle=provenance.get('settle'))


def refusals(baseline, candidate, allow_same_build=False):
    """Every reason the two summaries cannot be compared, as a list of messages (empty when they can)."""
    problems = []
    base_corpora, cand_corpora = baseline.get('corpora') or {}, candidate.get('corpora') or {}
    if set(base_corpora) != set(cand_corpora):
        problems.append('corpora differ: baseline has %s, candidate has %s' % (sorted(base_corpora),
                                                                              sorted(cand_corpora)))
    for name in sorted(set(base_corpora) & set(cand_corpora)):
        base, cand = base_corpora[name], cand_corpora[name]
        for label, key in (('commit', lambda entry: entry.get('commit')),
                           ('dirty file count', lambda entry: entry.get('dirty')),
                           ('workload sha256', lambda entry: (entry.get('workload') or {}).get('sha256'))):
            if key(base) != key(cand):
                problems.append('corpus %s: %s differs (baseline %s, candidate %s)' % (
                    name, label, key(base), key(cand)))
    base_header, cand_header = provenance_of(baseline), provenance_of(candidate)
    for label, key in (('reps', 'reps'), ('settle', 'settle'), ('threads', 'threads'),
                       ('measured-process environment', 'env'), ('allocator', 'allocator')):
        if base_header[key] != cand_header[key]:
            problems.append('%s differs (baseline %s, candidate %s)' % (label, base_header[key], cand_header[key]))
    identities = (base_header['build_identity'], cand_header['build_identity'])
    if None in identities:
        problems.append('a run has no build identity (no admitted run): baseline %s, candidate %s' % identities)
    elif identities[0] == identities[1] and not allow_same_build:
        problems.append('baseline and candidate have the same build identity %s (--allow-same-build compares them '
                        'anyway, for a noise check)' % identities[0])
    return problems


def stats_of(summary, corpus, key):
    """The {'min', 'median', 'max', 'n'} of a metric in one corpus, or None when it is absent or unavailable."""
    stats = ((summary.get('corpora') or {}).get(corpus, {}).get('metrics') or {}).get(key)
    return stats if isinstance(stats, dict) and stats.get('n') else None


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
    absolute, fraction = reduction >= GATE_ABSOLUTE, share(reduction, base['median'])
    effect = 'both' if absolute and fraction else 'absolute' if absolute else 'fraction' if fraction else 'none'
    passed = separated and effect != 'none'
    return {'outcome': 'pass' if passed else 'fail', 'separated': separated, 'reduction': reduction, 'effect': effect,
            'detail': '%s, reduction %.1f MiB, effect %s' % ('separated' if separated else 'not separated',
                                                             reduction / MIB, effect)}


def judge_not_worse(base, cand):
    ok = Fraction(cand['median']) <= NOT_WORSE_RATIO * Fraction(base['median'])
    return {'outcome': 'ok' if ok else 'worse', 'detail': ''}


def judge_unchanged(base, cand):
    ok = abs(Fraction(cand['median']) - Fraction(base['median'])) <= UNCHANGED_BYTES
    return {'outcome': 'ok' if ok else 'changed', 'detail': ''}


JUDGES = {GATE: judge_gate, NOT_WORSE: judge_not_worse, UNCHANGED: judge_unchanged}


def corpus_row(role, base, cand):
    """One corpus of one metric: both series, the difference of the medians and the outcome."""
    row = {'baseline': base, 'candidate': cand, 'difference': None, 'percent': None}
    if base is None or cand is None:
        return dict(row, outcome='not measured', detail='')
    row['difference'] = cand['median'] - base['median']
    row['percent'] = 100.0 * row['difference'] / base['median'] if base['median'] else None
    return dict(row, **JUDGES[role](base, cand))


def metric_verdict(role, rows):
    """The verdict of one metric from its per-corpus outcomes."""
    outcomes = [row['outcome'] for row in rows.values()]
    if all(outcome == 'not measured' for outcome in outcomes):
        return 'not measured'
    if role == GATE:
        passes, short = outcomes.count('pass'), outcomes.count('insufficient runs')
        if passes >= GATE_CORPORA:
            return 'pass'
        return 'insufficient runs' if passes + short >= GATE_CORPORA else 'fail'
    bad = 'worse' if role == NOT_WORSE else 'changed'
    return bad if bad in outcomes else 'ok'


def compare(baseline, candidate, experiment, allow_same_build=False):
    """The comparison of two summary.json contents under the pinned rule of `experiment`.

    Raises ValueError, naming every difference, when the runs are not comparable.
    """
    if experiment not in EXPERIMENTS:
        raise ValueError('unknown experiment %r (known: %s)' % (experiment, ', '.join(sorted(EXPERIMENTS))))
    problems = refusals(baseline, candidate, allow_same_build)
    if problems:
        raise ValueError('; '.join(problems))
    names = sorted(baseline['corpora'])
    metrics = []
    for key, label, unit, role in EXPERIMENTS[experiment]:
        rows = {name: corpus_row(role, stats_of(baseline, name, key), stats_of(candidate, name, key))
                for name in names}
        metrics.append({'key': key, 'label': label, 'unit': unit, 'role': role, 'corpora': rows,
                        'verdict': metric_verdict(role, rows)})
    base_header, cand_header = provenance_of(baseline), provenance_of(candidate)
    return {'experiment': experiment, 'same_build': base_header['build_identity'] == cand_header['build_identity'],
            'baseline': base_header, 'candidate': cand_header,
            'corpora': {name: {'commit': baseline['corpora'][name].get('commit'),
                               'dirty': baseline['corpora'][name].get('dirty'),
                               'workload_sha256': (baseline['corpora'][name].get('workload') or {}).get('sha256')}
                        for name in names},
            'metrics': metrics}


def rule_lines():
    return [
        '- **gate**, per corpus: *separated* = candidate max < baseline min; *reduction* = baseline median - '
        'candidate median; *effect* = reduction of at least %d MiB or at least %g %% of the baseline median. The '
        'corpus passes when both series have at least %d runs and it is separated and has an effect; with fewer '
        'runs it is "insufficient runs", neither a pass nor a fail. The metric passes when at least %d corpora '
        'pass.' % (GATE_ABSOLUTE // MIB, 100 * GATE_FRACTION, GATE_MIN_RUNS, GATE_CORPORA),
        '- **not worse**, per corpus: ok when the candidate median is at most %g times the baseline median. The '
        'metric is ok when it is ok on every corpus that has both series.' % NOT_WORSE_RATIO,
        '- **unchanged**, per corpus: ok when the medians differ by at most %d MiB.' % (UNCHANGED_BYTES // MIB),
        '- A series that is missing on either side is "not measured" for that corpus; a metric absent from both '
        'runs is "not measured" and decides nothing.',
    ]


def number(value, unit, signed=False):
    """A byte count in MiB with one decimal, a duration in whole ms; n/a when it is unavailable."""
    if value is None:
        return 'n/a'
    if unit == 'bytes':
        return ('%+.1f' if signed else '%.1f') % (value / MIB)
    return ('%+d' if signed else '%d') % round(value)


def series_text(stats, unit):
    if stats is None:
        return 'n/a'
    return '%s (%s–%s, n=%d)' % (number(stats['median'], unit), number(stats['min'], unit),
                                  number(stats['max'], unit), stats['n'])


def difference_text(row, unit):
    if row['difference'] is None:
        return 'n/a'
    percent = 'n/a' if row['percent'] is None else '%+.1f %%' % row['percent']
    return '%s (%s)' % (number(row['difference'], unit, signed=True), percent)


def outcome_text(row):
    return row['outcome'] + (' (%s)' % row['detail'] if row['detail'] else '')


def provenance_lines(label, header, source):
    env = ', '.join('`%s=%s`' % (name, header['env'][name]) for name in sorted(header['env'] or {})) or 'none'
    return ['- %s%s: build identity `%s`, git `%s`, CPG cache version %s, allocator %s, threads %s, environment %s, '
            '%s repetitions, %s s settle.' % (
                label, ' (%s)' % source if source else '', header['build_identity'], header['git_sha'],
                header['cpg_cache_version'], header['allocator'], header['threads'], env, header['reps'],
                header['settle'])]


def to_markdown(result):
    """The Markdown report of a `compare` result (a `sources` entry, if present, names the run directories)."""
    sources = result.get('sources') or {}
    lines = ['# prism-mcp memory comparison: experiment %s' % result['experiment'], '',
             'Baseline against candidate under a pinned rule. Bytes are MiB (one decimal), durations whole ms; '
             'a value that is unavailable is n/a, never 0.', '', '## Provenance', '']
    lines += provenance_lines('Baseline', result['baseline'], sources.get('baseline'))
    lines += provenance_lines('Candidate', result['candidate'], sources.get('candidate'))
    if result['same_build']:
        lines.append('- **Baseline and candidate are the same build** (`--allow-same-build`): this is a noise check.')
    lines += ['', '| Corpus | Commit | Dirty files | Workload sha256 |', '|---|---|---|---|']
    for name, corpus in result['corpora'].items():
        commit, digest = corpus['commit'], corpus['workload_sha256']
        lines.append('| %s | %s | %s | %s |' % (name, '`%s`' % commit[:12] if commit else 'n/a',
                                                'n/a' if corpus['dirty'] is None else corpus['dirty'],
                                                '`%s`' % digest[:12] if digest else 'n/a'))
    lines += ['', '## Rule', ''] + rule_lines()
    for metric in result['metrics']:
        lines += ['', '## %s (%s, %s)' % (metric['label'], 'MiB' if metric['unit'] == 'bytes' else metric['unit'],
                                         ROLE_NAMES[metric['role']]), '',
                  '| Corpus | Baseline median (min–max, n) | Candidate median (min–max, n) | '
                  'Difference (candidate - baseline) | Outcome |', '|---|---|---|---|---|']
        for name, row in metric['corpora'].items():
            lines.append('| %s | %s | %s | %s | %s |' % (
                name, series_text(row['baseline'], metric['unit']), series_text(row['candidate'], metric['unit']),
                difference_text(row, metric['unit']), outcome_text(row)))
    lines += ['', '## Verdict', '', '| Metric | Role | Verdict | Corpora |', '|---|---|---|---|']
    for metric in result['metrics']:
        grouped = {}
        for name, row in metric['corpora'].items():
            grouped.setdefault(row['outcome'], []).append(name)
        lines.append('| %s | %s | %s | %s |' % (metric['label'], ROLE_NAMES[metric['role']], metric['verdict'],
                                                '; '.join('%s: %s' % (outcome, ', '.join(names))
                                                          for outcome, names in sorted(grouped.items()))))
    return '\n'.join(lines) + '\n'


def load_summary(directory):
    path = Path(directory) / 'summary.json'
    try:
        return json.loads(path.read_text())
    except (OSError, ValueError) as error:
        raise SystemExit('%s: cannot read summary.json (%s); run or summarize that directory first' % (
            directory, error))


def cmd_compare(args):
    baseline, candidate = load_summary(args.baseline), load_summary(args.candidate)
    try:
        result = compare(baseline, candidate, args.experiment, args.allow_same_build)
    except ValueError as error:
        raise SystemExit('refusing to compare: %s' % error)
    result['sources'] = {'baseline': str(args.baseline), 'candidate': str(args.candidate)}
    Path(args.out).write_text(to_markdown(result))
    print('wrote %s' % args.out)
    if args.json:
        Path(args.json).write_text(json.dumps(result, indent=1, sort_keys=True) + '\n')
        print('wrote %s' % args.json)
