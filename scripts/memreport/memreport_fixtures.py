"""Canned reports and a fake prism-mcp for the memreport tests (shared by the test modules)."""
import hashlib
import json
import stat
from pathlib import Path

MIB = 1024 * 1024

ATTRIBUTION = {
    'files': 10, 'source_bytes': 4 * MIB, 'tree_visible_nodes': 1000, 'cpg_nodes': 500, 'cpg_edges': 900,
    'cpg_node_inline_bytes': 20 * MIB, 'node_strings': 300, 'node_string_bytes': 3000,
    'node_string_heap_bytes_estimate': 100 * MIB, 'node_unique_strings': 100, 'node_unique_string_bytes': 1000,
    'node_unique_string_heap_bytes_estimate': 40 * MIB, 'node_string_unique_ratio': 0.4,
}


def stage(name, footprint, peak, rss=None, heap=None, tree=None, elapsed=0):
    return {'name': name, 'elapsed_ms': elapsed, 'rss_bytes': rss if rss is not None else footprint,
            'phys_footprint_bytes': footprint, 'compressed_bytes': 0, 'peak_bytes': peak,
            'heap_in_use_bytes': heap, 'tree_sitter_heap_bytes': tree}


HIT = {'cpg_hit': True, 'call_edges_loaded': True}


def report(stages, attribution=None, workload=None, refresh=None, version=3, env=None, build=None, cache=None,
           threads=4):
    return {'prism_mem_report': version, 'build': build or {'build_identity': 'b', 'binary_input_dirty': False},
            'platform': {'os': 'macos'}, 'config': {}, 'env': env if env is not None else {},
            'cache': dict(HIT) if cache is None else cache, 'threads': threads,
            'stages': stages, 'ru_maxrss_bytes': 1, 'attribution': attribution or {},
            'workload': workload, 'refresh': refresh}


def stage_named(report_dict, name):
    """The stage called `name` in a canned report (the same dict, so a test can change it)."""
    return next(entry for entry in report_dict['stages'] if entry['name'] == name)


def workload_section(sha256='f' * 64, requests=8, responses=None, errors=0):
    return {'requests': requests, 'responses': requests if responses is None else responses,
            'errors': errors, 'response_bytes': 10, 'sha256': sha256}


def canned(kind, sha256='f' * 64, requests=8, heap=900 * MIB, tree=300 * MIB):
    """A report that validates for `kind`."""
    if kind == 'warm':
        return report([stage('start', 10 * MIB, 10 * MIB, heap=5 * MIB),
                       stage('after_load_repo', 100 * MIB, 120 * MIB, heap=50 * MIB),
                       stage('after_index', 400 * MIB, 700 * MIB, heap=350 * MIB),
                       stage('after_call_edges', 410 * MIB, 710 * MIB, heap=360 * MIB),
                       stage('after_settle', 410 * MIB, 710 * MIB, heap=360 * MIB)])
    if kind == 'build':
        return report([stage('start', 10 * MIB, 10 * MIB),
                       stage('after_load_repo', 100 * MIB, 120 * MIB, elapsed=1000),
                       stage('after_index', 600 * MIB, 900 * MIB, elapsed=9000),
                       stage('after_call_edges', 610 * MIB, 910 * MIB, elapsed=9500)],
                      cache={'cpg_hit': False, 'call_edges_loaded': False})
    if kind == 'cold':
        return report([stage('start', 10 * MIB, 10 * MIB), stage('after_call_edges', 500 * MIB, 800 * MIB)],
                      cache={'cpg_hit': None, 'call_edges_loaded': False})
    if kind == 'refresh':
        return report([stage('start', 10 * MIB, 10 * MIB), stage('after_refresh', 500 * MIB, 900 * MIB)],
                      refresh={'status': 'refreshed', 'generation': 1})
    if kind == 'workload':
        return report([stage('start', 10 * MIB, 10 * MIB),
                       stage('after_settle', 1000 * MIB, 1200 * MIB, heap=heap, tree=tree)],
                      attribution=dict(ATTRIBUTION), workload=workload_section(sha256, requests))
    raise ValueError(kind)


def emit(report_dict, before=''):
    """Fake-binary body that prints `before` lines, then the report line, on stderr."""
    return ('%sprint(json.dumps(json.loads(%r)), file=sys.stderr)\n'
            % (before, json.dumps(report_dict)))


def noise(text):
    return 'print(%r, file=sys.stderr)\n' % text


def fake_binary(directory, body, name='fake-prism-mcp'):
    """A stand-in for prism-mcp: a Python script that runs `body`."""
    path = Path(directory) / name
    path.write_text('#!/usr/bin/env python3\nimport json, sys, time\n' + body)
    path.chmod(path.stat().st_mode | stat.S_IXUSR)
    return path


def digest(data):
    return hashlib.sha256(data).hexdigest()


