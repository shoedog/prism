"""Raw results -> corpus entries -> summary.json (imported by memreport.py)."""
import json
import os
import statistics
from pathlib import Path

from memreport_derive import derive
from memreport_render import KINDS
from memreport_rule import evaluate
from memreport_validate import BUILD, PRIME, build_cache_reason, is_number, validate

RUN_KINDS = KINDS + (BUILD,)  # every kind `run` accepts; the readout lists only KINDS


def summarize(values):
    values = [value for value in values if value is not None]
    if not values:
        return None
    return {'min': min(values), 'median': statistics.median(values), 'max': max(values), 'n': len(values)}


def write_json_atomically(path, data):
    temporary = path.with_name(path.name + '.tmp')
    temporary.write_text(json.dumps(data, indent=2, sort_keys=True) + '\n')
    os.replace(temporary, path)


def effective_status(result, expected=None):
    """(status, reason) of a stored result after validating its report again with today's rules.

    The report carries the cache evidence (`cache`), so the cache rules are re-applied too. A result can only
    lose admission here: markers seen on stderr at run time are not stored, so an inadmissible result stays
    inadmissible whatever its report says.
    """
    status = result.get('status')
    if status != 'ok':
        return status or 'inadmissible', result.get('reason') or 'result has no status'
    if not isinstance(result.get('report'), dict):
        return 'inadmissible', 'result has no report'
    reason = validate(result['kind'], result['report'], expected)
    if reason is None and result['kind'] == BUILD:
        reason = build_cache_reason(result.get('cache_files'))
    return ('ok', None) if reason is None else ('inadmissible', reason)


def prime_peak(prime):
    """Lifetime peak (bytes) at the last stage of the priming run, the build that writes the cache.

    None when the priming result is absent, was not admitted, reports no peak, or did not build
    the CPG (a cache directory that was already primed makes the "priming" run a warm load, whose
    peak is not a first-build peak).
    """
    if not isinstance(prime, dict) or prime.get('status') != 'ok':
        return None
    report = prime.get('report')
    if not isinstance(report, dict) or validate(PRIME, report) is not None or not report['stages']:
        return None
    if report['cache'].get('cpg_hit') is not False:
        return None
    peak = report['stages'][-1].get('peak_bytes')
    return peak if is_number(peak) else None


def build_entry(corpus, results, prime=None):
    """One corpus's entry of summary.json from its corpus.json content, its raw results and its priming result.

    Pure: no subprocess, no clock, no filesystem. A result that is not admissible contributes to
    `inadmissible` and to nothing else. Every admitted result of a corpus must come from the build the first
    admitted one came from.
    """
    shown = KINDS + ((BUILD,) if any(item['kind'] == BUILD for item in results) else ())
    entry = dict(corpus, admissible={kind: 0 for kind in shown}, inadmissible=[], metrics={},
                 attribution=None, build=None, env=None, threads=None, first_build_peak=prime_peak(prime))
    values, identity = {}, None
    for result in sorted(results, key=lambda item: (RUN_KINDS.index(item['kind']), item['rep'])):
        kind, rep = result['kind'], result['rep']
        status, reason = effective_status(result, corpus.get('workload') if kind == 'workload' else None)
        if status == 'ok':
            this_identity = result['report']['build']['build_identity']
            if identity is None:
                identity = this_identity
            elif this_identity != identity:
                status, reason = 'inadmissible', "build identity differs from this corpus's first admitted run"
        if status != 'ok':
            entry['inadmissible'].append({'kind': kind, 'rep': rep, 'status': status, 'reason': reason})
            continue
        entry['admissible'][kind] += 1
        report = result['report']
        entry['build'], entry['env'], entry['threads'] = report.get('build'), report.get('env'), report.get('threads')
        if kind == 'workload':
            entry['attribution'] = report['attribution']
        for metric, value in derive(kind, report).items():
            values.setdefault(metric, []).append(value)
    entry['metrics'] = {metric: summarize(series) for metric, series in sorted(values.items())}
    return entry


def exclude_entry(entry, reason):
    """A copy of a corpus entry with every admitted run moved to `inadmissible` and nothing left to measure."""
    excluded = dict(entry, admissible={kind: 0 for kind in entry['admissible']}, metrics={}, attribution=None,
                    build=None, env=None, threads=None, first_build_peak=None,
                    cache_files=dict.fromkeys(entry.get('cache_files') or {}))
    excluded['inadmissible'] = list(entry['inadmissible']) + [
        {'kind': kind, 'rep': 'all %d' % count, 'status': 'inadmissible', 'reason': reason}
        for kind, count in entry['admissible'].items() if count]
    return excluded


def assemble_summary(settings, entries):
    """Settings, the corpora and the verdict. Every corpus must come from the build of the first corpus (by
    name) that has an admitted run; one that does not is excluded whole."""
    provenance = dict(settings, build=None, env=None, threads=None)
    checked = {}
    for name in sorted(entries):
        entry = entries[name]
        build = entry.get('build')
        if build is not None and provenance['build'] is None:
            provenance.update(build=build, env=entry.get('env'), threads=entry.get('threads'))
        elif build is not None and build.get('build_identity') != provenance['build'].get('build_identity'):
            entry = exclude_entry(entry, "build identity differs from the first corpus's (%s)" % (
                provenance['build'].get('build_identity'),))
        checked[name] = entry
    return {'provenance': provenance, 'corpora': checked, 'verdict': evaluate(checked)}


def write_summary(out, summary):
    path = Path(out) / 'summary.json'
    write_json_atomically(path, summary)
    return path
