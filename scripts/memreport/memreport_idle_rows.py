"""Join production facts, external samples and client journal without inventing success."""
from memreport_idle import identity

UNKNOWN = None
FIELDS = ('binary_sha256 commit features corpus corpus_pin workload_hash cache_class mode arm '
          'requested_idle_secs effective_idle_secs requested_relief effective_relief pid os_start '
          'instance client_count first_call_wait_secs cycle attempt generation first_request '
          'cpg sidecar heap_delta_bytes relief_returned_bytes demand_readiness_ns build_time_ns '
          'first_served_ns failure_text last_response_epoch_ns disposal_end_epoch_ns footprint_bytes '
          'rss_bytes pair_id edges_unused resident_pairing declared_binary_sha256').split()


def number(value):
    return value if isinstance(value, (int,float)) and not isinstance(value, bool) else None


def span(end, start):
    return end-start if end is not None and start is not None and end >= start else None


def rows_for_run(meta, events, journal, samples, exclusions):
    """One row per original demand; retries stay linked and do not reset its clock."""
    demands = [x for x in journal if x['kind'] == 'demand']
    rows = []
    for demand in demands:
        row = {k: meta.get(k) for k in FIELDS}
        row.update(status='UNKNOWN', run_reasons=list(exclusions), reasons=list(exclusions), events=[], journal=[], samples={},
                   polling=[], prewarm=[], outcome='unpublished', boundary='idle30', ending_unload=False,
                   demand_readiness_ns=None, build_time_ns=None, first_served_ns=None,
                   stage_durations_ns={k:None for k in ('load_parse_ns','f_validation_ns','index_ns','bootstrap_ns')},
                   footprint_bytes=None, rss_bytes=None, heap_delta_bytes=None, sidecar=None)
        linked = [x for x in journal if x.get('demand_id') == demand['demand_id']]
        attempts = {x['attempt'] for x in linked if x.get('attempt') is not None}
        row['journal'] = linked
        row['polling'] = [x for x in linked if x['kind'] == 'poll']
        row['prewarm'] = [x for x in linked if x.get('prewarm')]
        row['first_request'] = demand.get('request_hash')
        row['cycle'] = demand.get('cycle', 1)
        row['score_memory'] = row['cycle'] == (1 if meta.get('arm')=='resident' else meta.get('score_cycle',1))
        row['changed_snapshot'] = row['cycle'] in meta.get('changed_snapshot_cycles',[])
        row['kind'] = 'reference' if meta.get('arm')=='reference' else 'reload' if row['cycle']>1 else 'initial'
        selected = [e for e in events if e.get('attempt') in attempts and
                    (meta.get('instance') is None or identity(e) == tuple(map(str,meta['instance'])))]
        row['events'] = selected
        publications = [e for e in selected if e['event'] == 'publication' and e['outcome'] == 'complete']
        publication = min(publications, key=lambda x:x['epoch_ns']) if publications else None
        served = [x for x in linked if x['kind'] == 'response' and x.get('outcome') == 'served']
        failures = [x for x in linked if x['kind'] in ('failed','abandoned')]
        warming = [x for x in linked if x.get('outcome') == 'warming']
        row['client_outcome'] = 'served' if served else 'warming' if warming else None
        if served:
            row['outcome'] = 'served' if publication else 'unpublished'
            row['first_served_ns'] = span(served[0]['epoch_ns'], demand['epoch_ns'])
            row['last_response_epoch_ns'] = served[-1]['epoch_ns']
        elif failures:
            row['outcome'] = failures[-1]['kind']
        elif warming:
            row['outcome'] = 'warming'
        if failures and (not served or failures[-1]['epoch_ns'] >= served[-1]['epoch_ns']):
            row['outcome'] = failures[-1]['kind']
        errors = [x.get('reason','') for x in failures]
        errors += [e['outcome'] for e in selected if e['outcome'].startswith('failed')]
        row['failure_text'] = '; '.join(errors) or None
        if publication:
            key = identity(publication)
            attempt, generation = publication['attempt'], publication['generation']
            same = [e for e in selected if identity(e) == key and e['attempt'] == attempt and
                    e['generation'] == generation]
            row.update(instance=list(key), pid=publication['pid'], attempt=attempt, generation=generation,
                       cpg=publication['cpg'], stage_durations_ns={k:number(publication.get(k)) for k in
                           ('load_parse_ns','f_validation_ns','index_ns','bootstrap_ns')},
                       effective_idle_secs=publication['configured_idle_secs'],
                       effective_relief=publication['effective_relief'],
                       demand_readiness_ns=span(publication['epoch_ns'],demand['epoch_ns']))
            starts = [e['epoch_ns'] for e in selected if e['event']=='build_start' and e['attempt']==attempt]
            row['build_time_ns'] = span(publication['epoch_ns'], min(starts)) if starts else None
            unloads = [e for e in same if e['event']=='unload' and e['outcome']=='complete']
            row['ending_unload'] = len(unloads) == 1
            row['sidecar'] = unloads[0]['sidecar'] if len(unloads)==1 else None
            disposed = [e for e in same if e['event']=='disposal_end' and e['outcome']=='complete']
            row['disposal_end_epoch_ns'] = disposed[0]['epoch_ns'] if len(disposed)==1 else None
            relief = [e for e in same if e['event']=='relief' and e['outcome']=='complete']
            if len(relief)==1:
                e = relief[0]
                before,after = number(e.get('heap_before')),number(e.get('heap_after'))
                row['heap_delta_bytes'] = after-before if before is not None and after is not None else None
                row['relief_returned_bytes'] = number(e['released_bytes'])
            for sample in samples:
                if sample.get('attempt') == attempt and sample.get('generation') == generation:
                    row['samples'][sample['boundary']] = dict(sample,reasons=list(sample.get('reasons',[])))
            pre = next((s for s in samples if s['boundary']=='pre_index'), None)
            if pre:
                pre = dict(pre,reasons=list(pre.get('reasons',[])))
                row['samples']['pre_index'] = pre
                if starts and pre.get('end_epoch_ns') is not None and pre['end_epoch_ns'] >= min(starts):
                    pre['reasons'].append('pre_index_after_build')
                if pre.get('end_epoch_ns') is not None and pre['end_epoch_ns'] >= demand['epoch_ns']:
                    pre['reasons'].append('pre_index_after_call')
            for boundary in ('pre_index','loaded','idle5','idle30'):
                if boundary not in row['samples']:
                    row['samples'][boundary] = dict(reasons=['missing_sample_'+boundary])
                sample = row['samples'][boundary]
                if sample.get('reasons') or sample.get('footprint_bytes') is None:
                    sample.update(status='UNKNOWN',footprint_bytes=None,rss_bytes=None)
                else:
                    sample['status'] = 'ADMITTED'
            row['pre_index_deltas'] = {}
            for field in ('footprint_bytes','rss_bytes'):
                loaded_value = row['samples'].get('loaded',{}).get(field)
                pre_value = row['samples'].get('pre_index',{}).get(field)
                row['pre_index_deltas'][field] = (loaded_value-pre_value if loaded_value is not None
                                                 and pre_value is not None else None)
            idle = row['samples'].get('idle30', {})
            row['reasons'].extend(idle.get('reasons',[]))
            row['footprint_bytes'],row['rss_bytes'] = idle.get('footprint_bytes'),idle.get('rss_bytes')
            if row['footprint_bytes'] is not None and not row['reasons']:
                row['status'] = 'ADMITTED'
        rows.append(row)
    if not rows:
        row = {k:meta.get(k) for k in FIELDS}
        row.update(outcome='abandoned', status='EXCLUDED', boundary='idle30', events=events,
                   journal=journal, samples=samples, polling=[], prewarm=[],
                   failure_text='; '.join(j.get('reason','') for j in journal if j['kind'] in
                                          ('failed','abandoned')) or 'no recorded demand',
                   reasons=list(exclusions)+['missing_demand'])
        rows.append(row)
    return rows
