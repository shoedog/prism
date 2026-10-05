"""R2 declaration identity, priming floors, recovery, and environment controls."""
import hashlib
from pathlib import Path
from types import SimpleNamespace as NS

import pytest

from tier_a.model import FunctionDef, Location
from tier_a.oracles import OracleError
from tier_a.tsserver import TsserverOracle


@pytest.mark.parametrize('shape,consumer', [(s,c) for s in ('field','static_field','named_property','overload')
                                          for c in ('callers','members') if s!='overload' or c=='callers'])
def test_native_declaration_binding_and_distinct_declaration_negatives(tmp_path, shape, consumer):
    if shape == 'overload':
        declaration = 'function run(x: string): string; function run(x: number): number; function run(x: any) { return x; }'
        source = declaration + '\n' + '\n'.join(
            f'namespace {owner} {{ export function run(x: number) {{ return x; }} }}'
            for owner in ('Base', 'Sibling', 'Override'))
        calls = ['run(2)', 'Base.run(2)', 'Sibling.run(2)', 'Override.run(2)']
    else:
        source = '\n'.join(
            (f'export const {owner} = {{ run: function run() {{ return 1; }} }};' if shape == 'named_property' else
             f'export class {owner}' + (' extends Target' if owner == 'Override' else '') +
             ' { ' + ('static ' if shape == 'static_field' else '') + 'run = () => 1; }')
            for owner in ('Target', 'Base', 'Sibling', 'Override'))
        calls = [f'{owner}.run()' if shape in ('static_field', 'named_property') else f'new {owner}().run()'
                 for owner in ('Target', 'Base', 'Sibling', 'Override')]
    source += '\nexport function use() {\n' + '\n'.join(c + ';' for c in calls) + '\n}\n'
    (tmp_path/'a.ts').write_text(source)
    o = TsserverOracle(['tsserver', '--disableAutomaticTypingAcquisition'], str(tmp_path), 'ts')
    from tier_a.member_sample import syntax_census, sample_members
    try:
        o.start()
        census = syntax_census(o, ['a.ts'])
        first_line = source.splitlines()[0]
        seed = FunctionDef('run', 'function', 'Target', Location('a.ts', 1, 1), 1, first_line.index('run'))
        sites = [s for s in census['sites'] if s['name'] == 'run']
        raw = [o.raw_definitions_at(s['file'], s['line'], s['character']) for s in sites]
        assert all(raw), 'native definition availability control'
        # One native family response; actual tsserver definition answers decide binding.
        spans = [{'start': {'line': s['line'], 'offset': s['character']+1},
                  'end': {'line': s['line'], 'offset': s['character']+4}} for s in sites]
        o._query = lambda *a: [{'from': {'file': str(tmp_path/'a.ts'), 'name': 'use',
            'span': {'start': {'line': 5, 'offset': 1}, 'end': {'line': 11, 'offset': 1}}}, 'fromSpans': spans}]
        if consumer == 'callers':
            edges = o.callers(seed)
            assert [e.call_site.start_line for e in edges] == [sites[0]['line']]
            assert len(o.oracle_filtered) == 3
        else:
            # Restrict the concrete frame to Target: Base/sibling/override stay excluded.
            census['declarations'] = [d for d in census['declarations'] if d['line'] == 1]
            result = sample_members(o, NS(callers=lambda *a: []), [seed], census, 100, 42)['shapes']['property_callable']
            assert result['missing'] == 1
            assert result['exclusions'] == {'nonconcrete_or_unsupported_definition': 3}
    finally:
        o.stop()


@pytest.mark.parametrize('errors,invalid', [(1, False), (4, True)])
def test_rust_priming_reuses_per_probe_error_floor(monkeypatch, tmp_path, errors, invalid):
    from test_r1_repair import _runner
    run = _runner(monkeypatch, tmp_path, prime_errors=errors)
    assert run['meta']['oracle_error_rate'] == errors/32
    assert run['meta']['baseline_invalid'] is invalid
    assert len([p for p in run['probes'] if not p.startswith('_')]) == 32
    assert sum(p.get('outcome') == 'oracle_error' for pid,p in run['probes'].items() if not pid.startswith('_')) == errors
    assert 'oracle_unavailable' not in run['meta']['invalid_reasons']


