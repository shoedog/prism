"""Tests of review round 2's fold of soak.py (Opus W-D, W-E = sol 1, S-A, S-F): the week's window
survives a resume, a round gates only on its own checkouts, every client log must show a reached
daemon, and the report says how much time the samples covered.

Run: python3 -m unittest discover -s scripts/soak -p 'test_*.py'
"""
import io
import json
import os
import sys
import tempfile
import time
import unittest
from contextlib import redirect_stdout
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import soak  # noqa: E402
from test_soak import ALL_READ, FAILED, MIB, REACHED, proc, sample  # noqa: E402

DAY = 86400


def stamp(epoch):
    return time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(epoch))


class Window(unittest.TestCase):
    def test_a_resumed_sampler_keeps_the_window_it_started(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'T0'
            self.assertEqual(soak.open_window(path, 7 * DAY, now=1000.0), (1000, 1000 + 7 * DAY, False))
            self.assertEqual(soak.open_window(path, 7 * DAY, now=1000.0 + 3 * DAY), (1000, 1000 + 7 * DAY, True))
            self.assertEqual(path.read_text().strip(), '1000')

    def test_a_day_two_failure_and_earlier_usage_days_survive_a_resume(self):
        with tempfile.TemporaryDirectory() as directory:
            window = Path(directory) / 'T0'
            soak.open_window(window, 7 * DAY, now=0.0)
            soak.open_window(window, 7 * DAY, now=3 * DAY + 5.0)   # the sampler restarted on day 4
            samples = Path(directory) / 's.jsonl'
            lines = []
            for day in range(7):
                for k, cpu in enumerate((10.0 * day, 10.0 * day + 5.0)):
                    lines.append(json.dumps(sample(day * DAY + 60 * k, [proc('daemon', 1, 400 * MIB, cpu=cpu)])))
            samples.write_text('\n'.join(lines) + '\n')
            client = Path(directory) / 'bridge.log'
            client.write_text('%s 7 %s\n' % (stamp(1 * DAY + 60), FAILED))
            out, data = Path(directory) / 'r.md', Path(directory) / 'r.json'
            with redirect_stdout(io.StringIO()):
                soak.main(['report', '--phase', 'week', '--samples', str(samples), '--window-file', str(window),
                           '--until', str(7 * DAY), '--client-log', str(client), '--crash-reports', directory,
                           '--out', str(out), '--json', str(data)])
            report = json.loads(data.read_text())
            self.assertEqual(report['verdicts']['no_fail_closed'], 'FAIL')
            self.assertEqual(len(report['summary']['usage_days']), 7)


class Round(unittest.TestCase):
    def built(self, t, *more):
        return sample(t, [proc('daemon', 1, 300 * MIB, cpu=9.0), proc('shim', 2), proc('shim', 3)] + list(more))

    def test_only_standalone_servers_of_the_round_s_checkouts_are_gated(self):
        with tempfile.TemporaryDirectory() as directory:
            checkout = os.path.join(directory, 'checkout')
            os.mkdir(checkout)
            alias = os.path.join(directory, 'alias')
            os.symlink(checkout, alias)
            samples = [self.built(1), self.built(2, proc('standalone', 9, 500 * MIB, repo='/elsewhere'))]
            summary = soak.summarize_samples(samples)
            events = soak.scan_lines([REACHED])
            elsewhere = soak.judge(summary, events, [], ALL_READ, round_repos=[checkout])
            self.assertEqual(elsewhere['no_new_standalone_index'], 'PASS')
            inside = soak.summarize_samples([self.built(1), self.built(2, proc('standalone', 9, 500 * MIB, repo=alias))])
            self.assertEqual(soak.judge(inside, events, [], ALL_READ, round_repos=[checkout])['no_new_standalone_index'],
                             'FAIL')

    def test_a_round_needs_a_reached_daemon_in_every_client_log(self):
        summary = soak.summarize_samples([self.built(1)])
        events = soak.scan_lines([REACHED])
        verdicts = soak.judge(summary, events, [], ALL_READ, round_repos=['/r'])
        use = soak.usage('round', summary, events, 2, 5, sources_reached=[True, False])
        self.assertEqual(soak.overall('round', verdicts, use), 'INADMISSIBLE')
        use = soak.usage('round', summary, events, 2, 5, sources_reached=[True, True])
        self.assertEqual(soak.overall('round', verdicts, use), 'PASS')
        self.assertEqual(soak.overall('round', verdicts, soak.usage('round', summary, events, 2, 5, sources_reached=[])),
                         'INADMISSIBLE')

    def test_a_round_report_without_its_checkouts_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            samples = Path(directory) / 's.jsonl'
            samples.write_text(json.dumps(self.built(time.time())) + '\n')
            with redirect_stdout(io.StringIO()), self.assertRaises(SystemExit):
                soak.main(['report', '--phase', 'round', '--samples', str(samples), '--out',
                           str(Path(directory) / 'r.md')])


class Coverage(unittest.TestCase):
    def test_the_largest_gap_between_samples_and_the_time_covered_are_reported(self):
        summary = soak.summarize_samples([sample(0, []), sample(60, []), sample(3 * DAY, []), sample(3 * DAY + 60, [])])
        self.assertEqual((summary['largest_gap_secs'], summary['covered_secs']), (3 * DAY - 60, 3 * DAY + 60))


if __name__ == '__main__':
    unittest.main()
