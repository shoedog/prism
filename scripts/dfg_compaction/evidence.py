"""Admission checks for the retained memreport/night-runner wire formats."""
import hashlib
import json
import math
from pathlib import Path
import re

PRIME_TIMERS = {'load.read_decode', 'build.dfg', 'build.call_graph', 'build.assemble',
                'build.registry', 'ctx.live_types',
                'cache.save', 'nav.maps', 'nav.call_edges.load', 'nav.call_edges.compute',
                'nav.call_edges.save', 'nav.call_edges'}
WARM_TIMERS = {'load.read_decode', 'load.validate', 'load.reconstruct_arrays',
               'load.reconstruct_maps', 'ctx.build_registry', 'ctx.live_types',
               'nav.maps', 'nav.call_edges.load', 'nav.call_edges'}
SAVE_ERRORS = ('failed to save CPG cache', 'nav edge cache: failed to save sidecar')
BUILD_STRINGS = ('git_sha', 'build_identity', 'package_version', 'grammar_fingerprint', 'allocator')


def load(path):
    value = json.loads(Path(path).read_text())
    if not isinstance(value, dict):
        raise ValueError(f'expected JSON object: {path}')
    return value


def text(value):
    if not isinstance(value, str) or not value.strip():
        raise ValueError('missing/empty provenance string')
    return value


def hex_digest(value, lengths=(64,)):
    if not isinstance(value, str) or len(value) not in lengths or not re.fullmatch('[0-9a-f]+', value):
        raise ValueError('missing/malformed provenance digest')
    return value


def number(value):
    if type(value) not in (int, float) or not math.isfinite(value) or value < 0:
        raise ValueError('missing/nonfinite/negative counter')
    return value


def build_fields(report):
    build = report['build']
    for key in BUILD_STRINGS:
        text(build[key])
    if type(build['binary_input_dirty']) is not bool:
        raise ValueError('missing binary_input_dirty')
    if type(build['cpg_cache_version']) is not int or build['cpg_cache_version'] <= 0:
        raise ValueError('missing/invalid cache version')


def corpus_fields(info):
    if not Path(text(info['path'])).is_absolute():
        raise ValueError('corpus path must be absolute')
    hex_digest(info['commit'], (40, 64))
    if type(info['dirty']) is not int or info['dirty'] != 0:
        raise ValueError('corpus dirty state must be known and clean (0)')
    workload = info['workload']
    hex_digest(workload['sha256'])
    if type(workload['requests']) is not int or workload['requests'] <= 0:
        raise ValueError('workload must have positive request count')


def receipts(path):
    """Consume the controller's existing binary-receipts.json without rewriting it."""
    rows = load(path)
    for side, version in [('base', 120), ('candidate', 121)]:
        r = rows[side]['probe']
        sha = hex_digest(r['source_sha'], (40, 64))
        if r['source_base'] != sha or r['activation_source_sha'] != rows['candidate']['probe']['source_sha']:
            raise ValueError('receipt source binding mismatch')
        if type(r['binary_input_dirty']) is not bool:
            raise ValueError('receipt dirty mode missing')
        for key in ('dirty_patch_sha256', 'source_manifest_sha256'):
            hex_digest(r[key])
        if (r['cache_version'] != version or r['navigation_sidecar_version'] != 66
                or type(r['cache_version']) is not int or type(r['navigation_sidecar_version']) is not int):
            raise ValueError('receipt cache versions must be base 120/candidate 121, sidecar 66')
        if r['profile'] != 'release' or r['features'] != ['mcp'] or r['dfg_oracle'] is not False:
            raise ValueError('receipt profile/features mismatch')
        for key in ('rustc', 'cargo', 'allocator'):
            text(r[key])
        out = r['build_output']
        for key in ('GIT_SHA', 'GRAMMAR_FINGERPRINT', 'PRISM_CACHE_BUILD_IDENTITY'):
            text(out[key])
        dirty = r['binary_input_dirty']
        if out['GIT_SHA'] != sha[:12] + ('-dirty' if dirty else '') or out['PRISM_BINARY_INPUT_DIRTY'] != str(int(dirty)):
            raise ValueError('receipt build output does not bind source/dirty mode')
        env = r['required_environment']
        if (env['PRISM_PROBE_NO_CENSUS'] != '1' or env['RAYON_NUM_THREADS'] is not None
                or (dirty and env['PRISM_NAV_EDGE_CACHE_LOAD_DIRTY'] != '1')):
            raise ValueError('receipt acceptance environment mismatch')
        hex_digest(r['binaries']['prism-mcp'])
        binary = Path(path).resolve().parent / 'bin' / side / 'prism-mcp'
        if digest_file(binary) != r['binaries']['prism-mcp']:
            raise ValueError(f'{side} binary SHA256 differs from expected receipt')
    a, b = (rows[s]['probe'] for s in ('base', 'candidate'))
    if a['source_sha'] == b['source_sha'] or a['build_output']['PRISM_CACHE_BUILD_IDENTITY'] == b['build_output']['PRISM_CACHE_BUILD_IDENTITY']:
        raise ValueError('base/candidate identities must be distinct')
    for field in ('profile', 'features', 'dfg_oracle', 'allocator', 'rustc', 'cargo', 'binary_input_dirty'):
        if a[field] != b[field]:
            raise ValueError(f'unmatched receipt {field}')
    # The original receipts carry compiler/profile/version IDs, but package_version
    # is recorded in the tiny preflight's build section. Bind that witness too.
    preflight = load(Path(path).parent / 'preflight-result.json')
    if preflight['result'] != 'PASS':
        raise ValueError('missing passing preflight receipt')
    for side in ('base', 'candidate'):
        expected, witness = rows[side]['probe'], preflight['sides'][side]
        if witness['result'] != 'PASS' or witness['source_sha'] != expected['source_sha']:
            raise ValueError('preflight source binding mismatch')
        bind_build({'build': witness['build'], 'env': expected['required_environment']}, expected)
        expected['report_build'] = witness['build']
    return rows


