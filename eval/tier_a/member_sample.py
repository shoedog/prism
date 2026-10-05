"""Definition-at-site measurement for properties/getters absent from call hierarchy.

The independent TypeScript syntax census chooses sites; native definitions decide
binding. Unsupported/abstract/external definitions remain explicit exclusions.
"""
from collections import Counter
import json
import random
from pathlib import Path
import shutil
import subprocess
import time

from .metrics import precision_recall
from .oracles import OracleError, OracleTimeout
from .sut import SutError, SutTimeout


def syntax_census(oracle, files):
    executable = shutil.which(oracle._cmd[0])
    if not executable:
        raise OracleError("tsserver executable unavailable for syntax census")
    compiler = Path(executable).resolve().parents[1] / 'lib/typescript.js'
    timeout = min(60, oracle.client.deadline - time.monotonic())
    if timeout <= 0:
        raise OracleTimeout("syntax census: oracle session budget exhausted")
    try:
        p = subprocess.run(['node', str(Path(__file__).with_name('ts_syntax.cjs')),
                            str(compiler), oracle.root], input=json.dumps(files),
                           capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired as exc:
        raise OracleTimeout("syntax census timed out") from exc
    if p.returncode:
        raise OracleError('syntax census failed: ' + p.stderr)
    return json.loads(p.stdout)


def definition_matches(declaration, definition):
    """Same declaration, including its initializer or contiguous overload names.

    Position containment alone would accept a nested same-name declaration.
    Older census rows carry only the exact name token, which remains supported.
    """
    point = {'line': definition['start']['line'], 'character': definition['start']['offset']-1}
    positions = declaration.get('definition_positions',
                                [{'line': declaration['line'], 'character': declaration['character']}])
    key = lambda p: (p['line'], p['character'])
    return (definition['file'] == declaration['file'] and point in positions and
            key(declaration.get('declaration_start', point)) <= key(point) <
            key(declaration.get('declaration_end', {'line': point['line'], 'character': point['character']+1})))


def sample_members(oracle, sut, inventory, census, sample, seed):
    declarations = census['declarations']
    supported_names = {shape: {d['name'] for d in declarations if d['shape'] == shape}
                       for shape in ('property_callable', 'getter')}
    out = {'policy': 'static definition binding; getter accesses separate from calls', 'shapes': {}}
    for shape, names in supported_names.items():
        candidates = [s for s in census['sites'] if s['name'] in names
                      and s['kind'] == ('access' if shape == 'getter' else 'call')]
        selected = random.Random(seed).sample(candidates, min(sample, len(candidates)))
        rows, tp, fp, fn = [], 0, 0, 0
        excluded = Counter()
        for site in selected:
            row = {'site': site}
            try:
                raw = oracle.raw_definitions_at(site['file'], site['line'], site['character'])
                # Preserve native columns: same-line properties are distinct targets.
                from .oracles import uri_to_rel
                row['definitions'] = [{'file': uri_to_rel(Path(d['file']).as_uri(), oracle.root),
                                       'start': d['start'], 'end': d['end']} for d in raw]
                local = [c for c in declarations if c['shape'] == shape and any(
                         definition_matches(c, d) for d in row['definitions'])]
                if not local:
                    row['outcome'] = 'nonconcrete_or_unsupported_definition'
                    excluded[row['outcome']] += 1
                else:
                    # Incoming SUT at the actual declaration includes implicit getter
                    # reads when implemented; absence is a measured recall miss.
                    answers = []
                    for declaration in local:
                        if sum(c['file'] == declaration['file'] and c['line'] == declaration['line']
                               and c['name'] == declaration['name'] for c in declarations) > 1:
                            raise SutError('seed_unaddressable: same-name properties share a source line')
                        fd = min((f for f in inventory if f.name == declaration['name']
                                  and f.location.file == declaration['file']
                                  and f.location.start_line <= declaration['line'] <= f.location.end_line),
                                 key=lambda f: f.location.end_line-f.location.start_line, default=None)
                        if fd is not None:
                            answers.extend(sut.callers(oracle.root, fd))
                    hit = any(e.call_site.file == site['file'] and
                              e.call_site.start_line <= site['line'] <= e.call_site.end_line for e in answers)
                    row['outcome'] = 'found' if hit else 'missing'
                    # This is site detection recall only. These selected-site queries
                    # cannot estimate precision of the complete SUT incoming set.
                    tp += int(hit); fn += int(not hit)
            except (OracleError, SutError) as exc:
                row['outcome'] = ('oracle_timeout' if isinstance(exc, OracleTimeout) else
                                  'sut_timeout' if isinstance(exc, SutTimeout) else
                                  'oracle_error' if isinstance(exc, OracleError) else 'sut_error')
                row['error'] = str(exc)
                excluded[row['outcome']] += 1
            rows.append(row)
        out['shapes'][shape] = {'declarations': sum(d['shape'] == shape for d in declarations),
             'candidate_sites': len(candidates), 'sampled_sites': len(selected),
             'exclusions': dict(excluded), 'rows': rows,
             'detection_recall': precision_recall(tp, fp, fn)['recall'] if tp + fn else None,
             'found': tp, 'missing': fn}
    return out
