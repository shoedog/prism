"""Idle readout rules in byte/ns units, independent of eager memreport scoring."""
from collections import Counter, defaultdict
from statistics import median

MIB = 1024 * 1024
PROVENANCE = ('binary_sha256', 'commit', 'features', 'corpus', 'corpus_pin', 'workload_hash',
              'cache_class', 'mode', 'arm', 'pair_id')
REQUIRED = PROVENANCE + ('requested_idle_secs', 'effective_idle_secs', 'requested_relief',
                         'effective_relief', 'instance', 'os_start', 'client_count',
                         'first_call_wait_secs', 'cycle', 'attempt', 'generation')


def admit(row):
    result = dict(row)
    reasons = list(row.get('reasons', []))
    if any(row.get(k) is None for k in REQUIRED):
        reasons.append('missing_row_field')
    if (row.get('requested_idle_secs') != row.get('effective_idle_secs') or
            row.get('requested_relief') != row.get('effective_relief')):
        reasons.append('settings_mismatch')
    if row.get('outcome') in ('failed', 'abandoned', 'warming', 'unpublished'):
        reasons.append('outcome_' + row['outcome'])
    result['reasons'] = sorted(set(reasons))
    if reasons or row.get('status') == 'EXCLUDED':
        result['status'] = 'EXCLUDED'
    elif row.get('footprint_bytes') is None or row.get('status') == 'UNKNOWN':
        result['status'] = 'UNKNOWN'
    else:
        result['status'] = 'ADMITTED'
    return result


def saving(base, candidate):
    if base['boundary'] != candidate['boundary']:
        raise ValueError('unmatched boundary')
    if any(base[k] != candidate[k] for k in ('pair_id', 'corpus', 'corpus_pin', 'workload_hash',
                                            'mode', 'binary_sha256', 'commit', 'features')):
        raise ValueError('unmatched pair')
    denominator = base['footprint_bytes']
    if denominator is None or candidate['footprint_bytes'] is None or denominator <= 0:
        return dict(bytes=None, fraction=None)
    delta = denominator - candidate['footprint_bytes']
    return dict(bytes=delta, fraction=delta / denominator,
                idle_fraction=candidate['footprint_bytes'] / denominator)


def matched_m(pairs):
    pairs = [(a,b) for a,b in pairs if admit(a)['status'] == admit(b)['status'] == 'ADMITTED'
             and relief_ok(b)]
    independent = min(len({tuple(a['instance']) for a,b in pairs}),
                      len({tuple(b['instance']) for a,b in pairs}))
    if independent < 3:
        return {'pass': False, 'reason': 'need_three_pairs', 'pairs': len(pairs)}
    deltas = [saving(a,b) for a,b in pairs]
    if any(d['bytes'] is None for d in deltas):
        return {'pass': False, 'reason': 'unknown_denominator', 'pairs': len(pairs)}
    bases = [a['footprint_bytes'] for a,b in pairs]
    candidates = [b['footprint_bytes'] for a,b in pairs]
    absolute, fraction = median(d['bytes'] for d in deltas), median(d['fraction'] for d in deltas)
    separated = max(candidates) < min(bases)
    return dict(**{'pass': separated and (absolute >= 200*MIB or fraction >= .15)},
                pairs=len(pairs), median_bytes=absolute, median_fraction=fraction,
                base_range=[min(bases),max(bases)], candidate_range=[min(candidates),max(candidates)],
                separated=separated, idle_fractions=[d['idle_fraction'] for d in deltas])


def exact_hit(reload, reference):
    equal = ('binary_sha256', 'commit', 'features', 'corpus_pin', 'workload_hash', 'mode',
             'cache_class', 'effective_idle_secs', 'effective_relief', 'first_call_wait_secs',
             'first_request')
    if any(reload.get(k) is None or reload.get(k) != reference.get(k) for k in equal):
        return False
    if not all(x.get('ending_unload') and x.get('cpg') == 'hit' for x in (reload, reference)):
        return False
    sidecars = (reload.get('sidecar'), reference.get('sidecar'))
    return sidecars == ('hit', 'hit') or (sidecars == ('not_attempted', 'not_attempted') and
                                         reload.get('edges_unused') and reference.get('edges_unused'))


