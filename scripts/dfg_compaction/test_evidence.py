"""Full CLI controls in the unchanged night-runner evidence layout."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from test_acceptance import raw
from memreport_fixtures import stage_named

PRIME = {'load.read_decode', 'build.dfg', 'build.call_graph', 'build.assemble',
         'build.registry', 'ctx.live_types',
         'cache.save', 'nav.maps', 'nav.call_edges.load', 'nav.call_edges.compute',
         'nav.call_edges.save', 'nav.call_edges'}
WARM = {'load.read_decode', 'load.validate', 'load.reconstruct_arrays',
        'load.reconstruct_maps', 'ctx.build_registry', 'ctx.live_types',
        'nav.maps', 'nav.call_edges.load', 'nav.call_edges'}


def write(path, value):
    path.write_text(json.dumps(value))


def change(path, fn):
    obj = json.loads(path.read_text())
    fn(obj)
    write(path, obj)


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name).resolve()
        self.out = self.root / 'set-1'
        self.out.mkdir()
        workload = b'{"method":"test"}\n'
        self.digest = hashlib.sha256(workload).hexdigest()
        self.commit = 'c' * 40
        manifest = {'tiny': {'path': '/corpus', 'commit': self.commit, 'dirty': 0,
                             'requests': 1, 'sha256': self.digest}}
        inv = {'corpora': {'tiny': {'path': '/corpus', 'commit': self.commit, 'status': ''}},
               'manifest': manifest, 'workloads': {'tiny.jsonl': self.digest,
               'MANIFEST.json': hashlib.sha256(json.dumps(manifest).encode()).hexdigest()}}
        for name in ('before', 'after', 'after-steps'):
            write(self.out / (name + '-inventory.json'), inv)
        receipts = {}
        for side, sha, version in [('base', 'a' * 40, 120), ('candidate', 'b' * 40, 121)]:
            binary = self.root / 'bin' / side / 'prism-mcp'
            binary.parent.mkdir(parents=True)
            binary.write_bytes(side.encode())
            receipts[side] = {'probe': {
                'source_sha': sha, 'source_base': sha, 'activation_source_sha': 'b' * 40,
                'binary_input_dirty': True, 'dirty_patch_sha256': 'd' * 64,
                'source_manifest_sha256': 'e' * 64, 'profile': 'release', 'features': ['mcp'],
                'dfg_oracle': False, 'allocator': 'system', 'rustc': 'rustc 1', 'cargo': 'cargo 1',
                'build_output': {'GIT_SHA': sha[:12] + '-dirty', 'GRAMMAR_FINGERPRINT': 'grammar',
                    'PRISM_BINARY_INPUT_DIRTY': '1', 'PRISM_CACHE_BUILD_IDENTITY': side},
                'cache_version': version, 'navigation_sidecar_version': 66,
                'binaries': {'prism-mcp': hashlib.sha256(side.encode()).hexdigest()},
                'required_environment': {'PRISM_PROBE_NO_CENSUS': '1',
                    'PRISM_NAV_EDGE_CACHE_LOAD_DIRTY': '1', 'RAYON_NUM_THREADS': None}}}
            directory = self.out / (side + '-tiny')
            rd = directory / 'raw/tiny'
            rd.mkdir(parents=True)
            cache = directory / 'cache/tiny'
            cache.mkdir(parents=True)
            for name in ('cpg-cache-v2.bin', 'resolved-call-edge-index.bin'):
                (cache / name).write_bytes(b'saved')
            write(rd / 'corpus.json', {'path': '/corpus', 'commit': self.commit, 'dirty': 0,
                'workload': {'sha256': self.digest, 'requests': 1},
                'cache_files': {'cpg_cache_bytes': 5, 'call_edge_sidecar_bytes': 5}})
            wp = directory / 'workloads'
            wp.mkdir()
            (wp / 'tiny.jsonl').write_bytes(workload)
            write(directory / 'run.json', {'reps': 2, 'settle': 10, 'timeout': 1800,
                                          'bin': str(binary), 'kinds': ['warm']})
            probe = self.out / (side + '-tiny.jsonl')
            records = []
            for pid, name, hit in [(1, 'prime', False), (2, 'warm-1', True), (3, 'warm-2', True)]:
                obj = raw(hit)
                obj['kind'] = 'warm' if hit else 'prime'
                if hit:
                    obj['rep'] = pid - 1
                r = obj['report']
                r['build'].update(git_sha=sha[:12] + '-dirty', build_identity=side,
                                  binary_input_dirty=True, cpg_cache_version=version)
                r['env'].update(PRISM_NAV_EDGE_CACHE_LOAD_DIRTY='1', PRISM_PROBE_OUT=str(probe))
                r['config'] = {'repo_root': '/corpus', 'cache': 'dir:' + str(cache),
                    'settle_secs': 10 if hit else 0, 'refresh_policy': 'warn-only',
                    'refresh': False, 'workload': None}
                stage_named(r, 'after_index')['heap_in_use_bytes'] = 1000 if side == 'base' else 600
                stage_named(r, 'after_call_edges')['peak_bytes'] = 2000
                obj['command'] = [str(binary), '--repo', '/corpus', '--eager',
                    '--mem-settle-secs', str(10 if hit else 0), '--cache-dir', str(cache)]
                write(rd / (name + '.json'), obj)
                records.append({'pid': pid, 'kind': 'meta', 'repo': '/corpus', 'cpg_hit': hit})
                records.extend({'pid': pid, 'kind': 'stage', 'name': stage, 'wall_ms': 1, 'cpu_ms': 1}
                               for stage in sorted(WARM if hit else PRIME))
            self.write_probe(side, records)
            (self.out / (side + '-tiny.log')).write_text('tiny warm #1: ok\ntiny warm #2: ok\nwrote summary.json\n')
        write(self.root / 'binary-receipts.json', receipts)
        preflight = {'result': 'PASS', 'sides': {}}
        for side in ('base', 'candidate'):
            build = json.loads((self.out / (side + '-tiny/raw/tiny/prime.json')).read_text())['report']['build']
            preflight['sides'][side] = {'result': 'PASS',
                'source_sha': receipts[side]['probe']['source_sha'], 'build': build}
        write(self.root / 'preflight-result.json', preflight)

    def write_probe(self, side, records):
        (self.out / (side + '-tiny.jsonl')).write_text('\n'.join(map(json.dumps, records)) + '\n')

    def reports(self, fn, side=None):
        for s in ([side] if side else ['base', 'candidate']):
            for name in ('prime', 'warm-1', 'warm-2'):
                change(self.out / (s + '-tiny') / 'raw/tiny' / (name + '.json'),
                       lambda obj: fn(obj['report']))

    def corpus(self, fn):
        for side in ('base', 'candidate'):
            change(self.out / (side + '-tiny/raw/tiny/corpus.json'), fn)

    def cli(self, *extra):
        args = [str(self.out / name) for name in
                ('base-tiny', 'candidate-tiny', 'base-tiny.jsonl', 'candidate-tiny.jsonl')]
        done = subprocess.run([sys.executable, str(Path(__file__).with_name('read_acceptance.py')),
                               *args, 'tiny', '300', *extra], capture_output=True, text=True)
        return done, json.loads(done.stdout)

    def invalid(self, *extra):
        done, result = self.cli(*extra)
        self.assertEqual(result.get('overall'), 'INVALID', result)
        self.assertEqual(done.returncode, 1)

    def test_valid_control(self):
        done, result = self.cli()
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertEqual(result['verdict']['overall'], 'PASS')

    def test_missing_build_fields(self):
        for field in ('git_sha', 'cpg_cache_version', 'grammar_fingerprint', 'package_version', 'allocator'):
            with self.subTest(field=field):
                saved = {p: p.read_bytes() for p in self.out.glob('*/raw/tiny/*.json')}
                self.reports(lambda r: r['build'].pop(field, None))
                self.invalid()
                for p, data in saved.items(): p.write_bytes(data)

    def test_null_corpus_identity(self):
        self.corpus(lambda c: c.update(commit=None, dirty=None, workload=None))
        self.invalid()

    def test_empty_workload(self):
        self.corpus(lambda c: c.update(workload={}))
        self.invalid()

    def test_zero_requests_and_empty_hash(self):
        self.corpus(lambda c: c.update(workload={'requests': 0, 'sha256': ''}))
        self.invalid()

    def test_candidate_wrong_corpus(self):
        self.reports(lambda r: r['config'].update(repo_root='/wrong-corpus'), 'candidate')
        self.invalid()

    def test_build_receipt_mismatch(self):
        self.reports(lambda r: r['build'].update(git_sha='wrong'), 'candidate')
        self.invalid()

    def test_binary_digest_mismatch(self):
        (self.root / 'bin/candidate/prism-mcp').write_bytes(b'other binary')
        self.invalid()

    def test_settings_drift(self):
        self.reports(lambda r: r['config'].update(refresh_policy='auto-full'), 'candidate')
        self.invalid()

    def test_missing_timers(self):
        for side in ('base', 'candidate'):
            self.write_probe(side, [row for pid, hit in [(1, False), (2, True), (3, True)]
                for row in [{'pid': pid, 'kind': 'meta', 'repo': '/corpus', 'cpg_hit': hit},
                            {'pid': pid, 'kind': 'stage'}]])
        self.invalid()

    def test_bad_timer_values(self):
        path = self.out / 'candidate-tiny.jsonl'
        saved = path.read_text()
        for field, value in [('wall_ms', None), ('wall_ms', float('nan')),
                             ('wall_ms', float('inf')), ('wall_ms', -1),
                             ('cpu_ms', -1), ('cpu_ms', None), ('cpu_ms', float('nan'))]:
            with self.subTest(field=field, value=value):
                rows = [json.loads(line) for line in saved.splitlines()]
                next(r for r in rows if r['kind'] == 'stage')[field] = value
                self.write_probe('candidate', rows)
                self.invalid()
        path.write_text(saved)

    def test_save_warning_in_run_log(self):
        (self.out / 'candidate-tiny.log').write_text('nav edge cache: failed to save sidecar: disk full\n')
        self.invalid()

    def test_save_warning_in_prime(self):
        change(self.out / 'candidate-tiny/raw/tiny/prime.json',
               lambda r: r.update(stderr_tail='failed to save CPG cache: disk full'))
        self.invalid()

    def test_no_stderr_is_disclosed(self):
        done, result = self.cli()
        self.assertEqual(done.returncode, 0, done.stdout)
        self.assertIn('stderr absent', json.dumps(result))

    def test_explicit_receipts_and_rescore_preserve_raw_evidence(self):
        before = {p: p.read_bytes() for p in self.root.rglob('*') if p.is_file()}
        done, result = self.cli('--expect-receipts', str(self.root / 'binary-receipts.json'), '--rescore')
        self.assertEqual(done.returncode, 0, done.stdout)
        self.assertEqual(result['verdict']['overall'], 'PASS')
        self.assertEqual(before, {p: p.read_bytes() for p in self.root.rglob('*') if p.is_file()})

    def test_rescore_requires_after_inventory_but_live_runner_does_not(self):
        (self.out / 'after-inventory.json').unlink()
        (self.out / 'after-steps-inventory.json').unlink()
        self.invalid('--rescore')
        self.assertEqual(self.cli()[1]['verdict']['overall'], 'PASS')

    def test_missing_expected_receipts(self):
        (self.root / 'binary-receipts.json').unlink()
        self.invalid()

    def test_individually_unknown_corpus_fields(self):
        paths = list(self.out.glob('*/raw/tiny/corpus.json'))
        saved = {p: p.read_bytes() for p in paths}
        for key in ('commit', 'dirty', 'workload'):
            for value in (None, ''):
                with self.subTest(key=key, value=value):
                    self.corpus(lambda c: c.update({key: value}))
                    self.invalid()
                    for p, data in saved.items(): p.write_bytes(data)
        for workload in ({'sha256': self.digest, 'requests': 0},
                         {'sha256': '', 'requests': 1}):
            with self.subTest(workload=workload):
                self.corpus(lambda c: c.update(workload=workload))
                self.invalid()
                for p, data in saved.items(): p.write_bytes(data)

    def test_every_report_is_bound_to_receipts(self):
        path = self.out / 'candidate-tiny/raw/tiny/warm-2.json'
        saved = path.read_bytes()
        for key, value in [('git_sha', 'wrong'), ('cpg_cache_version', 120),
                           ('grammar_fingerprint', 'wrong'), ('build_identity', 'wrong')]:
            with self.subTest(key=key):
                change(path, lambda obj: obj['report']['build'].update({key: value}))
                self.invalid()
                path.write_bytes(saved)

    def test_each_required_timer_name(self):
        path = self.out / 'candidate-tiny.jsonl'
        saved = path.read_text()
        rows = [json.loads(line) for line in saved.splitlines()]
        for pid, required in [(1, PRIME), (2, WARM)]:
            for name in required:
                with self.subTest(pid=pid, name=name):
                    self.write_probe('candidate', [r for r in rows
                        if not (r['pid'] == pid and r.get('name') == name)])
                    self.invalid()
        path.write_text(saved)

    def test_unknown_timer_name_is_invalid(self):
        path = self.out / 'candidate-tiny.jsonl'
        rows = [json.loads(line) for line in path.read_text().splitlines()]
        next(r for r in rows if r['kind'] == 'stage')['name'] = ''
        self.write_probe('candidate', rows)
        self.invalid()

    def test_absent_cpu_supported_only_on_unsupported_platform(self):
        for side in ('base', 'candidate'):
            path = self.out / (side + '-tiny.jsonl')
            rows = [json.loads(line) for line in path.read_text().splitlines()]
            for row in rows:
                if row['kind'] == 'stage': row['cpu_ms'] = None
            self.write_probe(side, rows)
        self.invalid()
        self.reports(lambda r: r['platform'].update(os='unsupported'))
        done, result = self.cli()
        self.assertEqual(done.returncode, 0, done.stdout)
        self.assertEqual(result['verdict']['overall'], 'PASS')

    def test_cache_save_inventories(self):
        directory = self.out / 'candidate-tiny'
        path = directory / 'cache/tiny/cpg-cache-v2.bin'
        for content in (b'', b'changed'):
            with self.subTest(content=content):
                path.write_bytes(content)
                self.invalid()
        path.unlink()
        self.invalid()

    def test_retained_stderr_is_scanned(self):
        path = self.out / 'candidate-tiny/raw/tiny/prime.stderr'
        path.write_text('no warning\n')
        done, result = self.cli()
        self.assertEqual(done.returncode, 0, done.stdout)
        self.assertEqual(result['candidate']['save_evidence']['stderr'], 'stderr scanned')
        path.write_text('failed to save CPG cache: full\n')
        self.invalid()

    def test_missing_or_empty_driver_log(self):
        path = self.out / 'candidate-tiny.log'
        path.write_text('')
        self.invalid()
        path.unlink()
        self.invalid()

    def test_workload_snapshot_is_bound(self):
        (self.out / 'candidate-tiny/workloads/tiny.jsonl').write_bytes(b'changed\n')
        self.invalid()

    def test_inventory_and_manifest_are_bound(self):
        path = self.out / 'before-inventory.json'
        saved = path.read_bytes()
        mutations = [lambda r: r['manifest']['tiny'].update(requests=0),
                     lambda r: r['manifest']['tiny'].update(commit=None),
                     lambda r: r['workloads'].pop('MANIFEST.json'),
                     lambda r: r['corpora']['tiny'].update(status=' M file')]
        # Keep after equal: this tests admission, rather than just drift detection.
        for mutate in mutations:
            for p in self.out.glob('*inventory.json'):
                p.write_bytes(saved)
                change(p, mutate)
            self.invalid('--rescore')

    def test_after_inventory_drift(self):
        change(self.out / 'after-inventory.json', lambda r: r['corpora']['tiny'].update(commit='d' * 40))
        self.invalid('--rescore')

    def test_receipt_identity_fields(self):
        path = self.root / 'binary-receipts.json'
        saved = path.read_bytes()
        for key, value in [('source_sha', 'd' * 40), ('cache_version', 120),
                           ('navigation_sidecar_version', 65), ('profile', 'debug'),
                           ('rustc', ''), ('features', ['mcp', 'dfg-oracle'])]:
            with self.subTest(key=key):
                change(path, lambda r: r['candidate']['probe'].update({key: value}))
                self.invalid()
                path.write_bytes(saved)

    def test_settings_unknown_fields_are_compared(self):
        self.reports(lambda r: r['config'].update(future_setting='different'), 'candidate')
        self.invalid()

    def test_settings_warm_run_drift(self):
        path = self.out / 'candidate-tiny/raw/tiny/warm-2.json'
        change(path, lambda r: r['report']['config'].update(future_setting='drift'))
        self.invalid()

    def test_malformed_probe_rows(self):
        self.write_probe('candidate', [None])
        self.invalid()

    def test_null_raw_sample(self):
        write(self.out / 'candidate-tiny/raw/tiny/prime.json', None)
        self.invalid()

    def test_package_version_binds_to_preflight(self):
        self.reports(lambda r: r['build'].update(package_version='unreviewed'))
        self.invalid()

    def test_preflight_binding_is_required(self):
        path = self.root / 'preflight-result.json'
        change(path, lambda r: r['sides']['candidate'].update(source_sha='d' * 40))
        self.invalid()
        path.unlink()
        self.invalid()

    def test_save_log_for_dotted_directory(self):
        import evidence
        directory = self.root / 'run.with.dot'
        directory.mkdir()
        (self.root / 'run.with.dot.log').write_text('run complete')
        self.assertIn('stderr absent', evidence.save_evidence(directory, [], {})['stderr'])


if __name__ == '__main__':
    unittest.main()