def digest_file(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def bind_build(report, expected):
    build_fields(report)
    out = expected['build_output']
    wanted = {'git_sha': out['GIT_SHA'], 'build_identity': out['PRISM_CACHE_BUILD_IDENTITY'],
              'grammar_fingerprint': out['GRAMMAR_FINGERPRINT'],
              'binary_input_dirty': expected['binary_input_dirty'],
              'allocator': expected['allocator'], 'cpg_cache_version': expected['cache_version']}
    if any(report['build'][k] != v for k, v in wanted.items()):
        raise ValueError('report build differs from expected receipt')
    if 'report_build' in expected and report['build'] != expected['report_build']:
        raise ValueError('report build differs from controller preflight receipt')
    for key, value in expected['required_environment'].items():
        if report['env'].get(key) != value:
            raise ValueError(f'report environment differs from expected receipt: {key}')


def inventory(directory, corpus, info, rescore=False):
    corpus_fields(info)
    before = load(directory.parent / 'before-inventory.json')
    after = directory.parent / 'after-inventory.json'
    if rescore and not after.is_file():
        raise ValueError('rescore requires final after-inventory.json')
    for path in (after, directory.parent / 'after-steps-inventory.json'):
        if path.exists() and load(path) != before:
            raise ValueError('before/after corpus or workload inventory differs')
    c = before['corpora'][corpus]
    if c != {'path': info['path'], 'commit': info['commit'], 'status': ''}:
        raise ValueError('corpus differs from controller inventory')
    m = before['manifest'][corpus]
    if any(m[k] != info[k] for k in ('path', 'commit', 'dirty')):
        raise ValueError('workload MANIFEST corpus differs')
    if any(m[k] != info['workload'][k] for k in ('requests', 'sha256')):
        raise ValueError('workload MANIFEST digest/count differs')
    hex_digest(before['workloads']['MANIFEST.json'])
    data = (directory / 'workloads' / (corpus + '.jsonl')).read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    if (digest != info['workload']['sha256'] or digest != before['workloads'][corpus + '.jsonl']
            or data.count(b'\n') != info['workload']['requests']):
        raise ValueError('retained workload hash/request count differs')


def settings(report, directory, corpus, probe_path, hit, run=None, raw=None):
    cfg = report['config']
    if cfg['repo_root'] != corpus['path']:
        raise ValueError('config.repo_root differs from declared corpus/probe')
    if cfg['cache'] != 'dir:' + str(directory / 'cache' / corpus['name']):
        raise ValueError('config cache is not the declared per-side cache path')
    if report['env'].get('PRISM_PROBE_OUT') != str(probe_path):
        raise ValueError('PRISM_PROBE_OUT differs from probe input')
    if not isinstance(report['platform'], dict) or any(not text(report['platform'][k]) for k in ('os', 'arch')):
        raise ValueError('missing platform')
    for key in ('refresh_policy', 'workload', 'refresh', 'settle_secs'):
        if key not in cfg:
            raise ValueError(f'missing config.{key}')
    if cfg['refresh_policy'] != 'warn-only' or cfg['refresh'] is not False or cfg['workload'] is not None:
        raise ValueError('unexpected acceptance configuration')
    if type(cfg['settle_secs']) is not int or cfg['settle_secs'] < 0:
        raise ValueError('invalid settle_secs')
    if run is not None:
        settle = run['settle'] if hit else 0
        if cfg['settle_secs'] != settle:
            raise ValueError('report settle_secs differs from driver settings')
        expected = [run['bin'], '--repo', corpus['path'], '--eager', '--mem-settle-secs',
                    str(settle), '--cache-dir', str(directory / 'cache' / corpus['name'])]
        if raw['command'] != expected:
            raise ValueError('raw command differs from expected binary/corpus/settings')
    return {k: v for k, v in cfg.items() if k != 'cache'}


def timers(records, hit, platform):
    stages = [r for r in records if r['kind'] == 'stage']
    names = set()
    for row in stages:
        names.add(text(row['name']))
        number(row['wall_ms'])
        # The MCP probe uses getrusage on these supported measurement hosts.
        if row['cpu_ms'] is not None or platform['os'] in ('macos', 'linux'):
            number(row['cpu_ms'])
    required = WARM_TIMERS if hit else PRIME_TIMERS
    if not required <= names:
        raise ValueError('missing required probe timers: ' + ', '.join(sorted(required - names)))


def save_evidence(directory, raw_samples, cache_sizes):
    stderr = list((directory / 'raw').rglob('*.stderr'))
    log = directory.parent / (directory.name + '.log')
    log_text = log.read_text()
    if not log_text.strip():
        raise ValueError('empty driver run log')
    sources = [json.dumps(raw_samples), log_text] + [p.read_text() for p in stderr]
    if any(marker in source for marker in SAVE_ERRORS for source in sources):
        raise ValueError('cache save failure in retained prime/report/run log/stderr')
    return {'stderr': 'stderr scanned' if stderr else 'stderr absent: driver discards successful-prime stderr',
            'checked': ['prime miss flags/status', 'post-prime cache inventory',
                        'post-run cache inventory', 'raw report fields', str(log)],
            'cache_files': cache_sizes}
