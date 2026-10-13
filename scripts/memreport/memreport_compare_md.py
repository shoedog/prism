"""Markdown report of a comparison result (imported by memreport_compare.py)."""
from pathlib import Path

from memreport_compare_rule import ROLE_NAMES
from memreport_render import MIB


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


def load_text(load):
    if load is None:
        return 'n/a'
    return 'median %.1f, max %.1f over %d runs' % (load['median'], load['max'], load['n'])


def flag(value):
    return 'n/a' if value is None else ('true' if value else 'false')


def provenance_lines(label, header, source):
    env = ', '.join('`%s=%s`' % (name, header['env'][name]) for name in sorted(header['env'] or {})) or 'none'
    platform = header['platform'] or {}
    return ['- %s%s: build identity `%s`, git `%s`, CPG cache version %s, allocator %s, binary_input_dirty %s, '
            'platform %s/%s, threads %s, environment %s, %s repetitions, %s s settle, '
            'host 1-minute load average %s.' % (
                label, ' (%s)' % Path(source).name if source else '', header['build_identity'], header['git_sha'],
                header['cpg_cache_version'], header['allocator'], flag(header['binary_input_dirty']),
                platform.get('os', 'n/a'), platform.get('arch', 'n/a'), header['threads'], env, header['reps'],
                header['settle'], load_text(header['host_load']))]


def excluded_lines(side, rows):
    lines = ['', '### %s' % side.capitalize(), '']
    if not rows:
        return lines + ['None.']
    lines += ['| Corpus | Kind | Repetition | Reason |', '|---|---|---|---|']
    return lines + ['| %s | %s | %s | %s |' % (row['corpus'], row['kind'], row['rep'], row['reason']) for row in rows]


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
    lines += ['- **%s**' % warning for warning in result.get('warnings') or []]
    lines += ['', '| Corpus | Commit | Dirty files | Workload sha256 |', '|---|---|---|---|']
    for name, corpus in result['corpora'].items():
        commit, digest = corpus['commit'], corpus['workload_sha256']
        lines.append('| %s | %s | %s | %s |' % (name, '`%s`' % commit[:12] if commit else 'n/a',
                                                'n/a' if corpus['dirty'] is None else corpus['dirty'],
                                                '`%s`' % digest[:12] if digest else 'n/a'))
    lines += ['', '## Rule', ''] + result['rule']['lines']
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
    lines += ['', '## Excluded runs']
    for side in ('baseline', 'candidate'):
        lines += excluded_lines(side, result['excluded'][side])
    return '\n'.join(lines) + '\n'
