"""Markdown readout for a memreport summary.json (imported by memreport.py)."""

MIB = 1024 * 1024
KINDS = ('warm', 'cold', 'workload', 'refresh')  # the run kinds, in the order the readout lists them

ESTIMATE_DEFINITIONS = (
    '- **M1** estimate: measured live tree-sitter heap after the workload and settle. Base: steady-state footprint.',
    '- **M2** estimate: size of the CPG cache file, an upper bound on the buffer that streaming the load would '
    'remove (one measurement per corpus). Base: peak through the index load.',
    '- **M3** estimate: estimated heap of repeated node strings. Base: steady-state footprint.',
)


def mib(stats):
    """median (min-max, n) of a metric summary in MiB; n/a when the metric is unavailable."""
    if not stats:
        return 'n/a'
    return '%.1f (%.1f–%.1f, n=%d)' % (stats['median'] / MIB, stats['min'] / MIB, stats['max'] / MIB, stats['n'])


def mib_value(value):
    return 'n/a' if value is None else '%.1f' % (value / MIB)


def na(value):
    return 'n/a' if value is None else value


def flag(value):
    return 'n/a' if value is None else ('true' if value else 'false')


def env_text(env):
    if env is None:
        return 'n/a'
    return ', '.join('`%s=%s`' % (name, env[name]) for name in sorted(env)) or 'none'


def median(metrics, name):
    stats = metrics.get(name)
    return stats['median'] if stats else None


def attributed_bytes(attribution):
    """Bytes the attribution accounts for in the index: node inline size plus estimated string heap."""
    parts = [attribution.get(key) for key in ('cpg_node_inline_bytes', 'node_string_heap_bytes_estimate')]
    return None if any(part is None for part in parts) else sum(parts)


def arm_text(arm):
    return 'n/a' if arm is None else ('—' if arm == 'none' else arm)


def n_text(row_n):
    return 'file' if row_n is None else str(row_n)


def decision_lines(summary):
    reps = summary['provenance'].get('reps')
    lines = ['## Decision rule', '',
             'A candidate advances to its experiment when its estimate is at least 200 MiB or 15 % of its base',
             'on at least two corpora. Each row shows the numbers the rule was applied to.', '',
             '| Candidate | Corpus | Estimate (MiB) | Base (MiB) | Estimate / base | Arm met | n (estimate / base) | Note |',
             '|---|---|---|---|---|---|---|---|']
    for candidate in sorted(summary['verdict']):
        rows = summary['verdict'][candidate].get('rows', {})
        for name in sorted(rows):
            row = rows[name]
            counts = [count for count in (row['estimate_n'], row['base_n']) if count is not None]
            low = reps is not None and any(count < reps for count in counts)
            percent = 'n/a' if row['percent'] is None else '%.1f %%' % row['percent']
            lines.append('| %s | %s | %s | %s | %s | %s | %s / %s | %s |' % (
                candidate, name, mib_value(row['estimate']), mib_value(row['base']), percent,
                arm_text(row['arm']), n_text(row['estimate_n']), n_text(row['base_n']),
                'low n (%s repetitions requested)' % reps if low else ''))
    lines += ['', *ESTIMATE_DEFINITIONS, '',
              '| Candidate | Corpora meeting the rule | Advances |', '|---|---|---|']
    for candidate in sorted(summary['verdict']):
        verdict = summary['verdict'][candidate]
        lines.append('| %s | %s | %s |' % (candidate, ', '.join(verdict['corpora']) or '—',
                                           'yes' if verdict['advance'] else 'no'))
    return lines


def render(summary):
    provenance = summary['provenance']
    build = provenance.get('build') or {}
    corpora = sorted(summary['corpora'].items())
    lines = [
        '# prism-mcp memory readout',
        '',
        'This is **coarse triage**: it ranks reduction candidates for an experiment and selects none.',
        'Footprints, peaks, live heap and the tree-sitter tree heap (M1) are measured on this process; the M2',
        'figure is a file size; the string figures (M3, "attributed") are estimates from component counts.',
        'A value that is unavailable is shown as n/a, never as 0.',
        '',
        '## Provenance',
        '',
        '- Binary: build identity `%s`, git `%s`, CPG cache version %s, allocator %s, binary_input_dirty %s.' % (
            na(build.get('build_identity')), na(build.get('git_sha')), na(build.get('cpg_cache_version')),
            na(build.get('allocator')), flag(build.get('binary_input_dirty'))),
        '- Environment of the measured process: %s.' % env_text(provenance.get('env')),
        '- %s repetitions per kind, %s s settle. Values are median (min–max, n) in MiB.' % (
            provenance.get('reps'), provenance.get('settle')),
        '',
        '| Corpus | Commit | Dirty files | Admissible runs (%s) |' % '/'.join(KINDS),
        '|---|---|---|---|',
    ]
    for name, entry in corpora:
        counts = '/'.join(str(entry['admissible'][kind]) for kind in KINDS)
        lines.append('| %s | `%s` | %s | %s |' % (name, (entry.get('commit') or '')[:12], entry.get('dirty'), counts))
    lines += ['', '## Footprint', '',
              '| Corpus | Steady state | Steady live heap | Peak through index load | '
              'Observed load overshoot (lower bound; 0 is uninformative) | Refresh peak | Cold peak |',
              '|---|---|---|---|---|---|---|']
    for name, entry in corpora:
        metrics = entry['metrics']
        lines.append('| %s | %s | %s | %s | %s | %s | %s |' % (
            name, mib(metrics.get('steady')), mib(metrics.get('steady_heap')), mib(metrics.get('peak')),
            mib(metrics.get('m2_observed_overshoot')), mib(metrics.get('refresh_peak')),
            mib(metrics.get('cold_peak'))))
    lines += ['', '## Attribution', '',
              '| Corpus | Files | Source (MiB) | Visible tree nodes | Tree heap, measured (M1) | CPG nodes / edges | '
              'Repeated node strings, estimate (M3) | Unique-byte ratio | Index live heap | Attributed (estimate) | '
              'Not attributed |',
              '|---|---|---|---|---|---|---|---|---|---|---|']
    for name, entry in corpora:
        attribution, metrics = entry.get('attribution') or {}, entry['metrics']
        ratio, source = attribution.get('node_string_unique_ratio'), attribution.get('source_bytes')
        index_heap, attributed = median(metrics, 'index_heap'), attributed_bytes(attribution)
        remainder = None if index_heap is None or attributed is None else index_heap - attributed
        lines.append('| %s | %s | %s | %s | %s | %s / %s | %s | %s | %s | %s | %s |' % (
            name, attribution.get('files', 'n/a'), mib_value(source), attribution.get('tree_visible_nodes', 'n/a'),
            mib(metrics.get('m1')), attribution.get('cpg_nodes', 'n/a'), attribution.get('cpg_edges', 'n/a'),
            mib(metrics.get('m3')), 'n/a' if ratio is None else '%.3f' % ratio,
            mib(metrics.get('index_heap')), mib_value(attributed), mib_value(remainder)))
    lines += [''] + decision_lines(summary)
    excluded = [(name, item) for name, entry in corpora for item in entry['inadmissible']]
    lines += ['', '## Excluded runs', '']
    if excluded:
        lines += ['| Corpus | Kind | Repetition | Status | Reason |', '|---|---|---|---|---|']
        lines += ['| %s | %s | %s | %s | %s |' % (name, item['kind'], item['rep'], item['status'], item['reason'])
                  for name, item in excluded]
    else:
        lines.append('None.')
    return '\n'.join(lines) + '\n'
