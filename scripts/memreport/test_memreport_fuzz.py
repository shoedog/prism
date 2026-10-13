#!/usr/bin/env python3
"""Structural safety of what `validate` accepts: nothing it accepts can break the rest of the driver.

`summarize` reads reports from disk, so a report the binary's typed serializer could never print can still
arrive. Every single-field corruption of a canned report is tried; whatever `validate` still accepts must go
through derive, build_entry, assemble_summary and render without raising and without a non-finite number.

Run every test module with:
    python3 -m unittest discover -s scripts/memreport -p 'test_*.py'
"""
import copy
import json
import math
import re
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memreport  # noqa: E402
from memreport_fixtures import canned, stage_named  # noqa: E402

JUNK = (None, 'x', True, False, [], {}, -1, 1.5, float('inf'), float('-inf'), float('nan'), 10 ** 400, 2 ** 64)
DELETE = object()
DIGEST = {'sha256': 'f' * 64, 'requests': 8}
CORPUS = {'path': '/c', 'commit': 'abc', 'dirty': 0, 'workload': DIGEST,
          'cache_files': {'cpg_cache_bytes': 7, 'call_edge_sidecar_bytes': None}}
KINDS = memreport.KINDS + (memreport.BUILD, memreport.PRIME)


def paths(node, prefix=()):
    """Every path (a tuple of dict keys and list indices) below `node`, containers before their members."""
    if isinstance(node, dict):
        items = node.items()
    elif isinstance(node, list):
        items = enumerate(node)
    else:
        return
    for key, child in items:
        yield prefix + (key,)
        yield from paths(child, prefix + (key,))


def corrupted(base, path, junk):
    """A copy of `base` with the value at `path` replaced by `junk`, or removed when `junk` is DELETE."""
    data = copy.deepcopy(base)
    parent = data
    for key in path[:-1]:
        parent = parent[key]
    if junk is DELETE:
        del parent[path[-1]]          # a list element is removed, shifting the ones after it
    else:
        parent[path[-1]] = junk
    return data


def mutations(kind):
    base = canned('warm' if kind == memreport.PRIME else kind)
    for path in paths(base):
        for junk in JUNK + (DELETE,):
            yield '%s=%r' % ('.'.join(map(str, path)), 'DELETE' if junk is DELETE else junk), \
                corrupted(base, path, junk)


def non_finite(node):
    """The first non-finite float anywhere in `node`, or None."""
    if isinstance(node, float) and not math.isfinite(node):
        return node
    children = node.values() if isinstance(node, dict) else node if isinstance(node, (list, tuple)) else ()
    for child in children:
        found = non_finite(child)
        if found is not None:
            return found
    return None


def through_the_pipeline(kind, data):
    """derive, build_entry (with the report as its priming run too), assemble_summary and render."""
    kind_for_entry = 'warm' if kind == memreport.PRIME else kind
    prime = {'kind': memreport.PRIME, 'status': 'ok', 'report': data}
    result = {'kind': kind_for_entry, 'rep': 1, 'status': 'ok', 'report': data,
              'cache_files': {'cpg_cache_bytes': 7, 'call_edge_sidecar_bytes': 3}}
    entry = memreport.build_entry(CORPUS, [result], prime)
    summary = memreport.assemble_summary({'reps': 1, 'settle': 0, 'timeout': 1, 'bin': 'b'}, {'a': entry})
    text = memreport.render(summary)
    json.dumps(summary, allow_nan=False)          # raises on infinity or NaN anywhere in the summary
    return entry, summary, text


class AcceptedReportsAreSafeTests(unittest.TestCase):
    def test_every_report_validate_accepts_survives_the_whole_pipeline(self):
        for kind in KINDS:
            accepted = rejected = 0
            for label, data in mutations(kind):
                if memreport.validate(kind, data, DIGEST if kind == 'workload' else None) is not None:
                    rejected += 1
                    continue
                accepted += 1
                try:
                    if kind != memreport.PRIME:
                        memreport.derive(kind, data)
                    entry, summary, text = through_the_pipeline(kind, data)
                except Exception as error:  # noqa: BLE001
                    self.fail('%s: %r on an accepted report with %s' % (kind, error, label))
                self.assertIsNone(non_finite(summary), '%s: %s' % (kind, label))
                self.assertIsNone(re.search(r'\b(nan|inf|infinity)\b', text, re.I), '%s: %s' % (kind, label))
            # The corruptions are not all rejected (so the pipeline really ran) and not all accepted.
            self.assertGreater(accepted, 20, kind)
            self.assertGreater(rejected, 100, kind)

    def test_the_mutations_include_junk_in_the_elements_of_stages(self):
        labels = [label for label, _ in mutations('warm')]
        self.assertIn('stages.0=None', labels)
        self.assertIn("stages.0='x'", labels)
        self.assertIn('stages.0={}', labels)
        self.assertIn("stages.1='DELETE'", labels)


