"""Baseline-against-candidate comparison of two run directories under a pinned rule (imported by memreport.py).

`compare` is pure: two summaries and an experiment name in, a result dict out; it raises ValueError naming the
difference when the two runs are not comparable. `cmd_compare` rebuilds both summaries from `raw/` with today's
rules (never trusting a saved summary.json), compares them and writes the Markdown (memreport_compare_md.py)
and, on request, the JSON. The rules themselves are in memreport_compare_rule.py.
"""
import json
import statistics
from pathlib import Path

from memreport_compare_md import to_markdown  # noqa: F401
from memreport_compare_rule import (EXPERIMENTS, GATE, NOT_WORSE,  # noqa: F401
                                    check_experiments, corpus_row, metric_verdict, rule_data)
from memreport_summary import rescore

HEADLINE = ('build_identity', 'git_sha', 'cpg_cache_version', 'allocator', 'binary_input_dirty')


def load_of(summary):
    """Median and maximum 1-minute load average over the admitted runs that recorded one, or None."""
    loads = [value for entry in (summary.get('corpora') or {}).values() for value in entry.get('load_avg_1m') or []]
    return {'median': statistics.median(loads), 'max': max(loads), 'n': len(loads)} if loads else None


def provenance_of(summary):
    """The settings and build header of one run that a comparison reports and checks."""
    provenance = summary.get('provenance') or {}
    build = provenance.get('build') or {}
    header = {key: build.get(key) for key in HEADLINE}
    return dict(header, threads=provenance.get('threads'), env=provenance.get('env'),
                platform=provenance.get('platform'), reps=provenance.get('reps'), settle=provenance.get('settle'),
                host_load=load_of(summary))


def platform_known(platform):
    return isinstance(platform, dict) and all(isinstance(platform.get(key), str) for key in ('os', 'arch'))


def corpus_conditions(entry):
    return {'threads': entry.get('threads'), 'measured-process environment': entry.get('env'),
            'allocator': (entry.get('build') or {}).get('allocator'), 'platform': entry.get('platform')}


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
        if base.get('build') is None or cand.get('build') is None:
            continue    # no admitted run on one side: nothing was measured there to compare
        for side, entry in (('baseline', base), ('candidate', cand)):
            if not platform_known(entry.get('platform')):
                problems.append('corpus %s: the %s platform is unknown' % (name, side))
        base_conditions, cand_conditions = corpus_conditions(base), corpus_conditions(cand)
        for label in base_conditions:
            known = label != 'platform' or (platform_known(base['platform']) and platform_known(cand['platform']))
            if known and base_conditions[label] != cand_conditions[label]:
                problems.append('corpus %s: %s differs (baseline %s, candidate %s)' % (
                    name, label, base_conditions[label], cand_conditions[label]))
    base_header, cand_header = provenance_of(baseline), provenance_of(candidate)
    for label in ('reps', 'settle'):
        if base_header[label] != cand_header[label]:
            problems.append('%s differs (baseline %s, candidate %s)' % (label, base_header[label], cand_header[label]))
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


def excluded_runs(summary):
    """Every run left out of a summary, as rows: corpus, kind, repetition, reason."""
    return [{'corpus': name, 'kind': item.get('kind'), 'rep': item.get('rep'), 'reason': item.get('reason')}
            for name, entry in sorted((summary.get('corpora') or {}).items())
            for item in entry.get('inadmissible') or []]


def warnings_for(base_header, cand_header, corpora):
    """Conditions the rule cannot see but a reader must: uncommitted changes in a binary's input or a corpus."""
    lines = []
    for side, header in (('baseline', base_header), ('candidate', cand_header)):
        if header['binary_input_dirty'] is True:
            lines.append('The %s binary was built from a tree with uncommitted changes (binary_input_dirty).' % side)
    for name, corpus in corpora.items():
        if corpus['dirty']:
            lines.append('Corpus %s has %s uncommitted files: equal counts are not equal trees.' % (
                name, corpus['dirty']))
    return lines


def compare(baseline, candidate, experiment, allow_same_build=False):
    """The comparison of two summaries under the pinned rule of `experiment`.

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
    corpora = {name: {'commit': baseline['corpora'][name].get('commit'),
                      'dirty': baseline['corpora'][name].get('dirty'),
                      'workload_sha256': (baseline['corpora'][name].get('workload') or {}).get('sha256')}
               for name in names}
    return {'experiment': experiment, 'same_build': base_header['build_identity'] == cand_header['build_identity'],
            'baseline': base_header, 'candidate': cand_header, 'corpora': corpora, 'metrics': metrics,
            'excluded': {'baseline': excluded_runs(baseline), 'candidate': excluded_runs(candidate)},
            'warnings': warnings_for(base_header, cand_header, corpora), 'rule': rule_data()}


def cmd_compare(args):
    baseline_dir, candidate_dir = Path(args.baseline), Path(args.candidate)
    if baseline_dir.resolve() == candidate_dir.resolve():
        raise SystemExit('baseline and candidate are the same directory (%s); a run is not compared with itself, '
                         'with or without --allow-same-build' % baseline_dir)
    baseline, candidate = rescore(baseline_dir), rescore(candidate_dir)
    try:
        result = compare(baseline, candidate, args.experiment, args.allow_same_build)
    except ValueError as error:
        raise SystemExit('refusing to compare: %s' % error)
    result['sources'] = {'baseline': str(baseline_dir), 'candidate': str(candidate_dir)}
    Path(args.out).write_text(to_markdown(result))
    print('wrote %s' % args.out)
    if args.json:
        Path(args.json).write_text(json.dumps(result, indent=1, sort_keys=True) + '\n')
        print('wrote %s' % args.json)
