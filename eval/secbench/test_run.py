import copy
import unittest
try:
    from .run import classify, data_root, propose_break, summarize, workstream
except ImportError:
    from run import classify, data_root, propose_break, summarize, workstream


class AccountingTests(unittest.TestCase):
    def setUp(self):
        self.source = {'file': 'x.js', 'start_line': 1, 'end_line': 4, 'features': [],
                       'data_parameters': [{'line': 1, 'names': ['input'], 'start_byte': 11, 'end_byte': 16,
                                            'destructure': False, 'rest': False, 'default': False}]}
        self.sink = {'file': 'x.js', 'line': 3, 'value_names': ['input'], 'value_occurrences': [{'name': 'input', 'start_byte': 31, 'end_byte': 36}]}
        self.row = {'source': self.source, 'sink': self.sink}
        self.root = {'Variable': {'file': 'x.js', 'line': 1, 'path': 'input', 'access': 'def', 'start_byte': 11}}
        self.target = {'Variable': {'file': 'x.js', 'line': 3, 'path': 'input', 'access': 'use', 'start_byte': 31}}

    def witness(self, source=None, verdict='Reached', sink=None):
        return {'reasoning': {'reachability': 'Reached', 'per_sink': [
            {'sink': sink or self.target, 'sources': [{'source': source or self.root, 'reachability': verdict}]}]}}

    def test_data_parameter_witness(self):
        self.assertEqual(classify(self.row, self.witness(), {}, {})[0], 'traced')

    def test_unrelated_callback_does_not_credit_traced(self):
        callback = {'Variable': {**self.root['Variable'], 'path': 'callback', 'start_byte': 18}}
        outcome, detail = classify(self.row, self.witness(callback), {}, {})
        self.assertEqual(outcome, 'reached_function_only')
        self.assertEqual(len(detail['unrelated_reached_roots']), 1)

    def test_same_name_wrong_byte_and_other_sink_value_rejected(self):
        other = {'Variable': {**self.root['Variable'], 'start_byte': 90}}
        self.assertFalse(data_root(other['Variable'], self.source))
        sink = {'Variable': {**self.target['Variable'], 'path': 'unrelated'}}
        self.assertNotEqual(classify(self.row, self.witness(sink=sink), {}, {})[0], 'traced')
        wrong_occurrence = {'Variable': {**self.target['Variable'], 'start_byte': 100}}
        self.assertNotEqual(classify(self.row, self.witness(sink=wrong_occurrence), {}, {})[0], 'traced')

    def test_bad_schema_is_error_not_miss(self):
        with self.assertRaises(ValueError):
            classify(self.row, {}, {}, {})

    def test_unique_synthetic_use_and_member_collision(self):
        occurrence = self.sink['value_occurrences'][0]
        occurrence['line_occurrences'] = 1
        synthetic = {'Variable': {**self.target['Variable'], 'start_byte': 20, 'end_byte': 20}}
        self.assertEqual(classify(self.row, self.witness(sink=synthetic), {}, {})[0], 'traced')
        occurrence['line_occurrences'] = 2
        self.assertNotEqual(classify(self.row, self.witness(sink=synthetic), {}, {})[0], 'traced')
        occurrence['path'] = 'input.name'
        self.assertNotEqual(classify(self.row, self.witness(), {}, {})[0], 'traced')

    def test_unresolved_callee_null_symbol_is_not_a_crash(self):
        self.assertEqual(classify(self.row, self.witness(verdict='NotReached'),
                                  {'items': [{'symbol': None}]}, {})[0], 'reached_function_only')

    def test_partial_and_not_reached_are_distinct(self):
        row = copy.deepcopy(self.row)
        row['sink']['file'] = 'y.js'
        target = {'Variable': {**self.target['Variable'], 'file': 'y.js'}}
        self.assertEqual(classify(row, self.witness(verdict='BoundaryExited', sink=target), {}, {})[0], 'partial')
        self.assertEqual(classify(row, self.witness(verdict='NotReached', sink=target), {}, {})[0], 'not_reached')

    def test_first_break_ignores_unrelated_syntax_and_reached_lines(self):
        self.source['features'] = [{'category': 'C-dynamic-key', 'line': 2, 'text': 'other[k]'}]
        self.assertEqual(propose_break(self.row, {})['category'], 'unresolved')
        self.source['features'][0]['text'] = 'input[k]'
        self.assertEqual(propose_break(self.row, {})['category'], 'C-dynamic-key')
        frontier = {'items': [{'symbol': {'Variable': {'file': 'x.js', 'line': 2}}}]}
        self.assertEqual(propose_break(self.row, frontier)['category'], 'unresolved')

    def test_complex_binding_and_mapping(self):
        self.source['data_parameters'][0]['destructure'] = True
        self.source['data_parameters'][0]['text'] = '{input}'
        self.assertEqual(propose_break(self.row, {})['category'], 'B-destructure')
        parameter = self.source['data_parameters'][0]
        parameter.update(destructure=False, rest=True, text='...input')
        self.assertEqual(propose_break(self.row, {})['category'], 'B-rest-spread')
        parameter.update(rest=False, default=True, text="input='fallback'")
        self.assertEqual(propose_break(self.row, {})['category'], 'B-default')
        parameter['default'] = False
        self.assertEqual(propose_break(self.row, {})['category'], 'unresolved')
        for category, stream in [('B-rest-spread', 2), ('C-merge', 3), ('D-cjs', 6), ('G-sink-model', 6), ('H-callback/promise/event', 6), ('F', 5), ('E', 4), ('J', None)]:
            self.assertEqual(workstream(category), stream)

    def test_call_chain_attribution_uses_first_unvisited_callable(self):
        for file, name, category in [('y.js', 'helper', 'D-cjs'),
                                     ('x.js', '<anonymous>', 'H-callback/promise/event'),
                                     ('x.js', 'helper', 'B-argument')]:
            with self.subTest(category=category):
                row = copy.deepcopy(self.row)
                row['sink'].update(file=file, line=15)
                row['syntactic_path'] = [row['source'], {'file': file, 'name': name, 'start_line': 10, 'end_line': 20}]
                self.assertEqual(propose_break(row, {})['category'], category)
                visited = {'items': [{'symbol': {'Variable': {'file': file, 'line': 11}}}]}
                self.assertEqual(propose_break(row, visited)['category'], 'unresolved')

    def test_exclusions_stay_in_denominator_and_are_not_opportunities(self):
        rows = [{'class': 'code-injection', 'outcome': outcome} for outcome in ('gt_unavailable', 'acquisition_excluded')]
        rows.append({'class': 'code-injection', 'outcome': 'prism_error', 'first_break': {'category': 'prism_error'}})
        s = summarize(rows)
        self.assertEqual(s['by_class']['code-injection']['entries'], 3)
        self.assertEqual(s['first_break_opportunities'], {})
        self.assertEqual(s['conversion_lower_bound']['3'], 0)

    def test_syntax_presence_cannot_prove_payload_path_presence(self):
        rows = [{'class': 'redos', 'outcome': 'traced', 'census': {'package_destructure': False, 'package_rest_spread': True, 'diagnostic_files': 0}}]
        v = summarize(rows)['by_class']['redos']['payload_path_syntax']
        self.assertEqual(v['destructure'], {'absent': 1})
        self.assertEqual(v['rest_spread'], {'unknown': 1})
        rows[0]['census']['unparsed_files'] = ['invalid.js']
        self.assertEqual(summarize(rows)['by_class']['redos']['payload_path_syntax']['destructure'], {'unknown': 1})
        rows[0]['source'] = {'data_parameters': [{'rest': True}]}
        self.assertEqual(summarize(rows)['by_class']['redos']['payload_path_syntax']['rest_spread'], {'present': 1})
        rows[0]['adjudication'] = {'path_destructure': False, 'path_rest_spread': True}
        self.assertEqual(summarize(rows)['by_class']['redos']['payload_path_syntax']['rest_spread'], {'present': 1})


if __name__ == '__main__':
    unittest.main()
