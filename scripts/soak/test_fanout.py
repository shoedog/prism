"""Tests of fanout.py: the pure parts, and whole runs against fake_prism_mcp.py.

Run: python3 -m unittest discover -s scripts/soak -p 'test_*.py'
"""
import hashlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import fanout  # noqa: E402

FAKE = HERE / 'fake_prism_mcp.py'


def dead_pid():
    """The pid of a process that has ended and been reaped."""
    child = subprocess.Popen(['true'])
    child.wait()
    return child.pid


def corpus_and_workload(root, requests=6):
    """A one-commit git corpus and a workload directory bound to it."""
    corpus = Path(root) / 'corpus'
    corpus.mkdir()
    (corpus / 'main.py').write_text('def run():\n    return 1\n')
    git = ['git', '-C', str(corpus), '-c', 'user.name=t', '-c', 'user.email=t@t']
    subprocess.run(git[:3] + ['init', '-q'], check=True)
    subprocess.run(git + ['add', '.'], check=True)
    subprocess.run(git + ['commit', '-qm', 'c'], check=True)
    commit = subprocess.run(git[:3] + ['rev-parse', 'HEAD'], check=True, capture_output=True, text=True).stdout.strip()
    lines = ''.join(json.dumps({'id': n, 'jsonrpc': '2.0', 'method': 'tools/call', 'params': {
        'name': ('nav_callers', 'nav_callees')[n % 2 == 0], 'arguments': {
            'seed': {'kind': 'symbol', 'name': 'f%d' % n}}}}) + '\n' for n in range(1, requests + 1)).encode()
    workloads = Path(root) / 'workloads'
    workloads.mkdir()
    (workloads / 'tiny.jsonl').write_bytes(lines)
    (workloads / 'MANIFEST.json').write_text(json.dumps({'tiny': {
        'commit': commit, 'requests': requests, 'sha256': hashlib.sha256(lines).hexdigest()}}))
    return corpus, workloads


class Pure(unittest.TestCase):
    def test_resident_clients_disable_idle_unload(self):
        for mode in ('shared', 'standalone'):
            argv = fanout.client_argv('/bin/prism-mcp', '/repo', '/cache', mode)
            self.assertEqual(argv.count('--index-idle-secs'), 1)
            self.assertEqual(argv[argv.index('--index-idle-secs') + 1], '0')

    def test_percentile_is_nearest_rank(self):
        values = list(range(1, 101))
        self.assertEqual(fanout.percentile(values, 0.50), 50)
        self.assertEqual(fanout.percentile(values, 0.95), 95)
        self.assertEqual(fanout.percentile(values, 0.99), 99)
        self.assertEqual(fanout.percentile([3, 1, 2], 0.95), 3)
        self.assertEqual(fanout.percentile([7], 0.5), 7)
        self.assertIsNone(fanout.percentile([], 0.5))

    def test_outcome_tells_the_four_answers_apart(self):
        self.assertEqual(fanout.outcome({'error': {'code': -32602}}), 'rpc_error')
        self.assertEqual(fanout.outcome({'result': {'isError': True, '_meta': {'prism/retryable': True}}}),
                         'retryable')
        self.assertEqual(fanout.outcome({'result': {'isError': True}}), 'tool_error')
        self.assertEqual(fanout.outcome({'result': {'isError': False}}), 'ok')

    def test_each_client_starts_its_own_share_of_the_way_through(self):
        items = list(range(10))
        self.assertEqual([fanout.rotated(items, c, 3)[0] for c in range(3)], [0, 3, 6])
        self.assertEqual(sorted(fanout.rotated(items, 2, 3)), items)
        self.assertEqual([fanout.rotated(list(range(200)), c, 8)[0] for c in range(8)],
                         [0, 25, 50, 75, 100, 125, 150, 175])


def record(ms, outcome='ok', refresh=False):
    return {'client': 0, 'seq': 0, 'tool': 't', 'refresh': refresh, 'start': 0.0, 'ms': ms, 'outcome': outcome}


