"""fanout.py against a real prism-mcp, on a three-file repository: three shims of one daemon, then
three standalone servers. Skipped unless PRISM_SOAK_BIN names a binary built with --features mcp:

    PRISM_SOAK_BIN=target/debug/prism-mcp python3 -m unittest discover -s scripts/soak -p 'test_*.py'
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

BIN = os.environ.get('PRISM_SOAK_BIN')


def call(n, name, symbol, file):
    return {'id': n, 'jsonrpc': '2.0', 'method': 'tools/call',
            'params': {'name': name, 'arguments': {'seed': {'kind': 'symbol', 'name': symbol, 'file': file}}}}


@unittest.skipUnless(BIN, 'set PRISM_SOAK_BIN to a prism-mcp binary to run the live fan-out test')
class Live(unittest.TestCase):
    def test_shared_clients_reach_one_daemon_and_leave_none_behind(self):
        with tempfile.TemporaryDirectory() as root:
            repo = Path(root) / 'live'
            repo.mkdir()
            (repo / 'util.py').write_text('def helper():\n    return 1\n')
            (repo / 'main.py').write_text('from util import helper\n\ndef run():\n    return helper()\n')
            git = ['git', '-C', str(repo), '-c', 'user.name=t', '-c', 'user.email=t@t']
            subprocess.run(git[:3] + ['init', '-q'], check=True)
            subprocess.run(git + ['add', '.'], check=True)
            subprocess.run(git + ['commit', '-qm', 'c'], check=True)
            commit = subprocess.run(git[:3] + ['rev-parse', 'HEAD'], check=True, capture_output=True,
                                    text=True).stdout.strip()
            requests = [call(1, 'nav_callers', 'helper', 'util.py'), call(2, 'nav_callees', 'run', 'main.py'),
                        call(3, 'nav_ego_graph', 'run', 'main.py'), call(4, 'nav_callers', 'run', 'main.py')]
            data = ''.join(json.dumps(r) + '\n' for r in requests).encode()
            workloads = Path(root) / 'workloads'
            workloads.mkdir()
            (workloads / 'live.jsonl').write_bytes(data)
            (workloads / 'MANIFEST.json').write_text(json.dumps({'live': {
                'commit': commit, 'requests': 4, 'sha256': hashlib.sha256(data).hexdigest()}}))
            runs = {}
            for mode in ('shared', 'standalone'):
                out = Path(root) / mode
                with redirect_stdout(io.StringIO()):
                    fanout.main(['run', '--bin', BIN, '--corpus', 'live=%s' % repo, '--cache-dir',
                                 str(Path(root) / 'cache'), '--mode', mode, '--out', str(out), '--clients', '3',
                                 '--workloads', str(workloads), '--timeout', '300'])
                runs[mode] = (json.loads((out / 'run.json').read_text()),
                              json.loads((out / 'summary.json').read_text()))
            shared_run, shared = runs['shared']
            self.assertTrue(shared['admissible'], shared['problems'])
            self.assertEqual(shared['outcomes'], {'ok': 12})
            self.assertEqual((shared['profile'], shared['refresh_outcome']), ('gate', 'ok'))
            self.assertEqual(shared['refresh_overlap']['refresh_ms'], shared['refresh_ms'])
            self.assertEqual(len(shared_run['daemon_instances']), 1)
            self.assertEqual((shared_run['shutdown']['exit'], shared_run['shutdown']['terminated']), (0, []))
            self.assertEqual(shared_run['exits'], [0, 0, 0])
            standalone_run, standalone = runs['standalone']
            self.assertTrue(standalone['admissible'], standalone['problems'])
            self.assertEqual(standalone['outcomes'], {'ok': 12})
            self.assertEqual(standalone_run['daemon_instances'], [])
            daemon_pid = shared_run['daemon_instances'][0][0]
            self.assertFalse(fanout.alive(daemon_pid), 'the daemon %d is still running' % daemon_pid)
            with redirect_stdout(io.StringIO()):
                fanout.main(['compare', '--baseline', str(Path(root) / 'standalone'), '--candidate',
                             str(Path(root) / 'shared'), '--out', str(Path(root) / 'compare.md')])
            self.assertIn('**Verdict:', (Path(root) / 'compare.md').read_text())


if __name__ == '__main__':
    unittest.main()
