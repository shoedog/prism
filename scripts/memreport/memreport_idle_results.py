"""Independent relief, physical cap, arm summaries and service denominators."""
from collections import Counter, defaultdict

MIB = 1024*1024


def relief_results(rows, eligible):
    groups = defaultdict(list)
    for row in rows:
        if row.get('arm') in ('A','A+C'):
            groups[row['mode']+'/'+row['arm']].append(row)
            groups[row['arm']].append(row)
    results = {}
    for key, group in groups.items():
        details = []
        for row in group:
            value = row.get('heap_delta_bytes')
            status = 'UNKNOWN' if value is None else 'PASS' if abs(value) <= MIB else 'FAIL'
            if not eligible(row):
                status = 'EXCLUDED'
            details.append(dict(instance=row.get('instance'),cycle=row.get('cycle'),status=status,
                                heap_delta_bytes=value,relief_returned_bytes=row.get('relief_returned_bytes'),
                                reasons=row.get('run_reasons',row.get('reasons',[])) if status=='EXCLUDED' else ['missing_heap'] if value is None else []))
        counts = Counter(x['status'] for x in details)
        status = 'FAIL' if counts['FAIL'] else 'UNKNOWN' if counts['UNKNOWN'] or counts['EXCLUDED'] else 'PASS'
        results[key] = dict(status=status,counts=dict(counts),comparisons=details)
    return results


def cap_result(rows, corpora):
    missing, violations, excluded = [], [], []
    for corpus in corpora:
        group = [x for x in rows if x.get('corpus') == corpus]
        admitted = [x for x in group if x['status']=='ADMITTED']
        if not admitted:
            missing.append(corpus)
        violations.extend(x for x in admitted if x['footprint_bytes'] > 128*MIB)
        excluded.extend(x for x in group if x['status']!='ADMITTED')
    status = 'FAIL' if violations else 'UNKNOWN' if missing else 'PASS'
    return dict(status=status,missing_cells=missing,violations=violations,
                admitted=sum(x['status']=='ADMITTED' for x in rows),excluded=excluded)


def arm_results(rows, matched, arms, corpora, duplicate):
    results = {}
    for arm in arms:
        modes = {}
        for mode in ('stdio','shared'):
            passes = sum(matched.get('/'.join((mode,arm,c)),{}).get('pass',False) for c in corpora)
            unknown = sum(matched.get('/'.join((mode,arm,c)),{}).get('status','UNKNOWN')=='UNKNOWN' for c in corpora)
            m = dict(status='PASS' if passes >= 2 else 'UNKNOWN' if passes+unknown >= 2 else 'FAIL',passing_corpora=passes,
                     corpora={c:matched.get('/'.join((mode,arm,c)),{'pass':False,'reason':'not_measured'})
                              for c in corpora})
            selected = [x for x in rows if (x.get('mode'),x.get('arm'),x.get('boundary')) ==
                        (mode,arm,'idle30') and x.get('score_memory',True)]
            cap = cap_result(selected,corpora)
            status = 'FAIL' if (mode,arm) in duplicate or cap['status']=='FAIL' or m['status']=='FAIL' else (
                'PASS' if m['status']==cap['status']=='PASS' else 'UNKNOWN')
            modes[mode] = dict(status=status,M=m,cap=cap)
        status = 'FAIL' if any(x['status']=='FAIL' for x in modes.values()) else (
            'PASS' if all(x['status']=='PASS' for x in modes.values()) else 'UNKNOWN')
        results[arm] = dict(status=status,**{'pass':status=='PASS'},modes=modes)
    return results


def service_results(rows, exact_hit):
    groups = defaultdict(list)
    for row in rows:
        kind = ('changed_snapshot' if row.get('changed_snapshot') is True else
                'exact_hit' if exact_hit(row,row) else
                'other_miss' if row.get('cpg') in ('miss','partial_hit') else 'unknown'
                ) if row.get('kind')=='reload' else 'initial_or_reference'
        corpus, mode, arm = (str(row.get(k)) for k in ('corpus','mode','arm'))
        keys = ['pooled','corpus/'+corpus,'mode/'+mode,'reload/'+kind,
                '/'.join(('cell',corpus,mode,arm,kind))]
        for key in keys:
            groups[key].append(row)
    results = {}
    for key, group in groups.items():
        counts = Counter({k:0 for k in ('served_within_wait','warming','failure','abandonment',
                                        'warming_responses','warming_poll_success')})
        for x in group:
            served = x.get('first_served_ns')
            wait = x.get('first_call_wait_secs')
            counts['served_within_wait'] += bool(x.get('outcome')=='served' and served is not None and
                                                 wait is not None and served <= wait*1000000000)
            warming = sum(j.get('outcome')=='warming' for j in x.get('journal',[]))
            counts['warming'] += bool(warming or x.get('outcome')=='warming')
            counts['warming_responses'] += warming
            counts['warming_poll_success'] += bool(warming and x.get('polling') and x.get('outcome')=='served')
            counts['failure'] += x.get('outcome')=='failed'
            counts['abandonment'] += x.get('outcome')=='abandoned'
        results[key] = dict(total=len(group),counts=dict(counts),
                            proportions={k:v/len(group) for k,v in counts.items()})
    return results