def run_info(mode='standalone', clients=1, requests=2, instances=None, failures=None, refresh_at=None):
    return {'mode': mode, 'clients': clients, 'workload': {'requests': requests, 'sha256': 'w'},
            'daemon_instances': instances if instances is not None else [], 'failures': failures or [],
            'corpus': {'commit': 'c', 'dirty': 0}, 'refresh_at': refresh_at, 'pressure': {'problems': []}}


class Summaries(unittest.TestCase):
    def test_a_complete_run_is_admissible_and_the_refresh_is_kept_apart(self):
        summary = fanout.summarize([record(1.0), record(3.0), record(50.0, refresh=True)], run_info())
        self.assertTrue(summary['admissible'], summary['problems'])
        self.assertEqual((summary['calls'], summary['p95_ms'], summary['refresh_ms']), (2, 3.0, 50.0))

    def test_a_retryable_or_failed_call_a_missing_call_or_two_daemons_is_not_admissible(self):
        self.assertFalse(fanout.summarize([record(1.0), record(1.0, 'retryable')], run_info())['admissible'])
        self.assertFalse(fanout.summarize([record(1.0), record(1.0, 'rpc_error')], run_info())['admissible'])
        self.assertFalse(fanout.summarize([record(1.0)], run_info())['admissible'])
        two = run_info('shared', instances=[(1, 7), (2, 7)])
        self.assertFalse(fanout.summarize([record(1.0), record(1.0)], two)['admissible'])
        one = run_info('shared', instances=[(1, 7)])
        self.assertTrue(fanout.summarize([record(1.0), record(1.0)], one)['admissible'])

    def test_a_tool_error_is_an_answer_not_a_failure(self):
        self.assertTrue(fanout.summarize([record(1.0), record(1.0, 'tool_error')], run_info())['admissible'])


def arm(p95, p99, admissible=True, sha='w'):
    run = run_info(refresh_at=0.5)
    run['workload']['sha256'] = sha
    return {'dir': 'd', 'run': run, 'summary': {'p50_ms': 1.0, 'p95_ms': p95, 'p99_ms': p99,
                                                 'admissible': admissible, 'problems': [] if admissible else ['x']}}


class Verdicts(unittest.TestCase):
    def test_the_rule_passes_at_its_bounds_and_fails_past_them(self):
        base = [arm(10.0, 20.0)]
        self.assertTrue(fanout.judge(base, [arm(11.0, 25.0)])['pass'])
        self.assertFalse(fanout.judge(base, [arm(11.01, 20.0)])['pass'])
        self.assertFalse(fanout.judge(base, [arm(10.0, 25.01)])['pass'])

    def test_medians_across_runs_are_compared(self):
        verdict = fanout.judge([arm(10.0, 20.0), arm(10.0, 20.0), arm(100.0, 200.0)],
                               [arm(10.5, 21.0), arm(10.5, 21.0), arm(1.0, 1.0)])
        self.assertTrue(verdict['pass'])
        self.assertEqual(verdict['figures']['baseline']['p95_ms'], 10.0)

    def test_an_inadmissible_run_or_another_workload_fails_the_comparison(self):
        self.assertFalse(fanout.judge([arm(10.0, 20.0)], [arm(1.0, 1.0, admissible=False)])['pass'])
        self.assertFalse(fanout.judge([arm(10.0, 20.0)], [arm(1.0, 1.0, sha='other')])['pass'])


class RunCase(unittest.TestCase):
    """Whole runs of fanout.py against fake_prism_mcp.py, in a directory of the test's own."""

    def setUp(self):
        self.root = tempfile.TemporaryDirectory()
        self.addCleanup(self.root.cleanup)
        self.corpus, self.workloads = corpus_and_workload(self.root.name)
        self.saved_env = dict(os.environ)
        self.addCleanup(lambda: (os.environ.clear(), os.environ.update(self.saved_env)))
        self.saved_retry = fanout.WARMUP_RETRY_SECS
        fanout.WARMUP_RETRY_SECS = 0.01
        self.addCleanup(lambda: setattr(fanout, 'WARMUP_RETRY_SECS', self.saved_retry))
        self.saved_pressure = fanout.read_pressure
        fanout.read_pressure = lambda: 1
        self.addCleanup(lambda: setattr(fanout, 'read_pressure', self.saved_pressure))

    def run_fanout(self, name, mode, clients=3, extra=()):
        out = Path(self.root.name) / name
        with redirect_stdout(io.StringIO()):
            fanout.main(['run', '--bin', str(FAKE), '--corpus', 'tiny=%s' % self.corpus, '--cache-dir',
                         str(Path(self.root.name) / 'cache'), '--mode', mode, '--out', str(out), '--clients',
                         str(clients), '--workloads', str(self.workloads), '--timeout', '60'] + list(extra))
        calls = [json.loads(line) for line in (out / 'calls.jsonl').read_text().splitlines()]
        return out, calls, json.loads((out / 'summary.json').read_text()), json.loads((out / 'run.json').read_text())


