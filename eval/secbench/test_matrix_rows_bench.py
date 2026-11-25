"""Well-formedness of the stage-T benchmark matrix rows (groups bench-findings and bench-negatives).

The rows are data for the S6a and S6b slices of the measurement kit; this test only checks that
the data is complete and consistent. It never runs a scorer, prism or any harness code.
"""
import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MATRIX = ROOT / 'scripts' / 'measure' / 'matrix'
ROWS = MATRIX / 'rows-bench.jsonl'
CASES = MATRIX / 'cases'

FIELDS = ['id', 'group', 'input', 'required', 'truth', 'source']
GROUPS = {'bench-findings': 'bf-', 'bench-negatives': 'bn-'}
SOURCES = {'probe', 'product', 'synthetic'}
LINE_HITS = {'sink_line', 'ambiguous', 'sink_file', 'elsewhere', 'none', 'error', 'not_scored'}
MATCHES = {'matched', 'ambiguous', 'off_target'}
REASONS = {
    'no_fixed_version', 'fix_commit_only', 'fixed_version_unparseable', 'fixed_not_newer', 'registry_missing',
    'metadata_mismatch', 'origin_refused', 'integrity_unsupported', 'integrity_mismatch', 'pin_drift',
    'limit_exceeded', 'extract_refused', 'gt_unavailable', 'no_vulnerable_positive', 'sink_function_unnamed',
    'function_not_found', 'function_moved', 'function_ambiguous', 'sink_occurrence_unmapped',
    'sink_occurrence_ambiguous', 'source_not_found', 'prism_error'}
COLUMNS = {'finding', 'reach', 'function_change', 'sink_removed'}


def load_rows():
    rows = []
    with ROWS.open() as handle:
        for number, line in enumerate(handle, 1):
            if line.strip():
                rows.append((number, json.loads(line)))
    return rows


def is_undetermined(value):
    return isinstance(value, dict) and list(value) == ['undetermined']


def files_under(path):
    return [p for p in sorted(path.rglob('*')) if p.is_file()]