FAKE_PRISM = r'''
import hashlib, json, os, sys, time
here = os.path.dirname(os.path.abspath(__file__))
cfg = json.load(open(os.path.join(here, 'cfg.json')))
argv = sys.argv[1:]


def opt(name):
    return argv[argv.index(name) + 1] if name in argv else None


def tick(label):
    path = os.path.join(here, 'calls-' + label)
    count = int(open(path).read()) if os.path.exists(path) else 0
    open(path, 'w').write(str(count + 1))
    return count


repo = os.path.basename(opt('--repo').rstrip('/'))
if '--mem-emit-workload' in argv:
    if repo in cfg.get('emit_sleep', []):
        time.sleep(30)
    if repo in cfg.get('emit_fail', []):
        print('cannot emit for ' + repo, file=sys.stderr)
        sys.exit(2)
    sys.stdout.write('{"id":1}\n' * cfg.get('emit', {}).get(repo, 3))
    sys.exit(0)
if '--no-cache' in argv:
    kind = 'cold'
elif '--mem-workload' in argv:
    kind = 'workload'
elif '--mem-refresh' in argv:
    kind = 'refresh'
else:
    kind = 'warm'
call = tick(repo + '-' + kind)
# A cache directory without the marker is empty: this run builds and writes the cache (a miss). The marker
# is not a cache file; the second run into the same directory hits.
marker = os.path.join(opt('--cache-dir') or '', '.fake-primed')
miss = bool(opt('--cache-dir')) and kind == 'warm' and not os.path.exists(marker)
if miss:
    d = os.path.join(opt('--cache-dir'), 'h1')
    os.makedirs(d, exist_ok=True)
    by_call = cfg.get('cache_bytes_by_call') or []
    sizes = {'cache_bytes': by_call[call] if call < len(by_call) else cfg.get('cache_bytes'),
             'sidecar_bytes': cfg.get('sidecar_bytes')}
    for name, key in (('cpg-cache.bin', 'cache_bytes'), ('resolved-call-edge-index.bin', 'sidecar_bytes')):
        if sizes[key] is not None:
            open(os.path.join(d, name), 'wb').write(b'x' * sizes[key])
    open(marker, 'w').write('1')
shape = 'build' if miss and 'build' in cfg['reports'] else kind
report = json.loads(json.dumps(cfg.get('repo_reports', {}).get(repo, {}).get(shape) or cfg['reports'][shape]))
if miss and cfg.get('miss_cache', True):
    report['cache'] = {'cpg_hit': False, 'call_edges_loaded': False}    # a first build builds both
if kind == 'workload':
    data = open(opt('--mem-workload'), 'rb').read()
    by_call = cfg.get('workload_sha_by_call') or []
    report['workload']['sha256'] = by_call[call] if call < len(by_call) else hashlib.sha256(data).hexdigest()
    report['workload']['requests'] = report['workload']['responses'] = data.count(b'\n')
    if cfg.get('mutate_source'):
        open(cfg['mutate_source'], 'wb').write(b'{"id":9}\n')
for text in cfg.get('stderr', {}).get(kind, []) + cfg.get('repo_stderr', {}).get(repo, {}).get(kind, []):
    print(text, file=sys.stderr)
if call in cfg.get('stderr_calls', {}).get(kind, []):
    print(cfg['stderr_call_text'], file=sys.stderr)
print(json.dumps(report), file=sys.stderr)
'''


def fake_prism(directory, reports=None, **cfg):
    """A configurable fake prism-mcp; `cfg` is written next to it as cfg.json (see FAKE_PRISM)."""
    directory = Path(directory)
    cfg['reports'] = reports or {kind: canned(kind) for kind in ('warm', 'cold', 'workload', 'refresh')}
    (directory / 'cfg.json').write_text(json.dumps(cfg))
    path = directory / 'fake-prism'
    path.write_text('#!/usr/bin/env python3\n' + FAKE_PRISM)
    path.chmod(path.stat().st_mode | stat.S_IXUSR)
    return path


def prime_result(report_dict=None, status='ok'):
    """The stored result of the priming run: by default a warm-shaped report that built both caches."""
    built = canned('warm')
    built['cache'] = {'cpg_hit': False, 'call_edges_loaded': False}
    return {'kind': 'prime', 'status': status, 'report': report_dict or built}


def ok_result(kind, rep, **canned_args):
    return {'kind': kind, 'rep': rep, 'status': 'ok', 'report': canned(kind, **canned_args)}


def sample_summary(reps=3):
    """A summary built through build_entry/assemble_summary: corpus 'big' is complete, 'small' lost runs.

    big:   M1 = 300 MiB of 1000 (both arms), M2 = 120 MiB cache file of a 700 MiB peak (fraction arm),
           M3 = 60 MiB of 1000 (neither arm).
    small: one workload run excluded, tree heap unavailable on the one that remains, no cache file size, no
           priming result.
    """
    import memreport
    corpora = {
        'big': (dict(path='/c/big', commit='a' * 40, dirty=0, workload={'sha256': 'f' * 64, 'requests': 8},
                     cache_files={'cpg_cache_bytes': 120 * MIB, 'call_edge_sidecar_bytes': 9 * MIB}),
                [ok_result(kind, rep) for kind in memreport.KINDS for rep in range(1, reps + 1)],
                prime_result()),
        'small': (dict(path='/c/small', commit='b' * 40, dirty=2, workload={'sha256': 'f' * 64, 'requests': 8},
                       cache_files={'cpg_cache_bytes': None, 'call_edge_sidecar_bytes': None}),
                  [ok_result('warm', 1), ok_result('workload', 1, tree=None),
                   {'kind': 'workload', 'rep': 2, 'status': 'inadmissible', 'reason': 'workload had 1 errors',
                    'report': canned('workload')}],
                  None),
    }
    entries = {name: memreport.build_entry(corpus, results, prime)
               for name, (corpus, results, prime) in corpora.items()}
    return memreport.assemble_summary({'reps': reps, 'settle': 5, 'timeout': 1800, 'bin': 'fake'}, entries)