class NamedShapesTests(unittest.TestCase):
    """The shapes the first fuzz test missed, each stated outright."""

    def rejected(self, kind, data):
        reason = memreport.validate(kind, data)
        self.assertIsNotNone(reason)
        return reason

    def test_junk_ahead_of_a_required_stage_is_rejected(self):
        for junk in (None, {}, 'x', 7, [], {'name': 5}):
            for kind in ('warm', 'cold', 'refresh', 'workload'):
                data = canned(kind)
                data['stages'].insert(0, junk)
                self.assertRegex(self.rejected(kind, data), r'^stages\[0\] is not an object with a string name$')

    def test_a_string_where_a_section_belongs_is_rejected(self):
        data = canned('workload')
        data['attribution']['source_bytes'] = 'x'
        self.assertIn('attribution.source_bytes', self.rejected('workload', data))
        for section in ('build', 'attribution', 'env', 'cache'):
            data = canned('workload')
            data[section] = 'x'
            self.assertIsNotNone(memreport.validate('workload', data), section)

    def test_the_number_1e309_is_infinity_and_is_rejected_in_every_counter(self):
        infinity = json.loads('1e309')
        self.assertEqual(infinity, float('inf'))      # Python reads it as infinity
        for kind, stage_name in (('warm', 'after_index'), ('cold', 'after_call_edges'),
                                 ('refresh', 'after_refresh'), ('workload', 'after_settle')):
            for field in ('peak_bytes', 'rss_bytes', 'phys_footprint_bytes'):
                data = canned(kind)
                stage_named(data, stage_name)[field] = infinity
                self.assertIn('%s.%s' % (stage_name, field), self.rejected(kind, data))
        data = canned('workload')
        data['attribution']['node_string_heap_bytes_estimate'] = infinity
        self.assertIn('attribution.node_string_heap_bytes_estimate', self.rejected('workload', data))
        data = canned('workload')
        data['ru_maxrss_bytes'] = infinity
        self.assertIn('ru_maxrss_bytes', self.rejected('workload', data))
        data = canned('workload')
        data['workload']['requests'] = infinity
        self.assertIn('workload.requests', self.rejected('workload', data))

    def test_counts_are_integers_from_zero_to_the_u64_maximum(self):
        for bad in (True, False, 1.0, -1, 2 ** 64, 10 ** 400):
            data = canned('workload')
            stage_named(data, 'after_settle')['peak_bytes'] = bad
            self.assertIsNotNone(memreport.validate('workload', data), bad)
        for good in (0, 1, 2 ** 64 - 1):
            data = canned('workload')
            stage_named(data, 'after_settle')['peak_bytes'] = good
            self.assertIsNone(memreport.validate('workload', data), good)

    def test_the_unique_ratio_is_a_finite_number_from_zero_to_one_or_null(self):
        for bad in ('x', True, -0.1, 1.5, float('inf'), float('nan'), []):
            data = canned('workload')
            data['attribution']['node_string_unique_ratio'] = bad
            self.assertIn('node_string_unique_ratio', self.rejected('workload', data), bad)
        for good in (None, 0, 0.4, 1, 1.0):
            data = canned('workload')
            data['attribution']['node_string_unique_ratio'] = good
            self.assertIsNone(memreport.validate('workload', data), good)

    def test_build_env_cache_and_threads_shapes(self):
        cases = [
            ('build', lambda d: d['build'].update(binary_input_dirty='no'), 'build.binary_input_dirty'),
            ('build', lambda d: d['build'].update(git_sha=5), 'build.git_sha'),
            ('build', lambda d: d['build'].pop('build_identity'), 'build.build_identity'),
            ('build', lambda d: d['build'].update(cpg_cache_version=-3), 'build.cpg_cache_version'),
            ('env', lambda d: d['env'].update(PRISM_A=1), 'env is not an object of strings'),
            ('cache', lambda d: d['cache'].update(cpg_hit='yes'), 'cache.cpg_hit'),
            ('cache', lambda d: d['cache'].pop('call_edges_loaded'), 'cache.call_edges_loaded'),
            ('threads', lambda d: d.update(threads=0), 'threads is 0'),
            ('threads', lambda d: d.update(threads=True), 'threads is True'),
            ('threads', lambda d: d.update(threads=2.0), 'threads is 2.0'),
            ('threads', lambda d: d.pop('threads'), 'threads is None'),
        ]
        for label, change, text in cases:
            data = canned('workload')
            change(data)
            self.assertIn(text, self.rejected('workload', data), label)

    def test_well_formed_applies_to_every_kind_including_the_priming_run(self):
        for kind in KINDS:
            data = canned('warm' if kind == memreport.PRIME else kind)
            data['stages'].insert(0, None)
            self.assertEqual(memreport.validate(kind, data), 'stages[0] is not an object with a string name', kind)

    def test_a_clean_report_of_every_kind_is_well_formed(self):
        for kind in memreport.KINDS:
            self.assertIsNone(memreport.well_formed(canned(kind)), kind)
        self.assertEqual(memreport.well_formed([]), 'report is not an object')


class PipelineSafetyOfLegitimateExtremesTests(unittest.TestCase):
    def test_the_largest_counters_the_binary_can_print_do_not_break_the_readout(self):
        data = canned('workload')
        for entry in data['stages']:
            entry.update(peak_bytes=2 ** 64 - 1, rss_bytes=2 ** 64 - 1, phys_footprint_bytes=2 ** 64 - 1,
                         tree_sitter_heap_bytes=2 ** 64 - 1)
        data['attribution']['node_string_heap_bytes_estimate'] = 2 ** 64 - 1
        self.assertIsNone(memreport.validate('workload', data))
        entry, summary, text = through_the_pipeline('workload', data)
        self.assertIsNone(non_finite(summary))
        self.assertIn('| M1 | a |', text)


if __name__ == '__main__':
    unittest.main()
