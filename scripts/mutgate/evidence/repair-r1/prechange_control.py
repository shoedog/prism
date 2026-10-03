#!/usr/bin/env python3
"""Run final regressions against committed original code/data, then repaired code/data."""
import importlib.machinery
import importlib.util
import json
import unittest
from pathlib import Path

from scripts.mutgate import test_mutgate as tests

HERE = Path(__file__).resolve().parent
fixed, fixed_lane = tests.mutgate, tests.LANE
loader = importlib.machinery.SourceFileLoader('mutgate_original', str(HERE / 'base-mutgate.py.txt'))
spec = importlib.util.spec_from_loader(loader.name, loader)
original = importlib.util.module_from_spec(spec)
loader.exec_module(original)
gate = tests.GateRegressionTests.gate
totals = {}
for label, implementation, lane in [('original', original, HERE / 'base-lane.json'),
                                    ('repaired', fixed, fixed_lane)]:
    tests.mutgate, tests.LANE = implementation, lane
    receipts = []

    def record(self, *args, **kwargs):
        status, summary = gate(self, *args, **kwargs)
        receipts.append({'case': self._testMethodName, 'mode': args[2], 'status': status,
                         'source': args[0], 'mutations': args[1],
                         'extra': kwargs.get('extra') or (args[3] if len(args) > 3 else None),
                         'timeout': kwargs.get('timeout', 5), 'summary': summary})
        return status, summary

    tests.GateRegressionTests.gate = record
    with (HERE / ('unit-' + label + '-current.log')).open('w') as log:
        result = unittest.TextTestRunner(stream=log, verbosity=2).run(
            unittest.defaultTestLoader.loadTestsFromModule(tests))
    (HERE / ('regression-' + label + '.json')).write_text(json.dumps(receipts, indent=2))
    totals[label] = {'tests': result.testsRun, 'failures': len(result.failures),
                     'errors': len(result.errors), 'compiled_cases': len(receipts)}
    print(label, totals[label])
tests.mutgate, tests.LANE = fixed, fixed_lane
tests.GateRegressionTests.gate = gate
(HERE / 'prechange-totals.json').write_text(json.dumps(totals, indent=2))
assert totals['original']['tests'] == totals['repaired']['tests'] == 33
assert totals['original']['failures'] == 39 and totals['original']['errors'] == 0
assert totals['repaired']['failures'] == totals['repaired']['errors'] == 0
