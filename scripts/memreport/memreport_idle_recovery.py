"""Conservative exceptional persistence without depending on the row renderer."""
from memreport_idle_rows import FIELDS


def recover(meta, state, error, epoch_ns):
    text = type(error).__name__+': '+str(error)
    events, journal, samples, reasons = (state[k] for k in ('events','journal','samples','reasons'))
    reasons = sorted(set(reasons+['collection_failed']))
    demands = [x for x in journal if x['kind']=='demand']
    rows = []
    for demand in demands or [None]:
        demand_id = demand['demand_id'] if demand else None
        journal.append(dict(kind='abandoned',demand_id=demand_id,epoch_ns=epoch_ns,reason=text))
        linked = [x for x in journal if x.get('demand_id')==demand_id]
        row = {k:meta.get(k) for k in FIELDS}
        row.update(outcome='abandoned',status='EXCLUDED',boundary='idle30',
                   cycle=demand.get('cycle',1) if demand else None,
                   score_memory=False,cap_eligible=False,m_eligible=False,
                   footprint_bytes=None,rss_bytes=None,failure_text=text,
                   run_reasons=reasons,reasons=reasons,events=events,journal=linked,samples={})
        rows.append(row)
    return dict(manifest=meta,events=events,journal=journal,samples=samples,rows=rows,
                reasons=reasons,wall_secs=None)