class Runs(RunCase):

    def test_every_client_makes_every_call_and_client_0_refreshes_once(self):
        # Three warming answers: with the extra warm-up call of the second tool, a warm-up that did not
        # retry would leave the first timed call answered retryable.
        os.environ['FAKE_WARMING'] = '3'
        _, calls, summary, run = self.run_fanout('standalone', 'standalone')
        self.assertTrue(summary['admissible'], summary['problems'])
        self.assertEqual(len([c for c in calls if not c['refresh']]), 18)
        refreshes = [c for c in calls if c['refresh']]
        self.assertEqual([(c['client'], c['seq'], c['outcome']) for c in refreshes], [(0, 3, 'ok')])
        self.assertEqual(run['server'], {'name': 'prism-mcp', 'version': 'fake'})
        self.assertEqual(run['exits'], [0, 0, 0])

    def test_no_refresh_means_none(self):
        _, calls, summary, _ = self.run_fanout('quiet', 'standalone', extra=['--no-refresh'])
        self.assertEqual([c for c in calls if c['refresh']], [])
        self.assertIsNone(summary['refresh_ms'])

    def test_a_shared_run_names_its_one_daemon_and_asks_it_to_shut_down(self):
        os.environ['FAKE_DAEMON_PID'] = str(dead_pid())
        _, _, summary, run = self.run_fanout('shared', 'shared')
        self.assertTrue(summary['admissible'], summary['problems'])
        self.assertEqual(run['daemon_instances'], [[int(os.environ['FAKE_DAEMON_PID']), 7]])
        self.assertEqual(run['shutdown']['exit'], 0)
        self.assertIn('--shared', run['argv'])

    def test_a_client_that_dies_makes_the_run_inadmissible_without_hanging_it(self):
        os.environ['FAKE_DIE_AFTER'] = '3'
        _, _, summary, run = self.run_fanout('dies', 'standalone', clients=2)
        self.assertFalse(summary['admissible'])
        self.assertTrue(run['failures'])

    def test_clients_start_from_the_default_output_settings(self):
        os.environ['PRISM_MCP_CONCISE_SHAPE'] = 'legacy'
        out, _, _, _ = self.run_fanout('settings', 'standalone', clients=1)
        self.assertIn('PRISM_MCP_CONCISE_SHAPE=<unset>', (out / 'clients' / '0.stderr').read_text())

    def test_clients_that_die_while_warming_up_end_the_run(self):
        out = Path(self.root.name) / 'warmup-death'
        env = dict(os.environ, FAKE_WARMING='5', FAKE_DIE_AFTER='1')
        done = subprocess.run([sys.executable, str(HERE / 'fanout.py'), 'run', '--bin', str(FAKE), '--corpus',
                               'tiny=%s' % self.corpus, '--cache-dir', str(Path(self.root.name) / 'cache'),
                               '--mode', 'standalone', '--out', str(out), '--clients', '2', '--workloads',
                               str(self.workloads), '--timeout', '600'],
                              env=env, capture_output=True, text=True, timeout=60)
        self.assertEqual(done.returncode, 0, done.stderr)
        summary = json.loads((out / 'summary.json').read_text())
        self.assertFalse(summary['admissible'])
        self.assertEqual(summary['calls'], 0)

    def test_a_used_run_directory_is_refused(self):
        self.run_fanout('used', 'standalone', clients=1)
        with self.assertRaises(SystemExit):
            self.run_fanout('used', 'standalone', clients=1)


if __name__ == '__main__':
    unittest.main()