def test_ancestor_node_modules_is_observed_and_invalidating(tmp_path):
    from tier_a.corpus import verify_source_manifest
    root = tmp_path/'parent'/'source'; root.mkdir(parents=True)
    (root/'a.ts').write_text('export function f() {}')
    manifest = tmp_path/'manifest'
    manifest.write_text(hashlib.sha256((root/'a.ts').read_bytes()).hexdigest()+'  a.ts\n')
    cfg = {'path': str(root), 'source_manifest': str(manifest),
           'source_manifest_sha256': hashlib.sha256(manifest.read_bytes()).hexdigest(), 'reject_node_modules': True}
    assert verify_source_manifest(cfg)[1] == []
    (root.parent/'node_modules').mkdir()
    assert any('oracle_node_modules_present:' in d and str(root.parent) in d for d in verify_source_manifest(cfg)[1])


@pytest.mark.parametrize('supported', [True, False])
def test_restart_rechecks_capability_before_priming(monkeypatch, supported):
    import time
    from tier_a import lsp_client
    from tier_a.oracles import LspOracle, OracleTimeout
    o = LspOracle(['fake'], '.', 'rust')
    fd = FunctionDef('f', 'function', None, Location('a.rs', 1, 1), 1)
    old = o.client; old.deadline = time.monotonic()+5; old.stop = lambda: None
    monkeypatch.setattr(lsp_client, 'LspClient', lambda *a,**k: NS(deadline=old.deadline))
    o.start = lambda: None; o.wait_ready = lambda: None; o.document_symbols = lambda *a: [fd]
    capability = []
    def cap():
        capability.append(o.client)
        return supported
    o.capability_probe = cap
    def callers(*a):
        if o.client is old:
            o.retries.append({'code': -32801})
            raise OracleTimeout('persistent cancellation')
        if not supported:
            raise OracleError('unsupported')
        return []
    o.callers = callers; o.callees = lambda *a: []
    if supported:
        assert o.prime_hierarchy([fd], ['a.rs'], [fd]) == {(fd,'callers'): [], (fd,'callees'): []}
    else:
        with pytest.raises(OracleError, match='capability'):
            o.prime_hierarchy([fd], ['a.rs'], [fd])
    assert capability == [o.client] and o.client is not old


def test_same_line_nested_initializer_is_a_distinct_binding(tmp_path):
    from tier_a.member_sample import syntax_census, sample_members
    source = 'export const target = {run: function run(){ return {run: function run(){}};}}; const sibling={run: function run(){}};\nexport function use(){\ntarget.run();\ntarget.run().run();\nsibling.run();\n}\n'
    (tmp_path/'a.ts').write_text(source)
    o = TsserverOracle(['tsserver', '--disableAutomaticTypingAcquisition'], str(tmp_path), 'ts')
    try:
        o.start(); census = syntax_census(o, ['a.ts'])
        outer = census['declarations'][0]
        census['declarations'] = [outer]
        result = sample_members(o, NS(callers=lambda *a: []), [], census, 100, 42)['shapes']['property_callable']
        # Two outer calls bind; the nested and sibling function names on line 1 do not.
        assert result['missing'] == 2
        assert result['exclusions'] == {'nonconcrete_or_unsupported_definition': 2}
    finally:
        o.stop()


def test_environment_records_parent_search(monkeypatch, tmp_path):
    from test_r1_repair import _runner
    root = tmp_path/'source'; root.mkdir(); (tmp_path/'node_modules').mkdir()
    run = _runner(monkeypatch, root, lang='ts')
    environment = run['meta']['oracle_environment']['node_modules']
    assert environment['present'] == [str(tmp_path/'node_modules')]
    assert str(root/'node_modules') in environment['search_paths']


def test_report_records_recovery_counts(monkeypatch, tmp_path):
    from test_r1_repair import _runner
    from tier_a.report import render_markdown
    run = _runner(monkeypatch, tmp_path)
    run['meta']['oracle_retries'] = [{}, {}]; run['meta']['oracle_restarts'] = [{}]
    assert 'oracle retries: 2 · restarts: 1' in render_markdown(run)