class MatrixRowsBench(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.rows = load_rows()

    def test_rows_exist(self):
        self.assertTrue(self.rows)
        self.assertEqual({r['group'] for _, r in self.rows}, set(GROUPS))

    def test_every_line_has_exactly_the_six_fields(self):
        for number, row in self.rows:
            with self.subTest(line=number):
                self.assertIsInstance(row, dict)
                self.assertEqual(sorted(row), sorted(FIELDS))

    def test_ids_unique_and_match_group(self):
        seen = set()
        for _, row in self.rows:
            with self.subTest(id=row['id']):
                self.assertNotIn(row['id'], seen)
                seen.add(row['id'])
                self.assertIn(row['group'], GROUPS)
                self.assertRegex(row['id'], r'^%s\d{3}$' % GROUPS[row['group']])

    def test_field_types(self):
        for _, row in self.rows:
            with self.subTest(id=row['id']):
                self.assertIsInstance(row['truth'], dict)
                self.assertIsInstance(row['input'], dict)
                self.assertTrue(isinstance(row['required'], str) and row['required'].strip())
                self.assertIn(row['source'], SOURCES)

    def case_dir(self, row):
        name = row['input'].get('case')
        self.assertIsInstance(name, str, row['id'])
        self.assertFalse(Path(name).is_absolute(), row['id'])
        return ROOT / name

    def test_named_case_directories_exist_and_are_not_empty(self):
        for _, row in self.rows:
            with self.subTest(id=row['id']):
                case = self.case_dir(row)
                self.assertTrue(case.is_dir(), case)
                self.assertTrue(files_under(case), 'empty case %s' % case)
                self.assertEqual(case.name, row['id'])

    def test_every_case_directory_is_referenced(self):
        referenced = {self.case_dir(r).resolve() for _, r in self.rows}
        for case in sorted(CASES.glob('b[fn]-*')):
            with self.subTest(case=case.name):
                self.assertIn(case.resolve(), referenced)

    def test_every_json_file_parses(self):
        for case in sorted(CASES.glob('b[fn]-*')):
            for path in files_under(case):
                if path.suffix not in ('.json', '.jsonl'):
                    continue
                with self.subTest(path=str(path.relative_to(MATRIX))):
                    text = path.read_text()
                    self.assertTrue(text.strip(), 'empty json file')
                    if path.suffix == '.jsonl':
                        for line in text.splitlines():
                            if line.strip():
                                json.loads(line)
                    else:
                        json.loads(text)

    def test_cases_carry_no_machine_paths(self):
        for case in sorted(CASES.glob('b[fn]-*')):
            for path in files_under(case):
                with self.subTest(path=str(path.relative_to(MATRIX))):
                    text = path.read_text(errors='replace')
                    for needle in ('/Users/', '/private/', '/home/', '/tmp/'):
                        self.assertNotIn(needle, text)

    def test_every_input_file_named_by_a_row_exists(self):
        for _, row in self.rows:
            case = self.case_dir(row)
            for key, value in row['input'].items():
                if key == 'case':
                    continue
                with self.subTest(id=row['id'], key=key):
                    self.assertIsInstance(value, str)
                    # not every optional output exists (a failed channel has no taint output)
                    if key == 'taint_output':
                        continue
                    self.assertTrue((case / value).exists(), value)

    def test_product_rows_have_their_source_tree_and_raw_output(self):
        for _, row in self.rows:
            if row['source'] != 'product':
                continue
            case = self.case_dir(row)
            with self.subTest(id=row['id']):
                if row['group'] == 'bench-findings':
                    self.assertTrue(files_under(case / 'pkg'))
                    self.assertTrue((case / 'seed.json').is_file())
                    record = json.loads((case / 'invocation.json').read_text())['taint']
                    self.assertIsNotNone(record)
                    if record['returncode'] == 0:
                        self.assertTrue((case / 'taint.json').is_file())
                else:
                    self.assertTrue(files_under(case / 'vuln'))
                    self.assertTrue(files_under(case / 'fixed'))
                    raw = [p for p in files_under(case / 'raw') if p.stat().st_size > 0]
                    self.assertTrue(raw)

    def test_bench_findings_truth_shape_and_consistency(self):
        for _, row in self.rows:
            if row['group'] != 'bench-findings':
                continue
            truth = row['truth']
            with self.subTest(id=row['id']):
                self.assertEqual(sorted(truth), ['findings', 'line_hit', 'rules'])
                line_hit = truth['line_hit']
                findings = truth['findings']
                if is_undetermined(line_hit):
                    self.assertTrue(line_hit['undetermined'].strip())
                else:
                    self.assertIn(line_hit, LINE_HITS)
                if is_undetermined(findings):
                    self.assertTrue(findings['undetermined'].strip())
                    findings = None
                self.assertIsInstance(truth['rules'], list)
                self.assert_rule_refs(truth['rules'])
                case = self.case_dir(row)
                taint = case / 'taint.json'
                emitted_findings = []
                if taint.is_file():
                    emitted_findings = [f for f in json.loads(taint.read_text())['findings']
                                        if f.get('category') == 'taint_sink']
                    for finding in emitted_findings:
                        self.assert_rule_refs(finding.get('rules', []))
                if findings is None:
                    self.assertEqual(line_hit, 'sink_line')
                    continue
                matched = 0
                for finding in findings:
                    self.assertEqual(sorted(finding), ['file', 'line', 'match'])
                    self.assertIn(finding['match'], MATCHES)
                    matched += finding['match'] == 'matched'
                self.assertLessEqual(matched, 1)
                if not is_undetermined(line_hit):
                    self.assertEqual(line_hit == 'sink_line', matched == 1)
                    if line_hit in ('error', 'not_scored', 'none'):
                        self.assertEqual(findings, [])
                else:
                    self.assertEqual(matched, 0)
                emitted = sorted((f['file'], f['line']) for f in emitted_findings)
                self.assertEqual(emitted, sorted((f['file'], f['line']) for f in findings))
                if not taint.is_file():
                    self.assertEqual(findings, [])
                # rules come from the matched finding's RuleRef list, never from other findings
                matched_rows = [f for f in findings if f['match'] == 'matched']
                expected_rules = []
                for finding in matched_rows:
                    for source in emitted_findings:
                        if (source['file'], source['line']) == (finding['file'], finding['line']):
                            expected_rules = source.get('rules', [])
                if not is_undetermined(line_hit):
                    self.assertEqual(truth['rules'], expected_rules)

    def assert_rule_refs(self, rules):
        """Lane 7 shape: RuleRef objects {id, cwe?, severity?}, sorted by id, at most one per id."""
        self.assertIsInstance(rules, list)
        for rule in rules:
            self.assertIsInstance(rule, dict)
            self.assertIn('id', rule)
            self.assertIsInstance(rule['id'], str)
            self.assertTrue(set(rule) <= {'id', 'cwe', 'severity'}, rule)
            if 'cwe' in rule:
                self.assertRegex(rule['cwe'], r'^CWE-\d+$')
        ids = [rule['id'] for rule in rules]
        self.assertEqual(ids, sorted(set(ids)))

    def test_bench_negatives_truth_shape(self):
        for _, row in self.rows:
            if row['group'] != 'bench-negatives':
                continue
            truth = row['truth']
            with self.subTest(id=row['id']):
                if 'undetermined' in truth:
                    self.assertEqual(list(truth), ['undetermined'])
                    self.assertTrue(truth['undetermined'].strip())
                elif truth['outcome'] == 'unmet':
                    self.assertEqual(truth['verified_pairs'], 0)
                    self.assertTrue(set(truth['per_entry'].values()) <= REASONS)
                else:
                    self.assertEqual(sorted(truth), ['columns', 'outcome'])
                    self.assertEqual(set(truth['columns']), COLUMNS)
                    columns = truth['columns']
                    if truth['outcome'] == 'verified_pair':
                        self.assertIn(columns['function_change'], ('unchanged', 'changed'))
                        if columns['sink_removed']:
                            self.assertIsNone(columns['finding'])
                            self.assertIsNone(columns['reach'])
                        else:
                            self.assertIsInstance(columns['finding'], bool)
                            self.assertIsInstance(columns['reach'], bool)
                            self.assertIs(columns['sink_removed'], False)
                    else:
                        self.assertIn(truth['outcome'], REASONS)
                        self.assertEqual(set(columns.values()), {None})

    def test_bench_negatives_case_files(self):
        for _, row in self.rows:
            if row['group'] != 'bench-negatives':
                continue
            case = self.case_dir(row)
            with self.subTest(id=row['id']):
                if 'set' in row['input']:
                    listing = json.loads((case / 'set.json').read_text())
                    for entry in listing['entries']:
                        self.assertTrue((case / 'entries' / entry['entry'] / 'meta.json').is_file())
                        self.assertTrue((case / 'entries' / entry['entry'] / 'observations.json').is_file())
                    continue
                meta = json.loads((case / 'meta.json').read_text())
                self.assertIn('vulnerableVersion', meta)
                self.assertIn('sink', meta)
                observations = json.loads((case / 'observations.json').read_text())
                self.assertTrue('vulnerable' in observations or observations.get('gt_status') == 'gt_unavailable')
                if 'vulnerable_tree' in row['input']:
                    self.assertTrue((case / 'vuln' / 'package.json').is_file())
                    self.assertTrue((case / 'fixed' / 'package.json').is_file())

    # ---- derivation checks for the product bench-negatives rows (tests-only, reads committed raw output) ----

    @staticmethod
    def mapped_line(source_text, meta, fixed_text):
        """Line of the one occurrence of the annotated callee text in the fixed file, or None."""
        _, line, column = meta['sink'].rsplit(':', 2)
        line, column = int(line), int(column)
        callee = re.match(r'[\w.$]+', source_text.splitlines()[line - 1][column - 1:])
        if not callee:
            return None
        lines = [n for n, text in enumerate(fixed_text.splitlines(), 1)
                 for _ in re.findall(r'(?<![\w.$])%s\(' % re.escape(callee.group(0)), text)]
        return lines[0] if len(lines) == 1 else None

    @staticmethod
    def observed(raw_dir, side, file, line):
        taint = json.loads((raw_dir / (side + '-taint.json')).read_text())
        frontier = json.loads((raw_dir / (side + '-frontier.json')).read_text())
        finding = any(f.get('category') == 'taint_sink' and f['file'] == file and f['line'] == line
                      for f in taint['findings'])
        reach = any(i['location']['file'] == file and i['location']['start_line'] <= line <= i['location']['end_line']
                    for i in frontier['items'])
        return finding, reach

    def product_negative_rows(self):
        return [r for _, r in self.rows
                if r['group'] == 'bench-negatives' and r['source'] == 'product' and 'vulnerable_tree' in r['input']]

    def test_raw_query_files_hold_no_error_unless_documented(self):
        for _, row in self.rows:
            if row['group'] != 'bench-negatives':
                continue
            case = self.case_dir(row)
            for path in files_under(case / 'raw') if (case / 'raw').is_dir() else []:
                if path.suffix != '.json':
                    continue
                with self.subTest(id=row['id'], path=path.name):
                    if 'error' in json.loads(path.read_text()):
                        # an error is no evidence of absence of flow
                        self.assertTrue(is_undetermined(row['truth']) or 'RAW ERROR:' in row['required'],
                                        'raw query error without an undetermined truth or a documented reason')

    def test_product_negative_columns_are_derived_from_the_raw_output(self):
        rows = self.product_negative_rows()
        self.assertGreaterEqual(len(rows), 5)
        for row in rows:
            case = self.case_dir(row)
            raw = case / 'raw'
            with self.subTest(id=row['id']):
                meta = json.loads((case / 'meta.json').read_text())
                file = meta['sink'].rsplit(':', 2)[0]
                vuln_text = (case / 'vuln' / file).read_text()
                fixed_text = (case / 'fixed' / file).read_text()
                line = self.mapped_line(vuln_text, meta, fixed_text)
                self.assertIsNotNone(line, 'the callee text must occur once in the fixed file')
                finding, reach = self.observed(raw, 'fixed', file, line)
                truth = row['truth']
                if is_undetermined(truth):
                    expected = 'finding %s, reach %s' % (str(finding).lower(), str(reach).lower())
                    self.assertIn(expected, row['required'])
                else:
                    columns = truth['columns']
                    self.assertEqual(columns['finding'], finding)
                    self.assertEqual(columns['reach'], reach)

    def test_product_negative_vulnerable_observations_match_the_raw_output(self):
        for row in self.product_negative_rows():
            case = self.case_dir(row)
            if not (case / 'raw' / 'vuln-taint.json').is_file():
                continue
            with self.subTest(id=row['id']):
                meta = json.loads((case / 'meta.json').read_text())
                file, line, _ = meta['sink'].rsplit(':', 2)
                finding, reach = self.observed(case / 'raw', 'vuln', file, int(line))
                vulnerable = json.loads((case / 'observations.json').read_text())['vulnerable']
                self.assertEqual(vulnerable['finding_line_hit'] == 'sink_line', finding)
                self.assertEqual(vulnerable['reach'] == 'traced', reach)

    def test_synthetic_stub_rows_keep_no_contradicting_raw_output(self):
        for _, row in self.rows:
            if row['group'] != 'bench-negatives' or row['source'] != 'synthetic':
                continue
            case = self.case_dir(row)
            if (case / 'observations.json').is_file() and (case / 'raw').is_dir():
                with self.subTest(id=row['id']):
                    self.assertEqual([p.name for p in files_under(case / 'raw') if p.name.startswith('vuln-')], [])

    def test_set_level_row_does_not_store_the_expected_reasons_with_its_inputs(self):
        for _, row in self.rows:
            if 'set' not in row['input']:
                continue
            case = self.case_dir(row)
            listing = json.loads((case / 'set.json').read_text())
            ids = []
            for entry in listing['entries']:
                with self.subTest(id=row['id'], entry=entry['entry']):
                    self.assertEqual(list(entry), ['entry'])
                    ids.append(json.loads((case / 'entries' / entry['entry'] / 'meta.json').read_text())['id'])
            self.assertEqual(len(ids), len(set(ids)))

    def test_group_coverage(self):
        ids = {g: [r['id'] for _, r in self.rows if r['group'] == g] for g in GROUPS}
        products = [r for _, r in self.rows if r['group'] == 'bench-findings' and r['source'] == 'product']
        self.assertGreaterEqual(len(products), 6)
        self.assertTrue(ids['bench-findings'] and ids['bench-negatives'])
        hits = {r['truth']['line_hit'] for _, r in self.rows
                if r['group'] == 'bench-findings' and not is_undetermined(r['truth']['line_hit'])}
        self.assertEqual(hits, LINE_HITS)


if __name__ == '__main__':
    unittest.main()