def latency(reloads, references):
    by_pair = {x['pair_id']: x for x in references}
    pairs = [(x, by_pair[x['pair_id']]) for x in reloads if x['pair_id'] in by_pair and
             exact_hit(x, by_pair[x['pair_id']]) and admit(x)['status'] == 'ADMITTED' and
             admit(by_pair[x['pair_id']])['status'] == 'ADMITTED']
    known = [(a,b) for a,b in pairs if a.get('first_served_ns') is not None and
             b.get('first_served_ns') is not None and a.get('demand_readiness_ns') is not None]
    if not known:
        return {'status': 'UNKNOWN', 'readiness_target': False, 'pairs': 0}
    a, b = median(x['first_served_ns'] for x,y in known), median(y['first_served_ns'] for x,y in known)
    return dict(status='PASS' if a <= b + max(b * .05, 100000000) else 'FAIL',
                reload_median_ns=a, reference_median_ns=b, pairs=len(known),
                readiness_target=all(x['demand_readiness_ns'] <= 3000000000 for x,y in known))


def relief_ok(row):
    heap = row.get('heap_delta_bytes')
    return heap is not None and abs(heap) <= MIB


def readout(rows, corpora=('py', 'e', 'p', 'ruff')):
    rows = list(rows)  # Failed, abandoned and UNKNOWN rows are never filtered away.
    binaries = {(x.get('binary_sha256'), x.get('commit'), tuple(x.get('features') or [])) for x in rows}
    if len(binaries) > 1:
        raise ValueError('mixed binaries/features in one batch')
    rows = [admit(x) for x in rows]
    outcomes = Counter(x.get('outcome', 'unpublished') for x in rows)
    resident, groups, duplicate = {}, defaultdict(list), False
    for x in rows:
        if x.get('score_memory', True) and x.get('arm') == 'resident' and x.get('boundary') == 'idle30':
            key = (x['corpus'], x['mode'], x['pair_id'])
            if key in resident:
                duplicate = True
            resident[key] = x
    seen = set()
    for x in rows:
        if not x.get('score_memory',True) or x.get('arm') not in ('A', 'A+C') or x.get('boundary') != 'idle30':
            continue
        key = (x['corpus'], x['mode'], x['pair_id'])
        full = key + (x['arm'],)
        if full in seen:
            duplicate = True
        seen.add(full)
        if key in resident:
            groups[(x['mode'],x['arm'],x['corpus'])].append((resident[key],x))
    results = {'/'.join(k): matched_m(v) for k,v in groups.items()}
    arms = sorted({x['arm'] for x in rows if x.get('arm') in ('A','A+C')})
    memory_gate = bool(arms) and not duplicate
    for arm in arms:
        for mode in ('stdio', 'shared'):
            passes = sum(results.get('/'.join((mode,arm,c)), {}).get('pass', False) for c in corpora)
            memory_gate &= passes >= 2
            for corpus in corpora:
                candidates = [x for x in rows if (x.get('mode'),x.get('arm'),x.get('corpus'),
                                                  x.get('boundary')) == (mode,arm,corpus,'idle30')
                              and x.get('score_memory',True)]
                memory_gate &= (len(candidates) >= 3 and all(x['status'] == 'ADMITTED' and
                                 x['footprint_bytes'] <= 128*MIB for x in candidates))
    total = len(rows)
    service = Counter()
    for x in rows:
        if x.get('outcome') == 'served' and x.get('first_served_ns') is not None:
            service['served_within_wait'] += x['first_served_ns'] <= x['first_call_wait_secs']*1000000000
        warming = sum(j.get('outcome') == 'warming' for j in x.get('journal',[]))
        service['warming_responses'] += warming
        service['warming_poll_success'] += bool(warming and x.get('polling') and x.get('outcome')=='served')
    reload_latency = {}
    for mode in ('stdio','shared'):
        for arm in arms:
            for corpus in corpora:
                selected = [x for x in rows if (x.get('mode'),x.get('corpus'))==(mode,corpus)]
                reload_latency['/'.join((mode,arm,corpus))] = latency(
                    [x for x in selected if x.get('arm')==arm and x.get('kind')=='reload'],
                    [x for x in selected if x.get('arm')=='reference'])
    return dict(rows=rows, outcomes=dict(outcomes), outcome_proportions={k:v/total for k,v in outcomes.items()},
                matched=results, memory_gate=bool(memory_gate), duplicate_pairs=duplicate,
                service_counts=dict(service), reload_latency=reload_latency,
                units={'memory':'bytes','duration':'ns'}, activation='OWNER_DECISION_REQUIRED',
                report_only=['RSS','pre_index_deltas','loaded','idle5','idle_fraction','relief_footprint'])


# Row construction is separate from decision rules to keep both responsibilities bounded.
from memreport_idle_rows import rows_for_run  # noqa: E402,F401
