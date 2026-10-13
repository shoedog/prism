#!/usr/bin/env python3
"""Tests that every admitted repetition of a corpus was measured under the same conditions.

Run every test module with:
    python3 -m unittest discover -s scripts/memreport -p 'test_*.py'
"""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport  # noqa: E402
from memreport_fixtures import canned  # noqa: E402

CORPUS = {'path': '/c', 'commit': 'abc', 'dirty': 0, 'cache_files': {}, 'workload': None}


def warm(rep, **changes):
    """An admitted warm result whose report is changed: threads=, env=, allocator=, platform=."""
    data = canned('warm')
    for key, value in changes.items():
        if key == 'allocator':
            data['build']['allocator'] = value
        else:
            data[key] = value
    return {'kind': 'warm', 'rep': rep, 'status': 'ok', 'report': data}


class WithinACorpusTests(unittest.TestCase):
    def second_is_excluded(self, **changes):
        entry = memreport.build_entry(CORPUS, [warm(1), warm(2, **changes)])
        return entry, [item['reason'] for item in entry['inadmissible']]

    def test_a_repetition_with_another_thread_count_is_inadmissible_and_named(self):
        entry, reasons = self.second_is_excluded(threads=8)
        self.assertEqual(entry['admissible']['warm'], 1)
        self.assertEqual(reasons, ["threads differs from this corpus's first admitted run"])

    def test_a_repetition_with_another_environment_is_inadmissible_and_named(self):
        entry, reasons = self.second_is_excluded(env={'PRISM_X': '1'})
        self.assertEqual((entry['admissible']['warm'], reasons),
                         (1, ["environment differs from this corpus's first admitted run"]))

    def test_a_repetition_with_another_allocator_is_inadmissible_and_named(self):
        entry, reasons = self.second_is_excluded(allocator='jemalloc')
        self.assertEqual((entry['admissible']['warm'], reasons),
                         (1, ["allocator differs from this corpus's first admitted run"]))

    def test_a_repetition_on_another_platform_is_inadmissible_and_named(self):
        for platform in ({'os': 'linux', 'arch': 'aarch64'}, {'os': 'macos', 'arch': 'x86_64'}):
            entry, reasons = self.second_is_excluded(platform=platform)
            self.assertEqual((entry['admissible']['warm'], reasons),
                             (1, ["platform differs from this corpus's first admitted run"]), platform)

    def test_the_first_admitted_run_decides_whatever_the_order_of_the_rest(self):
        entry = memreport.build_entry(CORPUS, [warm(1, threads=8), warm(2), warm(3, threads=8)])
        self.assertEqual((entry['admissible']['warm'], entry['threads']), (2, 8))

    def test_a_repetition_equal_in_every_setting_is_kept(self):
        entry = memreport.build_entry(CORPUS, [warm(1), warm(2), warm(3)])
        self.assertEqual(entry['admissible']['warm'], 3)

    def test_the_settings_are_checked_across_kinds_too(self):
        cold = {'kind': 'cold', 'rep': 1, 'status': 'ok', 'report': canned('cold')}
        cold['report']['threads'] = 8
        entry = memreport.build_entry(CORPUS, [warm(1), cold])
        self.assertEqual(entry['admissible']['cold'], 0)

    def test_the_platform_is_carried_into_the_entry_and_the_summary_provenance(self):
        entry = memreport.build_entry(CORPUS, [warm(1)])
        self.assertEqual(entry['platform'], {'os': 'macos', 'arch': 'aarch64'})
        summary = memreport.assemble_summary({'reps': 1}, {'a': entry})
        self.assertEqual(summary['provenance']['platform'], {'os': 'macos', 'arch': 'aarch64'})

    def test_a_report_without_a_platform_has_an_unknown_one(self):
        data = warm(1)
        del data['report']['platform']
        self.assertIsNone(memreport.build_entry(CORPUS, [data])['platform'])
        data['report']['platform'] = {'os': 'macos'}    # no arch: not known well enough
        self.assertIsNone(memreport.build_entry(CORPUS, [data])['platform'])

    def test_an_entry_without_admitted_runs_has_no_platform(self):
        entry = memreport.build_entry(CORPUS, [{'kind': 'warm', 'rep': 1, 'status': 'timeout', 'reason': 'timeout'}])
        self.assertIsNone(entry['platform'])


class BuildDirectoriesLeftTests(unittest.TestCase):
    def build(self, rep, **changes):
        data = canned('build')
        result = {'kind': 'build', 'rep': rep, 'status': 'ok', 'report': data,
                  'cache_files': {'cpg_cache_bytes': 1000, 'call_edge_sidecar_bytes': 100}}
        result.update(changes)
        return result

    def test_a_build_result_that_left_its_directory_behind_is_listed_admitted_or_not(self):
        entry = memreport.build_entry(CORPUS, [self.build(1), self.build(2, cache_dir_removed=False),
                                               self.build(3, cache_dir_removed=False, status='inadmissible',
                                                          reason='x'), self.build(4, cache_dir_removed=True)])
        self.assertEqual(entry['build_dirs_left'], [2, 3])

    def test_an_entry_with_nothing_left_has_an_empty_list(self):
        self.assertEqual(memreport.build_entry(CORPUS, [self.build(1)])['build_dirs_left'], [])
        self.assertEqual(memreport.build_entry(CORPUS, [warm(1)])['build_dirs_left'], [])


class HostLoadTests(unittest.TestCase):
    def test_only_admitted_runs_count_in_the_load_average_list(self):
        results = [warm(1), warm(2), warm(3)]
        for result, load in zip(results, (1.0, 99.0, 3.0)):
            result['host_before'] = {'loadavg': [load, 1.0, 1.0]}
        results[1]['report']['cache']['cpg_hit'] = False    # run 2 is excluded by the cache rule
        entry = memreport.build_entry(CORPUS, results)
        self.assertEqual(entry['load_avg_1m'], [1.0, 3.0])
        self.assertEqual(len(entry['inadmissible']), 1)

    def test_a_missing_or_malformed_load_average_is_not_counted(self):
        results = [warm(1), warm(2), warm(3), warm(4)]
        results[0]['host_before'] = {'loadavg': None}
        results[1]['host_before'] = {'loadavg': []}
        results[2]['host_before'] = {'loadavg': ['x', 1, 1]}
        entry = memreport.build_entry(CORPUS, results)
        self.assertEqual(entry['load_avg_1m'], [])


if __name__ == '__main__':
    unittest.main()
